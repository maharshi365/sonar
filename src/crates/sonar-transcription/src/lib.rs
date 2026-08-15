//! Speech-to-text transcription via transcribe-cpp (whisper.cpp / ggml).
//!
//! Ported and simplified from Handy's `managers::transcription`. Sonar keeps
//! only what a single-user live-dictation loop needs:
//!
//! - Load a GGUF/ggml model into a [`Session`] and hold it resident.
//! - A [`StreamRouter`] that the audio recorder feeds 16 kHz frames into.
//! - A streaming worker that decodes incrementally and reports UI text through
//!   a caller-supplied callback (committed prefix + tentative suffix).
//! - `finalize_stream` to flush the stream and return the full text.
//!
//! Streaming is transcribe-cpp only; if a model doesn't advertise streaming the
//! worker idles and `finalize_stream` returns `None` so the caller can fall
//! back to a batch [`Session::run`].

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use transcribe_cpp::{
    Backend, Feature, Model, ModelOptions, RunExtension, RunOptions, Session, StreamOptions,
    WhisperRunOptions,
};

const FINALIZE_TIMEOUT: Duration = Duration::from_secs(30);

/// User-facing inference accelerator selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accelerator {
    Auto,
    Cpu,
    Gpu,
}

impl Accelerator {
    /// Parse the native settings value.
    ///
    /// # Errors
    /// Returns an error when the value is not `auto`, `cpu`, or `gpu`.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            _ => Err(format!(
                "invalid accelerator '{value}'; expected auto, cpu, or gpu"
            )),
        }
    }

    const fn backend(self) -> Backend {
        match self {
            Self::Auto => Backend::Auto,
            Self::Cpu => Backend::Cpu,
            Self::Gpu => gpu_backend(),
        }
    }
}

#[cfg(target_os = "macos")]
const fn gpu_backend() -> Backend {
    Backend::Metal
}

#[cfg(not(target_os = "macos"))]
const fn gpu_backend() -> Backend {
    Backend::Vulkan
}

/// Model-load settings that form part of resident model identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InferenceConfig {
    pub accelerator: Accelerator,
    /// transcribe-cpp registry index. Zero asks the backend to auto-select.
    pub gpu_device: i32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            accelerator: Accelerator::Auto,
            gpu_device: 0,
        }
    }
}

/// Compute device information suitable for presentation by a host binding.
pub struct ComputeDevice {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub kind: String,
    pub memory: u64,
}

#[must_use]
pub fn list_compute_devices() -> Vec<ComputeDevice> {
    initialize_backend();
    transcribe_cpp::devices()
        .into_iter()
        .filter_map(|device| {
            let index = device.index?;
            let id = device
                .device_id
                .unwrap_or_else(|| format!("{}:{}", device.kind, device.name));
            Some(ComputeDevice {
                id,
                index,
                name: if device.description.is_empty() {
                    device.name
                } else {
                    device.description
                },
                kind: device.kind,
                memory: device.memory_total,
            })
        })
        .collect()
}

/// Resolve a persisted hardware identifier to the current backend registry.
///
/// # Errors
/// Returns an error when the selected device is no longer available.
pub fn resolve_compute_device(id: &str) -> Result<i32, String> {
    if id.is_empty() {
        return Ok(0);
    }
    let device = list_compute_devices()
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| format!("selected compute device '{id}' is unavailable"))?;
    i32::try_from(device.index).map_err(|_| "compute device index exceeds i32 range".to_owned())
}

/// Initialize the native inference backend once before loading any model.
///
/// Dynamic-backend builds require both compute backend registration and, on
/// Windows, a DLL search path rooted beside the final addon module.
pub fn initialize_backend() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        #[cfg(target_os = "windows")]
        {
            if let Some(dir) = windows_dll::own_module_dir() {
                windows_dll::add_search_dir(&dir);
            } else {
                log::warn!(
                    "could not resolve the host module directory; \
                     transcribe-cpp's backend modules may not be found"
                );
            }
        }

        transcribe_cpp::init_logging();
        match transcribe_cpp::init_backends_default() {
            Ok(()) => {
                let devices = transcribe_cpp::devices();
                log::info!(
                    "transcribe-cpp initialized with {} compute device(s): [{}]",
                    devices.len(),
                    devices
                        .iter()
                        .map(|d| format!("{} ({})", d.name, d.kind))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            Err(error) => log::warn!("failed to initialize transcribe-cpp backends: {error}"),
        }
    });
}

#[cfg(target_os = "windows")]
mod windows_dll {
    use std::ffi::c_void;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
    static MODULE_MARKER: u8 = 0;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleExW(
            dw_flags: u32,
            lp_module_name: *const c_void,
            ph_module: *mut *mut c_void,
        ) -> i32;
        fn GetModuleFileNameW(h_module: *mut c_void, lp_filename: *mut u16, n_size: u32) -> u32;
        fn SetDllDirectoryW(lp_path_name: *const u16) -> i32;
    }

    pub fn own_module_dir() -> Option<PathBuf> {
        let marker = std::ptr::from_ref(&MODULE_MARKER).cast::<c_void>();
        let mut module: *mut c_void = std::ptr::null_mut();
        let ok = unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                marker,
                &raw mut module,
            )
        };
        if ok == 0 || module.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 512];
        loop {
            let capacity = u32::try_from(buffer.len()).ok()?;
            let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), capacity) };
            if len == 0 {
                return None;
            }
            let len = usize::try_from(len).ok()?;
            if len < buffer.len().saturating_sub(1) {
                buffer.truncate(len);
                break;
            }
            buffer.resize(buffer.len().checked_mul(2)?, 0);
        }

        PathBuf::from(std::ffi::OsString::from_wide(&buffer))
            .parent()
            .map(Path::to_path_buf)
    }

    pub fn add_search_dir(dir: &Path) {
        let wide: Vec<u16> = dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            SetDllDirectoryW(wide.as_ptr());
        }
    }
}

/// A UI text snapshot forwarded to JS during streaming. `committed` is the
/// append-only, flicker-free prefix; `tentative` is the volatile suffix the
/// model may still rewrite.
#[derive(Clone, Debug)]
pub struct StreamText {
    pub committed: String,
    pub tentative: String,
}

/// Callback invoked on the streaming worker thread whenever the live text
/// changes. Keep it cheap (forward to a threadsafe function).
pub type StreamTextCallback = Arc<dyn Fn(StreamText) + Send + Sync + 'static>;

enum StreamCmd {
    Feed(Vec<f32>),
    /// Flush and reply with the final text, or `None` if no usable stream ran.
    Finalize(mpsc::Sender<Option<String>>),
    Cancel,
}

/// Routes real-time audio frames to the active streaming worker. The audio
/// recorder holds an `Arc<StreamRouter>` and calls [`StreamRouter::feed`] for
///
/// every 16 kHz frame; when no stream is open that's a single relaxed atomic
/// load.
pub struct StreamRouter {
    tx: Mutex<Option<mpsc::Sender<StreamCmd>>>,
    open: AtomicBool,
}

impl StreamRouter {
    const fn new() -> Self {
        Self {
            tx: Mutex::new(None),
            open: AtomicBool::new(false),
        }
    }

    fn open(&self) -> mpsc::Receiver<StreamCmd> {
        let (tx, rx) = mpsc::channel::<StreamCmd>();
        *lock(&self.tx) = Some(tx);
        self.open.store(true, Ordering::Relaxed);
        rx
    }

    fn take(&self) -> Option<mpsc::Sender<StreamCmd>> {
        self.open.store(false, Ordering::Relaxed);
        lock(&self.tx).take()
    }

    fn clear(&self) {
        self.open.store(false, Ordering::Relaxed);
        *lock(&self.tx) = None;
    }

    /// Forward a 16 kHz frame to the active worker. Cheap no-op when idle.
    pub fn feed(&self, frame: &[f32]) {
        if !self.open.load(Ordering::Relaxed) {
            return;
        }
        if let Some(tx) = lock(&self.tx).as_ref() {
            let _ = tx.send(StreamCmd::Feed(frame.to_vec()));
        }
    }

    /// Return whether a streaming worker currently accepts audio.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Relaxed)
    }
}

/// Owns the resident model session and manages streaming lifecycles.
pub struct TranscriptionEngine {
    /// The loaded session, taken out of the mutex while a stream worker owns it.
    session: Mutex<Option<Session>>,
    current_model: Mutex<Option<String>>,
    current_inference: Mutex<Option<InferenceConfig>>,
    router: Arc<StreamRouter>,
    /// True while a stream worker exists (so a second one can't start).
    worker_active: AtomicBool,
}

impl TranscriptionEngine {
    /// Create an engine without a loaded model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            current_model: Mutex::new(None),
            current_inference: Mutex::new(None),
            router: Arc::new(StreamRouter::new()),
            worker_active: AtomicBool::new(false),
        }
    }

    /// The shared frame router. Hand this to the audio recorder's frame
    /// callback so live audio reaches the streaming worker.
    #[must_use]
    pub fn router(&self) -> Arc<StreamRouter> {
        Arc::clone(&self.router)
    }

    /// Return whether a model is loaded or owned by an active worker.
    #[must_use]
    pub fn is_model_loaded(&self) -> bool {
        lock(&self.session).is_some() || self.worker_active.load(Ordering::Acquire)
    }

    /// Return the identifier of the resident model, if any.
    #[must_use]
    pub fn current_model_id(&self) -> Option<String> {
        lock(&self.current_model).clone()
    }

    /// Load a GGUF/ggml model, replacing any currently loaded one. Idempotent:
    /// a no-op when `model_id` is already loaded.
    ///
    /// # Errors
    ///
    /// Returns an error if the model or its inference session cannot be loaded.
    pub fn load_model(
        &self,
        model_id: &str,
        model_path: &Path,
        inference: InferenceConfig,
    ) -> Result<(), String> {
        if lock(&self.current_model).as_deref() == Some(model_id)
            && lock(&self.current_inference).as_ref() == Some(&inference)
            && lock(&self.session).is_some()
        {
            return Ok(());
        }

        if inference.gpu_device < 0 {
            return Err(
                "gpu device index must be 0 (auto) or a positive registry index".to_owned(),
            );
        }
        let backend = inference.accelerator.backend();
        if inference.accelerator == Accelerator::Gpu && !transcribe_cpp::backend_available(backend)
        {
            return Err(format!("requested GPU backend {backend:?} is unavailable"));
        }
        let model = Model::load_with(
            model_path,
            &ModelOptions {
                backend,
                gpu_device: inference.gpu_device,
            },
        )
        .map_err(|e| format!("failed to load model {model_id}: {e}"))?;
        let session = model
            .session()
            .map_err(|e| format!("failed to create session for {model_id}: {e}"))?;

        let caps = session.model().capabilities();
        log::info!(
            "Loaded model '{model_id}' (streaming={}, translate={}, langs={})",
            caps.supports_streaming,
            caps.supports_translate,
            caps.languages.len()
        );

        *lock(&self.session) = Some(session);
        *lock(&self.current_model) = Some(model_id.to_owned());
        *lock(&self.current_inference) = Some(inference);
        Ok(())
    }

    pub fn unload_model(&self) {
        *lock(&self.session) = None;
        *lock(&self.current_model) = None;
        *lock(&self.current_inference) = None;
    }

    /// Whether the loaded model advertises live streaming.
    #[must_use]
    pub fn supports_streaming(&self) -> bool {
        lock(&self.session)
            .as_ref()
            .is_some_and(|session| session.model().capabilities().supports_streaming)
    }

    /// Whether the loaded model can consume a decode-time initial prompt.
    #[must_use]
    pub fn supports_initial_prompt(&self) -> bool {
        lock(&self.session)
            .as_ref()
            .is_some_and(|session| session.model().supports(Feature::InitialPrompt))
    }

    /// Begin a streaming session. Spawns a worker that takes the session out of
    /// the mutex, opens a transcribe-cpp `Stream`, and emits text via
    /// `on_text` as audio is fed through the router. Non-blocking.
    ///
    /// If the model can't stream, the worker idles until finalize/cancel and
    /// `finalize_stream` returns `None` so the caller falls back to batch.
    pub fn start_stream(self: &Arc<Self>, on_text: StreamTextCallback) {
        if self.router.is_open() || self.worker_active.swap(true, Ordering::AcqRel) {
            log::warn!("start_stream called while a stream is already active");
            return;
        }
        let rx = self.router.open();
        let engine = Arc::clone(self);
        let _worker = thread::spawn(move || engine.run_stream_worker(&rx, &on_text));
    }

    fn run_stream_worker(&self, rx: &mpsc::Receiver<StreamCmd>, on_text: &StreamTextCallback) {
        // Ensure worker_active is always cleared, even on early return/panic.
        struct ActiveGuard<'a>(&'a AtomicBool);
        impl Drop for ActiveGuard<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = ActiveGuard(&self.worker_active);

        let model_id = self.current_model_id().unwrap_or_default();

        // Take the session out so we own it for the stream's lifetime.
        let Some(mut session) = lock(&self.session).take() else {
            log::info!("Live preview: no model loaded; falling back to batch");
            self.router.clear();
            drain_until_finalize(rx);
            return;
        };

        let supports_streaming = session.model().capabilities().supports_streaming;
        if !supports_streaming {
            log::info!("Live preview: model '{model_id}' has no streaming; using batch");
            self.return_session(session, &model_id);
            self.router.clear();
            drain_until_finalize(rx);
            return;
        }

        let run_options = RunOptions::default();

        let mut finalize_reply: Option<mpsc::Sender<Option<String>>> = None;
        let mut finalize_text: Option<Option<String>> = None;

        let stream_began = {
            let mut stream = match session.stream(&run_options, &StreamOptions::default()) {
                Ok(s) => Some(s),
                Err(e) => {
                    log::error!("Failed to begin stream: {e}");
                    None
                }
            };
            let began = stream.is_some();
            if let Some(ref mut stream) = stream {
                log::info!("Live streaming started (model '{model_id}')");
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        StreamCmd::Feed(pcm) => match stream.feed(&pcm) {
                            Ok(update) => {
                                if update.committed_changed || update.tentative_changed {
                                    let text = stream.text();
                                    on_text(StreamText {
                                        committed: text.committed,
                                        tentative: text.tentative,
                                    });
                                }
                            }
                            Err(e) => log::warn!("stream feed failed: {e}"),
                        },
                        StreamCmd::Finalize(reply) => {
                            let text = match stream.finalize() {
                                Ok(_) => Some(stream.text().full),
                                Err(e) => {
                                    log::error!("stream finalize failed: {e}");
                                    None
                                }
                            };
                            finalize_reply = Some(reply);
                            finalize_text = Some(text);
                            break;
                        }
                        StreamCmd::Cancel => {
                            stream.reset();
                            break;
                        }
                    }
                }
            }
            began
        };

        self.return_session(session, &model_id);

        if !stream_began {
            drain_until_finalize(rx);
            return;
        }

        if let (Some(reply), Some(text)) = (finalize_reply, finalize_text) {
            let _ = reply.send(text);
        }
    }

    fn return_session(&self, session: Session, expected_model_id: &str) {
        let still_current = lock(&self.current_model).as_deref() == Some(expected_model_id);
        if still_current {
            *lock(&self.session) = Some(session);
        } else {
            log::info!("Model changed during stream; dropping stale session");
        }
    }

    /// Flush the active stream and return its final text. `Ok(None)` means no
    /// usable stream ran (caller should fall back to batch).
    ///
    /// # Errors
    ///
    /// Returns an error if the streaming worker does not finalize before the
    /// timeout.
    pub fn finalize_stream(&self) -> Result<Option<String>, String> {
        let Some(tx) = self.router.take() else {
            return Ok(None);
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        if tx.send(StreamCmd::Finalize(reply_tx)).is_err() {
            return Ok(None);
        }
        match reply_rx.recv_timeout(FINALIZE_TIMEOUT) {
            Ok(text) => Ok(text),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(None),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
                "timed out waiting {FINALIZE_TIMEOUT:?} to finalize"
            )),
        }
    }

    /// Abandon any active stream without producing text.
    pub fn cancel_stream(&self) {
        if let Some(tx) = self.router.take() {
            let _ = tx.send(StreamCmd::Cancel);
        }
    }

    /// Batch transcription over a full 16 kHz mono buffer. Used as the fallback
    /// when the model doesn't support streaming.
    ///
    /// # Errors
    ///
    /// Returns an error if no model is loaded or inference fails.
    pub fn transcribe(&self, audio: &[f32], custom_words: &[String]) -> Result<String, String> {
        let prompt = custom_words
            .iter()
            .map(|word| word.trim())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        let result = {
            let mut session_guard = lock(&self.session);
            let session = session_guard
                .as_mut()
                .ok_or_else(|| "no model loaded".to_owned())?;
            let options = if !prompt.is_empty() && session.model().supports(Feature::InitialPrompt)
            {
                RunOptions {
                    family: Some(RunExtension::Whisper(WhisperRunOptions {
                        initial_prompt: Some(prompt),
                        ..Default::default()
                    })),
                    ..Default::default()
                }
            } else {
                RunOptions::default()
            };
            let result = session.run(audio, &options);
            drop(session_guard);
            result
        };
        result
            .map(|transcription| transcription.text)
            .map_err(|e| format!("transcription failed: {e}"))
    }
}

impl Default for TranscriptionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Drain the command channel until finalize/cancel so the caller's handshake
/// completes even when no stream ran.
fn drain_until_finalize(rx: &mpsc::Receiver<StreamCmd>) {
    while let Ok(cmd) = rx.recv() {
        match cmd {
            StreamCmd::Finalize(reply) => {
                let _ = reply.send(None);
                break;
            }
            StreamCmd::Cancel => break,
            StreamCmd::Feed(_) => {}
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::Accelerator;

    #[test]
    fn parses_accelerators_case_insensitively() {
        assert_eq!(Accelerator::parse("AUTO"), Ok(Accelerator::Auto));
        assert_eq!(Accelerator::parse(" cpu "), Ok(Accelerator::Cpu));
        assert_eq!(Accelerator::parse("gpu"), Ok(Accelerator::Gpu));
        assert!(Accelerator::parse("cuda").is_err());
    }
}
