use std::{
    path::PathBuf,
    sync::{mpsc::Sender, Arc},
    thread,
    time::Duration,
};

use sonar_dictation::{Pipeline, SessionCallbacks, SessionConfig};
use sonar_models::{DownloadProgress, Manager, ModelStatus};
use sonar_transcription::{resolve_compute_device, InferenceConfig, StreamText};

use crate::settings::{Accelerator, AutoSubmitKey, OutputMethod, Settings};

#[derive(Clone, Debug)]
pub enum ServiceEvent {
    RecordingStarted {
        model_id: String,
    },
    StreamText {
        committed: String,
        tentative: String,
    },
    Levels(Vec<f32>),
    RecordingFinished {
        model_id: String,
        result: Result<String, String>,
        delivery_error: Option<String>,
    },
    RecordingCancelled,
    DownloadProgress(DownloadProgress),
    ModelsChanged,
    Error(String),
}

pub struct SonarService {
    pipeline: Arc<Pipeline>,
    models: Arc<Manager>,
    runtime: Arc<tokio::runtime::Runtime>,
    events: Sender<ServiceEvent>,
}

impl SonarService {
    pub fn new(models_dir: PathBuf, events: Sender<ServiceEvent>) -> Result<Arc<Self>, String> {
        sonar_transcription::initialize_backend();
        let models = Arc::new(Manager::new(models_dir.clone())?);
        let pipeline = Arc::new(Pipeline::new());
        pipeline.set_models_dir(models_dir);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("sonar-async")
            .build()
            .map_err(|error| format!("failed to start async runtime: {error}"))?;
        Ok(Arc::new(Self {
            pipeline,
            models,
            runtime: Arc::new(runtime),
            events,
        }))
    }

    pub fn models(&self) -> Vec<ModelStatus> {
        self.models.list()
    }

    pub fn start(self: &Arc<Self>, settings: Settings) {
        if self.pipeline.is_recording() {
            return;
        }
        let Some(model) = selected_model(&self.models.list(), &settings) else {
            let _ = self.events.send(ServiceEvent::Error(
                "No speech model installed. Download one from the Models page.".to_owned(),
            ));
            return;
        };
        let pipeline = Arc::clone(&self.pipeline);
        let events = self.events.clone();
        thread::Builder::new()
            .name("sonar-record-start".to_owned())
            .spawn(move || {
                let inference = match inference_config(&settings) {
                    Ok(config) => config,
                    Err(error) => {
                        let _ = events.send(ServiceEvent::Error(error));
                        return;
                    }
                };
                let config = SessionConfig {
                    input_device_id: non_empty(&settings.audio.input_device_id),
                    custom_words: settings.transcription.custom_words,
                    filler_word_removal: settings.transcription.filler_word_removal,
                    custom_filler_words: settings.transcription.custom_filler_words,
                    word_correction_threshold: settings.transcription.word_correction_threshold,
                    inference,
                };
                let text_events = events.clone();
                let level_events = events.clone();
                let callbacks = SessionCallbacks {
                    on_text: Arc::new(move |text: StreamText| {
                        let _ = text_events.send(ServiceEvent::StreamText {
                            committed: text.committed,
                            tentative: text.tentative,
                        });
                    }),
                    on_level: Arc::new(move |levels| {
                        let _ = level_events.send(ServiceEvent::Levels(levels));
                    }),
                };
                match pipeline.start(&model.id, &model.filename, &config, &callbacks) {
                    Ok(()) => {
                        let _ = events.send(ServiceEvent::RecordingStarted { model_id: model.id });
                    }
                    Err(error) => {
                        pipeline.cancel();
                        let _ = events.send(ServiceEvent::Error(error));
                    }
                }
            })
            .ok();
    }

    pub fn stop(self: &Arc<Self>, model_id: String, settings: Settings) {
        let pipeline = Arc::clone(&self.pipeline);
        let events = self.events.clone();
        thread::Builder::new()
            .name("sonar-record-stop".to_owned())
            .spawn(move || {
                let buffer = settings.transcription.extra_recording_buffer_ms.max(0) as u64;
                if buffer > 0 {
                    thread::sleep(Duration::from_millis(buffer));
                }
                let result = pipeline.stop();
                let delivery_error = result
                    .as_ref()
                    .ok()
                    .filter(|text| !text.trim().is_empty())
                    .and_then(|text| deliver_text(text, &settings).err());
                let _ = events.send(ServiceEvent::RecordingFinished {
                    model_id,
                    result,
                    delivery_error,
                });
            })
            .ok();
    }

    pub fn cancel(&self) {
        self.pipeline.cancel();
        let _ = self.events.send(ServiceEvent::RecordingCancelled);
    }

    pub fn unload_model(&self) {
        self.pipeline.unload_model();
    }

    pub fn download(self: &Arc<Self>, model_id: String, token: Option<String>) {
        let manager = Arc::clone(&self.models);
        let events = self.events.clone();
        self.runtime.spawn(async move {
            let progress_events = events.clone();
            let result = manager
                .download(&model_id, token, move |progress| {
                    let _ = progress_events.send(ServiceEvent::DownloadProgress(progress));
                })
                .await;
            match result {
                Ok(()) => {
                    let _ = events.send(ServiceEvent::ModelsChanged);
                }
                Err(error) if error.contains("cancelled") => {
                    let _ = events.send(ServiceEvent::ModelsChanged);
                }
                Err(error) => {
                    let _ = events.send(ServiceEvent::Error(error));
                    let _ = events.send(ServiceEvent::ModelsChanged);
                }
            }
        });
    }

    pub fn cancel_download(&self, model_id: &str) {
        self.models.cancel(model_id);
    }

    pub fn remove(self: &Arc<Self>, model_id: String) {
        let manager = Arc::clone(&self.models);
        let events = self.events.clone();
        self.runtime.spawn(async move {
            if let Err(error) = manager.remove(&model_id).await {
                let _ = events.send(ServiceEvent::Error(error));
            }
            let _ = events.send(ServiceEvent::ModelsChanged);
        });
    }
}

fn selected_model(models: &[ModelStatus], settings: &Settings) -> Option<ModelStatus> {
    models
        .iter()
        .find(|model| model.is_downloaded && model.id == settings.general.tts_model)
        .or_else(|| models.iter().find(|model| model.is_downloaded))
        .cloned()
}

fn inference_config(settings: &Settings) -> Result<InferenceConfig, String> {
    let accelerator = match settings.inference.accelerator {
        Accelerator::Auto => sonar_transcription::Accelerator::Auto,
        Accelerator::Cpu => sonar_transcription::Accelerator::Cpu,
        Accelerator::Gpu => sonar_transcription::Accelerator::Gpu,
    };
    Ok(InferenceConfig {
        accelerator,
        gpu_device: resolve_compute_device(&settings.inference.gpu_device_id)?,
    })
}

fn deliver_text(text: &str, settings: &Settings) -> Result<(), String> {
    if settings.output.method == OutputMethod::None {
        return Ok(());
    }
    let output = if settings.output.append_trailing_space {
        format!("{text} ")
    } else {
        text.to_owned()
    };
    match settings.output.method {
        OutputMethod::Paste => {
            sonar_input::insert_text(&output)?;
            if settings.output.auto_submit {
                thread::sleep(Duration::from_millis(100));
                let key = match settings.output.auto_submit_key {
                    AutoSubmitKey::Enter => "enter",
                    AutoSubmitKey::ControlEnter => "ctrl_enter",
                    AutoSubmitKey::CommandEnter => "cmd_enter",
                };
                sonar_input::submit(key)?;
            }
        }
        OutputMethod::Clipboard => {
            arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.set_text(output))
                .map_err(|error| format!("failed to copy transcription: {error}"))?;
        }
        OutputMethod::None => {}
    }
    Ok(())
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}
