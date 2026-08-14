//! napi-rs bindings for Sonar's Rust core.
//!
//! Exposes the model catalog and download manager to the Electron main process.
//! Progress is delivered through a JavaScript callback (a napi
//! `ThreadsafeFunction`) so the main process can forward it to the renderer.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use num_traits::ToPrimitive;

use sonar_dictation::{Pipeline, SessionCallbacks};
use sonar_models::{DownloadProgress, Manager, ModelStatus};

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
    pub supports_streaming: bool,
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
            size_bytes: s.size_bytes.to_f64().unwrap_or(f64::MAX),
            languages: s.languages,
            supports_streaming: s.supports_streaming,
            recommended: s.recommended,
            is_downloaded: s.is_downloaded,
            is_downloading: s.is_downloading,
            partial_bytes: s.partial_bytes.to_f64().unwrap_or(f64::MAX),
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
            downloaded: p.downloaded.to_f64().unwrap_or(f64::MAX),
            total: p.total.to_f64().unwrap_or(f64::MAX),
            percentage: p.percentage,
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
///
/// # Errors
///
/// Returns an error if the models directory or bundled catalog is invalid.
#[napi]
pub fn init_models(models_dir: String) -> Result<()> {
    if MANAGER.get().is_some() {
        return Ok(());
    }

    // Must run before any model load.
    sonar_transcription::initialize_backend();

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
///
/// # Errors
///
/// Returns an error if the model manager has not been initialized.
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
///
/// # Errors
///
/// Returns an error if the manager is uninitialized or the JavaScript promise
/// and progress callback cannot be created.
#[allow(clippy::needless_pass_by_value)] // N-API owns JavaScript function arguments.
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
///
/// # Errors
///
/// Returns an error if the model manager has not been initialized.
#[napi]
pub fn cancel_download(model_id: String) -> Result<bool> {
    let mgr = manager()?;
    let model_id = model_id.into_boxed_str();
    Ok(mgr.cancel(&model_id))
}

/// Remove a downloaded model (and any partial) from disk.
///
/// # Errors
///
/// Returns an error if the manager is uninitialized, the model is unknown, or
/// its files cannot be removed.
#[allow(clippy::trailing_empty_array)]
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
///
/// # Errors
///
/// Returns an error if the pipeline is uninitialized or the model cannot load.
#[napi]
pub fn load_model(model_id: String, filename: String) -> Result<()> {
    let pl = pipeline()?;
    let model_id = model_id.into_boxed_str();
    let filename = filename.into_boxed_str();
    pl.load_model(&model_id, &filename)
        .map_err(Error::from_reason)
}

/// Unload the currently loaded model, freeing memory.
///
/// # Errors
///
/// Returns an error if the pipeline has not been initialized.
#[napi]
pub fn unload_model() -> Result<()> {
    pipeline()?.unload_model();
    Ok(())
}

/// Whether a recording session is currently in progress.
///
/// # Errors
///
/// Returns an error if the pipeline has not been initialized.
#[napi]
pub fn is_recording() -> Result<bool> {
    Ok(pipeline()?.is_recording())
}

/// The id of the currently loaded model, if any.
///
/// # Errors
///
/// Returns an error if the pipeline has not been initialized.
#[napi]
pub fn current_model() -> Result<Option<String>> {
    Ok(pipeline()?.current_model_id())
}

/// Start a recording + live transcription session.
///
/// `on_text` receives the live committed/tentative text as it evolves;
/// `on_level` receives 16 audio-spectrum buckets (0..1) for the dock waveform.
/// Both fire on background threads. Returns once the microphone is capturing.
///
/// # Errors
///
/// Returns an error if callbacks cannot be created or model/microphone startup
/// fails.
#[allow(clippy::needless_pass_by_value)] // N-API owns JavaScript function arguments.
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

    let model_id = model_id.into_boxed_str();
    let filename = filename.into_boxed_str();
    pl.start(&model_id, &filename, &callbacks)
        .map_err(Error::from_reason)
}

/// Stop recording and resolve with the final transcript.
///
/// # Errors
///
/// Returns an error if the pipeline is uninitialized or the promise cannot be
/// created.
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
///
/// # Errors
///
/// Returns an error if the pipeline has not been initialized.
#[napi]
pub fn cancel_transcription() -> Result<()> {
    pipeline()?.cancel();
    Ok(())
}

/// Insert text into the currently focused application.
///
/// # Errors
///
/// Returns an error if clipboard publication or input injection fails.
#[allow(clippy::needless_pass_by_value)] // N-API owns JavaScript string arguments.
#[napi]
pub fn insert_text(text: String) -> Result<()> {
    sonar_input::insert_text(&text).map_err(Error::from_reason)
}
