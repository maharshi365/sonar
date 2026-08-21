use gpui::{div, prelude::*, rgb, AnyElement, Context};

use super::super::{
    components::{header, setting_row, settings_group, toggle_button, value_button, value_pill},
    BORDER, MUTED, PRIMARY,
};
use crate::{
    app::SonarApp,
    settings::{Accelerator, ModelUnloadTimeout, OutputMethod},
};

impl SonarApp {
    pub(in crate::app::ui) fn settings_page(&self, cx: &mut Context<Self>) -> AnyElement {
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
            .child(header(
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
