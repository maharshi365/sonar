use chrono::{DateTime, Local};
use gpui::{div, prelude::*, px, rgb, AnyElement, Context, FontWeight};

use super::super::{
    components::{action_button, danger_button, empty_state, header},
    BORDER, CARD, MUTED,
};
use crate::app::SonarApp;

impl SonarApp {
    pub(in crate::app::ui) fn history_page(&self, cx: &mut Context<Self>) -> AnyElement {
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
                                    div()
                                        .text_xs()
                                        .font_family("monospace")
                                        .text_color(rgb(MUTED))
                                        .child(entry.model_id),
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
                    .child(
                        div()
                            .child(
                                div()
                                    .mb_2()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x761cff))
                                    .child("S T O R E D   O N   T H I S   D E V I C E"),
                            )
                            .child(header(
                                "Transcription history",
                                "Revisit and copy your completed transcriptions. Nothing leaves your device.",
                            )),
                    )
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
}
