mod ui;

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

use gpui::{
    px, size, App, AppContext, Bounds, Context, Entity, KeyBinding, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions,
};
use sonar_models::{DownloadProgress, ModelStatus};

use self::ui::overlay::OverlayView;
use self::ui::text_input::{self, InputEvent, TextInput};
use crate::{
    history::{HistoryEntry, HistoryStore},
    hotkeys::{HotkeyAction, Hotkeys},
    service::{ServiceEvent, SonarService},
    settings::{ModelUnloadTimeout, Settings, SettingsStore},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Transcribe,
    History,
    Models,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RecordingState {
    Idle,
    Starting,
    Recording,
    Transcribing,
}

struct SonarApp {
    page: Page,
    settings_tab: usize,
    settings_store: SettingsStore,
    settings: Settings,
    history: HistoryStore,
    service: Arc<SonarService>,
    events: mpsc::Receiver<ServiceEvent>,
    hotkeys: Option<Hotkeys>,
    models: Vec<ModelStatus>,
    history_entries: Vec<HistoryEntry>,
    state: RecordingState,
    active_model_id: Option<String>,
    committed: String,
    tentative: String,
    levels: Vec<f32>,
    error: Option<String>,
    progress: HashMap<String, DownloadProgress>,
    overlay: Option<WindowHandle<OverlayView>>,
    unload_generation: Arc<AtomicU64>,
    history_limit_input: Entity<TextInput>,
    buffer_ms_input: Entity<TextInput>,
    threshold_input: Entity<TextInput>,
    custom_words_input: Entity<TextInput>,
    hf_token_input: Entity<TextInput>,
}

impl SonarApp {
    fn new(
        mut settings_store: SettingsStore,
        history: HistoryStore,
        service: Arc<SonarService>,
        events: mpsc::Receiver<ServiceEvent>,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = settings_store.load().clone();
        let hotkeys = Hotkeys::new(&settings.shortcuts.transcribe, &settings.shortcuts.cancel).ok();
        let models = service.models();
        let history_entries = history
            .list(None, Some(100))
            .map(|page| page.entries)
            .unwrap_or_default();
        cx.spawn(async move |view, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            if view
                .update(cx, |app, cx| {
                    app.poll(cx);
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
        let history_limit_input = cx.new(|cx| {
            TextInput::new(cx, settings.general.history_limit.to_string(), "100")
        });
        cx.subscribe(&history_limit_input, |app, input, _: &InputEvent, cx| {
            let Ok(value) = input.read(cx).content().trim().parse::<i64>() else {
                return;
            };
            let value = value.clamp(0, 10_000);
            if value == app.settings.general.history_limit {
                return;
            }
            app.settings.general.history_limit = value;
            let limit = usize::try_from(value).unwrap_or_default();
            if let Err(error) = app.history.prune(limit) {
                app.error = Some(format!("Failed to prune history: {error}"));
            }
            app.reload_history();
            app.persist_settings();
            cx.notify();
        })
        .detach();
        let buffer_ms_input = cx.new(|cx| {
            TextInput::new(
                cx,
                settings.transcription.extra_recording_buffer_ms.to_string(),
                "0",
            )
        });
        cx.subscribe(&buffer_ms_input, |app, input, _: &InputEvent, cx| {
            let Ok(value) = input.read(cx).content().trim().parse::<i64>() else {
                return;
            };
            let value = value.clamp(0, 5_000);
            if value == app.settings.transcription.extra_recording_buffer_ms {
                return;
            }
            app.settings.transcription.extra_recording_buffer_ms = value;
            app.persist_settings();
            cx.notify();
        })
        .detach();
        let threshold_input = cx.new(|cx| {
            TextInput::new(
                cx,
                settings.transcription.word_correction_threshold.to_string(),
                "0.18",
            )
        });
        cx.subscribe(&threshold_input, |app, input, _: &InputEvent, cx| {
            let Ok(value) = input.read(cx).content().trim().parse::<f64>() else {
                return;
            };
            let value = value.clamp(0.0, 1.0);
            if (value - app.settings.transcription.word_correction_threshold).abs() < f64::EPSILON
            {
                return;
            }
            app.settings.transcription.word_correction_threshold = value;
            app.persist_settings();
            cx.notify();
        })
        .detach();
        let custom_words_input = cx.new(|cx| {
            TextInput::new(
                cx,
                settings.transcription.custom_words.join(", "),
                "e.g. Sonar, GPUI, whisper",
            )
        });
        cx.subscribe(&custom_words_input, |app, input, _: &InputEvent, cx| {
            let words: Vec<String> = input
                .read(cx)
                .content()
                .split(',')
                .map(str::trim)
                .filter(|word| !word.is_empty())
                .map(str::to_owned)
                .collect();
            if words == app.settings.transcription.custom_words {
                return;
            }
            app.settings.transcription.custom_words = words;
            app.persist_settings();
            cx.notify();
        })
        .detach();
        let hf_token_input = cx.new(|cx| {
            TextInput::new(cx, settings.auth.hugging_face_token.clone(), "hf_...")
        });
        cx.subscribe(&hf_token_input, |app, input, _: &InputEvent, cx| {
            let token = input.read(cx).content().trim().to_owned();
            if token == app.settings.auth.hugging_face_token {
                return;
            }
            app.settings.auth.hugging_face_token = token;
            app.persist_settings();
            cx.notify();
        })
        .detach();
        Self {
            page: Page::Transcribe,
            settings_tab: 0,
            settings_store,
            settings,
            history,
            service,
            events,
            hotkeys,
            models,
            history_entries,
            state: RecordingState::Idle,
            active_model_id: None,
            committed: String::new(),
            tentative: String::new(),
            levels: vec![0.08; 16],
            error: None,
            progress: HashMap::new(),
            overlay: None,
            unload_generation: Arc::new(AtomicU64::new(0)),
            history_limit_input,
            buffer_ms_input,
            threshold_input,
            custom_words_input,
            hf_token_input,
        }
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        if let Some(action) = self.hotkeys.as_ref().and_then(Hotkeys::poll) {
            match action {
                HotkeyAction::Transcribe => self.toggle_recording(cx),
                HotkeyAction::Cancel => self.cancel_recording(cx),
            }
        }
        while let Ok(event) = self.events.try_recv() {
            self.handle_event(event, cx);
        }
    }

    fn handle_event(&mut self, event: ServiceEvent, cx: &mut Context<Self>) {
        match event {
            ServiceEvent::RecordingStarted { model_id } => {
                self.state = RecordingState::Recording;
                self.active_model_id = Some(model_id);
                self.open_overlay(cx);
            }
            ServiceEvent::StreamText {
                committed,
                tentative,
            } => {
                self.committed = committed;
                self.tentative = tentative;
                self.update_overlay(cx);
            }
            ServiceEvent::Levels(levels) => {
                self.levels = levels;
                self.update_overlay(cx);
            }
            ServiceEvent::RecordingFinished {
                model_id,
                result,
                delivery_error,
            } => {
                self.state = RecordingState::Idle;
                self.active_model_id = None;
                self.close_overlay(cx);
                match result {
                    Ok(text) => {
                        if !text.trim().is_empty() {
                            let limit = usize::try_from(self.settings.general.history_limit)
                                .unwrap_or_default();
                            if let Err(error) = self.history.save(&text, &model_id, limit) {
                                self.error = Some(format!("Failed to save history: {error}"));
                            }
                            self.reload_history();
                        }
                    }
                    Err(error) => self.error = Some(error),
                }
                if let Some(error) = delivery_error {
                    self.error = Some(format!(
                        "Transcription completed, but text insertion failed: {error}"
                    ));
                }
                self.schedule_unload(cx);
            }
            ServiceEvent::RecordingCancelled => {
                self.state = RecordingState::Idle;
                self.active_model_id = None;
                self.close_overlay(cx);
            }
            ServiceEvent::DownloadProgress(progress) => {
                self.progress.insert(progress.model_id.clone(), progress);
                self.models = self.service.models();
            }
            ServiceEvent::ModelsChanged => {
                self.models = self.service.models();
                self.progress.retain(|id, _| {
                    self.models
                        .iter()
                        .any(|model| model.id == *id && model.is_downloading)
                });
            }
            ServiceEvent::Error(error) => {
                self.error = Some(error);
                if self.state == RecordingState::Starting {
                    self.state = RecordingState::Idle;
                }
            }
        }
    }

    fn toggle_recording(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        match self.state {
            RecordingState::Idle => {
                self.unload_generation.fetch_add(1, Ordering::Relaxed);
                self.committed.clear();
                self.tentative.clear();
                self.state = RecordingState::Starting;
                self.service.start(self.settings.clone());
            }
            RecordingState::Recording => {
                self.state = RecordingState::Transcribing;
                self.update_overlay(cx);
                let model_id = self
                    .active_model_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_owned());
                self.service.stop(model_id, self.settings.clone());
            }
            RecordingState::Starting | RecordingState::Transcribing => {}
        }
    }

    fn cancel_recording(&mut self, cx: &mut Context<Self>) {
        if self.state != RecordingState::Idle {
            self.service.cancel();
            self.state = RecordingState::Idle;
            self.close_overlay(cx);
        }
    }

    fn open_overlay(&mut self, cx: &mut Context<Self>) {
        if self.overlay.is_some() {
            return;
        }
        let bounds = Bounds::centered(None, size(px(640.0), px(512.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: false,
            kind: WindowKind::PopUp,
            is_movable: true,
            is_resizable: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };
        let committed = self.committed.clone();
        let tentative = self.tentative.clone();
        let levels = self.levels.clone();
        let transcribing = self.state == RecordingState::Transcribing;
        if let Ok(handle) = cx.open_window(options, move |_, cx| {
            cx.new(|_| OverlayView::new(committed, tentative, levels, transcribing))
        }) {
            self.overlay = Some(handle);
        }
    }

    fn update_overlay(&self, cx: &mut Context<Self>) {
        let Some(overlay) = self.overlay else {
            return;
        };
        let committed = self.committed.clone();
        let tentative = self.tentative.clone();
        let levels = self.levels.clone();
        let transcribing = self.state == RecordingState::Transcribing;
        let _ = overlay.update(cx, move |view, _, cx| {
            view.update(committed, tentative, levels, transcribing);
            cx.notify();
        });
    }

    fn close_overlay(&mut self, cx: &mut Context<Self>) {
        if let Some(overlay) = self.overlay.take() {
            let _ = overlay.update(cx, |_, window, _| window.remove_window());
        }
    }

    fn schedule_unload(&self, cx: &mut Context<Self>) {
        let delay = match self.settings.general.model_unload_timeout {
            ModelUnloadTimeout::Never => return,
            ModelUnloadTimeout::Immediately => Duration::ZERO,
            ModelUnloadTimeout::TwoMinutes => Duration::from_secs(120),
            ModelUnloadTimeout::FiveMinutes => Duration::from_secs(300),
            ModelUnloadTimeout::TenMinutes => Duration::from_secs(600),
            ModelUnloadTimeout::FifteenMinutes => Duration::from_secs(900),
            ModelUnloadTimeout::OneHour => Duration::from_secs(3600),
        };
        let service = Arc::clone(&self.service);
        let generation = self.unload_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let unload_generation = Arc::clone(&self.unload_generation);
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(delay).await;
            if unload_generation.load(Ordering::Relaxed) == generation {
                service.unload_model();
            }
        })
        .detach();
    }

    fn reload_history(&mut self) {
        self.history_entries = self
            .history
            .list(None, Some(100))
            .map(|page| page.entries)
            .unwrap_or_default();
    }

    fn persist_settings(&mut self) {
        if let Err(error) = self.settings_store.replace(self.settings.clone()) {
            self.error = Some(format!("Failed to save settings: {error}"));
        }
    }
}

fn app_data_dir(settings_store: &SettingsStore) -> Result<PathBuf, String> {
    settings_store
        .path()
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "settings path has no parent directory".to_owned())
}

pub(crate) fn run() {
    let settings_store = SettingsStore::new().unwrap_or_else(|error| {
        panic!("failed to locate Sonar settings directory: {error}");
    });
    let data_dir = app_data_dir(&settings_store).unwrap_or_else(|error| panic!("{error}"));
    fs::create_dir_all(&data_dir)
        .unwrap_or_else(|error| panic!("failed to create Sonar data directory: {error}"));
    let history = HistoryStore::new(data_dir.join("history.db"))
        .unwrap_or_else(|error| panic!("failed to open Sonar history: {error}"));
    let (events_tx, events_rx) = mpsc::channel();
    let service = SonarService::new(data_dir.join("models"), events_tx)
        .unwrap_or_else(|error| panic!("failed to initialize Sonar: {error}"));

    gpui_platform::application()
        .with_assets(crate::assets::Assets)
        .run(move |cx: &mut App| {
            cx.bind_keys([
                KeyBinding::new("backspace", text_input::Backspace, Some("TextInput")),
                KeyBinding::new("delete", text_input::Delete, Some("TextInput")),
                KeyBinding::new("left", text_input::Left, Some("TextInput")),
                KeyBinding::new("right", text_input::Right, Some("TextInput")),
                KeyBinding::new("shift-left", text_input::SelectLeft, Some("TextInput")),
                KeyBinding::new("shift-right", text_input::SelectRight, Some("TextInput")),
                KeyBinding::new("home", text_input::Home, Some("TextInput")),
                KeyBinding::new("end", text_input::End, Some("TextInput")),
                KeyBinding::new("ctrl-a", text_input::SelectAll, Some("TextInput")),
                KeyBinding::new("ctrl-v", text_input::Paste, Some("TextInput")),
                KeyBinding::new("ctrl-c", text_input::Copy, Some("TextInput")),
                KeyBinding::new("ctrl-x", text_input::Cut, Some("TextInput")),
                KeyBinding::new("cmd-a", text_input::SelectAll, Some("TextInput")),
                KeyBinding::new("cmd-v", text_input::Paste, Some("TextInput")),
                KeyBinding::new("cmd-c", text_input::Copy, Some("TextInput")),
                KeyBinding::new("cmd-x", text_input::Cut, Some("TextInput")),
                KeyBinding::new(
                    "ctrl-cmd-space",
                    text_input::ShowCharacterPalette,
                    Some("TextInput"),
                ),
            ]);
            let bounds = Bounds::centered(None, size(px(1120.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(760.0), px(560.0))),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Sonar".into()),
                        ..Default::default()
                    }),
                    window_background: WindowBackgroundAppearance::Opaque,
                    ..Default::default()
                },
                move |_, cx| {
                    cx.new(|cx| SonarApp::new(settings_store, history, service, events_rx, cx))
                },
            )
            .unwrap_or_else(|error| panic!("failed to open Sonar window: {error}"));
            cx.activate(true);
        });
}
