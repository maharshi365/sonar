//! napi-rs bindings for Sonar's Rust core.
//!
//! Exposes the model catalog and download manager to the Electron main process.
//! Progress is delivered through a JavaScript callback (a napi
//! `ThreadsafeFunction`) so the main process can forward it to the renderer.

mod catalog;
mod download;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;

use download::{DownloadProgress, Manager, ModelStatus};

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
    let mgr = Manager::new(PathBuf::from(models_dir)).map_err(Error::from_reason)?;
    // If another thread won the race, that's fine — ignore the returned value.
    let _ = MANAGER.set(Arc::new(mgr));
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
