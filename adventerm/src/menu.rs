use adventerm_lib::SaveSlot;

use crate::config::BoundAction;
use crate::ui::accel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuOption {
    NewGame,
    LoadGame,
    Options,
    Quit,
}

impl MainMenuOption {
    pub fn available(any_saves: bool) -> Vec<MainMenuOption> {
        let mut v = Vec::with_capacity(4);
        v.push(MainMenuOption::NewGame);
        if any_saves {
            v.push(MainMenuOption::LoadGame);
        }
        v.push(MainMenuOption::Options);
        v.push(MainMenuOption::Quit);
        v
    }

    pub fn label(self) -> &'static str {
        match self {
            MainMenuOption::NewGame => "New Game",
            MainMenuOption::LoadGame => "Load Game",
            MainMenuOption::Options => "Options",
            MainMenuOption::Quit => "Quit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuOption {
    Resume,
    Save,
    Quit,
}

impl PauseMenuOption {
    pub const ALL: [PauseMenuOption; 3] = [
        PauseMenuOption::Resume,
        PauseMenuOption::Save,
        PauseMenuOption::Quit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PauseMenuOption::Resume => "Resume",
            PauseMenuOption::Save => "Save",
            PauseMenuOption::Quit => "Quit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsRow {
    ColorScheme,
    Keybind(BoundAction),
    ResetDefaults,
    Back,
}

impl OptionsRow {
    pub fn all() -> Vec<OptionsRow> {
        let mut v: Vec<OptionsRow> = Vec::with_capacity(BoundAction::ALL.len() + 3);
        v.push(OptionsRow::ColorScheme);
        for a in BoundAction::ALL {
            v.push(OptionsRow::Keybind(a));
        }
        v.push(OptionsRow::ResetDefaults);
        v.push(OptionsRow::Back);
        v
    }
}

/// A list of options the user can navigate with up/down and select with enter
/// or a hotkey. Owns its own cursor; one screen, one MenuState.
pub struct MenuState<T> {
    options: Vec<T>,
    cursor: usize,
}

impl<T: Copy> MenuState<T> {
    pub fn new(options: Vec<T>) -> Self {
        Self { options, cursor: 0 }
    }

    pub fn options(&self) -> &[T] {
        &self.options
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn up(&mut self) {
        if !self.options.is_empty() {
            self.cursor = (self.cursor + self.options.len() - 1) % self.options.len();
        }
    }

    pub fn down(&mut self) {
        if !self.options.is_empty() {
            self.cursor = (self.cursor + 1) % self.options.len();
        }
    }

    pub fn current(&self) -> Option<T> {
        self.options.get(self.cursor).copied()
    }

    /// Looks up an option by accelerator hotkey from the given labels (one per
    /// option, in the same order). On a match, advances the cursor and returns
    /// the option at that position.
    pub fn select_hotkey(&mut self, key: char, labels: &[&str]) -> Option<T> {
        let idx = accel::find_by_hotkey(labels, key)?;
        self.cursor = idx;
        self.options.get(idx).copied()
    }
}

/// Status line shown on a screen. Cleared on transition by construction —
/// each screen owns its own Status, so there is nothing to forget to reset.
pub enum Status {
    None,
    Info(String),
    Error(String),
}

impl Status {
    pub fn text(&self) -> Option<&str> {
        match self {
            Status::None => None,
            Status::Info(s) | Status::Error(s) => Some(s),
        }
    }
}

/// A list of saves with a cursor and an optional pending delete confirmation.
/// Used by both the load-game screen and the save-slot picker.
pub struct SaveBrowser {
    pub saves: Vec<SaveSlot>,
    pub cursor: usize,
    pub pending_delete: Option<usize>,
}

impl SaveBrowser {
    pub fn new(saves: Vec<SaveSlot>) -> Self {
        Self {
            saves,
            cursor: 0,
            pending_delete: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.saves.is_empty()
    }

    pub fn up(&mut self, total: usize) {
        if total > 0 {
            self.cursor = (self.cursor + total - 1) % total;
        }
    }

    pub fn down(&mut self, total: usize) {
        if total > 0 {
            self.cursor = (self.cursor + 1) % total;
        }
    }

    pub fn clamp(&mut self, total: usize) {
        if total == 0 {
            self.cursor = 0;
        } else if self.cursor >= total {
            self.cursor = total - 1;
        }
    }
}
