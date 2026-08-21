use gpui::{div, prelude::*, px, rgb, Context, Div, FontWeight, Render, Stateful, Window};

use super::{BACKGROUND, BORDER, DANGER, MUTED, SIDEBAR};
use crate::app::{Page, SonarApp};

impl SonarApp {
    fn nav_item(&self, page: Page, label: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let selected = self.page == page;
        div()
            .id(label)
            .px_4()
            .py_3()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .when(selected, |item| {
                item.bg(rgb(0x302c38)).text_color(rgb(0xffffff))
            })
            .when(!selected, |item| {
                item.text_color(rgb(MUTED))
                    .hover(|style| style.bg(rgb(0x2b2930)).text_color(rgb(0xffffff)))
            })
            .on_click(cx.listener(move |app, _, _, cx| {
                app.page = page;
                app.error = None;
                cx.notify();
            }))
            .child(label)
    }
}

impl Render for SonarApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page {
            Page::Transcribe => self.home(cx),
            Page::History => self.history_page(cx),
            Page::Models => self.models_page(cx),
            Page::Settings => self.settings_page(cx),
        };
        div()
            .size_full()
            .flex()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(0xf8f7fa))
            .font_family("Inter")
            .child(
                div()
                    .w(px(224.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SIDEBAR))
                    .p_4()
                    .child(
                        div()
                            .px_3()
                            .py_4()
                            .mb_4()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("SONAR"),
                    )
                    .child(self.nav_item(Page::Transcribe, "Transcribe", cx))
                    .child(self.nav_item(Page::History, "History", cx))
                    .child(self.nav_item(Page::Models, "Models", cx))
                    .child(div().flex_1())
                    .child(self.nav_item(Page::Settings, "Settings", cx)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .p_8()
                    .flex()
                    .flex_col()
                    .when_some(self.error.clone(), |root, error| {
                        root.child(
                            div()
                                .mb_4()
                                .p_3()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(DANGER))
                                .bg(rgb(0x321d22))
                                .text_sm()
                                .text_color(rgb(0xffa2a2))
                                .child(error),
                        )
                    })
                    .child(content),
            )
    }
}
