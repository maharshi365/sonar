mod history;
mod hotkeys;
mod service;
mod settings;

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

use chrono::{DateTime, Local};
use gpui::{
    div, prelude::*, px, rgb, size, AnyElement, App, Bounds, Context, Div, FontWeight,
    SharedString, Stateful, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle,
    WindowKind, WindowOptions,
};
use hotkeys::{HotkeyAction, Hotkeys};
use service::{ServiceEvent, SonarService};
use settings::{Accelerator, ModelUnloadTimeout, OutputMethod, Settings, SettingsStore};
use sonar_models::{DownloadProgress, ModelStatus};

use crate::history::{HistoryEntry, HistoryStore};

const BACKGROUND: u32 = 0x17161b;
const SIDEBAR: u32 = 0x232228;
const CARD: u32 = 0x222127;
const MUTED: u32 = 0xa9a6b2;
const BORDER: u32 = 0x39373f;
const PRIMARY: u32 = 0x7135cb;
const PRIMARY_HOVER: u32 = 0x8248dc;
const DANGER: u32 = 0xd85858;
const GREEN: u32 = 0x85c940;

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
        let bounds = Bounds::centered(None, size(px(560.0), px(170.0)), cx);
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
            cx.new(|_| OverlayView {
                committed,
                tentative,
                levels,
                transcribing,
            })
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
            view.committed = committed;
            view.tentative = tentative;
            view.levels = levels;
            view.transcribing = transcribing;
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

    fn nav_item(&self, page: Page, label: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let selected = self.page == page;
        div()
            .id(label)
            .px_4()
            .py_3()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .when(selected, |item| {
                item.bg(rgb(0x302c38)).text_color(rgb(0xffffff))
            })
            .when(!selected, |item| {
                item.text_color(rgb(MUTED))
                    .hover(|style| style.bg(rgb(0x2b2930)).text_color(rgb(0xffffff)))
            })
            .on_click(cx.listener(move |app, _, _, cx| {
                app.page = page;
                app.error = None;
                cx.notify();
            }))
            .child(label)
    }

    fn header(title: &'static str, description: &'static str) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(div().text_sm().text_color(rgb(MUTED)).child(description))
    }

    fn home(&self, cx: &mut Context<Self>) -> AnyElement {
        let (label, button_label, color) = match self.state {
            RecordingState::Idle => ("Press to start speaking", "MIC", PRIMARY),
            RecordingState::Starting => ("Starting microphone...", "...", PRIMARY),
            RecordingState::Recording => ("Listening... press to stop", "STOP", DANGER),
            RecordingState::Transcribing => ("Transcribing...", "...", PRIMARY),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x9a65e8))
                    .child("LOCAL TRANSCRIPTION"),
            )
            .child(
                div()
                    .mt_2()
                    .text_3xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Ready when you are."),
            )
            .child(
                div()
                    .mt_2()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child("Press the button or global shortcut and Sonar will type the transcript into your active app."),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .child(
                        div()
                            .id("record-button")
                            .size(px(104.0))
                            .rounded_full()
                            .bg(rgb(color))
                            .hover(move |style| style.bg(rgb(if color == DANGER { 0xe96a6a } else { PRIMARY_HOVER })))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .on_click(cx.listener(|app, _, _, cx| app.toggle_recording(cx)))
                            .child(button_label),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::MEDIUM)
                            .child(label),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(CARD))
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(self.settings.shortcuts.transcribe.clone()),
                    ),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .pt_5()
                    .flex()
                    .gap_8()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("Whisper and Moonshine local inference")
                    .child("Audio stays on this device"),
            )
            .into_any_element()
    }

    fn history_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let entries = self.history_entries.iter().cloned().map(|entry| {
            let copy_text = entry.text.clone();
            let id = entry.id;
            let created = DateTime::from_timestamp_millis(entry.created_at)
                .map(|date| {
                    date.with_timezone(&Local)
                        .format("%b %-d, %Y at %-I:%M %p")
                        .to_string()
                })
                .unwrap_or_else(|| "Unknown date".to_owned());
            div()
                .border_1()
                .border_color(rgb(BORDER))
                .rounded_lg()
                .bg(rgb(CARD))
                .child(
                    div()
                        .px_4()
                        .py_3()
                        .border_b_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(created),
                                )
                                .child(
                                    div().text_xs().text_color(rgb(MUTED)).child(entry.model_id),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(action_button(format!("copy-{id}"), "Copy").on_click(
                                    cx.listener(move |app, _, _, _| {
                                        if let Err(error) =
                                            arboard::Clipboard::new().and_then(|mut clipboard| {
                                                clipboard.set_text(copy_text.clone())
                                            })
                                        {
                                            app.error = Some(format!("Failed to copy: {error}"));
                                        }
                                    }),
                                ))
                                .child(danger_button(format!("delete-{id}"), "Delete").on_click(
                                    cx.listener(move |app, _, _, cx| {
                                        if let Err(error) = app.history.delete(id) {
                                            app.error =
                                                Some(format!("Failed to delete history: {error}"));
                                        }
                                        app.reload_history();
                                        cx.notify();
                                    }),
                                )),
                        ),
                )
                .child(
                    div()
                        .p_4()
                        .text_sm()
                        .line_height(px(26.0))
                        .child(entry.text),
                )
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .child(Self::header(
                        "History",
                        "Transcriptions are stored only on this device.",
                    ))
                    .child(
                        danger_button("clear-history", "Clear history").on_click(cx.listener(
                            |app, _, _, cx| {
                                if let Err(error) = app.history.clear() {
                                    app.error = Some(format!("Failed to clear history: {error}"));
                                }
                                app.reload_history();
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(
                div()
                    .id("history-list")
                    .mt_6()
                    .flex_1()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .when(self.history_entries.is_empty(), |list| {
                        list.child(empty_state(
                            "No transcriptions yet",
                            "Your local dictation history will appear here.",
                        ))
                    })
                    .children(entries),
            )
            .into_any_element()
    }

    fn models_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let cards = self.models.iter().cloned().map(|model| {
            let model_id = model.id.clone();
            let is_downloading = model.is_downloading;
            let progress = self
                .progress
                .get(&model.id)
                .map_or(0.0, |value| value.percentage);
            let button = if is_downloading {
                action_button(format!("cancel-{}", model.id), "Cancel").on_click(
                    cx.listener(move |app, _, _, _| app.service.cancel_download(&model_id)),
                )
            } else if model.is_downloaded {
                let remove_id = model.id.clone();
                danger_button(format!("remove-{}", model.id), "Remove").on_click(cx.listener(
                    move |app, _, _, _| {
                        app.service.remove(remove_id.clone());
                        if app.settings.general.tts_model == remove_id {
                            app.settings.general.tts_model.clear();
                            app.persist_settings();
                        }
                    },
                ))
            } else {
                let download_id = model.id.clone();
                primary_button(format!("download-{}", model.id), "Download").on_click(cx.listener(
                    move |app, _, _, _| {
                        let token = (!app.settings.auth.hugging_face_token.trim().is_empty())
                            .then(|| app.settings.auth.hugging_face_token.clone());
                        app.service.download(download_id.clone(), token);
                    },
                ))
            };
            let size = if model.size_bytes >= 1_000_000_000 {
                format!("{:.1} GB", model.size_bytes as f64 / 1_000_000_000.0)
            } else {
                format!("{} MB", model.size_bytes / 1_000_000)
            };
            div()
                .border_1()
                .border_color(rgb(BORDER))
                .rounded_lg()
                .bg(rgb(CARD))
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap_4()
                        .child(
                            div()
                                .min_w_0()
                                .child(div().font_weight(FontWeight::SEMIBOLD).child(
                                    if model.recommended {
                                        format!("{}  Recommended", model.name)
                                    } else {
                                        model.name
                                    },
                                ))
                                .child(
                                    div()
                                        .mt_1()
                                        .text_xs()
                                        .text_color(rgb(MUTED))
                                        .child(model.description),
                                ),
                        )
                        .child(button),
                )
                .when(is_downloading, |card| {
                    card.child(
                        div().h(px(5.0)).rounded_full().bg(rgb(BORDER)).child(
                            div()
                                .h_full()
                                .rounded_full()
                                .bg(rgb(PRIMARY))
                                .w(gpui::relative(progress as f32 / 100.0)),
                        ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("{progress:.0}%")),
                    )
                })
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(if model.supports_streaming {
                            "Live streaming"
                        } else {
                            "Transcribes on stop"
                        })
                        .child(size)
                        .child(model.languages.join(", ")),
                )
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(Self::header(
                "Speech models",
                "Install and manage local transcription models.",
            ))
            .child(
                div()
                    .id("models-list")
                    .mt_6()
                    .flex_1()
                    .overflow_y_scroll()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .children(cards),
            )
            .into_any_element()
    }

    fn settings_page(&self, cx: &mut Context<Self>) -> AnyElement {
        const TABS: [&str; 7] = [
            "General",
            "Audio",
            "Shortcuts",
            "Output",
            "Transcription",
            "Performance",
            "Auth",
        ];
        let tabs = TABS.into_iter().enumerate().map(|(index, label)| {
            let selected = self.settings_tab == index;
            div()
                .id(format!("settings-{label}"))
                .px_3()
                .py_3()
                .cursor_pointer()
                .text_xs()
                .text_center()
                .border_b_2()
                .border_color(rgb(if selected { PRIMARY } else { BORDER }))
                .text_color(rgb(if selected { 0xffffff } else { MUTED }))
                .on_click(cx.listener(move |app, _, _, cx| {
                    app.settings_tab = index;
                    cx.notify();
                }))
                .child(label)
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(Self::header(
                "Settings",
                "Configure how Sonar works on this device.",
            ))
            .child(div().mt_6().grid().grid_cols(7).children(tabs))
            .child(
                div()
                    .id("settings-content")
                    .pt_5()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.settings_content(cx)),
            )
            .into_any_element()
    }

    fn settings_content(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.settings_tab {
            0 => self.general_settings(cx),
            1 => self.audio_settings(cx),
            2 => settings_group(
                "Global shortcuts",
                vec![
                    setting_row(
                        "Toggle dictation",
                        "Start or stop dictation from any application.",
                        value_pill(self.settings.shortcuts.transcribe.clone()),
                    ),
                    setting_row(
                        "Cancel dictation",
                        "Discard the active recording.",
                        value_pill(self.settings.shortcuts.cancel.clone()),
                    ),
                ],
            )
            .into_any_element(),
            3 => self.output_settings(cx),
            4 => self.transcription_settings(cx),
            5 => self.performance_settings(cx),
            _ => settings_group(
                "Hugging Face",
                vec![setting_row(
                    "Access token",
                    "The existing local token is used for model downloads.",
                    value_pill(if self.settings.auth.hugging_face_token.is_empty() {
                        "Not configured".to_owned()
                    } else {
                        "Configured".to_owned()
                    }),
                )],
            )
            .into_any_element(),
        }
    }

    fn general_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let downloaded: Vec<_> = self
            .models
            .iter()
            .filter(|model| model.is_downloaded)
            .collect();
        let current_name = downloaded
            .iter()
            .find(|model| model.id == self.settings.general.tts_model)
            .map_or("Automatic", |model| model.name.as_str())
            .to_owned();
        settings_group(
            "Models and history",
            vec![
                setting_row(
                    "Default speech model",
                    "Click to cycle through downloaded models.",
                    value_button("default-model", current_name).on_click(cx.listener(
                        |app, _, _, cx| {
                            let downloaded: Vec<_> = app
                                .models
                                .iter()
                                .filter(|model| model.is_downloaded)
                                .collect();
                            if !downloaded.is_empty() {
                                let next = downloaded
                                    .iter()
                                    .position(|model| model.id == app.settings.general.tts_model)
                                    .map_or(0, |index| (index + 1) % downloaded.len());
                                app.settings.general.tts_model = downloaded[next].id.clone();
                                app.persist_settings();
                                cx.notify();
                            }
                        },
                    )),
                ),
                setting_row(
                    "Unload model",
                    "Release model memory after Sonar is idle.",
                    value_button(
                        "unload-timeout",
                        unload_label(self.settings.general.model_unload_timeout),
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.settings.general.model_unload_timeout =
                            next_unload_timeout(app.settings.general.model_unload_timeout);
                        app.persist_settings();
                        cx.notify();
                    })),
                ),
                setting_row(
                    "History limit",
                    "Click to cycle 0, 100, 500, and 1000 entries.",
                    value_button(
                        "history-limit",
                        self.settings.general.history_limit.to_string(),
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.settings.general.history_limit =
                            match app.settings.general.history_limit {
                                0 => 100,
                                1..=100 => 500,
                                101..=500 => 1000,
                                _ => 0,
                            };
                        let limit =
                            usize::try_from(app.settings.general.history_limit).unwrap_or_default();
                        if let Err(error) = app.history.prune(limit) {
                            app.error = Some(format!("Failed to prune history: {error}"));
                        }
                        app.reload_history();
                        app.persist_settings();
                        cx.notify();
                    })),
                ),
            ],
        )
        .into_any_element()
    }

    fn audio_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let devices = sonar_audio::list_input_devices().unwrap_or_default();
        let current = if self.settings.audio.input_device_id.is_empty() {
            "System default".to_owned()
        } else {
            devices
                .iter()
                .find(|device| device.index == self.settings.audio.input_device_id)
                .map_or_else(
                    || self.settings.audio.input_device_id.clone(),
                    |device| device.name.clone(),
                )
        };
        setting_row(
            "Microphone",
            "Click to cycle through available input devices.",
            value_button("microphone", current).on_click(cx.listener(|app, _, _, cx| {
                let devices = sonar_audio::list_input_devices().unwrap_or_default();
                let next = devices
                    .iter()
                    .position(|device| device.index == app.settings.audio.input_device_id)
                    .map_or(0, |index| index + 1);
                app.settings.audio.input_device_id = if next >= devices.len() {
                    String::new()
                } else {
                    devices[next].index.clone()
                };
                app.persist_settings();
                cx.notify();
            })),
        )
        .into_any_element()
    }

    fn output_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        settings_group(
            "Text delivery",
            vec![
                setting_row(
                    "After transcription",
                    "Paste into the focused app, copy only, or do nothing.",
                    value_button(
                        "output-method",
                        match self.settings.output.method {
                            OutputMethod::Paste => "Paste into app",
                            OutputMethod::Clipboard => "Copy to clipboard",
                            OutputMethod::None => "Do nothing",
                        },
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.settings.output.method = match app.settings.output.method {
                            OutputMethod::Paste => OutputMethod::Clipboard,
                            OutputMethod::Clipboard => OutputMethod::None,
                            OutputMethod::None => OutputMethod::Paste,
                        };
                        app.persist_settings();
                        cx.notify();
                    })),
                ),
                setting_row(
                    "Trailing space",
                    "Append one space to delivered text.",
                    toggle_button("trailing-space", self.settings.output.append_trailing_space)
                        .on_click(cx.listener(|app, _, _, cx| {
                            app.settings.output.append_trailing_space =
                                !app.settings.output.append_trailing_space;
                            app.persist_settings();
                            cx.notify();
                        })),
                ),
                setting_row(
                    "Submit after paste",
                    "Send Enter after text is inserted.",
                    toggle_button("auto-submit", self.settings.output.auto_submit).on_click(
                        cx.listener(|app, _, _, cx| {
                            app.settings.output.auto_submit = !app.settings.output.auto_submit;
                            app.persist_settings();
                            cx.notify();
                        }),
                    ),
                ),
            ],
        )
        .into_any_element()
    }

    fn transcription_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        settings_group(
            "Recognition",
            vec![
                setting_row(
                    "Custom words",
                    "Names and technical terms retained from your existing settings.",
                    value_pill(format!(
                        "{} terms",
                        self.settings.transcription.custom_words.len()
                    )),
                ),
                setting_row(
                    "Remove filler words",
                    "Remove conservative fillers such as uh, uhm, and hmm.",
                    toggle_button(
                        "filler-removal",
                        self.settings.transcription.filler_word_removal,
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.settings.transcription.filler_word_removal =
                            !app.settings.transcription.filler_word_removal;
                        app.persist_settings();
                        cx.notify();
                    })),
                ),
                setting_row(
                    "Trailing audio buffer",
                    "Click to add 250 ms, wrapping after 1000 ms.",
                    value_button(
                        "audio-buffer",
                        format!(
                            "{} ms",
                            self.settings.transcription.extra_recording_buffer_ms
                        ),
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        app.settings.transcription.extra_recording_buffer_ms =
                            if app.settings.transcription.extra_recording_buffer_ms >= 1000 {
                                0
                            } else {
                                app.settings.transcription.extra_recording_buffer_ms + 250
                            };
                        app.persist_settings();
                        cx.notify();
                    })),
                ),
                setting_row(
                    "Correction threshold",
                    "Click to increase fuzzy correction strength.",
                    value_button(
                        "correction-threshold",
                        format!(
                            "{:.2}",
                            self.settings.transcription.word_correction_threshold
                        ),
                    )
                    .on_click(cx.listener(|app, _, _, cx| {
                        let current = app.settings.transcription.word_correction_threshold;
                        app.settings.transcription.word_correction_threshold =
                            if current >= 0.95 { 0.0 } else { current + 0.05 };
                        app.persist_settings();
                        cx.notify();
                    })),
                ),
            ],
        )
        .into_any_element()
    }

    fn performance_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let label = match self.settings.inference.accelerator {
            Accelerator::Auto => "Automatic",
            Accelerator::Cpu => "CPU",
            Accelerator::Gpu => "GPU",
        };
        settings_group(
            "Inference",
            vec![setting_row(
                "Accelerator",
                "Auto chooses the fastest available backend.",
                value_button("accelerator", label).on_click(cx.listener(|app, _, _, cx| {
                    app.settings.inference.accelerator = match app.settings.inference.accelerator {
                        Accelerator::Auto => Accelerator::Cpu,
                        Accelerator::Cpu => Accelerator::Gpu,
                        Accelerator::Gpu => Accelerator::Auto,
                    };
                    app.persist_settings();
                    cx.notify();
                })),
            )],
        )
        .into_any_element()
    }
}

impl Render for SonarApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page {
            Page::Transcribe => self.home(cx),
            Page::History => self.history_page(cx),
            Page::Models => self.models_page(cx),
            Page::Settings => self.settings_page(cx),
        };
        div()
            .size_full()
            .flex()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(0xf8f7fa))
            .font_family("Inter")
            .child(
                div()
                    .w(px(224.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SIDEBAR))
                    .p_4()
                    .child(
                        div()
                            .px_3()
                            .py_4()
                            .mb_4()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("SONAR"),
                    )
                    .child(self.nav_item(Page::Transcribe, "Transcribe", cx))
                    .child(self.nav_item(Page::History, "History", cx))
                    .child(self.nav_item(Page::Models, "Models", cx))
                    .child(div().flex_1())
                    .child(self.nav_item(Page::Settings, "Settings", cx)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .p_8()
                    .flex()
                    .flex_col()
                    .when_some(self.error.clone(), |root, error| {
                        root.child(
                            div()
                                .mb_4()
                                .p_3()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(DANGER))
                                .bg(rgb(0x321d22))
                                .text_sm()
                                .text_color(rgb(0xffa2a2))
                                .child(error),
                        )
                    })
                    .child(content),
            )
    }
}

struct OverlayView {
    committed: String,
    tentative: String,
    levels: Vec<f32>,
    transcribing: bool,
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bars = self
            .levels
            .iter()
            .copied()
            .enumerate()
            .map(|(index, level)| {
                div()
                    .id(format!("level-{index}"))
                    .w(px(5.0))
                    .h(px(8.0 + level.clamp(0.0, 1.0) * 38.0))
                    .rounded_full()
                    .bg(rgb(GREEN))
            });
        let transcript = if self.committed.is_empty() && self.tentative.is_empty() {
            "Start speaking...".to_owned()
        } else {
            format!("{}{}", self.committed, self.tentative)
        };
        div().size_full().p_3().bg(gpui::transparent_black()).child(
            div()
                .size_full()
                .rounded_xl()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(0x1b1a1f))
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().size_2().rounded_full().bg(rgb(DANGER)))
                                .child(if self.transcribing {
                                    "Transcribing..."
                                } else {
                                    "Listening"
                                }),
                        )
                        .child(
                            div()
                                .h(px(48.0))
                                .flex()
                                .items_center()
                                .gap_1()
                                .children(bars),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_sm()
                        .line_height(px(23.0))
                        .text_color(rgb(if self.committed.is_empty() {
                            MUTED
                        } else {
                            0xf5f4f7
                        }))
                        .child(transcript),
                ),
        )
    }
}

fn action_button(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id.into())
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .hover(|style| style.bg(rgb(0x323038)))
        .cursor_pointer()
        .text_xs()
        .child(label.into())
}

fn primary_button(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Stateful<Div> {
    action_button(id, label)
        .bg(rgb(PRIMARY))
        .border_color(rgb(PRIMARY))
        .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
}

fn danger_button(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Stateful<Div> {
    action_button(id, label)
        .bg(rgb(0x3a2024))
        .border_color(rgb(0x71383e))
        .text_color(rgb(0xffaaaa))
}

fn value_button(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Stateful<Div> {
    action_button(id, label).min_w(px(140.0)).text_center()
}

fn value_pill(label: impl Into<SharedString>) -> Div {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(0x2e2c33))
        .text_xs()
        .text_color(rgb(MUTED))
        .child(label.into())
}

fn toggle_button(id: impl Into<SharedString>, enabled: bool) -> Stateful<Div> {
    value_button(id, if enabled { "On" } else { "Off" }).when(enabled, |button| {
        button.bg(rgb(PRIMARY)).border_color(rgb(PRIMARY))
    })
}

fn setting_row(label: &'static str, description: &'static str, control: impl IntoElement) -> Div {
    div()
        .px_4()
        .py_4()
        .border_b_1()
        .border_color(rgb(BORDER))
        .flex()
        .items_center()
        .justify_between()
        .gap_6()
        .child(
            div()
                .flex_1()
                .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label))
                .child(
                    div()
                        .mt_1()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(description),
                ),
        )
        .child(control)
}

fn settings_group(title: &'static str, rows: Vec<Div>) -> Div {
    div()
        .max_w(px(760.0))
        .child(
            div()
                .mb_2()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child(title.to_uppercase()),
        )
        .child(
            div()
                .border_1()
                .border_color(rgb(BORDER))
                .rounded_lg()
                .bg(rgb(CARD))
                .children(rows),
        )
}

fn empty_state(title: &'static str, description: &'static str) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(div().text_lg().font_weight(FontWeight::MEDIUM).child(title))
        .child(div().text_sm().text_color(rgb(MUTED)).child(description))
}

fn unload_label(value: ModelUnloadTimeout) -> &'static str {
    match value {
        ModelUnloadTimeout::Immediately => "Immediately",
        ModelUnloadTimeout::TwoMinutes => "After 2 minutes",
        ModelUnloadTimeout::FiveMinutes => "After 5 minutes",
        ModelUnloadTimeout::TenMinutes => "After 10 minutes",
        ModelUnloadTimeout::FifteenMinutes => "After 15 minutes",
        ModelUnloadTimeout::OneHour => "After 1 hour",
        ModelUnloadTimeout::Never => "Never",
    }
}

fn next_unload_timeout(value: ModelUnloadTimeout) -> ModelUnloadTimeout {
    match value {
        ModelUnloadTimeout::Immediately => ModelUnloadTimeout::TwoMinutes,
        ModelUnloadTimeout::TwoMinutes => ModelUnloadTimeout::FiveMinutes,
        ModelUnloadTimeout::FiveMinutes => ModelUnloadTimeout::TenMinutes,
        ModelUnloadTimeout::TenMinutes => ModelUnloadTimeout::FifteenMinutes,
        ModelUnloadTimeout::FifteenMinutes => ModelUnloadTimeout::OneHour,
        ModelUnloadTimeout::OneHour => ModelUnloadTimeout::Never,
        ModelUnloadTimeout::Never => ModelUnloadTimeout::Immediately,
    }
}

fn app_data_dir(settings_store: &SettingsStore) -> Result<PathBuf, String> {
    settings_store
        .path()
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "settings path has no parent directory".to_owned())
}

fn main() {
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

    gpui_platform::application().run(move |cx: &mut App| {
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
