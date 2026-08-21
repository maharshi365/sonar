use gpui::{div, prelude::*, px, rgb, Context, Div, FontWeight, Render, Stateful, Window};

use super::{
    components::{app_logo, icon},
    BACKGROUND, DANGER, MUTED, SIDEBAR,
};
use crate::app::{Page, SonarApp};

impl SonarApp {
    fn nav_item(&self, page: Page, label: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let selected = self.page == page;
        let icon_name = match page {
            Page::Transcribe => "transcribe",
            Page::History => "history",
            Page::Models => "models",
            Page::Settings => "settings",
        };
        div()
            .id(label)
            .px_2()
            .h(px(34.0))
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .when(selected, |item| {
                item.bg(rgb(0x292a2e)).text_color(rgb(0xffffff))
            })
            .when(!selected, |item| {
                item.text_color(rgb(MUTED))
                    .hover(|style| style.bg(rgb(0x232429)).text_color(rgb(0xffffff)))
            })
            .on_click(cx.listener(move |app, _, _, cx| {
                app.page = page;
                app.error = None;
                cx.notify();
            }))
            .child(icon(icon_name))
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
                    .w(px(223.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(SIDEBAR))
                    .p_2()
                    .child(
                        div()
                            .h(px(57.0))
                            .px_2()
                            .mb_2()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(app_logo())
                            .child(div().text_sm().font_weight(FontWeight::BOLD).child("SONAR"))
                            .child(div().flex_1())
                            .child(icon("panel")),
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
                    .pt(px(44.0))
                    .px(px(48.0))
                    .pb(px(32.0))
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
