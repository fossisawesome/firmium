use iced::widget::{button, checkbox, column, container, row, scrollable, stack, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length};

use firmium_backend::config::SavedAccount;
use firmium_backend::commands::subsonic::RemotePlayQueue;
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::App;

impl App {
    pub(crate) fn add_to_playlist_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let Some(song) = &self.add_to_playlist_song else {
            return container(text("")).into();
        };

        // Click-catching dim backdrop; taps outside the card dismiss the modal.
        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::CloseAddToPlaylist)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let close = with_tooltip(icon_button(icons::CLOSE, 16.0, t.muted, t, spotify, Message::CloseAddToPlaylist), "Close", t);
        let header = row![
            text("Add to Playlist").size(16).style(tstyle(t.text)).width(Length::Fill),
            close,
        ]
        .align_y(Alignment::Center);

        let subtitle = text(song.title.clone()).size(12).style(tstyle(t.muted));

        let create_row = row![
            text_input("New playlist name…", &self.new_playlist_name)
                .on_input(Message::NewPlaylistNameInput)
                .on_submit(Message::CreatePlaylistAndAdd)
                .padding(8)
                .size(13)
                .width(Length::Fill)
                .style(text_input_style(t, spotify)),
            button(icons::icon(icons::PLUS, 16.0, t.bg))
                .padding(8)
                .on_press(Message::CreatePlaylistAndAdd)
                .style(primary_button(t, spotify)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut list = column![].spacing(2);
        if self.playlists.is_empty() {
            list = list.push(text("No playlists yet").size(12).style(tstyle(t.muted)));
        } else {
            for p in &self.playlists {
                let id = p.id.clone();
                let name = p.name.clone();
                let count = p.tracks.len();
                let synced = p.server_id.is_some();
                let mut label = row![
                    icons::icon(icons::LIST, 16.0, t.muted),
                    text(name).size(13).style(tstyle(t.text)).width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center);
                if synced {
                    label = label.push(icons::icon(icons::CLOUD, 12.0, t.muted));
                }
                label = label.push(text(format!("{count}")).size(11).style(tstyle(t.muted)));
                list = list.push(
                    button(label)
                        .width(Length::Fill)
                        .padding(8)
                        .on_press(Message::AddToPlaylist(id))
                        .style(list_row_style(t, spotify)),
                );
            }
        }

        let card = container(
            column![
                header,
                subtitle,
                create_row,
                text("Your playlists").size(11).style(tstyle(t.muted)),
                scrollable(list).height(Length::Fixed(260.0)).direction(scrollable::Direction::Vertical(self.make_scrollbar())).style(thin_scroll_style(t)),
            ]
            .spacing(14),
        )
        .width(Length::Fixed(420.0))
        .padding(20)
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

    pub(crate) fn create_playlist_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let can_create = !self.create_playlist_name.trim().is_empty();

        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::CloseCreatePlaylist)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let create_msg = can_create.then(|| Message::CreatePlaylist(self.create_playlist_name.clone()));
        let card = container(
            column![
                text("New Playlist").size(16).style(tstyle(t.text)),
                text_input("Playlist name…", &self.create_playlist_name)
                    .on_input(Message::CreatePlaylistNameInput)
                    .on_submit(Message::CreatePlaylist(self.create_playlist_name.clone()))
                    .padding(10)
                    .size(13)
                    .style(text_input_style(t, spotify)),
                row![
                    button(text("Cancel").size(13).style(tstyle(t.muted)))
                        .padding(8)
                        .on_press(Message::CloseCreatePlaylist)
                        .style(list_row_style(t, spotify)),
                    button(text("Create").size(13).style(tstyle(if can_create { t.bg } else { t.muted })))
                        .padding([8, 16])
                        .on_press_maybe(create_msg)
                        .style(primary_button(t, spotify)),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ]
            .spacing(16),
        )
        .width(Length::Fixed(360.0))
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

    pub(crate) fn account_switcher_overlay(&self) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";

        let backdrop = button(container(text("")).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::ToggleAccountSwitcher)
            .style(|_th, _status| button::Style {
                background: Some(Background::Color(Color { a: 0.55, ..Color::BLACK })),
                ..button::Style::default()
            });

        let card: Element<'_, Message> = if self.authed {
            let (cur_server, cur_username) = {
                let conn = self.backend.app_state.connection.read();
                (conn.server.clone().unwrap_or_default(), conn.username.clone().unwrap_or_default())
            };
            let server_display = cur_server
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/')
                .to_string();
            let other_accounts: Vec<&SavedAccount> = self
                .accounts
                .iter()
                .filter(|a| a.server != cur_server || a.username != cur_username)
                .collect();

            let disconnect_btn = button(text("DISCONNECT").size(13))
                .on_press(Message::Logout)
                .padding(14)
                .width(Length::Fixed(320.0))
                .style(move |_, status| {
                    use iced::widget::button::Status;
                    let bg = match status {
                        Status::Hovered | Status::Pressed => Color {
                            r: t.error.r * 0.85,
                            g: t.error.g * 0.85,
                            b: t.error.b * 0.85,
                            ..t.error
                        },
                        _ => t.error,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::BLACK,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                });

            let close = with_tooltip(icon_button(icons::CLOSE, 16.0, t.muted, t, spotify, Message::ToggleAccountSwitcher), "Close", t);
            let header = row![
                text("Connected").size(26).style(tstyle(t.accent)).width(Length::Fill),
                close,
            ]
            .align_y(Alignment::Center);

            let mut card_col = column![
                header,
                text(server_display).size(13).style(tstyle(t.muted)),
            ]
            .spacing(20)
            .align_x(Alignment::Start);

            if !other_accounts.is_empty() {
                let mut switch_col = column![text("SWITCH ACCOUNT").size(11).style(tstyle(t.muted))].spacing(8);
                for acct in &other_accounts {
                    switch_col = switch_col.push(self.saved_account_row(acct));
                }
                card_col = card_col.push(switch_col);
            }
            card_col = card_col.push(disconnect_btn);

            container(card_col)
                .width(Length::Fixed(400.0))
                .padding(40)
                .style(move |_th| container::Style {
                    background: Some(Background::Color(t.surface)),
                    border: Border { radius: 10.0.into(), width: 1.0, color: t.border },
                    ..container::Style::default()
                })
                .into()
        } else {
            let mut card_col = column![].spacing(20).align_x(Alignment::Start);
            if !self.accounts.is_empty() {
                let mut switch_col = column![text("SAVED ACCOUNTS").size(11).style(tstyle(t.muted))].spacing(8);
                for acct in &self.accounts {
                    switch_col = switch_col.push(self.saved_account_row(acct));
                }
                card_col = card_col.push(switch_col);
                card_col = card_col.push(text("OR CONNECT TO A NEW SERVER").size(11).style(tstyle(t.muted)));
            }

            let save_pw_row = row![
                checkbox(self.save_password)
                    .on_toggle(Message::ToggleSavePassword)
                    .style(move |_, status| {
                        use iced::widget::checkbox::{Status, Style};
                        let checked = matches!(status, Status::Active { is_checked: true } | Status::Hovered { is_checked: true });
                        Style {
                            background: Background::Color(if checked { t.accent } else { t.surface }),
                            icon_color: t.bg,
                            border: Border { color: if checked { t.accent } else { t.border }, width: 1.0, radius: 3.0.into() },
                            text_color: None,
                        }
                    }),
                text("SAVE PASSWORD").size(11).style(tstyle(t.muted)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fixed(320.0));

            let form = column![
                text_input("https://music.example.com", &self.server_input)
                    .on_input(Message::ServerInput)
                    .padding(10)
                    .width(Length::Fixed(320.0))
                    .style(text_input_style(t, spotify)),
                text_input("username", &self.username_input)
                    .on_input(Message::UsernameInput)
                    .padding(10)
                    .width(Length::Fixed(320.0))
                    .style(text_input_style(t, spotify)),
                text_input("password", &self.password_input)
                    .on_input(Message::PasswordInput)
                    .secure(true)
                    .padding(10)
                    .width(Length::Fixed(320.0))
                    .style(text_input_style(t, spotify)),
                save_pw_row,
                button(text("CONNECT").size(13))
                    .on_press(Message::Connect)
                    .padding(14)
                    .width(Length::Fixed(320.0))
                    .style(primary_button(t, spotify)),
            ]
            .spacing(12)
            .align_x(Alignment::Start);
            card_col = card_col.push(form);

            container(card_col)
                .width(Length::Fixed(400.0))
                .padding(40)
                .style(move |_| container::Style {
                    background: Some(Background::Color(t.surface)),
                    border: Border { color: t.border, width: 1.0, radius: 10.0.into() },
                    ..container::Style::default()
                })
                .into()
        };

        stack![
            backdrop,
            container(card).center_x(Length::Fill).center_y(Length::Fill),
        ]
        .into()
    }

    pub(crate) fn saved_account_row(&self, acct: &SavedAccount) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let server_display = acct
            .server
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        button(
            column![
                text(acct.username.clone()).size(13).style(tstyle(t.text)),
                text(server_display).size(11).style(tstyle(t.muted)),
            ]
            .spacing(2),
        )
        .width(Length::Fixed(320.0))
        .padding(10)
        .on_press(Message::SwitchAccount(acct.clone()))
        .style(list_row_style(t, spotify))
        .into()
    }

    pub(crate) fn resume_banner(&self, q: &RemotePlayQueue) -> Element<'_, Message> {
        let t = self.tokens;
        let spotify = self.ui_theme_id == "spotify";
        let track = q
            .current
            .as_deref()
            .and_then(|cur| q.entries.iter().find(|s| s.id == cur))
            .or_else(|| q.entries.first());
        let label = match track {
            Some(s) => format!("Resume “{}” — {}", s.title, s.artist),
            None => "Resume your last queue".to_string(),
        };

        let resume = button(text("Resume").size(12).style(tstyle(t.bg)))
            .padding([6, 14])
            .on_press(Message::ResumeQueue)
            .style(primary_button(t, spotify));
        let dismiss = button(text("Dismiss").size(12).style(tstyle(t.muted)))
            .padding([6, 12])
            .on_press(Message::DismissResume)
            .style(move |_th, status| {
                let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: if h { Some(Background::Color(t.surface2)) } else { None },
                    text_color: t.muted,
                    border: Border { radius: 4.0.into(), ..Border::default() },
                    ..button::Style::default()
                }
            });

        container(
            row![
                icons::icon(icons::QUEUE, 16.0, t.accent),
                text(label).size(13).style(tstyle(t.text)).width(Length::Fill),
                resume,
                dismiss,
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([10, 16])
        .style(move |_th| container::Style {
            background: Some(Background::Color(t.surface)),
            border: Border { width: 1.0, color: t.accent, radius: 0.0.into() },
            ..container::Style::default()
        })
        .into()
    }
}
