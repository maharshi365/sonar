use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const SETTINGS_FILE_NAME: &str = "settings.json";
const LEGACY_WINDOWS_CANCEL_SHORTCUT: &str = "CommandOrControl+Shift+Escape";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: i64,
    pub general: GeneralSettings,
    pub shortcuts: ShortcutSettings,
    pub audio: AudioSettings,
    pub output: OutputSettings,
    pub transcription: TranscriptionSettings,
    pub inference: InferenceSettings,
    pub auth: AuthSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            general: GeneralSettings::default(),
            shortcuts: ShortcutSettings::default(),
            audio: AudioSettings::default(),
            output: OutputSettings::default(),
            transcription: TranscriptionSettings::default(),
            inference: InferenceSettings::default(),
            auth: AuthSettings::default(),
        }
    }
}

impl Settings {
    fn validated(mut self) -> Option<Self> {
        if self.shortcuts.transcribe.is_empty() || self.shortcuts.cancel.is_empty() {
            return None;
        }

        self.general.history_limit = self.general.history_limit.clamp(0, 10_000);
        self.transcription.extra_recording_buffer_ms =
            self.transcription.extra_recording_buffer_ms.clamp(0, 5_000);
        self.transcription.word_correction_threshold =
            self.transcription.word_correction_threshold.clamp(0.0, 1.0);
        Some(self)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneralSettings {
    pub tts_model: String,
    pub model_unload_timeout: ModelUnloadTimeout,
    pub history_limit: i64,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            tts_model: String::new(),
            model_unload_timeout: ModelUnloadTimeout::FiveMinutes,
            history_limit: 100,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ModelUnloadTimeout {
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "immediately")]
    Immediately,
    #[serde(rename = "2m")]
    TwoMinutes,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "10m")]
    TenMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShortcutSettings {
    pub transcribe: String,
    pub cancel: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            transcribe: "CommandOrControl+Shift+Space".to_owned(),
            cancel: "CommandOrControl+Shift+Backspace".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AudioSettings {
    pub input_device_id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OutputSettings {
    pub method: OutputMethod,
    pub append_trailing_space: bool,
    pub auto_submit: bool,
    pub auto_submit_key: AutoSubmitKey,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            method: OutputMethod::Paste,
            append_trailing_space: false,
            auto_submit: false,
            auto_submit_key: AutoSubmitKey::Enter,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMethod {
    Paste,
    Clipboard,
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum AutoSubmitKey {
    #[serde(rename = "enter")]
    Enter,
    #[serde(rename = "ctrl_enter")]
    ControlEnter,
    #[serde(rename = "cmd_enter")]
    CommandEnter,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TranscriptionSettings {
    pub custom_words: Vec<String>,
    pub filler_word_removal: bool,
    pub custom_filler_words: Vec<String>,
    pub extra_recording_buffer_ms: i64,
    pub word_correction_threshold: f64,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            custom_words: Vec::new(),
            filler_word_removal: true,
            custom_filler_words: Vec::new(),
            extra_recording_buffer_ms: 0,
            word_correction_threshold: 0.18,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InferenceSettings {
    pub accelerator: Accelerator,
    pub gpu_device_id: String,
}

impl Default for InferenceSettings {
    fn default() -> Self {
        Self {
            accelerator: Accelerator::Auto,
            gpu_device_id: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Accelerator {
    Auto,
    Cpu,
    Gpu,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AuthSettings {
    pub hugging_face_token: String,
}

/// A cached settings store backed by `settings.json` in Sonar's user data directory.
pub struct SettingsStore {
    path: PathBuf,
    current: Settings,
}

impl SettingsStore {
    pub fn new() -> io::Result<Self> {
        let base_dirs = BaseDirs::new().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not locate user data directory",
            )
        })?;
        Ok(Self::from_user_data_dir(
            base_dirs.config_dir().join("Sonar"),
        ))
    }

    /// Creates a store rooted at an explicit directory, primarily for tests and embedding.
    pub fn from_user_data_dir(directory: impl Into<PathBuf>) -> Self {
        Self {
            path: directory.into().join(SETTINGS_FILE_NAME),
            current: Settings::default(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads and caches settings. Missing, unreadable, or invalid files yield defaults.
    pub fn load(&mut self) -> &Settings {
        self.current = fs::read_to_string(&self.path)
            .ok()
            .and_then(|contents| serde_json::from_str::<Settings>(&contents).ok())
            .and_then(Settings::validated)
            .unwrap_or_default();

        if migrate_legacy_cancel_shortcut(&mut self.current, cfg!(target_os = "windows")) {
            let _ = self.save();
        }
        &self.current
    }

    pub fn save(&self) -> io::Result<()> {
        write_settings(&self.path, &self.current)
    }

    pub fn replace(&mut self, settings: Settings) -> io::Result<&Settings> {
        let next = settings
            .validated()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid settings"))?;
        write_settings(&self.path, &next)?;
        self.current = next;
        Ok(&self.current)
    }
}

fn migrate_legacy_cancel_shortcut(settings: &mut Settings, windows: bool) -> bool {
    if windows && settings.shortcuts.cancel == LEGACY_WINDOWS_CANCEL_SHORTCUT {
        settings.shortcuts.cancel = ShortcutSettings::default().cancel;
        true
    } else {
        false
    }
}

fn write_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "settings path has no parent")
    })?;
    fs::create_dir_all(parent)?;

    let temporary_path = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_path)?;
        serde_json::to_writer_pretty(&mut file, settings).map_err(io::Error::other)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);

        match fs::rename(&temporary_path, path) {
            Ok(()) => Ok(()),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
                ) =>
            {
                fs::remove_file(path)?;
                fs::rename(&temporary_path, path)
            }
            Err(error) => Err(error),
        }
    })();

    if result.is_err() {
        let _ = fs::remove_file(temporary_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        std::env::temp_dir().join(format!(
            "sonar-settings-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn defaults_match_typescript_schema_and_use_camel_case() {
        let json = serde_json::to_value(Settings::default()).unwrap_or_default();
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["general"]["modelUnloadTimeout"], "5m");
        assert_eq!(
            json["shortcuts"]["cancel"],
            "CommandOrControl+Shift+Backspace"
        );
        assert_eq!(json["output"]["autoSubmitKey"], "enter");
        assert_eq!(json["transcription"]["wordCorrectionThreshold"], 0.18);
    }

    #[test]
    fn load_fills_missing_fields_and_clamps_ranges() {
        let directory = test_directory("clamp");
        fs::create_dir_all(&directory).unwrap_or_default();
        fs::write(
            directory.join(SETTINGS_FILE_NAME),
            r#"{"general":{"historyLimit":20000},"transcription":{"extraRecordingBufferMs":-4,"wordCorrectionThreshold":2.0}}"#,
        )
        .unwrap_or_default();

        let mut store = SettingsStore::from_user_data_dir(&directory);
        let loaded = store.load();
        assert_eq!(loaded.general.history_limit, 10_000);
        assert_eq!(loaded.transcription.extra_recording_buffer_ms, 0);
        assert_eq!(loaded.transcription.word_correction_threshold, 1.0);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn corrupt_file_recovers_and_update_is_persisted() {
        let directory = test_directory("persist");
        fs::create_dir_all(&directory).unwrap_or_default();
        fs::write(directory.join(SETTINGS_FILE_NAME), "not json").unwrap_or_default();
        let mut store = SettingsStore::from_user_data_dir(&directory);
        assert_eq!(store.load(), &Settings::default());

        let mut next = store.load().clone();
        next.general.history_limit = 321;
        assert!(store.replace(next).is_ok());
        let mut reloaded = SettingsStore::from_user_data_dir(&directory);
        assert_eq!(reloaded.load().general.history_limit, 321);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn legacy_windows_cancel_shortcut_is_migrated() {
        let mut settings = Settings::default();
        settings.shortcuts.cancel = LEGACY_WINDOWS_CANCEL_SHORTCUT.to_owned();
        assert!(migrate_legacy_cancel_shortcut(&mut settings, true));
        assert_eq!(
            settings.shortcuts.cancel,
            ShortcutSettings::default().cancel
        );
    }
}
