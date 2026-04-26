use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;
use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type Rgb = [u8; 3];

pub fn rgb_to_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb[0], rgb[1], rgb[2])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKey {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Space,
    Esc,
    Tab,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    F(u8),
}

impl NamedKey {
    fn label(self) -> String {
        match self {
            NamedKey::Up => "Up".into(),
            NamedKey::Down => "Down".into(),
            NamedKey::Left => "Left".into(),
            NamedKey::Right => "Right".into(),
            NamedKey::Enter => "Enter".into(),
            NamedKey::Space => "Space".into(),
            NamedKey::Esc => "Esc".into(),
            NamedKey::Tab => "Tab".into(),
            NamedKey::Backspace => "Backspace".into(),
            NamedKey::Delete => "Delete".into(),
            NamedKey::Home => "Home".into(),
            NamedKey::End => "End".into(),
            NamedKey::PageUp => "PageUp".into(),
            NamedKey::PageDown => "PageDown".into(),
            NamedKey::Insert => "Insert".into(),
            NamedKey::F(n) => format!("F{n}"),
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "Up" => Some(NamedKey::Up),
            "Down" => Some(NamedKey::Down),
            "Left" => Some(NamedKey::Left),
            "Right" => Some(NamedKey::Right),
            "Enter" => Some(NamedKey::Enter),
            "Space" => Some(NamedKey::Space),
            "Esc" => Some(NamedKey::Esc),
            "Tab" => Some(NamedKey::Tab),
            "Backspace" => Some(NamedKey::Backspace),
            "Delete" => Some(NamedKey::Delete),
            "Home" => Some(NamedKey::Home),
            "End" => Some(NamedKey::End),
            "PageUp" => Some(NamedKey::PageUp),
            "PageDown" => Some(NamedKey::PageDown),
            "Insert" => Some(NamedKey::Insert),
            other if other.starts_with('F') => {
                other[1..].parse::<u8>().ok().filter(|n| (1..=12).contains(n)).map(NamedKey::F)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Named(NamedKey),
    Char(char),
}

impl Key {
    pub fn label(&self) -> String {
        match self {
            Key::Named(n) => n.label(),
            Key::Char(c) => c.to_string(),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        if let Some(named) = NamedKey::parse(s) {
            return Some(Key::Named(named));
        }
        let mut chars = s.chars();
        let first = chars.next()?;
        if chars.next().is_none() {
            return Some(Key::Char(first));
        }
        None
    }

    pub fn matches(&self, code: &KeyCode) -> bool {
        match (self, code) {
            (Key::Named(NamedKey::Up), KeyCode::Up) => true,
            (Key::Named(NamedKey::Down), KeyCode::Down) => true,
            (Key::Named(NamedKey::Left), KeyCode::Left) => true,
            (Key::Named(NamedKey::Right), KeyCode::Right) => true,
            (Key::Named(NamedKey::Enter), KeyCode::Enter) => true,
            (Key::Named(NamedKey::Space), KeyCode::Char(' ')) => true,
            (Key::Named(NamedKey::Esc), KeyCode::Esc) => true,
            (Key::Named(NamedKey::Tab), KeyCode::Tab) => true,
            (Key::Named(NamedKey::Backspace), KeyCode::Backspace) => true,
            (Key::Named(NamedKey::Delete), KeyCode::Delete) => true,
            (Key::Named(NamedKey::Home), KeyCode::Home) => true,
            (Key::Named(NamedKey::End), KeyCode::End) => true,
            (Key::Named(NamedKey::PageUp), KeyCode::PageUp) => true,
            (Key::Named(NamedKey::PageDown), KeyCode::PageDown) => true,
            (Key::Named(NamedKey::Insert), KeyCode::Insert) => true,
            (Key::Named(NamedKey::F(n)), KeyCode::F(m)) => n == m,
            (Key::Char(a), KeyCode::Char(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        }
    }

    pub fn from_key_code(code: KeyCode) -> Option<Self> {
        Some(match code {
            KeyCode::Up => Key::Named(NamedKey::Up),
            KeyCode::Down => Key::Named(NamedKey::Down),
            KeyCode::Left => Key::Named(NamedKey::Left),
            KeyCode::Right => Key::Named(NamedKey::Right),
            KeyCode::Enter => Key::Named(NamedKey::Enter),
            KeyCode::Esc => Key::Named(NamedKey::Esc),
            KeyCode::Tab => Key::Named(NamedKey::Tab),
            KeyCode::Backspace => Key::Named(NamedKey::Backspace),
            KeyCode::Delete => Key::Named(NamedKey::Delete),
            KeyCode::Home => Key::Named(NamedKey::Home),
            KeyCode::End => Key::Named(NamedKey::End),
            KeyCode::PageUp => Key::Named(NamedKey::PageUp),
            KeyCode::PageDown => Key::Named(NamedKey::PageDown),
            KeyCode::Insert => Key::Named(NamedKey::Insert),
            KeyCode::F(n) => Key::Named(NamedKey::F(n)),
            KeyCode::Char(' ') => Key::Named(NamedKey::Space),
            KeyCode::Char(c) => Key::Char(c.to_ascii_lowercase()),
            _ => return None,
        })
    }
}

impl Serialize for Key {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.label())
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Key::from_str(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid key: {s}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundAction {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Escape,
    Delete,
}

impl BoundAction {
    pub const ALL: [BoundAction; 7] = [
        BoundAction::Up,
        BoundAction::Down,
        BoundAction::Left,
        BoundAction::Right,
        BoundAction::Confirm,
        BoundAction::Escape,
        BoundAction::Delete,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BoundAction::Up => "Move Up",
            BoundAction::Down => "Move Down",
            BoundAction::Left => "Move Left",
            BoundAction::Right => "Move Right",
            BoundAction::Confirm => "Confirm",
            BoundAction::Escape => "Escape",
            BoundAction::Delete => "Delete",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindMap {
    pub up: Vec<Key>,
    pub down: Vec<Key>,
    pub left: Vec<Key>,
    pub right: Vec<Key>,
    pub confirm: Vec<Key>,
    pub escape: Vec<Key>,
    pub delete: Vec<Key>,
}

impl Default for KeybindMap {
    fn default() -> Self {
        Self {
            up: vec![Key::Named(NamedKey::Up), Key::Char('w'), Key::Char('k')],
            down: vec![Key::Named(NamedKey::Down), Key::Char('s'), Key::Char('j')],
            left: vec![Key::Named(NamedKey::Left), Key::Char('a'), Key::Char('h')],
            right: vec![Key::Named(NamedKey::Right), Key::Char('d'), Key::Char('l')],
            confirm: vec![Key::Named(NamedKey::Enter), Key::Named(NamedKey::Space)],
            escape: vec![Key::Named(NamedKey::Esc)],
            delete: vec![Key::Named(NamedKey::Delete)],
        }
    }
}

impl KeybindMap {
    pub fn keys_for(&self, action: BoundAction) -> &[Key] {
        match action {
            BoundAction::Up => &self.up,
            BoundAction::Down => &self.down,
            BoundAction::Left => &self.left,
            BoundAction::Right => &self.right,
            BoundAction::Confirm => &self.confirm,
            BoundAction::Escape => &self.escape,
            BoundAction::Delete => &self.delete,
        }
    }

    /// Bind `key` to `action`, replacing the action's existing list and removing
    /// `key` from any other action it was previously bound to.
    pub fn set(&mut self, action: BoundAction, key: Key) {
        self.unbind_everywhere(key);
        let slot = match action {
            BoundAction::Up => &mut self.up,
            BoundAction::Down => &mut self.down,
            BoundAction::Left => &mut self.left,
            BoundAction::Right => &mut self.right,
            BoundAction::Confirm => &mut self.confirm,
            BoundAction::Escape => &mut self.escape,
            BoundAction::Delete => &mut self.delete,
        };
        slot.clear();
        slot.push(key);
    }

    pub fn unbind_everywhere(&mut self, key: Key) {
        for list in [
            &mut self.up,
            &mut self.down,
            &mut self.left,
            &mut self.right,
            &mut self.confirm,
            &mut self.escape,
            &mut self.delete,
        ] {
            list.retain(|k| *k != key);
        }
    }

    pub fn lookup(&self, code: &KeyCode) -> Option<BoundAction> {
        for action in BoundAction::ALL {
            if self.keys_for(action).iter().any(|k| k.matches(code)) {
                return Some(action);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldPalette {
    pub player: Rgb,
    pub interactive: Rgb,
    pub floor: Rgb,
    pub wall: Rgb,
    pub background: Rgb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuPalette {
    pub background: Rgb,
    pub text: Rgb,
    pub title: Rgb,
    pub cursor_fg: Rgb,
    pub cursor_bg: Rgb,
    pub accel: Rgb,
    pub status: Rgb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub name: String,
    pub world: WorldPalette,
    pub menu: MenuPalette,
}

const BUILTIN_DEFAULT: &str = include_str!("../schemes/default.json");
const BUILTIN_HIGH_CONTRAST: &str = include_str!("../schemes/high-contrast.json");
const BUILTIN_DIM: &str = include_str!("../schemes/dim.json");

pub const DEFAULT_SCHEME_NAME: &str = "default";

pub struct SchemeRegistry {
    schemes: HashMap<String, ColorScheme>,
    default_scheme: ColorScheme,
}

impl SchemeRegistry {
    pub fn load(data_dir: &Path) -> Self {
        let mut schemes: HashMap<String, ColorScheme> = HashMap::new();
        for json in [BUILTIN_DEFAULT, BUILTIN_HIGH_CONTRAST, BUILTIN_DIM] {
            if let Ok(scheme) = serde_json::from_str::<ColorScheme>(json) {
                schemes.insert(scheme.name.clone(), scheme);
            }
        }
        let default_scheme = schemes
            .get(DEFAULT_SCHEME_NAME)
            .cloned()
            .expect("built-in default scheme must parse");

        let dir = data_dir.join("schemes");
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let Ok(bytes) = fs::read(&path) else { continue };
                if let Ok(scheme) = serde_json::from_slice::<ColorScheme>(&bytes) {
                    schemes.insert(scheme.name.clone(), scheme);
                }
            }
        }

        Self {
            schemes,
            default_scheme,
        }
    }

    pub fn names_sorted(&self) -> Vec<String> {
        let mut v: Vec<String> = self.schemes.keys().cloned().collect();
        v.sort();
        v
    }

    pub fn resolve(&self, name: &str) -> &ColorScheme {
        self.schemes.get(name).unwrap_or(&self.default_scheme)
    }

    pub fn next_after(&self, name: &str) -> String {
        let names = self.names_sorted();
        if names.is_empty() {
            return DEFAULT_SCHEME_NAME.to_string();
        }
        let idx = names.iter().position(|n| n == name).unwrap_or(0);
        names[(idx + 1) % names.len()].clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub keybinds: KeybindMap,
    pub color_scheme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keybinds: KeybindMap::default(),
            color_scheme: DEFAULT_SCHEME_NAME.to_string(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).expect("Config serializes");
        fs::write(path, bytes)
    }
}

pub fn config_path_for(data_dir: &Path) -> PathBuf {
    data_dir.join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_round_trips_through_json() {
        let cases = [
            Key::Named(NamedKey::Up),
            Key::Named(NamedKey::Enter),
            Key::Named(NamedKey::Space),
            Key::Named(NamedKey::F(5)),
            Key::Char('w'),
            Key::Char('k'),
        ];
        for key in cases {
            let s = serde_json::to_string(&key).unwrap();
            let back: Key = serde_json::from_str(&s).unwrap();
            assert_eq!(key, back, "round trip for {key:?} via {s}");
        }
    }

    #[test]
    fn default_config_round_trips() {
        let cfg = Config::default();
        let bytes = serde_json::to_vec(&cfg).unwrap();
        let back: Config = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(cfg.color_scheme, back.color_scheme);
        assert_eq!(cfg.keybinds.up, back.keybinds.up);
        assert_eq!(cfg.keybinds.confirm, back.keybinds.confirm);
    }

    #[test]
    fn lookup_finds_action() {
        let binds = KeybindMap::default();
        assert_eq!(binds.lookup(&KeyCode::Up), Some(BoundAction::Up));
        assert_eq!(binds.lookup(&KeyCode::Char('w')), Some(BoundAction::Up));
        assert_eq!(binds.lookup(&KeyCode::Char('W')), Some(BoundAction::Up));
        assert_eq!(binds.lookup(&KeyCode::Enter), Some(BoundAction::Confirm));
        assert_eq!(binds.lookup(&KeyCode::Char(' ')), Some(BoundAction::Confirm));
        assert_eq!(binds.lookup(&KeyCode::Esc), Some(BoundAction::Escape));
        assert_eq!(binds.lookup(&KeyCode::Char('z')), None);
    }

    #[test]
    fn set_replaces_existing_binding() {
        let mut binds = KeybindMap::default();
        binds.unbind_everywhere(Key::Char('t'));
        binds.set(BoundAction::Up, Key::Char('t'));
        assert_eq!(binds.up, vec![Key::Char('t')]);
        assert_eq!(binds.lookup(&KeyCode::Char('t')), Some(BoundAction::Up));
    }

    #[test]
    fn builtin_schemes_parse() {
        let registry = SchemeRegistry::load(Path::new("/nonexistent-test-dir-xyz"));
        let names = registry.names_sorted();
        assert!(names.iter().any(|n| n == "default"));
        assert!(names.iter().any(|n| n == "high-contrast"));
        assert!(names.iter().any(|n| n == "dim"));
        let scheme = registry.resolve("default");
        assert_eq!(scheme.name, "default");
    }

    #[test]
    fn next_after_cycles() {
        let registry = SchemeRegistry::load(Path::new("/nonexistent-test-dir-xyz"));
        let names = registry.names_sorted();
        let first = &names[0];
        let next = registry.next_after(first);
        assert_eq!(next, names[1 % names.len()]);
    }
}
