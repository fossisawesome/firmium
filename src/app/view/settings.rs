use iced::widget::{button, column, container, pick_list, row, scrollable, slider, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::commands::themes::ThemeEntry;
use crate::fonts::FONT_OPTIONS;
use crate::theme::Tokens;
use crate::icons;

use super::super::message::Message;
use super::super::styles::*;
use super::super::format::*;
use super::super::types::*;
use super::super::App;

impl App {
    pub(crate) fn settings_view(&self) -> Element<'_, Message> {
        let t = self.tokens;

        // Left rail: category nav.
        let cats = [
            (SettingsCategory::Appearance, icons::PALETTE, "Appearance"),
            (SettingsCategory::Playback, icons::PLAY, "Playback"),
            (SettingsCategory::Equalizer, icons::EQUALIZER, "Equalizer"),
            (SettingsCategory::Downloads, icons::DOWNLOAD, "Downloads"),
            (SettingsCategory::Services, icons::GLOBE, "Services"),
            (SettingsCategory::Account, icons::USER, "Account"),
            (SettingsCategory::Debug, icons::INFO, "Debug"),
        ];
        let mut nav = column![text("SETTINGS").size(11).style(tstyle(t.muted))]
            .spacing(2)
            .padding([4, 8]);
        for (cat, icon_src, label_str) in cats {
            let active = self.settings_category == cat;
            nav = nav.push(
                button(
                    row![
                        icons::icon(icon_src, 16.0, if active { t.accent } else { t.muted }),
                        text(label_str).size(13).style(tstyle(if active { t.accent } else { t.text })),
                    ]
                    .spacing(9)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([7, 10])
                .on_press(Message::SetSettingsCategory(cat))
                .style(move |_theme, status| {
                    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if active {
                            t.accent_dim
                        } else if hovered {
                            t.surface
                        } else {
                            Color::TRANSPARENT
                        })),
                        text_color: if active { t.accent } else { t.text },
                        border: Border { radius: 6.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                }),
            );
        }
        let sidebar = container(nav)
            .width(Length::Fixed(180.0))
            .height(Length::Fill)
            .style(fill_bg(t.bg));

        let sep = container(text(""))
            .width(Length::Fixed(1.0))
            .height(Length::Fill)
            .style(fill_bg(t.border));

        let panel = scrollable(match self.settings_category {
            SettingsCategory::Appearance => self.settings_appearance(t),
            SettingsCategory::Playback => self.settings_playback(t),
            SettingsCategory::Equalizer => self.settings_equalizer(t),
            SettingsCategory::Downloads => self.settings_downloads(t),
            SettingsCategory::Services => self.settings_services(t),
            SettingsCategory::Account => self.settings_account(t),
            SettingsCategory::Debug => self.settings_debug(t),
        })
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(thin_scrollbar()))
        .style(thin_scroll_style(t));

        row![sidebar, sep, container(panel).padding([0, 4]).width(Length::Fill)]
            .height(Length::Fill)
            .into()
    }

    pub(crate) fn settings_appearance(&self, t: Tokens) -> Element<'_, Message> {
        let selected = self.themes.iter().find(|e| e.id == self.theme_id).cloned();
        let theme_picker = pick_list(self.themes.clone(), selected, |entry: ThemeEntry| {
            Message::SelectTheme(entry.id)
        })
        .width(Length::Fixed(200.0))
        .into();
        let font_selected = FONT_OPTIONS.iter().find(|f| **f == self.font_family.as_str()).copied();
        let font_picker = column![
            pick_list(FONT_OPTIONS, font_selected, |name: &str| {
                Message::SelectFont(name.to_string())
            })
            .width(Length::Fixed(200.0)),
            text("Restart to apply").size(11).style(tstyle(t.muted)),
        ]
        .spacing(4)
        .align_x(Alignment::End)
        .into();
        column![
            sett_panel_title("Appearance", t),
            sett_row(
                "Window Decorations",
                "Show native title bar and borders",
                t,
                toggler(self.window_decorations).on_toggle(Message::SetDecorations).style(toggler_style(t)).into(),
            ),
            sett_row(
                "Cover-Colored Visualizer",
                "Tint the visualizer with the current album's artwork. When off, it follows your theme colors.",
                t,
                toggler(self.viz_cover_colors).on_toggle(Message::SetVizCoverColors).style(toggler_style(t)).into(),
            ),
            sett_row("Theme", "Color scheme for the interface", t, theme_picker),
            sett_row("Font", "Interface font, applies after restart", t, font_picker),
            sett_row(
                "Scrollbar Width",
                "Adjust scrollbar thickness (6-20px)",
                t,
                row![
                    slider(6.0..=20.0, self.scrollbar_width as f32, |v| {
                        Message::SetScrollbarWidth(v as u32)
                    })
                    .width(Length::Fixed(150.0))
                    .style(slider_style(t)),
                    text(format!("{} px", self.scrollbar_width))
                        .size(12)
                        .style(tstyle(t.text))
                        .width(Length::Fixed(50.0)),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
            ),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_playback(&self, t: Tokens) -> Element<'_, Message> {
        let bp = |label: &'static str, mode: &'static str| -> Element<'_, Message> {
            let active = self.bit_perfect_mode == mode;
            button(text(label).size(12).style(tstyle(if active { t.bg } else { t.text })))
                .padding(8)
                .on_press(Message::SetBitPerfect(mode.to_string()))
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if active {
                            t.accent
                        } else if h {
                            t.surface
                        } else {
                            t.surface2
                        })),
                        text_color: if active { t.bg } else { t.text },
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        let crossfade_dur: Element<'_, Message> = if self.crossfade_enabled {
            sett_row(
                "Crossfade Duration",
                "Length of the blend in seconds",
                t,
                row![
                    slider(1.0..=12.0, self.crossfade_duration, Message::SetCrossfadeDuration)
                        .step(1.0)
                        .width(Length::Fixed(100.0))
                        .style(slider_style(t)),
                    text(format!("{:.0}s", self.crossfade_duration)).size(12).style(tstyle(t.muted)),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            )
        } else {
            column![].into()
        };
        column![
            sett_panel_title("Playback", t),
            sett_row("Crossfade", "Smoothly blend between tracks", t,
                toggler(self.crossfade_enabled).on_toggle(Message::SetCrossfadeEnabled).style(toggler_style(t)).into()),
            crossfade_dur,
            sett_row("Gapless Playback", "Pre-buffer the next track for seamless transitions", t,
                toggler(self.gapless_enabled).on_toggle(Message::SetGapless).style(toggler_style(t)).into()),
            sett_row("ReplayGain", "Normalize track loudness using server-provided gain values", t,
                toggler(self.replay_gain_enabled).on_toggle(Message::SetReplayGain).style(toggler_style(t)).into()),
            sett_row("Continue playing after queue ends", "Smart Radio keeps the music going by adding similar tracks when the queue runs out", t,
                toggler(self.auto_continue).on_toggle(Message::SetAutoContinue).style(toggler_style(t)).into()),
            sett_row("Bit-Perfect Audio", "Matches native sample rate; crossfade is disabled", t,
                row![bp("Off", "off"), bp("Relaxed", "relaxed"), bp("Strict", "strict")].spacing(4).into()),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_equalizer(&self, t: Tokens) -> Element<'_, Message> {
        column![
            sett_panel_title("Equalizer", t),
            sett_row(
                "Graphic Equalizer",
                "Open the multi-band EQ in the side panel",
                t,
                button(text("Open Equalizer").size(13).style(tstyle(t.text)))
                    .padding(10)
                    .on_press(Message::TogglePanel(Panel::Equalizer))
                    .style(move |_t, status| {
                        let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style {
                            background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                            text_color: t.text,
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..button::Style::default()
                        }
                    })
                    .into(),
            ),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_downloads(&self, t: Tokens) -> Element<'_, Message> {
        fn fmt_label(id: &str) -> &'static str {
            match id {
                "mp3" => "MP3",
                "flac" => "FLAC",
                "wav" => "WAV",
                "opus" => "Opus",
                _ => "Original",
            }
        }
        let selected = fmt_label(&self.download_format);
        let fmt_picker = pick_list(
            ["Original", "MP3", "FLAC", "WAV", "Opus"],
            Some(selected),
            |label: &'static str| {
                let id = match label {
                    "MP3" => "mp3",
                    "FLAC" => "flac",
                    "WAV" => "wav",
                    "Opus" => "opus",
                    _ => "raw",
                };
                Message::SetDownloadFormat(id.to_string())
            },
        )
        .width(Length::Fixed(200.0))
        .into();
        column![
            sett_panel_title("Downloads", t),
            sett_row(
                "Download Format",
                "Format used when downloading tracks and albums. \"Original\" saves the file exactly as stored on the server.",
                t,
                fmt_picker,
            ),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_services(&self, t: Tokens) -> Element<'_, Message> {
        let mut col = column![sett_panel_title("Services", t)].spacing(0);
        col = col.push(sett_row(
            "Last.fm Integration",
            "Fetch richer artist bio and photo using your own Last.fm API key",
            t,
            toggler(self.lastfm_enabled).on_toggle(Message::SetLastfmEnabled).style(toggler_style(t)).into(),
        ));
        if self.lastfm_enabled {
            col = col.push(sett_row(
                "Last.fm API Key",
                "From your Last.fm API account",
                t,
                text_input("API key…", &self.lastfm_key)
                    .on_input(Message::SetLastfmKey)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .style(text_input_style(t))
                    .into(),
            ));
            col = col.push(sett_row(
                "Last.fm Secret",
                "Shared secret for your API account",
                t,
                text_input("Secret…", &self.lastfm_secret)
                    .on_input(Message::SetLastfmSecret)
                    .secure(true)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .style(text_input_style(t))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "ListenBrainz Scrobbling",
            "Submit each completed track to ListenBrainz using your user token",
            t,
            toggler(self.listenbrainz_enabled).on_toggle(Message::SetListenbrainzEnabled).style(toggler_style(t)).into(),
        ));
        if self.listenbrainz_enabled {
            col = col.push(sett_row(
                "ListenBrainz Token",
                "From your ListenBrainz profile settings",
                t,
                text_input("User token…", &self.listenbrainz_token)
                    .on_input(Message::SetListenbrainzToken)
                    .secure(true)
                    .padding([6, 10])
                    .width(Length::Fixed(220.0))
                    .style(text_input_style(t))
                    .into(),
            ));
        }
        col = col.push(sett_row(
            "External Lyrics (LRCLIB)",
            "Fetch synced lyrics from lrclib.net when your server has none. Sends song title and artist name.",
            t,
            toggler(self.lrclib_enabled).on_toggle(Message::SetLrclibEnabled).style(toggler_style(t)).into(),
        ));
        col = col.push(sett_row(
            "Word-by-Word Lyrics Animation",
            "Karaoke-style fill on the active lyric line, with per-word timing estimated from the line's timestamps. Disable for plain line-by-line highlighting.",
            t,
            toggler(self.lyrics_word_fill).on_toggle(Message::SetLyricsWordFill).style(toggler_style(t)).into(),
        ));
        col.into()
    }

    pub(crate) fn settings_account(&self, t: Tokens) -> Element<'_, Message> {
        let (server, username) = {
            let conn = self.backend.app_state.connection.read();
            (conn.server.clone().unwrap_or_default(), conn.username.clone().unwrap_or_default())
        };
        let conn_desc = if self.authed {
            format!("{username} @ {server}")
        } else {
            "Not connected".to_string()
        };
        let conn_btn: Element<'_, Message> = if self.authed {
            button(text("Log out").size(13).style(tstyle(t.error)))
                .padding(10)
                .on_press(Message::Logout)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: if h { Some(Background::Color(t.surface)) } else { None },
                        text_color: t.error,
                        border: Border { color: t.error, width: 1.0, radius: 4.0.into() },
                        ..button::Style::default()
                    }
                })
                .into()
        } else {
            button(text("Connect").size(13).style(tstyle(t.text)))
                .padding(10)
                .on_press(Message::ToggleAccountSwitcher)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };

        let sec_btn = |label: &'static str, msg: Message| -> Element<'_, Message> {
            button(text(label).size(13).style(tstyle(t.text)))
                .padding(10)
                .on_press(msg)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: t.text,
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        let stats_section: Element<'_, Message> = match &self.history_summary {
            Some(s) if s.total_plays > 0 => column![
                stat_row("Total plays", s.total_plays.to_string(), t),
                stat_row("Listening time", fmt_hours(s.total_seconds), t),
                stat_row("Unique tracks", s.unique_tracks.to_string(), t),
                stat_row("Unique artists", s.unique_artists.to_string(), t),
                stat_row("Unique albums", s.unique_albums.to_string(), t),
                row![
                    sec_btn("Export CSV", Message::ExportStats("csv".to_string())),
                    sec_btn("Export JSON", Message::ExportStats("json".to_string())),
                    sec_btn("View Recap", Message::Navigate(View::Recap)),
                ]
                .spacing(8),
            ]
            .spacing(10)
            .into(),
            _ => text("No play history yet — start listening to build your stats.")
                .size(12)
                .style(tstyle(t.muted))
                .into(),
        };

        column![
            sett_panel_title("Account", t),
            sett_row("Connection", conn_desc, t, conn_btn),
            sett_panel_title("Listening Stats", t),
            container(stats_section).padding([15, 10]),
        ]
        .spacing(0)
        .into()
    }

    pub(crate) fn settings_debug(&self, t: Tokens) -> Element<'_, Message> {
        let version = crate::commands::app_info::get_app_version();
        let debug_btn = |label: &'static str, msg: Message, danger: bool| -> Element<'_, Message> {
            button(text(label).size(13).style(tstyle(if danger { t.error } else { t.text })))
                .padding(10)
                .on_press(msg)
                .style(move |_t, status| {
                    let h = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if h { t.surface } else { t.surface2 })),
                        text_color: if danger { t.error } else { t.text },
                        border: Border {
                            color: if danger { t.error } else { Color::TRANSPARENT },
                            width: if danger { 1.0 } else { 0.0 },
                            radius: 4.0.into(),
                        },
                        ..button::Style::default()
                    }
                })
                .into()
        };
        column![
            sett_panel_title("Debug", t),
            sett_row("App Version", version, t, text("").into()),
            sett_row("Wipe Cache", "Clear cached cover art", t,
                debug_btn("Wipe", Message::WipeCoverCache, false)),
            sett_row("Delete Settings", "Reset all preferences to defaults", t,
                debug_btn("Delete", Message::DeleteSettings, true)),
        ]
        .spacing(0)
        .into()
    }
}
