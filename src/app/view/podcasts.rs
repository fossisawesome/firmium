use iced::widget::{button, column, container, row, scrollable, stack, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn podcasts_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let header = row![
            container(page_header(format!("Podcasts ({})", self.podcast_channels.len()), t, spotify))
                .width(Length::Fill),
            button(
                row![icons::icon(icons::PLUS, 12.0, t.accent), text("Add podcast").size(12).style(tstyle(t.accent))]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
            .padding([6, 14])
            .on_press(Message::OpenAddPodcastModal)
            .style(list_row_style(t)),
        ]
        .align_y(Alignment::Center);

        if self.podcast_channels.is_empty() {
            return column![
                header,
                text("No podcasts yet. Add one by RSS feed URL.").size(13).style(tstyle(t.muted))
            ]
            .spacing(16)
            .into();
        }

        let mut list = column![].spacing(2);
        for channel in &self.podcast_channels {
            let mut title_text = text(&channel.title).size(if spotify { 15 } else { 14 }).style(tstyle(t.text));
            if spotify {
                title_text = title_text.font(iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::MONOSPACE });
            }
            list = list.push(
                button(
                    column![
                        title_text,
                        text(channel.description.clone().unwrap_or_default()).size(12).style(tstyle(t.muted)),
                    ]
                    .spacing(4),
                )
                .width(Length::Fill)
                .padding(if spotify { 12 } else { 10 })
                .on_press(Message::Navigate(View::PodcastDetail(channel.id.clone())))
                .style(list_row_style(t)),
            );
        }

        column![
            header,
            scrollable(list)
                .height(Length::Fill)
                .direction(scrollable::Direction::Vertical(self.make_scrollbar()))
                .style(thin_scroll_style(t))
        ]
        .spacing(16)
        .into()
    }

    pub(crate) fn podcast_detail_view(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let View::PodcastDetail(channel_id) = self.view.clone() else {
            return text("No channel selected").size(13).style(tstyle(t.muted)).into();
        };

        let channel = self.podcast_channels.iter().find(|c| c.id == channel_id);
        let channel_title = channel.map(|c| c.title.clone()).unwrap_or_default();
        let feed_url = channel.map(|c| c.feed_url.clone()).unwrap_or_default();

        let header = row![
            text(channel_title).size(20).style(tstyle(t.text)).width(Length::Fill),
            button(text("Refresh").size(12).style(tstyle(t.text)))
                .padding([6, 12])
                .on_press(Message::RefreshPodcastChannel(channel_id.clone(), feed_url))
                .style(list_row_style(t)),
            button(text("Unsubscribe").size(12).style(tstyle(t.text)))
                .padding([6, 12])
                .on_press(Message::UnsubscribePodcastChannel(channel_id.clone()))
                .style(list_row_style(t)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if self.podcast_episodes.is_empty() {
            return column![header, text("No episodes found.").size(13).style(tstyle(t.muted))]
                .spacing(16)
                .into();
        }

        let mut list = column![].spacing(2);
        for episode in &self.podcast_episodes {
            let duration_label = episode
                .duration_seconds
                .map(|s| fmt_time(s as f64))
                .unwrap_or_default();
            list = list.push(
                row![
                    column![
                        text(&episode.title).size(13).style(tstyle(t.text)),
                        text(duration_label).size(11).style(tstyle(t.muted)),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(icons::icon(icons::PLAY, 14.0, t.accent))
                        .padding(8)
                        .on_press(Message::PlayPodcastEpisode(episode.clone()))
                        .style(list_row_style(t)),
                ]
                .spacing(12)
                .padding(10)
                .align_y(Alignment::Center),
            );
        }

        column![
            header,
            scrollable(list)
                .height(Length::Fill)
                .direction(scrollable::Direction::Vertical(self.make_scrollbar()))
                .style(thin_scroll_style(t))
        ]
        .spacing(16)
        .into()
    }

    pub(crate) fn add_podcast_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let can_add = !self.podcast_add_url_input.trim().is_empty();

        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::CloseAddPodcastModal)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let add_msg = can_add.then_some(Message::SubmitAddPodcastChannel);
        let mut card_col = column![
            text("Add a podcast").size(16).style(tstyle(t.text)),
            text_input("RSS feed URL…", &self.podcast_add_url_input)
                .on_input(Message::PodcastAddUrlChanged)
                .on_submit(Message::SubmitAddPodcastChannel)
                .padding(10)
                .size(13)
                .style(text_input_style(t)),
        ]
        .spacing(16);

        if let Some(err) = &self.podcast_add_error {
            card_col = card_col.push(text(err).size(12).style(tstyle(t.muted)));
        }

        card_col = card_col.push(
            row![
                button(text("Cancel").size(13).style(tstyle(t.muted)))
                    .padding(8)
                    .on_press(Message::CloseAddPodcastModal)
                    .style(list_row_style(t)),
                button(text("Add").size(13).style(tstyle(if can_add { t.bg } else { t.muted })))
                    .padding([8, 16])
                    .on_press_maybe(add_msg)
                    .style(primary_button(t)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );

        let card = container(card_col)
            .width(Length::Fixed(420.0))
            .padding(24)
            .style(move |_th| container::Style {
                background: Some(Background::Color(t.surface)),
                border: Border { radius: 10.0.into(), width: 1.0, color: t.border },
                ..container::Style::default()
            });

        stack![
            backdrop,
            container(card).center_x(Length::Fill).center_y(Length::Fill),
        ]
        .into()
    }
}
