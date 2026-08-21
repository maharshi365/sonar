//! Embedded [`AssetSource`] for Sonar, modeled on gpui's `examples/svg/svg.rs`.
//!
//! gpui resolves `svg().path(...)` and `img(...)` lookups through the asset
//! source registered with [`gpui::Application::with_assets`]. Sonar embeds its
//! assets in the binary at compile time so the app remains a single portable
//! executable.

use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

macro_rules! sonar_assets {
    ($($path:literal),+ $(,)?) => {
        &[$(($path, include_bytes!(concat!("../assets/", $path)) as &[u8])),+]
    };
}

const ASSETS: &[(&str, &[u8])] = sonar_assets![
    "icons/copy.svg",
    "icons/download.svg",
    "icons/history.svg",
    "icons/mic.svg",
    "icons/models.svg",
    "icons/panel.svg",
    "icons/settings.svg",
    "icons/transcribe.svg",
    "icons/trash.svg",
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ASSETS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ASSETS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}
