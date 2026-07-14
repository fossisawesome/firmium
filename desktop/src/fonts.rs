//! Maps the user-facing font display names (Settings > Appearance) to the
//! `iced::Font` to apply at boot. `None` means "no override" — let iced's
//! own default render, matching the "System"/"Iced" options.

pub const FONT_OPTIONS: &[&str] = &[
    "Inter",
    "Liberation Mono",
    "Monospace",
    "System",
    "Iced",
    "Comic Sans",
    "Sans Serif",
    "BigBlue Terminal",
    "Cousine",
    "FiraCode",
    "Hack",
];

pub fn resolve_font(display_name: &str) -> Option<iced::Font> {
    match display_name {
        "Inter" => Some(iced::Font::with_name("Inter")),
        "Liberation Mono" => Some(iced::Font::with_name("Liberation Mono")),
        "Monospace" => Some(iced::Font::with_name("monospace")),
        "Comic Sans" => Some(iced::Font::with_name("Comic Sans MS")),
        "Sans Serif" => Some(iced::Font::with_name("sans-serif")),
        "BigBlue Terminal" => Some(iced::Font::with_name("BigBlue TerminalPlus")),
        "Cousine" => Some(iced::Font::with_name("Cousine")),
        "FiraCode" => Some(iced::Font::with_name("Fira Code")),
        "Hack" => Some(iced::Font::with_name("Hack")),
        // "System" and "Iced" fall through here too — both mean "no override".
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_resolve_to_their_real_family_name() {
        assert_eq!(resolve_font("FiraCode"), Some(iced::Font::with_name("Fira Code")));
        assert_eq!(resolve_font("BigBlue Terminal"), Some(iced::Font::with_name("BigBlue TerminalPlus")));
        assert_eq!(resolve_font("Inter"), Some(iced::Font::with_name("Inter")));
    }

    #[test]
    fn generic_names_resolve_to_generic_families() {
        assert_eq!(resolve_font("Monospace"), Some(iced::Font::with_name("monospace")));
        assert_eq!(resolve_font("Sans Serif"), Some(iced::Font::with_name("sans-serif")));
    }

    #[test]
    fn system_and_iced_resolve_to_no_override() {
        assert_eq!(resolve_font("System"), None);
        assert_eq!(resolve_font("Iced"), None);
    }

    #[test]
    fn unknown_name_resolves_to_no_override() {
        assert_eq!(resolve_font("Nonexistent Font"), None);
    }

    #[test]
    fn font_options_has_eleven_entries_in_dropdown_order() {
        assert_eq!(FONT_OPTIONS.len(), 11);
        assert_eq!(FONT_OPTIONS[0], "Inter");
        assert_eq!(FONT_OPTIONS[10], "Hack");
    }
}
