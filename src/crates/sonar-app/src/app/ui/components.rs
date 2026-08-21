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
    svg()
        .path(SharedString::from(format!("icons/{kind}.svg")))
        .size(px(16.0))
        .text_color(rgb(0xe7e5eb))
        .flex_none()
}

pub(in crate::app::ui) fn app_logo() -> Div {
    div()
        .size(px(34.0))
        .rounded_md()
        .bg(rgb(PRIMARY))
        .flex()
        .items_center()
        .justify_center()
        .child(icon("transcribe").size(px(20.0)).text_color(rgb(0xffffff)))
}

pub(in crate::app::ui) fn action_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    button_base(id, None, label).hover(|style| style.bg(rgb(0x323038)))
}

pub(in crate::app::ui) fn action_icon_button(
    id: impl Into<SharedString>,
    icon_name: &'static str,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    button_base(id, Some(icon(icon_name).size(px(14.0))), label)
        .hover(|style| style.bg(rgb(0x323038)))
}

fn button_base(
    id: impl Into<SharedString>,
    icon_element: Option<Svg>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
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
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .when_some(icon_element, gpui::ParentElement::child)
        .child(label.into())
}

pub(in crate::app::ui) fn primary_icon_button(
    id: impl Into<SharedString>,
    icon_name: &'static str,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    button_base(
        id,
        Some(icon(icon_name).size(px(14.0)).text_color(rgb(0xffffff))),
        label,
    )
    .bg(rgb(PRIMARY))
    .border_color(rgb(PRIMARY))
    .hover(|style| style.bg(rgb(PRIMARY_HOVER)))
}

pub(in crate::app::ui) fn danger_icon_button(
    id: impl Into<SharedString>,
    icon_name: &'static str,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    button_base(
        id,
        Some(icon(icon_name).size(px(14.0)).text_color(rgb(0xffaaaa))),
        label,
    )
    .hover(|style| style.bg(rgb(0x323038)))
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
