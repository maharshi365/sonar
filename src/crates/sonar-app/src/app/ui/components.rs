use gpui::{div, prelude::*, px, rgb, Div, FontWeight, SharedString, Stateful};

use super::{BORDER, CARD, MUTED, PRIMARY, PRIMARY_HOVER};

pub(in crate::app::ui) fn header(title: &'static str, description: &'static str) -> Div {
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

pub(in crate::app::ui) fn action_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    button_base(id, label).hover(|style| style.bg(rgb(0x323038)))
}

fn button_base(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id.into())
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .cursor_pointer()
        .text_xs()
        .child(label.into())
}

pub(in crate::app::ui) fn primary_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    button_base(id, label)
        .bg(rgb(PRIMARY))
        .border_color(rgb(PRIMARY))
        .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
}

pub(in crate::app::ui) fn danger_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    action_button(id, label)
        .bg(rgb(0x3a2024))
        .border_color(rgb(0x71383e))
        .text_color(rgb(0xffaaaa))
}

pub(in crate::app::ui) fn value_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    action_button(id, label).min_w(px(140.0)).text_center()
}

pub(in crate::app::ui) fn value_pill(label: impl Into<SharedString>) -> Div {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(0x2e2c33))
        .text_xs()
        .text_color(rgb(MUTED))
        .child(label.into())
}

pub(in crate::app::ui) fn toggle_button(
    id: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    value_button(id, if enabled { "On" } else { "Off" }).when(enabled, |button| {
        button.bg(rgb(PRIMARY)).border_color(rgb(PRIMARY))
    })
}

pub(in crate::app::ui) fn setting_row(
    label: &'static str,
    description: &'static str,
    control: impl IntoElement,
) -> Div {
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

pub(in crate::app::ui) fn settings_group(title: &'static str, rows: Vec<Div>) -> Div {
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

pub(in crate::app::ui) fn empty_state(title: &'static str, description: &'static str) -> Div {
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
