use std::mem;

use adventerm_lib::{Direction, GameState, PauseMenuOption};
use crossterm::event::Event;

use crate::input::Action;

pub struct App {
    state: GameState,
    main_menu_cursor: usize,
    pause_menu_cursor: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: GameState::new(),
            main_menu_cursor: 0,
            pause_menu_cursor: 0,
        }
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn main_menu_cursor(&self) -> usize {
        self.main_menu_cursor
    }

    pub fn pause_menu_cursor(&self) -> usize {
        self.pause_menu_cursor
    }

    pub fn should_quit(&self) -> bool {
        self.state.is_quit()
    }

    pub fn handle_event(&mut self, event: &Event) {
        let Some(action) = crate::input::translate(event) else {
            return;
        };
        match &self.state {
            GameState::MainMenu => self.handle_main_menu(action),
            GameState::Playing(_) => self.handle_playing(action),
            GameState::Paused(_) => self.handle_pause_menu(action),
            GameState::Quit => {}
        }
    }

    fn handle_main_menu(&mut self, action: Action) {
        let options = self.state.main_menu_options();
        match action {
            Action::Up => self.main_menu_cursor = prev_cursor(self.main_menu_cursor, options.len()),
            Action::Down => {
                self.main_menu_cursor = next_cursor(self.main_menu_cursor, options.len())
            }
            Action::Confirm => {
                let option = options[self.main_menu_cursor];
                self.transition(|s| s.select_main_menu(option));
            }
            _ => {}
        }
    }

    fn handle_playing(&mut self, action: Action) {
        match action {
            Action::Up => self.state.move_player(Direction::Up),
            Action::Down => self.state.move_player(Direction::Down),
            Action::Left => self.state.move_player(Direction::Left),
            Action::Right => self.state.move_player(Direction::Right),
            Action::Escape => {
                self.pause_menu_cursor = 0;
                self.transition(GameState::pause);
            }
            _ => {}
        }
    }

    fn handle_pause_menu(&mut self, action: Action) {
        let options = self.state.pause_menu_options();
        match action {
            Action::Up => {
                self.pause_menu_cursor = prev_cursor(self.pause_menu_cursor, options.len())
            }
            Action::Down => {
                self.pause_menu_cursor = next_cursor(self.pause_menu_cursor, options.len())
            }
            Action::Confirm => {
                let option = options[self.pause_menu_cursor];
                self.transition(|s| s.select_pause_menu(option));
            }
            Action::Escape => {
                self.transition(|s| s.select_pause_menu(PauseMenuOption::Resume));
            }
            _ => {}
        }
    }

    fn transition(&mut self, f: impl FnOnce(GameState) -> GameState) {
        let prev = mem::replace(&mut self.state, GameState::Quit);
        self.state = f(prev);
    }
}

fn next_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (cursor + 1) % len }
}

fn prev_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (cursor + len - 1) % len }
}
