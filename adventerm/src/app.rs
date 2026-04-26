use std::mem;

use adventerm_lib::{Direction, World};
use crossterm::event::Event;

use crate::input::Action;
use crate::menu::{MainMenuOption, PauseMenuOption};
use crate::ui::accel;

pub enum Screen {
    MainMenu,
    Playing(World),
    Paused(World),
    Quit,
}

pub struct App {
    screen: Screen,
    main_menu_cursor: usize,
    pause_menu_cursor: usize,
    hummus: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::MainMenu,
            main_menu_cursor: 0,
            pause_menu_cursor: 0,
            hummus: false,
        }
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn main_menu_cursor(&self) -> usize {
        self.main_menu_cursor
    }

    pub fn pause_menu_cursor(&self) -> usize {
        self.pause_menu_cursor
    }

    pub fn hummus(&self) -> bool {
        self.hummus
    }

    pub fn should_quit(&self) -> bool {
        matches!(self.screen, Screen::Quit)
    }

    pub fn handle_event(&mut self, event: &Event) {
        let Some(action) = crate::input::translate(event) else {
            return;
        };
        match &self.screen {
            Screen::MainMenu => self.handle_main_menu(action),
            Screen::Playing(_) => self.handle_playing(action),
            Screen::Paused(_) => self.handle_pause_menu(action),
            Screen::Quit => {}
        }
    }

    fn handle_main_menu(&mut self, action: Action) {
        let options = &MainMenuOption::ALL;
        match action {
            Action::Up => self.main_menu_cursor = prev_cursor(self.main_menu_cursor, options.len()),
            Action::Down => {
                self.main_menu_cursor = next_cursor(self.main_menu_cursor, options.len())
            }
            Action::Confirm => self.select_main_option(options[self.main_menu_cursor]),
            Action::Hotkey(c) => {
                if let Some(i) = accel::find_by_hotkey(options.len(), |i| options[i].label(), c) {
                    self.main_menu_cursor = i;
                    self.select_main_option(options[i]);
                }
            }
            _ => {}
        }
    }

    fn select_main_option(&mut self, option: MainMenuOption) {
        match option {
            MainMenuOption::Play => self.screen = Screen::Playing(World::new()),
            MainMenuOption::Quit => self.screen = Screen::Quit,
        }
    }

    fn handle_playing(&mut self, action: Action) {
        match action {
            Action::Up => self.with_world(|w| w.move_player(Direction::Up)),
            Action::Down => self.with_world(|w| w.move_player(Direction::Down)),
            Action::Left => self.with_world(|w| w.move_player(Direction::Left)),
            Action::Right => self.with_world(|w| w.move_player(Direction::Right)),
            Action::Escape => {
                self.pause_menu_cursor = 0;
                self.transition_screen(|s| match s {
                    Screen::Playing(w) => Screen::Paused(w),
                    other => other,
                });
            }
            _ => {}
        }
    }

    fn handle_pause_menu(&mut self, action: Action) {
        let options = &PauseMenuOption::ALL;
        match action {
            Action::Up => {
                self.pause_menu_cursor = prev_cursor(self.pause_menu_cursor, options.len())
            }
            Action::Down => {
                self.pause_menu_cursor = next_cursor(self.pause_menu_cursor, options.len())
            }
            Action::Confirm => self.select_pause_option(options[self.pause_menu_cursor]),
            Action::Escape => self.select_pause_option(PauseMenuOption::Resume),
            Action::Hotkey(c) => {
                if let Some(i) = accel::find_by_hotkey(options.len(), |i| options[i].label(), c) {
                    self.pause_menu_cursor = i;
                    self.select_pause_option(options[i]);
                }
            }
            _ => {}
        }
    }

    fn select_pause_option(&mut self, option: PauseMenuOption) {
        match option {
            PauseMenuOption::Resume => self.transition_screen(|s| match s {
                Screen::Paused(w) => Screen::Playing(w),
                other => other,
            }),
            PauseMenuOption::Hummus => {
                self.hummus = !self.hummus;
                self.transition_screen(|s| match s {
                    Screen::Paused(w) => Screen::Playing(w),
                    other => other,
                })
            },
            PauseMenuOption::Quit => self.screen = Screen::Quit,
        }
    }

    fn with_world(&mut self, f: impl FnOnce(&mut World)) {
        if let Screen::Playing(world) = &mut self.screen {
            f(world);
        }
    }

    fn transition_screen(&mut self, f: impl FnOnce(Screen) -> Screen) {
        let prev = mem::replace(&mut self.screen, Screen::Quit);
        self.screen = f(prev);
    }
}

fn next_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (cursor + 1) % len }
}

fn prev_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (cursor + len - 1) % len }
}
