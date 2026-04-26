pub mod accel;
mod gameplay;
mod main_menu;
mod pause_menu;

use ratatui::Frame;
use ratatui::style::Color;

use crate::app::{App, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    match app.screen() {
        Screen::MainMenu => main_menu::render(frame, app.main_menu_cursor()),
        Screen::Playing(world) => gameplay::render(frame, world),
        Screen::Paused(world) => {
            gameplay::render(frame, world);
            pause_menu::render(frame, app.pause_menu_cursor());
        }
        Screen::Quit => {}
    }

    if app.hummus() {
        let buffer = frame.buffer_mut();
        for cell in buffer.content.iter_mut() {
            cell.fg = Color::Green;
        }
    }
}
