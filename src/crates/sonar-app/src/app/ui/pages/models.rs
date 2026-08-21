use gpui::{div, prelude::*, px, rgb, AnyElement, Context, FontWeight};

use super::super::{
    components::{action_button, danger_button, header, primary_button},
    BORDER, CARD, MUTED, PRIMARY,
};
use crate::app::SonarApp;

impl SonarApp {
    pub(in crate::app::ui) fn models_page(&self, cx: &mut Context<Self>) -> AnyElement {
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
            .child(header(
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
}
