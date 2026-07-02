use iced::widget::{button, column, container, row, scrollable, slider, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme};

use crate::icons;
use crate::theme::Tokens;

use super::message::Message;
use super::types::Energy;

pub(crate) fn tstyle(c: Color) -> impl Fn(&Theme) -> text::Style {
    move |_| text::Style { color: Some(c) }
}

pub(crate) fn fill_bg(c: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(c)),
        ..container::Style::default()
    }
}

pub(crate) fn primary_button(t: Tokens) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if hovered {
                Color { a: 0.85, ..t.accent }
            } else {
                t.accent
            })),
            text_color: t.bg,
            border: Border { radius: 2.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    }
}

pub(crate) fn section_label(label: &'static str, t: Tokens) -> Element<'static, Message> {
    text(label).size(11).style(tstyle(t.muted)).into()
}
/// One settings row: bold title + muted description on the left, control on the right.
pub(crate) fn sett_row<'a>(
    title: impl Into<String>,
    desc: impl Into<String>,
    t: Tokens,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        column![
            text(title.into()).size(14).style(tstyle(t.text)).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            }),
            text(desc.into()).size(11).style(tstyle(t.muted)),
        ]
        .spacing(4)
        .width(Length::Fill),
        control,
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .padding([15, 10])
    .into()
}

/// Settings category heading with a bottom separator line.
pub(crate) fn sett_panel_title<'a>(title: impl Into<String>, t: Tokens) -> Element<'a, Message> {
    column![
        container(
            text(title.into()).size(16).style(tstyle(t.text)).font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::MONOSPACE
            }),
        )
        .padding(iced::Padding { top: 16.0, right: 10.0, bottom: 12.0, left: 10.0 })
        .width(Length::Fill),
        container(text(""))
            .height(Length::Fixed(1.0))
            .width(Length::Fill)
            .style(fill_bg(t.border)),
    ]
    .spacing(0)
    .into()
}

pub(crate) fn list_row_style(t: Tokens) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered { Some(Background::Color(t.surface)) } else { None },
            text_color: t.text,
            border: Border { radius: 2.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    }
}
pub(crate) fn back_button<'a>(t: Tokens) -> Element<'a, Message> {
    button(
        row![
            icons::icon(icons::BACK, 14.0, t.muted),
            text("Back").size(12).style(tstyle(t.muted)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding(6)
    .on_press(Message::NavigateBack)
    .style(move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered { Some(Background::Color(t.surface)) } else { None },
            text_color: t.muted,
            border: Border { radius: 4.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    })
    .into()
}

pub(crate) fn setting_toggle<'a>(label: &'a str, on: bool, on_toggle: fn(bool) -> Message, t: Tokens) -> Element<'a, Message> {
    row![
        text(label).size(13).style(tstyle(t.text)).width(Length::Fill),
        toggler(on).on_toggle(on_toggle).style(toggler_style(t)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

pub(crate) fn icon_button<'a>(src: &'static str, size: f32, color: Color, t: Tokens, msg: Message) -> Element<'a, Message> {
    button(icons::icon(src, size, color))
        .padding(8)
        .on_press(msg)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                text_color: color,
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
}

/// Circular player-control button (matches the old `.ctrl-btn` style).
pub(crate) fn ctrl_button<'a>(src: &'static str, size: f32, color: Color, t: Tokens, msg: Message) -> Element<'a, Message> {
    button(icons::icon(src, size, color))
        .padding(10)
        .on_press(msg)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if hovered { Some(Background::Color(t.surface2)) } else { None },
                text_color: color,
                border: Border { radius: 100.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
}

/// Main play/pause button — always shows a circle background, accent on hover.
pub(crate) fn main_ctrl_button<'a>(src: &'static str, size: f32, t: Tokens, msg: Message) -> Element<'a, Message> {
    button(icons::icon(src, size, t.text))
        .padding(10)
        .on_press(msg)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if hovered { t.accent } else { t.surface2 })),
                text_color: if hovered { t.bg } else { t.text },
                border: Border { radius: 100.0.into(), ..Border::default() },
                ..button::Style::default()
            }
        })
        .into()
}
pub(crate) fn text_input_style(t: Tokens) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme, status| {
        let focused = matches!(status, text_input::Status::Focused { .. });
        text_input::Style {
            background: Background::Color(t.bg),
            border: Border {
                color: if focused { t.accent } else { t.border },
                width: 1.0,
                radius: 2.0.into(),
            },
            icon: t.muted,
            placeholder: t.muted,
            value: t.text,
            selection: t.accent_dim,
        }
    }
}

pub(crate) fn slider_style(t: Tokens) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_theme, status| {
        let active = matches!(status, slider::Status::Hovered | slider::Status::Dragged);
        slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    Background::Color(t.accent),
                    Background::Color(t.surface2),
                ),
                width: 4.0,
                border: Border { radius: 10.0.into(), ..Border::default() },
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius: if active { 8.5 } else { 7.0 } },
                background: Background::Color(t.accent),
                border_width: if active { 1.0 } else { 0.0 },
                border_color: if active { t.bg } else { Color::TRANSPARENT },
            },
        }
    }
}

pub(crate) fn scrollbar_width(width: u32) -> scrollable::Scrollbar {
    let scroller = (width / 2).max(3);
    scrollable::Scrollbar::new()
        .width(width)
        .margin(2)
        .scroller_width(scroller)
}

#[allow(dead_code)]
pub(crate) fn thin_scrollbar() -> scrollable::Scrollbar {
    scrollbar_width(10)
}

pub(crate) fn thin_scroll_style(t: Tokens) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |_, _| {
        let rail = scrollable::Rail {
            background: Some(Background::Color(Color { a: 0.08, ..t.muted })),
            border: Border { radius: 3.0.into(), ..Border::default() },
            scroller: scrollable::Scroller {
                background: Background::Color(Color { a: 0.55, ..t.muted }),
                border: Border { radius: 3.0.into(), ..Border::default() },
            },
        };
        scrollable::Style {
            container: container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Background::Color(t.surface),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: t.muted,
            },
        }
    }
}

pub(crate) fn toggler_style(t: Tokens) -> impl Fn(&Theme, toggler::Status) -> toggler::Style {
    move |_theme, status| {
        let on = matches!(
            status,
            toggler::Status::Active { is_toggled: true } | toggler::Status::Hovered { is_toggled: true }
        );
        toggler::Style {
            background: if on { Background::Color(t.accent) } else { Background::Color(t.surface2) },
            background_border_width: if on { 0.0 } else { 1.0 },
            background_border_color: if on { t.accent } else { t.border },
            foreground: if on { Background::Color(t.bg) } else { Background::Color(t.muted) },
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: None,
            border_radius: None,
            padding_ratio: 0.15,
        }
    }
}

pub(crate) fn stat_row(label: &'static str, val: String, t: Tokens) -> Element<'static, Message> {
    row![
        text(label).size(12).style(tstyle(t.muted)).width(Length::Fill),
        text(val).size(12).style(tstyle(t.text)),
    ]
    .into()
}

pub(crate) fn mix_button<'a>(label: &'static str, e: Energy, t: Tokens) -> Element<'a, Message> {
    button(text(label).size(14).style(tstyle(t.bg)))
        .padding(14)
        .on_press(Message::GenerateMix(e))
        .style(primary_button(t))
        .into()
}
