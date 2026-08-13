//! Model download manager.
//!
//! Handles downloading catalog models from Hugging Face into a local models
//! directory, streaming progress back to the caller, resuming interrupted
//! downloads, and removing models. Downloads run on Rust's own tokio runtime;
//! progress is reported through a caller-supplied callback so the higher layer
//! (napi) can forward it to JavaScript.

use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use serde::Serialize;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::catalog::{self, CatalogModel};

/// Local, on-disk status of a catalog model.
#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub size_bytes: u64,
    pub languages: Vec<String>,
    pub recommended: bool,
    /// The full file exists on disk and matches the expected size.
    pub is_downloaded: bool,
    /// A download is currently in flight.
    pub is_downloading: bool,
    /// Bytes present in a partial (resumable) download, if any.
    pub partial_bytes: u64,
}

/// Progress event emitted during a download.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
}

/// Errors surfaced to the caller. Kept as strings so they cross the napi
/// boundary cleanly.
pub type DownloadResult<T> = Result<T, String>;

/// Tracks in-flight downloads so we can report status and cancel them.
#[derive(Default)]
struct Registry {
    /// model_id -> cancel flag. Set to true to request cancellation.
    downloading: HashMap<String, Arc<AtomicBool>>,
}

/// The download manager owns the models directory and the in-flight registry.
pub struct Manager {
    models_dir: PathBuf,
    registry: Mutex<Registry>,
}

impl Manager {
    /// Create a manager rooted at `models_dir`, creating the directory if
    /// needed.
    pub fn new(models_dir: PathBuf) -> DownloadResult<Self> {
        std::fs::create_dir_all(&models_dir)
            .map_err(|e| format!("failed to create models dir: {e}"))?;
        Ok(Self {
            models_dir,
            registry: Mutex::new(Registry::default()),
        })
    }

    fn model_path(&self, model: &CatalogModel) -> PathBuf {
        self.models_dir.join(&model.filename)
    }

    fn partial_path(&self, model: &CatalogModel) -> PathBuf {
        self.models_dir.join(format!("{}.partial", model.filename))
    }

    fn is_downloading(&self, id: &str) -> bool {
        self.registry.lock().unwrap().downloading.contains_key(id)
    }

    /// List every catalog model annotated with its on-disk status.
    pub fn list(&self) -> Vec<ModelStatus> {
        catalog::all()
            .iter()
            .map(|m| self.status_of(m))
            .collect()
    }

    fn status_of(&self, model: &CatalogModel) -> ModelStatus {
        let path = self.model_path(model);
        let partial = self.partial_path(model);

        let downloaded_len = std::fs::metadata(&path).map(|m| m.len()).ok();
        let is_downloaded = downloaded_len.is_some_and(|len| len == model.size_bytes);
        let partial_bytes = std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);

        ModelStatus {
            id: model.id.clone(),
            name: model.name.clone(),
            description: model.description.clone(),
            filename: model.filename.clone(),
            size_bytes: model.size_bytes,
            languages: model.languages.clone(),
            recommended: model.recommended,
            is_downloaded,
            is_downloading: self.is_downloading(&model.id),
            partial_bytes,
        }
    }

    /// Request cancellation of an in-flight download. Returns true if a
    /// download was actually in flight.
    pub fn cancel(&self, model_id: &str) -> bool {
        let reg = self.registry.lock().unwrap();
        if let Some(flag) = reg.downloading.get(model_id) {
            flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Remove a downloaded model (and any partial) from disk.
    pub async fn remove(&self, model_id: &str) -> DownloadResult<()> {
        let model = catalog::find(model_id)
            .ok_or_else(|| format!("unknown model: {model_id}"))?;

        let path = self.model_path(model);
        let partial = self.partial_path(model);

        if fs::try_exists(&path).await.unwrap_or(false) {
            fs::remove_file(&path)
                .await
                .map_err(|e| format!("failed to remove model: {e}"))?;
        }
        if fs::try_exists(&partial).await.unwrap_or(false) {
            let _ = fs::remove_file(&partial).await;
        }
        Ok(())
    }

    /// Download a model, reporting progress through `on_progress`.
    ///
    /// `hf_token` is optional; when present it is sent as a Bearer token which
    /// can lift Hugging Face's anonymous rate limits and speed up downloads.
    ///
    /// Resumes from a `.partial` file when one is present. On success the
    /// partial is atomically renamed to the final filename.
    pub async fn download<F>(
        &self,
        model_id: &str,
        hf_token: Option<String>,
        on_progress: F,
    ) -> DownloadResult<()>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        let model = catalog::find(model_id)
            .ok_or_else(|| format!("unknown model: {model_id}"))?;

        let final_path = self.model_path(model);
        let partial_path = self.partial_path(model);

        // Already fully downloaded — nothing to do.
        if fs::try_exists(&final_path).await.unwrap_or(false) {
            if let Ok(meta) = fs::metadata(&final_path).await {
                if meta.len() == model.size_bytes {
                    on_progress(DownloadProgress {
                        model_id: model.id.clone(),
                        downloaded: model.size_bytes,
                        total: model.size_bytes,
                        percentage: 100.0,
                    });
                    return Ok(());
                }
            }
        }

        // Register the in-flight download with a fresh cancel flag.
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut reg = self.registry.lock().unwrap();
            if reg.downloading.contains_key(model_id) {
                return Err(format!("{model_id} is already downloading"));
            }
            reg.downloading.insert(model_id.to_string(), cancel.clone());
        }

        // Ensure we always deregister, even on early return / error.
        let _guard = InFlightGuard {
            registry: &self.registry,
            model_id: model_id.to_string(),
        };

        let result = self
            .download_inner(model, &final_path, &partial_path, hf_token, &cancel, on_progress)
            .await;

        // On cancellation we keep the partial so the user can resume later.
        result
    }

    async fn download_inner<F>(
        &self,
        model: &CatalogModel,
        final_path: &Path,
        partial_path: &Path,
        hf_token: Option<String>,
        cancel: &AtomicBool,
        on_progress: F,
    ) -> DownloadResult<()>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        let url = catalog::download_url(model);
        let total = model.size_bytes;

        // Resume: how many bytes do we already have?
        let mut have: u64 = fs::metadata(partial_path).await.map(|m| m.len()).unwrap_or(0);
        if have > total {
            // Corrupt/oversized partial — start over.
            let _ = fs::remove_file(partial_path).await;
            have = 0;
        }

        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| format!("failed to build http client: {e}"))?;

        let mut request = client.get(&url);
        if have > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
        }
        if let Some(token) = hf_token.as_deref() {
            if !token.is_empty() {
                request = request.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {token}"),
                );
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("download failed with HTTP {status}"));
        }

        // If we asked to resume but the server ignored the Range header (200
        // instead of 206), restart from scratch.
        let resuming = have > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;
        if have > 0 && !resuming {
            have = 0;
        }

        let mut file: File = if resuming {
            let mut f = OpenOptions::new()
                .write(true)
                .open(partial_path)
                .await
                .map_err(|e| format!("failed to open partial: {e}"))?;
            f.seek(SeekFrom::Start(have))
                .await
                .map_err(|e| format!("failed to seek partial: {e}"))?;
            f
        } else {
            File::create(partial_path)
                .await
                .map_err(|e| format!("failed to create partial: {e}"))?
        };

        let mut downloaded = have;
        let mut stream = response.bytes_stream();
        let mut last_emit = std::time::Instant::now();

        // Emit an initial progress event so the UI shows resume position.
        on_progress(DownloadProgress {
            model_id: model.id.clone(),
            downloaded,
            total,
            percentage: pct(downloaded, total),
        });

        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::SeqCst) {
                file.flush().await.ok();
                return Err("download cancelled".to_string());
            }

            let chunk = chunk.map_err(|e| format!("stream error: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("write error: {e}"))?;
            downloaded += chunk.len() as u64;

            // Throttle progress emission to ~10/sec, but always emit the last.
            if last_emit.elapsed().as_millis() >= 100 {
                last_emit = std::time::Instant::now();
                on_progress(DownloadProgress {
                    model_id: model.id.clone(),
                    downloaded,
                    total,
                    percentage: pct(downloaded, total),
                });
            }
        }

        file.flush().await.map_err(|e| format!("flush error: {e}"))?;
        drop(file);

        // Final progress event.
        on_progress(DownloadProgress {
            model_id: model.id.clone(),
            downloaded,
            total,
            percentage: 100.0,
        });

        // Atomically promote the partial to the final file.
        fs::rename(partial_path, final_path)
            .await
            .map_err(|e| format!("failed to finalize download: {e}"))?;

        Ok(())
    }
}

fn pct(downloaded: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (downloaded as f64 / total as f64) * 100.0
    }
}

/// Deregisters an in-flight download when dropped (success, error, or panic).
struct InFlightGuard<'a> {
    registry: &'a Mutex<Registry>,
    model_id: String,
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut reg) = self.registry.lock() {
            reg.downloading.remove(&self.model_id);
        }
    }
}
