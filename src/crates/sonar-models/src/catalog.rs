//! Bundled model catalog.
//!
//! The catalog is a small, curated list of speech models compiled directly into
//! the binary via `include_str!`. This means the list of available models is
//! always present with zero network access. Each entry points at a public
//! Hugging Face repo + filename that can be downloaded on demand.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// A single downloadable model as described by the bundled catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogModel {
    /// Stable identifier used everywhere (settings, IPC, filenames).
    pub id: String,
    pub name: String,
    pub description: String,
    /// Hugging Face repository id, e.g. `ggerganov/whisper.cpp`.
    pub repo_id: String,
    /// File within the repo to download, e.g. `ggml-base.bin`.
    pub filename: String,
    /// Expected download size in bytes (used for progress + display).
    pub size_bytes: u64,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub recommended: bool,
}

#[derive(Debug, Deserialize)]
struct CatalogRoot {
    #[allow(dead_code)]
    catalog_version: u32,
    models: Vec<CatalogModel>,
}

static ROOT: Lazy<CatalogRoot> = Lazy::new(|| {
    serde_json::from_str(include_str!("../catalog.json"))
        .expect("bundled catalog.json is valid and matches the schema")
});

/// All models in the bundled catalog.
pub fn all() -> &'static [CatalogModel] {
    &ROOT.models
}

/// Look up a single model by id.
pub fn find(id: &str) -> Option<&'static CatalogModel> {
    ROOT.models.iter().find(|m| m.id == id)
}

/// Build the Hugging Face download URL for a catalog model.
///
/// Uses the public `resolve/main` endpoint which serves the raw file.
pub fn download_url(model: &CatalogModel) -> String {
    format!(
        "https://huggingface.co/{}/resolve/main/{}?download=true",
        model.repo_id, model.filename
    )
}
