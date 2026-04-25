#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuOption {
    Play,
    Quit,
}

impl MainMenuOption {
    pub const ALL: [MainMenuOption; 2] = [MainMenuOption::Play, MainMenuOption::Quit];

    pub fn label(self) -> &'static str {
        match self {
            MainMenuOption::Play => "Play",
            MainMenuOption::Quit => "Quit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuOption {
    Resume,
    Quit,
}

impl PauseMenuOption {
    pub const ALL: [PauseMenuOption; 2] = [PauseMenuOption::Resume, PauseMenuOption::Quit];

    pub fn label(self) -> &'static str {
        match self {
            PauseMenuOption::Resume => "Resume",
            PauseMenuOption::Quit => "Quit",
        }
    }
}
