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
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use transcribe_cpp::{Model, RunOptions, Session, StreamOptions};

const FINALIZE_TIMEOUT: Duration = Duration::from_secs(30);

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

    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x00000004;

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
        let marker = own_module_dir as *const () as *const c_void;
        let mut module: *mut c_void = std::ptr::null_mut();
        let ok = unsafe {
            GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, marker, &mut module)
        };
        if ok == 0 || module.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 512];
        loop {
            let len =
                unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) };
            if len == 0 {
                return None;
            }
            if (len as usize) < buffer.len() - 1 {
                buffer.truncate(len as usize);
                break;
            }
            buffer.resize(buffer.len() * 2, 0);
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
/// every 16 kHz frame; when no stream is open that's a single relaxed atomic
/// load.
pub struct StreamRouter {
    tx: Mutex<Option<mpsc::Sender<StreamCmd>>>,
    open: AtomicBool,
}

impl StreamRouter {
    fn new() -> Self {
        Self {
            tx: Mutex::new(None),
            open: AtomicBool::new(false),
        }
    }

    fn open(&self) -> mpsc::Receiver<StreamCmd> {
        let (tx, rx) = mpsc::channel::<StreamCmd>();
        *self.tx.lock().unwrap() = Some(tx);
        self.open.store(true, Ordering::Relaxed);
        rx
    }

    fn take(&self) -> Option<mpsc::Sender<StreamCmd>> {
        self.open.store(false, Ordering::Relaxed);
        self.tx.lock().unwrap().take()
    }

    fn clear(&self) {
        self.open.store(false, Ordering::Relaxed);
        *self.tx.lock().unwrap() = None;
    }

    /// Forward a 16 kHz frame to the active worker. Cheap no-op when idle.
    pub fn feed(&self, frame: &[f32]) {
        if !self.open.load(Ordering::Relaxed) {
            return;
        }
        if let Some(tx) = self.tx.lock().unwrap().as_ref() {
            let _ = tx.send(StreamCmd::Feed(frame.to_vec()));
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Relaxed)
    }
}

/// Owns the resident model session and manages streaming lifecycles.
#[allow(dead_code)]
pub struct TranscriptionEngine {
    /// The loaded session, taken out of the mutex while a stream worker owns it.
    session: Mutex<Option<Session>>,
    current_model: Mutex<Option<String>>,
    router: Arc<StreamRouter>,
    /// True while a stream worker exists (so a second one can't start).
    worker_active: AtomicBool,
}

impl TranscriptionEngine {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            current_model: Mutex::new(None),
            router: Arc::new(StreamRouter::new()),
            worker_active: AtomicBool::new(false),
        }
    }

    /// The shared frame router. Hand this to the audio recorder's frame
    /// callback so live audio reaches the streaming worker.
    pub fn router(&self) -> Arc<StreamRouter> {
        Arc::clone(&self.router)
    }

    pub fn is_model_loaded(&self) -> bool {
        self.session.lock().unwrap().is_some() || self.worker_active.load(Ordering::Acquire)
    }

    pub fn current_model_id(&self) -> Option<String> {
        self.current_model.lock().unwrap().clone()
    }

    /// Load a GGUF/ggml model, replacing any currently loaded one. Idempotent:
    /// a no-op when `model_id` is already loaded.
    pub fn load_model(&self, model_id: &str, model_path: &Path) -> Result<(), String> {
        if self.current_model.lock().unwrap().as_deref() == Some(model_id)
            && self.session.lock().unwrap().is_some()
        {
            return Ok(());
        }

        let model =
            Model::load(model_path).map_err(|e| format!("failed to load model {model_id}: {e}"))?;
        let session = model
            .session()
            .map_err(|e| format!("failed to create session for {model_id}: {e}"))?;

        let caps = session.model().capabilities();
        log::info!(
            "Loaded model '{}' (streaming={}, translate={}, langs={})",
            model_id,
            caps.supports_streaming,
            caps.supports_translate,
            caps.languages.len()
        );

        *self.session.lock().unwrap() = Some(session);
        *self.current_model.lock().unwrap() = Some(model_id.to_string());
        Ok(())
    }

    pub fn unload_model(&self) {
        *self.session.lock().unwrap() = None;
        *self.current_model.lock().unwrap() = None;
    }

    /// Whether the loaded model advertises live streaming.
    pub fn supports_streaming(&self) -> bool {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.model().capabilities().supports_streaming)
            .unwrap_or(false)
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
        thread::spawn(move || engine.run_stream_worker(rx, on_text));
    }

    fn run_stream_worker(&self, rx: mpsc::Receiver<StreamCmd>, on_text: StreamTextCallback) {
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
        let mut session = match self.session.lock().unwrap().take() {
            Some(s) => s,
            None => {
                log::info!("Live preview: no model loaded; falling back to batch");
                self.router.clear();
                drain_until_finalize(rx);
                return;
            }
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
        let still_current =
            self.current_model.lock().unwrap().as_deref() == Some(expected_model_id);
        if still_current {
            *self.session.lock().unwrap() = Some(session);
        } else {
            log::info!("Model changed during stream; dropping stale session");
        }
    }

    /// Flush the active stream and return its final text. `Ok(None)` means no
    /// usable stream ran (caller should fall back to batch).
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
    pub fn transcribe(&self, audio: Vec<f32>) -> Result<String, String> {
        let mut guard = self.session.lock().unwrap();
        let session = guard
            .as_mut()
            .ok_or_else(|| "no model loaded".to_string())?;
        let result = session
            .run(&audio, &RunOptions::default())
            .map_err(|e| format!("transcription failed: {e}"))?;
        Ok(result.text)
    }
}

impl Default for TranscriptionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Drain the command channel until finalize/cancel so the caller's handshake
/// completes even when no stream ran.
fn drain_until_finalize(rx: mpsc::Receiver<StreamCmd>) {
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
