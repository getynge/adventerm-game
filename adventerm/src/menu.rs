use crate::config::BoundAction;

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
