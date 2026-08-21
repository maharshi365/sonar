use gpui::{div, prelude::*, px, rgb, Context, Render, Window};

use super::{BORDER, DANGER, GREEN, MUTED};

pub(in crate::app) struct OverlayView {
    committed: String,
    tentative: String,
    levels: Vec<f32>,
    transcribing: bool,
}

impl OverlayView {
    pub(in crate::app) fn new(
        committed: String,
        tentative: String,
        levels: Vec<f32>,
        transcribing: bool,
    ) -> Self {
        Self {
            committed,
            tentative,
            levels,
            transcribing,
        }
    }

    pub(in crate::app) fn update(
        &mut self,
        committed: String,
        tentative: String,
        levels: Vec<f32>,
        transcribing: bool,
    ) {
        self.committed = committed;
        self.tentative = tentative;
        self.levels = levels;
        self.transcribing = transcribing;
    }
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
