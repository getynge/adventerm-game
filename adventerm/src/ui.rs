mod gameplay;
mod main_menu;
pub mod menu;
mod pause_menu;

use adventerm_lib::GameState;
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    match app.state() {
        GameState::MainMenu => {
            main_menu::render(frame, app.state().main_menu_options(), app.main_menu_cursor());
        }
        GameState::Playing(world) => gameplay::render(frame, world),
        GameState::Paused(world) => {
            gameplay::render(frame, world);
            pause_menu::render(
                frame,
                app.state().pause_menu_options(),
                app.pause_menu_cursor(),
            );
        }
        GameState::Quit => {}
    }
}
