use gpui::{div, prelude::*, px, rgb, svg, Div, FontWeight, SharedString, Stateful, Svg};

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

pub(in crate::app::ui) fn icon(kind: &'static str) -> Svg {
    let data: &'static [u8] = match kind {
        "history" => br#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5M12 7v5l3 2"/></svg>"#,
        "models" => br#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m12 2 8 4.5v9L12 20l-8-4.5v-9L12 2Z"/><path d="m4 6.5 8 4.5 8-4.5M12 11v9"/></svg>"#,
        "settings" => br#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1a1.7 1.7 0 0 0 1.9.3A1.7 1.7 0 0 0 10 3v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z"/></svg>"#,
        "panel" => br#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="1"/><path d="M14 4v16"/></svg>"#,
        _ => br#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M4 10v4M8 7v10M12 4v16M16 7v10M20 10v4"/></svg>"#,
    };
    svg().data(data).size(px(18.0)).text_color(rgb(0xe7e5eb))
}

pub(in crate::app::ui) fn app_logo() -> Svg {
    svg()
        .data(include_bytes!("../../../../../../build/icon.svg"))
        .size(px(34.0))
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
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
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
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x17181c))
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
        .w_full()
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
                .rounded_none()
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
