use iced::widget::{column, row, text};
use iced::Element;


use super::super::message::Message;
use super::super::styles::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn mix_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        column![
            page_header("Mix", t, spotify),
            text("Generate a shuffled queue by energy level").size(12).style(tstyle(t.muted)),
            row![
                mix_button("Chill", Energy::Chill, t, spotify),
                mix_button("Mid", Energy::Mid, t, spotify),
                mix_button("High", Energy::High, t, spotify),
            ]
            .spacing(12),
        ]
        .spacing(16)
        .into()
    }
}
