use iced::widget::{button, column, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::App;

impl App {
    pub(crate) fn genre_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let name = self.genre_detail_name.clone().unwrap_or_default();
        if self.genre_songs.is_empty() {
            return column![back_button(t), text("Loading…").size(13).style(tstyle(t.muted))]
                .spacing(12)
                .into();
        }
        let play = button(
            row![
                icons::icon(icons::PLAY, 14.0, t.bg),
                text("Play all").size(12).style(tstyle(t.bg)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .on_press(Message::PlayGenreAt(0))
        .style(primary_button(t));

        let mut list = column![].spacing(2);
        for (i, song) in self.genre_songs.iter().enumerate() {
            list = list.push(self.track_row(i, song, Message::PlayGenreAt(i)));
        }

        column![
            back_button(t),
            text(name).size(24).style(tstyle(t.text)),
            text(format!("{} songs", self.genre_songs.len())).size(11).style(tstyle(t.muted)),
            play,
            scrollable(list).height(Length::Fill).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)),
        ]
        .spacing(12)
        .into()
    }
}
