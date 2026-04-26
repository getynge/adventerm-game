use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::game::GameState;

pub const SAVE_VERSION: u32 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    pub version: u32,
    pub name: String,
    pub state: GameState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlot {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
}

#[derive(Debug)]
pub enum SaveError {
    Format(serde_json::Error),
    UnsupportedVersion { found: u32, expected: u32 },
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Format(e) => write!(f, "save format error: {e}"),
            SaveError::UnsupportedVersion { found, expected } => write!(
                f,
                "unsupported save version {found} (expected {expected})"
            ),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SaveError::Format(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self {
        SaveError::Format(e)
    }
}

impl Save {
    pub fn new(name: String, state: GameState) -> Self {
        Self {
            version: SAVE_VERSION,
            name,
            state,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("Save serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        let mut save: Save = serde_json::from_slice(bytes)?;
        if save.version != SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion {
                found: save.version,
                expected: SAVE_VERSION,
            });
        }
        save.state.refresh_visibility();
        Ok(save)
    }
}

pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = true;
    for c in name.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("save");
    }
    out
}

pub fn slot_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.json", slugify(name)))
}

pub fn list_saves(dir: &Path) -> io::Result<Vec<SaveSlot>> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut slots = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let save: Save = match serde_json::from_slice(&bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if save.version != SAVE_VERSION {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        slots.push(SaveSlot {
            path,
            name: save.name,
            modified,
        });
    }
    slots.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(slots)
}

pub fn delete_save(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Direction;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("adventerm-{label}-{nanos}-{n}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn round_trip_preserves_state() {
        let mut state = GameState::new_seeded(101);
        state.move_player(Direction::Right);
        state.move_player(Direction::Down);
        let save = Save::new("My Run".into(), state.clone());
        let bytes = save.to_bytes();
        let recovered = Save::from_bytes(&bytes).expect("decode");
        assert_eq!(recovered.state, state);
        assert_eq!(recovered.name, "My Run");
    }

    #[test]
    fn version_mismatch_errors() {
        let state = GameState::new_seeded(7);
        let save = Save {
            version: SAVE_VERSION + 1,
            name: "x".into(),
            state,
        };
        let bytes = save.to_bytes();
        match Save::from_bytes(&bytes) {
            Err(SaveError::UnsupportedVersion { .. }) => {}
            other => panic!("expected version mismatch, got {other:?}"),
        }
    }

    #[test]
    fn slugify_handles_punctuation_and_case() {
        assert_eq!(slugify("My Cool Save!"), "my-cool-save");
        assert_eq!(slugify("---weird---"), "weird");
        assert_eq!(slugify(""), "save");
        assert_eq!(slugify("???"), "save");
    }

    #[test]
    fn list_saves_reads_back_named_saves() {
        let dir = unique_temp_dir("list");
        let state = GameState::new_seeded(1);
        let a = Save::new("Alpha".into(), state.clone());
        let b = Save::new("Beta".into(), state);
        fs::write(slot_path(&dir, "Alpha"), a.to_bytes()).unwrap();
        fs::write(slot_path(&dir, "Beta"), b.to_bytes()).unwrap();

        let mut slots = list_saves(&dir).unwrap();
        slots.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].name, "Alpha");
        assert_eq!(slots[1].name, "Beta");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_saves_skips_invalid_files() {
        let dir = unique_temp_dir("invalid");
        fs::write(dir.join("garbage.json"), b"not json").unwrap();
        fs::write(dir.join("readme.txt"), b"ignored").unwrap();
        let slots = list_saves(&dir).unwrap();
        assert!(slots.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_saves_returns_empty_for_missing_dir() {
        let dir = std::env::temp_dir().join("adventerm-nonexistent-dir-xyz");
        let _ = fs::remove_dir_all(&dir);
        let slots = list_saves(&dir).unwrap();
        assert!(slots.is_empty());
    }
}
