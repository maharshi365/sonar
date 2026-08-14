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

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::audio::AudioRecorder;
use crate::transcription::{StreamText, TranscriptionEngine};

/// Callbacks the host (napi layer) supplies for a recording session.
#[derive(Clone)]
pub struct SessionCallbacks {
    pub on_text: Arc<dyn Fn(StreamText) + Send + Sync + 'static>,
    pub on_level: Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>,
}

pub struct Pipeline {
    engine: Arc<TranscriptionEngine>,
    recorder: Mutex<Option<AudioRecorder>>,
    recording: Mutex<bool>,
    models_dir: Mutex<Option<PathBuf>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(TranscriptionEngine::new()),
            recorder: Mutex::new(None),
            recording: Mutex::new(false),
            models_dir: Mutex::new(None),
        }
    }

    /// Set the directory model files live in. Required before `start`.
    pub fn set_models_dir(&self, dir: PathBuf) {
        *self.models_dir.lock().unwrap() = Some(dir);
    }

    pub fn is_recording(&self) -> bool {
        *self.recording.lock().unwrap()
    }

    pub fn current_model_id(&self) -> Option<String> {
        self.engine.current_model_id()
    }

    /// Preload a model without recording (e.g. when the user picks one).
    pub fn load_model(&self, model_id: &str, filename: &str) -> Result<(), String> {
        let path = self.model_path(filename)?;
        self.engine.load_model(model_id, &path)
    }

    pub fn unload_model(&self) {
        self.engine.unload_model();
    }

    fn model_path(&self, filename: &str) -> Result<PathBuf, String> {
        let dir = self
            .models_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "models directory not set".to_string())?;
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
    pub fn start(
        &self,
        model_id: &str,
        filename: &str,
        callbacks: SessionCallbacks,
    ) -> Result<(), String> {
        {
            let mut recording = self.recording.lock().unwrap();
            if *recording {
                return Err("already recording".to_string());
            }
            *recording = true;
        }

        // On any early error, clear the recording flag.
        let result = self.start_inner(model_id, filename, callbacks);
        if result.is_err() {
            *self.recording.lock().unwrap() = false;
        }
        result
    }

    fn start_inner(
        &self,
        model_id: &str,
        filename: &str,
        callbacks: SessionCallbacks,
    ) -> Result<(), String> {
        let path = self.model_path(filename)?;
        self.engine.load_model(model_id, &path)?;

        // Begin the streaming worker before opening the mic so early frames
        // aren't dropped. Frames sent before the stream begins queue harmlessly.
        let on_text = callbacks.on_text.clone();
        TranscriptionEngine::start_stream(&self.engine, on_text);

        let router = self.engine.router();
        let on_level = callbacks.on_level.clone();

        let mut recorder = AudioRecorder::new()
            .map_err(|e| format!("failed to create recorder: {e}"))?
            .with_level_callback(move |buckets| on_level(buckets))
            .with_audio_callback(move |frame| router.feed(frame));

        recorder
            .open(None)
            .map_err(|e| format!("failed to open microphone: {e}"))?;
        let ready = recorder
            .start()
            .map_err(|e| format!("failed to start recording: {e}"))?;
        // Wait until the first mic sample flows so the caller knows capture began.
        let _ = ready.recv();

        *self.recorder.lock().unwrap() = Some(recorder);
        Ok(())
    }

    /// Stop recording and return the final transcript.
    ///
    /// Finalizes the live stream; if the model couldn't stream, runs a batch
    /// pass over the captured audio instead.
    pub fn stop(&self) -> Result<String, String> {
        {
            let mut recording = self.recording.lock().unwrap();
            if !*recording {
                return Err("not recording".to_string());
            }
            *recording = false;
        }

        let recorder = self.recorder.lock().unwrap().take();
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
        match self.engine.finalize_stream()? {
            Some(text) => Ok(text),
            None => {
                if samples.is_empty() {
                    return Ok(String::new());
                }
                self.engine.transcribe(samples)
            }
        }
    }

    /// Cancel an in-flight recording, discarding any transcript.
    pub fn cancel(&self) {
        *self.recording.lock().unwrap() = false;
        if let Some(mut rec) = self.recorder.lock().unwrap().take() {
            let _ = rec.stop();
            let _ = rec.close();
        }
        self.engine.cancel_stream();
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
