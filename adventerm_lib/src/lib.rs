pub mod menu;
pub mod world;

pub use menu::{MainMenuOption, PauseMenuOption};
pub use world::{Direction, Tile, World};

#[derive(Debug)]
pub enum GameState {
    MainMenu,
    Playing(World),
    Paused(World),
    Quit,
}

impl GameState {
    pub fn new() -> Self {
        GameState::MainMenu
    }

    pub fn is_quit(&self) -> bool {
        matches!(self, GameState::Quit)
    }

    pub fn main_menu_options(&self) -> &'static [MainMenuOption] {
        &MainMenuOption::ALL
    }

    pub fn pause_menu_options(&self) -> &'static [PauseMenuOption] {
        &PauseMenuOption::ALL
    }

    pub fn select_main_menu(self, option: MainMenuOption) -> GameState {
        match (self, option) {
            (GameState::MainMenu, MainMenuOption::Play) => GameState::Playing(World::new()),
            (GameState::MainMenu, MainMenuOption::Quit) => GameState::Quit,
            (other, _) => other,
        }
    }

    pub fn pause(self) -> GameState {
        match self {
            GameState::Playing(world) => GameState::Paused(world),
            other => other,
        }
    }

    pub fn select_pause_menu(self, option: PauseMenuOption) -> GameState {
        match (self, option) {
            (GameState::Paused(world), PauseMenuOption::Resume) => GameState::Playing(world),
            (GameState::Paused(_), PauseMenuOption::Quit) => GameState::Quit,
            (other, _) => other,
        }
    }

    pub fn move_player(&mut self, direction: Direction) {
        if let GameState::Playing(world) = self {
            world.move_player(direction);
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_play_starts_game() {
        let state = GameState::new().select_main_menu(MainMenuOption::Play);
        assert!(matches!(state, GameState::Playing(_)));
    }

    #[test]
    fn main_menu_quit_quits() {
        let state = GameState::new().select_main_menu(MainMenuOption::Quit);
        assert!(state.is_quit());
    }

    #[test]
    fn pause_transitions_from_playing() {
        let state = GameState::new()
            .select_main_menu(MainMenuOption::Play)
            .pause();
        assert!(matches!(state, GameState::Paused(_)));
    }

    #[test]
    fn pause_resume_returns_to_game() {
        let state = GameState::new()
            .select_main_menu(MainMenuOption::Play)
            .pause()
            .select_pause_menu(PauseMenuOption::Resume);
        assert!(matches!(state, GameState::Playing(_)));
    }

    #[test]
    fn pause_quit_quits() {
        let state = GameState::new()
            .select_main_menu(MainMenuOption::Play)
            .pause()
            .select_pause_menu(PauseMenuOption::Quit);
        assert!(state.is_quit());
    }

    #[test]
    fn move_player_only_in_playing() {
        let mut state = GameState::new();
        state.move_player(Direction::Up);
        assert!(matches!(state, GameState::MainMenu));

        let mut state = GameState::new().select_main_menu(MainMenuOption::Play);
        let GameState::Playing(world) = &state else {
            panic!("expected Playing");
        };
        let start = world.player();
        state.move_player(Direction::Up);
        let GameState::Playing(world) = &state else {
            panic!("expected Playing");
        };
        assert_eq!(world.player(), (start.0, start.1 - 1));
    }
}
