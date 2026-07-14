use std::collections::HashMap;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    PlayPause,
    NextTrack,
    PrevTrack,
    NavDown,
    NavUp,
    NavLeft,
    NavRight,
    Search,
    Activate,
    Quit,
}

const ACTIONS: &[(Action, &str)] = &[
    (Action::PlayPause, "play_pause"),
    (Action::NextTrack, "next_track"),
    (Action::PrevTrack, "prev_track"),
    (Action::NavDown, "nav_down"),
    (Action::NavUp, "nav_up"),
    (Action::NavLeft, "nav_left"),
    (Action::NavRight, "nav_right"),
    (Action::Search, "search"),
    (Action::Activate, "activate"),
    (Action::Quit, "quit"),
];

fn default_key_for(action: Action) -> KeyEvent {
    let code = match action {
        Action::PlayPause => KeyCode::Char(' '),
        Action::NextTrack => KeyCode::Char('n'),
        Action::PrevTrack => KeyCode::Char('p'),
        Action::NavDown => KeyCode::Char('j'),
        Action::NavUp => KeyCode::Char('k'),
        Action::NavLeft => KeyCode::Char('h'),
        Action::NavRight => KeyCode::Char('l'),
        Action::Search => KeyCode::Char('/'),
        Action::Activate => KeyCode::Enter,
        Action::Quit => KeyCode::Char('q'),
    };
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Arrow keys always work as an alt binding for nav, regardless of user config.
const ALT_NAV: &[(KeyCode, Action)] = &[
    (KeyCode::Down, Action::NavDown),
    (KeyCode::Up, Action::NavUp),
    (KeyCode::Left, Action::NavLeft),
    (KeyCode::Right, Action::NavRight),
];

fn parse_key_string(s: &str) -> Option<KeyEvent> {
    let code = match s {
        "space" => KeyCode::Char(' '),
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        s if s.chars().count() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => return None,
    };
    Some(KeyEvent::new(code, KeyModifiers::NONE))
}

#[derive(Deserialize, Default)]
struct RawKeymap {
    play_pause: Option<String>,
    next_track: Option<String>,
    prev_track: Option<String>,
    nav_down: Option<String>,
    nav_up: Option<String>,
    nav_left: Option<String>,
    nav_right: Option<String>,
    search: Option<String>,
    activate: Option<String>,
    quit: Option<String>,
}

impl RawKeymap {
    fn get(&self, name: &str) -> Option<&str> {
        match name {
            "play_pause" => self.play_pause.as_deref(),
            "next_track" => self.next_track.as_deref(),
            "prev_track" => self.prev_track.as_deref(),
            "nav_down" => self.nav_down.as_deref(),
            "nav_up" => self.nav_up.as_deref(),
            "nav_left" => self.nav_left.as_deref(),
            "nav_right" => self.nav_right.as_deref(),
            "search" => self.search.as_deref(),
            "activate" => self.activate.as_deref(),
            "quit" => self.quit.as_deref(),
            _ => None,
        }
    }
}

pub struct Keymap {
    bindings: HashMap<KeyEvent, Action>,
}

impl Keymap {
    /// Returns the built keymap plus any warnings for invalid key strings
    /// (action keeps its default binding when its override is invalid).
    pub fn from_toml_str_with_warnings(s: &str) -> (Self, Vec<String>) {
        let raw: RawKeymap = toml::from_str(s).unwrap_or_default();
        let mut bindings = HashMap::new();
        let mut warnings = Vec::new();

        // Pass 1: defaults for actions with no override. Pass 2 (below) inserts
        // overrides afterward so an explicit user binding always wins a
        // collision with another action's default (e.g. overriding next_track
        // to 'l' takes it away from nav_right's default 'l').
        for (action, name) in ACTIONS {
            if raw.get(name).is_none() {
                bindings.insert(default_key_for(*action), *action);
            }
        }
        for (action, name) in ACTIONS {
            if let Some(user_key_str) = raw.get(name) {
                let key = match parse_key_string(user_key_str) {
                    Some(k) => k,
                    None => {
                        warnings.push(format!(
                            "termium_keymap.toml: invalid key '{user_key_str}' for '{name}', using default"
                        ));
                        default_key_for(*action)
                    }
                };
                bindings.insert(key, *action);
            }
        }

        for (code, action) in ALT_NAV {
            bindings
                .entry(KeyEvent::new(*code, KeyModifiers::NONE))
                .or_insert(*action);
        }

        (Self { bindings }, warnings)
    }

    pub fn resolve(&self, key: KeyEvent) -> Option<Action> {
        self.bindings.get(&key).copied()
    }

    /// Loads `~/.config/com.fossisawesome.firmium/termium_keymap.toml` if present,
    /// falling back to built-in defaults. Warnings are printed to stderr at
    /// startup (before the alt screen takes over stdout).
    pub fn load() -> Self {
        let path = firmium_backend::paths::config_dir().join("termium_keymap.toml");
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        let (km, warnings) = Self::from_toml_str_with_warnings(&contents);
        for w in warnings {
            eprintln!("{w}");
        }
        km
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn default_keymap_resolves_play_pause() {
        let km = Keymap::from_toml_str_with_warnings("").0;
        assert_eq!(km.resolve(key(' ')), Some(Action::PlayPause));
    }

    #[test]
    fn default_keymap_resolves_vim_nav() {
        let km = Keymap::from_toml_str_with_warnings("").0;
        assert_eq!(km.resolve(key('j')), Some(Action::NavDown));
        assert_eq!(km.resolve(key('k')), Some(Action::NavUp));
        assert_eq!(km.resolve(key('h')), Some(Action::NavLeft));
        assert_eq!(km.resolve(key('l')), Some(Action::NavRight));
    }

    #[test]
    fn user_override_replaces_default_binding() {
        let km = Keymap::from_toml_str_with_warnings(r#"next_track = "l""#).0;
        assert_eq!(km.resolve(key('l')), Some(Action::NextTrack));
        // overridden action's old key ('n') is no longer bound to NextTrack
        assert_eq!(km.resolve(key('n')), None);
    }

    #[test]
    fn invalid_key_string_falls_back_to_default_and_reports_warning() {
        let (km, warnings) = Keymap::from_toml_str_with_warnings(r#"quit = "not-a-real-key""#);
        assert_eq!(km.resolve(key('q')), Some(Action::Quit)); // default kept
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("quit"));
    }

    #[test]
    fn arrow_keys_work_as_alt_nav_binding() {
        let km = Keymap::from_toml_str_with_warnings("").0;
        assert_eq!(
            km.resolve(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            Some(Action::NavDown)
        );
    }

    #[test]
    fn default_keymap_resolves_activate_to_enter() {
        let km = Keymap::from_toml_str_with_warnings("").0;
        assert_eq!(
            km.resolve(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Action::Activate)
        );
    }
}
