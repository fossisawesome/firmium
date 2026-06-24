//! Theme tokens parsed from the TOML themes (`commands/themes.rs`) into iced
//! `Color`s, plus a derived `iced::Theme` for default widget colors. The 11
//! tokens mirror the CSS custom properties the Svelte UI used.

use iced::Color;

use crate::commands::themes::ThemeEntry;

/// Parsed color tokens for the active theme. `Copy` so it can be moved into the
/// per-widget styling closures `view()` builds.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub bg: Color,
    pub surface: Color,
    pub surface2: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub error: Color,
    #[allow(dead_code)]
    pub is_dark: bool,
}

impl Default for Tokens {
    fn default() -> Self {
        Self::firmium()
    }
}

impl Tokens {
    /// The built-in default ("firmium") palette, used as a fallback.
    pub fn firmium() -> Self {
        Tokens {
            bg: rgb(0x0f, 0x0f, 0x0f),
            surface: rgb(0x1a, 0x1a, 0x1a),
            surface2: rgb(0x24, 0x24, 0x24),
            border: rgba(255, 255, 255, 0.08),
            text: rgb(0xf0, 0xf0, 0xf0),
            muted: rgb(0xa0, 0xa0, 0xa0),
            accent: rgb(0xe8, 0xc9, 0x7e),
            accent_dim: rgba(232, 201, 126, 0.15),
            error: rgb(0xe0, 0x60, 0x60),
            is_dark: true,
        }
    }

    /// Build tokens from a loaded theme entry, falling back to firmium values
    /// for any token that fails to parse.
    pub fn from_entry(entry: &ThemeEntry) -> Self {
        let f = Self::firmium();
        let c = &entry.colors;
        Tokens {
            bg: parse_color(&c.bg).unwrap_or(f.bg),
            surface: parse_color(&c.surface).unwrap_or(f.surface),
            surface2: parse_color(&c.surface2).unwrap_or(f.surface2),
            border: parse_color(&c.border).unwrap_or(f.border),
            text: parse_color(&c.text).unwrap_or(f.text),
            muted: parse_color(&c.muted).unwrap_or(f.muted),
            accent: parse_color(&c.accent).unwrap_or(f.accent),
            accent_dim: parse_color(&c.accent_dim).unwrap_or(f.accent_dim),
            error: parse_color(&c.error).unwrap_or(f.error),
            is_dark: entry.color_scheme != "light",
        }
    }

    /// Override the accent (dynamic accent extracted from cover art).
    #[allow(dead_code)]
    pub fn with_accent(mut self, accent: Color) -> Self {
        self.accent = accent;
        self.accent_dim = Color { a: 0.15, ..accent };
        self
    }

    /// Derived `iced::Theme` driving default widget colors (text, base background).
    pub fn iced_theme(&self) -> iced::Theme {
        iced::Theme::custom(
            "Firmium".to_string(),
            iced::theme::Palette {
                background: self.bg,
                text: self.text,
                primary: self.accent,
                success: self.accent,
                warning: self.accent,
                danger: self.error,
            },
        )
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
}

/// Parses `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(r,g,b)`, or `rgba(r,g,b,a)`.
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|x| x.strip_suffix(')')) {
        let p: Vec<&str> = inner.split(',').map(str::trim).collect();
        if p.len() == 4 {
            return Some(Color::from_rgba(
                p[0].parse::<f32>().ok()? / 255.0,
                p[1].parse::<f32>().ok()? / 255.0,
                p[2].parse::<f32>().ok()? / 255.0,
                p[3].parse::<f32>().ok()?,
            ));
        }
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|x| x.strip_suffix(')')) {
        let p: Vec<&str> = inner.split(',').map(str::trim).collect();
        if p.len() == 3 {
            return Some(Color::from_rgb(
                p[0].parse::<f32>().ok()? / 255.0,
                p[1].parse::<f32>().ok()? / 255.0,
                p[2].parse::<f32>().ok()? / 255.0,
            ));
        }
    }
    None
}

fn parse_hex(hex: &str) -> Option<Color> {
    let h = hex.trim();
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    match h.len() {
        6 => Some(Color::from_rgb8(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color::from_rgba8(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            u8::from_str_radix(&h[6..8], 16).ok()? as f32 / 255.0,
        )),
        3 => {
            let dup = |c: char| u8::from_str_radix(&format!("{c}{c}"), 16).ok();
            let mut ch = h.chars();
            Some(Color::from_rgb8(dup(ch.next()?)?, dup(ch.next()?)?, dup(ch.next()?)?))
        }
        _ => None,
    }
}
