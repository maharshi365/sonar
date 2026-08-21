use gpui::{div, prelude::*, px, rgb, Context, FontWeight, Render, Window};

use super::{BACKGROUND, BORDER, DANGER, GREEN, MUTED};

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

    fn waveform(&self, prefix: &str, height: f32) -> gpui::Div {
        let bars = self
            .levels
            .iter()
            .copied()
            .enumerate()
            .map(|(index, level)| {
                div()
                    .id(format!("{prefix}-level-{index}"))
                    .w(px(3.0))
                    .h(px(5.0 + level.clamp(0.0, 1.0) * height))
                    .rounded_full()
                    .bg(rgb(GREEN))
            });
        div()
            .h(px(height + 5.0))
            .flex()
            .items_center()
            .gap_1()
            .children(bars)
    }
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let transcript = if self.committed.is_empty() && self.tentative.is_empty() {
            "Start speaking...".to_owned()
        } else {
            format!("{}{}", self.committed, self.tentative)
        };
        let state = if self.transcribing {
            "Transcribing"
        } else {
            "Recording"
        };

        div()
            .size_full()
            .bg(rgb(BACKGROUND))
            .p_4()
            .font_family("Inter")
            .text_color(rgb(0xf4f2f7))
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .w(px(126.0))
                    .h(px(37.0))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(0x07080a))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().size_2().rounded_full().bg(rgb(DANGER)))
                    .child(self.waveform("compact", 23.0)),
            )
            .child(
                div()
                    .flex_1()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(0x07080a))
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(53.0))
                            .px_5()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .child("LIVE TRANSCRIPT"),
                            )
                            .child(div().size_2().rounded_full().bg(rgb(DANGER)))
                            .child(div().text_color(rgb(MUTED)).child(state)),
                    )
                    .child(
                        div()
                            .p_5()
                            .text_sm()
                            .line_height(px(24.0))
                            .text_color(rgb(if self.committed.is_empty() { MUTED } else { 0xf4f2f7 }))
                            .child(transcript),
                    ),
            )
            .child(
                div()
                    .h(px(64.0))
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(0x07080a))
                    .px_4()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(div().size_2().rounded_full().bg(rgb(DANGER)))
                    .child(self.waveform("status", 30.0))
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .line_height(px(20.0))
                            .child("Transcribing speech locally with ultra-low latency directly into your active window..."),
                    )
                    .child(
                        div()
                            .size(px(34.0))
                            .rounded_full()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(MUTED))
                            .child("^"),
                    ),
            )
    }
}
