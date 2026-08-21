use gpui::{div, prelude::*, px, rgb, AnyElement, Context, FontWeight};

use super::super::{BORDER, CARD, DANGER, MUTED, PRIMARY, PRIMARY_HOVER};
use crate::app::{RecordingState, SonarApp};

impl SonarApp {
    pub(in crate::app::ui) fn home(&self, cx: &mut Context<Self>) -> AnyElement {
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
}
