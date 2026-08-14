//! napi-rs bindings for Sonar's Rust core.
//!
//! Exposes the model catalog and download manager to the Electron main process.
//! Progress is delivered through a JavaScript callback (a napi
//! `ThreadsafeFunction`) so the main process can forward it to the renderer.

mod audio;
mod catalog;
mod download;
mod pipeline;
mod transcription;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;

use download::{DownloadProgress, Manager, ModelStatus};
use pipeline::{Pipeline, SessionCallbacks};

/// On-disk status of a catalog model, mirrored to TypeScript.
#[napi(object)]
pub struct JsModelStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    // napi maps u64 to JS BigInt; sizes fit comfortably in f64 so we use that
    // for ergonomic numbers on the JS side.
    pub size_bytes: f64,
    pub languages: Vec<String>,
    pub recommended: bool,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub partial_bytes: f64,
}

impl From<ModelStatus> for JsModelStatus {
    fn from(s: ModelStatus) -> Self {
        Self {
            id: s.id,
            name: s.name,
            description: s.description,
            filename: s.filename,
            size_bytes: s.size_bytes as f64,
            languages: s.languages,
            recommended: s.recommended,
            is_downloaded: s.is_downloaded,
            is_downloading: s.is_downloading,
            partial_bytes: s.partial_bytes as f64,
        }
    }
}

/// Progress payload delivered to the JS progress callback.
#[napi(object)]
pub struct JsDownloadProgress {
    pub model_id: String,
    pub downloaded: f64,
    pub total: f64,
    pub percentage: f64,
}

impl From<DownloadProgress> for JsDownloadProgress {
    fn from(p: DownloadProgress) -> Self {
        Self {
            model_id: p.model_id,
            downloaded: p.downloaded as f64,
            total: p.total as f64,
            percentage: p.percentage,
        }
    }
}

/// Initialize the transcribe-cpp native backend once, before any model load:
/// routes native/ggml diagnostics into the `log` facade and registers compute
/// backend modules. Must run before the first [`transcription::TranscriptionEngine::load_model`]
/// call — in a `dynamic-backends` build (Windows x86_64, Linux) skipping this
/// leaves zero compute backend modules registered, so `Model::load` has
/// nothing to run inference on.
///
/// On Windows this first widens the process's DLL search path to include our
/// own addon's directory (where `build.rs`'s `stage_transcribe_runtime_libs`
/// copies the dlopen'd ggml modules): transcribe-cpp otherwise resolves them
/// relative to the *main executable's* directory, which for Electron in dev
/// mode is `node_modules/electron/dist`, not this crate. Linux instead bakes
/// an `$ORIGIN` rpath into the compiled `.node` addon at link time (see
/// `build.rs`), so no runtime step is needed there. A static build (macOS
/// `metal`) makes `init_backends_default` a harmless no-op.
///
/// Ported from Handy's `init_transcribe_backend`
/// (src-tauri/src/managers/transcription.rs).
fn init_transcribe_backend() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        #[cfg(target_os = "windows")]
        {
            if let Some(dir) = windows_dll::own_module_dir() {
                windows_dll::add_search_dir(&dir);
            } else {
                log::warn!(
                    "could not resolve sonar-core's own module directory; \
                     transcribe-cpp's dlopen'd backend modules may not be found"
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
            Err(e) => log::warn!("failed to initialize transcribe-cpp backends: {e}"),
        }
    });
}

/// Minimal WinAPI shims for widening the process's DLL search path to this
/// addon's own directory. No extra crate dependency (e.g. `windows-sys`) is
/// pulled in just for these two calls.
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

    /// The directory containing *this compiled addon* (the `.node` file),
    /// found via the address of a function inside it rather than assuming
    /// any particular working directory or executable path.
    pub fn own_module_dir() -> Option<PathBuf> {
        let marker = own_module_dir as *const () as *const c_void;
        let mut h_module: *mut c_void = std::ptr::null_mut();
        let ok = unsafe {
            GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, marker, &mut h_module)
        };
        if ok == 0 || h_module.is_null() {
            return None;
        }

        let mut buf = vec![0u16; 512];
        loop {
            let len = unsafe { GetModuleFileNameW(h_module, buf.as_mut_ptr(), buf.len() as u32) };
            if len == 0 {
                return None;
            }
            if (len as usize) < buf.len() - 1 {
                buf.truncate(len as usize);
                break;
            }
            buf.resize(buf.len() * 2, 0);
        }

        PathBuf::from(std::ffi::OsString::from_wide(&buf))
            .parent()
            .map(Path::to_path_buf)
    }

    /// Add `dir` to the process-wide DLL search path used by subsequent
    /// (implicit or explicit) `LoadLibrary` calls that don't specify a full
    /// path — including transcribe-cpp's internal `dlopen` of its ggml
    /// backend modules.
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

/// Process-wide manager. Initialized once with the models directory that the
/// main process resolves from Electron's userData path.
static MANAGER: OnceLock<Arc<Manager>> = OnceLock::new();

fn manager() -> Result<Arc<Manager>> {
    MANAGER
        .get()
        .cloned()
        .ok_or_else(|| Error::from_reason("model manager not initialized; call initModels first"))
}

/// Initialize the model manager with the directory where models are stored.
///
/// Safe to call multiple times; only the first call takes effect.
#[napi]
pub fn init_models(models_dir: String) -> Result<()> {
    if MANAGER.get().is_some() {
        return Ok(());
    }

    // Must run before any model load (see `init_transcribe_backend`'s docs).
    init_transcribe_backend();

    let dir = PathBuf::from(models_dir);
    let mgr = Manager::new(dir.clone()).map_err(Error::from_reason)?;
    // If another thread won the race, that's fine — ignore the returned value.
    let _ = MANAGER.set(Arc::new(mgr));

    // Initialize the transcription pipeline against the same models directory.
    let pl = Pipeline::new();
    pl.set_models_dir(dir);
    let _ = PIPELINE.set(Arc::new(pl));
    Ok(())
}

/// Return every catalog model with its current on-disk status.
#[napi]
pub fn list_models() -> Result<Vec<JsModelStatus>> {
    let mgr = manager()?;
    Ok(mgr.list().into_iter().map(Into::into).collect())
}

/// Download a model, invoking `on_progress(progress)` as bytes arrive.
///
/// Returns a Promise that resolves when the download completes and rejects on
/// error or cancellation. `hf_token` is optional and, when provided, is used as
/// a Bearer token to authenticate with Hugging Face for faster /
/// higher-rate-limit downloads.
///
/// The `JsFunction` is turned into a thread-safe function synchronously (before
/// any await point) because `JsFunction` itself is not `Send`; the resulting
/// `ThreadsafeFunction` is `Send` and is what the async download task holds.
#[napi(ts_return_type = "Promise<void>")]
pub fn download_model(
    env: Env,
    model_id: String,
    hf_token: Option<String>,
    #[napi(ts_arg_type = "(progress: JsDownloadProgress) => void")] on_progress: JsFunction,
) -> Result<napi::JsObject> {
    let mgr = manager()?;

    let tsfn: ThreadsafeFunction<JsDownloadProgress, ErrorStrategy::Fatal> =
        on_progress.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;

    let (deferred, promise) = env.create_deferred()?;

    napi::tokio::spawn(async move {
        let callback = move |progress: DownloadProgress| {
            tsfn.call(progress.into(), ThreadsafeFunctionCallMode::NonBlocking);
        };

        match mgr.download(&model_id, hf_token, callback).await {
            Ok(()) => deferred.resolve(|_| Ok(())),
            Err(e) => deferred.reject(Error::from_reason(e)),
        }
    });

    Ok(promise)
}

/// Request cancellation of an in-flight download. Returns true if a download
/// was actually in flight. The partial download is kept for later resume.
#[napi]
pub fn cancel_download(model_id: String) -> Result<bool> {
    let mgr = manager()?;
    Ok(mgr.cancel(&model_id))
}

/// Remove a downloaded model (and any partial) from disk.
#[napi]
pub async fn remove_model(model_id: String) -> Result<()> {
    let mgr = manager()?;
    mgr.remove(&model_id).await.map_err(Error::from_reason)
}

// ---------------------------------------------------------------------------
// Live transcription pipeline
// ---------------------------------------------------------------------------

/// Process-wide transcription pipeline (microphone + streaming STT).
static PIPELINE: OnceLock<Arc<Pipeline>> = OnceLock::new();

fn pipeline() -> Result<Arc<Pipeline>> {
    PIPELINE
        .get()
        .cloned()
        .ok_or_else(|| Error::from_reason("pipeline not initialized; call initModels first"))
}

/// Live text snapshot delivered to the JS stream callback while recording.
#[napi(object)]
pub struct JsStreamText {
    /// Append-only, flicker-free prefix.
    pub committed: String,
    /// Volatile suffix the model may still rewrite.
    pub tentative: String,
}

/// Preload a model into memory without recording. `filename` is the model file
/// within the models directory (e.g. `ggml-base.bin`).
#[napi]
pub fn load_model(model_id: String, filename: String) -> Result<()> {
    let pl = pipeline()?;
    pl.load_model(&model_id, &filename)
        .map_err(Error::from_reason)
}

/// Unload the currently loaded model, freeing memory.
#[napi]
pub fn unload_model() -> Result<()> {
    pipeline()?.unload_model();
    Ok(())
}

/// Whether a recording session is currently in progress.
#[napi]
pub fn is_recording() -> Result<bool> {
    Ok(pipeline()?.is_recording())
}

/// The id of the currently loaded model, if any.
#[napi]
pub fn current_model() -> Result<Option<String>> {
    Ok(pipeline()?.current_model_id())
}

/// Start a recording + live transcription session.
///
/// `on_text` receives the live committed/tentative text as it evolves;
/// `on_level` receives 16 audio-spectrum buckets (0..1) for the dock waveform.
/// Both fire on background threads. Returns once the microphone is capturing.
#[napi]
pub fn start_transcription(
    model_id: String,
    filename: String,
    #[napi(ts_arg_type = "(text: JsStreamText) => void")] on_text: JsFunction,
    #[napi(ts_arg_type = "(levels: number[]) => void")] on_level: JsFunction,
) -> Result<()> {
    let pl = pipeline()?;

    let text_tsfn: ThreadsafeFunction<JsStreamText, ErrorStrategy::Fatal> =
        on_text.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;
    let level_tsfn: ThreadsafeFunction<Vec<f64>, ErrorStrategy::Fatal> =
        on_level.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;

    let callbacks = SessionCallbacks {
        on_text: Arc::new(move |text| {
            text_tsfn.call(
                JsStreamText {
                    committed: text.committed,
                    tentative: text.tentative,
                },
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }),
        on_level: Arc::new(move |buckets| {
            let levels: Vec<f64> = buckets.into_iter().map(f64::from).collect();
            level_tsfn.call(levels, ThreadsafeFunctionCallMode::NonBlocking);
        }),
    };

    pl.start(&model_id, &filename, callbacks)
        .map_err(Error::from_reason)
}

/// Stop recording and resolve with the final transcript.
#[napi(ts_return_type = "Promise<string>")]
pub fn stop_transcription(env: Env) -> Result<napi::JsObject> {
    let pl = pipeline()?;
    let (deferred, promise) = env.create_deferred()?;

    // stop() blocks on stream finalize / batch inference; run it off the JS
    // thread so the main process stays responsive.
    napi::tokio::task::spawn_blocking(move || match pl.stop() {
        Ok(text) => deferred.resolve(move |_| Ok(text)),
        Err(e) => deferred.reject(Error::from_reason(e)),
    });

    Ok(promise)
}

/// Cancel an in-flight recording, discarding any transcript.
#[napi]
pub fn cancel_transcription() -> Result<()> {
    pipeline()?.cancel();
    Ok(())
}
