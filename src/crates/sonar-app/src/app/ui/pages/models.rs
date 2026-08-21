use gpui::{div, prelude::*, px, rgb, AnyElement, Context, Div, FontWeight};
use sonar_models::ModelStatus;

use super::super::{
    components::{action_button, danger_button, header, icon, primary_button},
    BORDER, CARD, MUTED, PRIMARY,
};
use crate::app::SonarApp;

impl SonarApp {
    pub(in crate::app::ui) fn models_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let installed: Vec<_> = self
            .models
            .iter()
            .filter(|model| model.is_downloaded)
            .cloned()
            .map(|model| self.model_card(model, cx))
            .collect();
        let available: Vec<_> = self
            .models
            .iter()
            .filter(|model| !model.is_downloaded)
            .cloned()
            .map(|model| self.model_card(model, cx))
            .collect();

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header(
                "Models",
                "Download speech models to use them for transcription. Models are stored locally and can be removed at any time.",
            ))
            .child(
                div()
                    .id("models-list")
                    .mt_6()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .mb_3()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(MUTED))
                            .child("YOUR INSTALLED MODELS"),
                    )
                    .child(div().grid().grid_cols(2).gap_3().children(installed))
                    .child(
                        div()
                            .mt_6()
                            .mb_3()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(MUTED))
                            .child("AVAILABLE MODELS"),
                    )
                    .child(div().grid().grid_cols(2).gap_3().children(available)),
            )
            .into_any_element()
    }

    fn model_card(&self, model: ModelStatus, cx: &mut Context<Self>) -> Div {
        let model_id = model.id.clone();
        let is_downloading = model.is_downloading;
        let progress = self
            .progress
            .get(&model.id)
            .map_or(0.0, |value| value.percentage);
        let button = if is_downloading {
            action_button(format!("cancel-{}", model.id), "Cancel")
                .on_click(cx.listener(move |app, _, _, _| app.service.cancel_download(&model_id)))
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
            .bg(rgb(CARD))
            .child(
                div()
                    .p_3()
                    .flex()
                    .items_start()
                    .gap_3()
                    .child(
                        div()
                            .size(px(34.0))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(0x202126))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("transcribe").text_color(rgb(0x8f24ff))),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(model.name),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .overflow_hidden()
                                    .child(model.description),
                            ),
                    )
                    .child(button),
            )
            .when(is_downloading, |card| {
                card.child(
                    div()
                        .mx_3()
                        .h(px(4.0))
                        .rounded_full()
                        .bg(rgb(BORDER))
                        .child(
                            div()
                                .h_full()
                                .rounded_full()
                                .bg(rgb(PRIMARY))
                                .w(gpui::relative(progress as f32 / 100.0)),
                        ),
                )
            })
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(0x1b1c20))
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
    }
}
