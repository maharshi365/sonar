//! The live transcription pipeline.
//!
//! Wires the microphone [`AudioRecorder`] to the [`TranscriptionEngine`]:
//!
//! 1. `start` loads the selected model (if needed), opens the mic, begins a
//!    streaming session, and routes each 16 kHz frame from the recorder into
//!    the engine's [`StreamRouter`]. Live text is pushed to `on_text`; audio
//!    levels (16 spectrum buckets) are pushed to `on_level`.
//! 2. `stop` stops the recorder, finalizes the stream (or runs a batch pass if
//!    the model can't stream), and returns the final transcript.
//!
//! One pipeline instance lives for the whole process (see the napi layer).

mod text;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sonar_audio::{input_device_by_id, AudioRecorder};
use sonar_transcription::{InferenceConfig, StreamText, TranscriptionEngine};

/// Settings fixed for the lifetime of one recording session.
#[derive(Clone, Default)]
pub struct SessionConfig {
    pub input_device_id: Option<String>,
    pub custom_words: Vec<String>,
    pub filler_word_removal: bool,
    pub custom_filler_words: Vec<String>,
    pub word_correction_threshold: f64,
    pub inference: InferenceConfig,
}

/// Callbacks the host (napi layer) supplies for a recording session.
#[derive(Clone)]
pub struct SessionCallbacks {
    pub on_text: Arc<dyn Fn(StreamText) + Send + Sync + 'static>,
    pub on_level: AudioLevelCallback,
}

/// Callback invoked with the current audio spectrum levels.
pub type AudioLevelCallback = Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>;

/// Coordinates microphone capture with the resident transcription engine.
pub struct Pipeline {
    engine: Arc<TranscriptionEngine>,
    recorder: Mutex<Option<AudioRecorder>>,
    recording: Mutex<bool>,
    models_dir: Mutex<Option<PathBuf>>,
    session_config: Mutex<Option<SessionConfig>>,
}

impl Pipeline {
    /// Create an idle dictation pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Arc::new(TranscriptionEngine::new()),
            recorder: Mutex::new(None),
            recording: Mutex::new(false),
            models_dir: Mutex::new(None),
            session_config: Mutex::new(None),
        }
    }

    /// Set the directory model files live in. Required before `start`.
    pub fn set_models_dir(&self, dir: PathBuf) {
        *lock(&self.models_dir) = Some(dir);
    }

    /// Return whether the pipeline is currently recording.
    #[must_use]
    pub fn is_recording(&self) -> bool {
        *lock(&self.recording)
    }

    /// Return the identifier of the resident model, if any.
    #[must_use]
    pub fn current_model_id(&self) -> Option<String> {
        self.engine.current_model_id()
    }

    /// Preload a model without recording (e.g. when the user picks one).
    ///
    /// # Errors
    ///
    /// Returns an error if the models directory is unset, the model file is
    /// missing, or the transcription engine cannot load the model.
    pub fn load_model(&self, model_id: &str, filename: &str) -> Result<(), String> {
        let path = self.model_path(filename)?;
        self.engine
            .load_model(model_id, &path, InferenceConfig::default())
    }

    pub fn unload_model(&self) {
        self.engine.unload_model();
    }

    fn model_path(&self, filename: &str) -> Result<PathBuf, String> {
        let dir = lock(&self.models_dir)
            .clone()
            .ok_or_else(|| "models directory not set".to_owned())?;
        let path = dir.join(filename);
        if !path.exists() {
            return Err(format!("model file not found: {}", path.display()));
        }
        Ok(path)
    }

    /// Begin a recording + live transcription session.
    ///
    /// Loads `model_id` (file `filename`) if not already resident, opens the
    /// microphone, and starts streaming. Live text and audio levels flow to the
    /// provided callbacks until [`Pipeline::stop`] is called.
    ///
    /// # Errors
    ///
    /// Returns an error when a recording is already active or when model
    /// loading or microphone startup fails.
    pub fn start(
        &self,
        model_id: &str,
        filename: &str,
        config: &SessionConfig,
        callbacks: &SessionCallbacks,
    ) -> Result<(), String> {
        {
            let mut recording = lock(&self.recording);
            if *recording {
                return Err("already recording".to_owned());
            }
            *recording = true;
        }

        // On any early error, clear the recording flag.
        let result = self.start_inner(model_id, filename, config, callbacks);
        if result.is_err() {
            *lock(&self.recording) = false;
        }
        result
    }

    fn start_inner(
        &self,
        model_id: &str,
        filename: &str,
        config: &SessionConfig,
        callbacks: &SessionCallbacks,
    ) -> Result<(), String> {
        let path = self.model_path(filename)?;
        self.engine.load_model(model_id, &path, config.inference)?;

        let router = self.engine.router();
        let on_level = callbacks.on_level.clone();

        let mut recorder = AudioRecorder::new()
            .map_err(|e| format!("failed to create recorder: {e}"))?
            .with_level_callback(move |buckets| on_level(buckets))
            .with_audio_callback(move |frame| router.feed(frame));

        let device = config
            .input_device_id
            .as_deref()
            .map(input_device_by_id)
            .transpose()
            .map_err(|error| format!("failed to select microphone: {error}"))?;
        recorder
            .open(device)
            .map_err(|e| format!("failed to open microphone: {e}"))?;

        // Opening builds the cpal stream but capture does not begin until
        // `start`, so the transcription worker can safely start here without
        // leaking a worker when device selection/opening fails.
        let on_text = callbacks.on_text.clone();
        TranscriptionEngine::start_stream(&self.engine, on_text);
        let ready = recorder
            .start()
            .map_err(|e| format!("failed to start recording: {e}"))?;
        // Wait until the first mic sample flows so the caller knows capture began.
        let _ = ready.recv();

        *lock(&self.recorder) = Some(recorder);
        *lock(&self.session_config) = Some(config.clone());
        Ok(())
    }

    /// Stop recording and return the final transcript.
    ///
    /// Finalizes the live stream; if the model couldn't stream, runs a batch
    /// pass over the captured audio instead.
    ///
    /// # Errors
    ///
    /// Returns an error if no recording is active, capture cannot stop, stream
    /// finalization fails, or fallback batch transcription fails.
    pub fn stop(&self) -> Result<String, String> {
        {
            let mut recording = lock(&self.recording);
            if !*recording {
                return Err("not recording".to_owned());
            }
            *recording = false;
        }

        let recorder = lock(&self.recorder).take();
        let samples = match recorder {
            Some(mut rec) => {
                let samples = rec
                    .stop()
                    .map_err(|e| format!("failed to stop recording: {e}"))?;
                let _ = rec.close();
                samples
            }
            None => Vec::new(),
        };

        // Prefer the streamed result; fall back to batch when no stream ran.
        let config = lock(&self.session_config).take().unwrap_or_default();
        let streamed = self.engine.finalize_stream()?;
        let should_prompt_batch = !config.custom_words.is_empty()
            && self.engine.supports_initial_prompt()
            && !samples.is_empty();
        let raw = if should_prompt_batch {
            self.engine.transcribe(&samples, &config.custom_words)?
        } else {
            streamed.map_or_else(
                || {
                    if samples.is_empty() {
                        Ok(String::new())
                    } else {
                        self.engine.transcribe(&samples, &config.custom_words)
                    }
                },
                Ok,
            )?
        };
        Ok(text::process(
            &raw,
            &config.custom_words,
            config.word_correction_threshold,
            config.filler_word_removal,
            &config.custom_filler_words,
        ))
    }

    /// Cancel an in-flight recording, discarding any transcript.
    pub fn cancel(&self) {
        *lock(&self.recording) = false;
        let recorder = lock(&self.recorder).take();
        if let Some(mut rec) = recorder {
            let _ = rec.stop();
            let _ = rec.close();
        }
        self.engine.cancel_stream();
        *lock(&self.session_config) = None;
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
