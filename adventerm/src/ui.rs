pub mod accel;
mod gameplay;
mod main_menu;
mod name_entry;
mod options;
mod pause_menu;
mod save_browser;

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::app::{App, Screen};
use crate::config::rgb_to_color;

pub fn render(frame: &mut Frame, app: &App) {
    let menu_palette = app.menu_palette();
    let bg = rgb_to_color(menu_palette.background);
    frame.render_widget(Block::default().style(Style::default().bg(bg)), frame.area());

    match app.screen() {
        Screen::MainMenu => main_menu::render(
            frame,
            app.main_menu_cursor(),
            app.any_saves(),
            app.status(),
            menu_palette,
        ),
        Screen::LoadGame => {
            main_menu::render(
                frame,
                app.main_menu_cursor(),
                app.any_saves(),
                app.status(),
                menu_palette,
            );
            save_browser::render(
                frame,
                app.saves(),
                app.save_list_cursor(),
                save_browser::Mode::Load,
                app.pending_delete(),
                menu_palette,
            );
        }
        Screen::Playing(state) => {
            gameplay::render(frame, state, app.status(), app.color_scheme())
        }
        Screen::Paused(state) => {
            gameplay::render(frame, state, app.status(), app.color_scheme());
            pause_menu::render(frame, app.pause_menu_cursor(), menu_palette);
        }
        Screen::SaveSlotPicker(state) => {
            gameplay::render(frame, state, app.status(), app.color_scheme());
            save_browser::render(
                frame,
                app.saves(),
                app.save_list_cursor(),
                save_browser::Mode::SavePicker,
                None,
                menu_palette,
            );
        }
        Screen::NameEntry(state) => {
            gameplay::render(frame, state, app.status(), app.color_scheme());
            name_entry::render(frame, app.name_buffer(), menu_palette);
        }
        Screen::Options | Screen::RebindCapture(_) => {
            main_menu::render(
                frame,
                app.main_menu_cursor(),
                app.any_saves(),
                None,
                menu_palette,
            );
            options::render(frame, app, menu_palette);
        }
        Screen::Quit => {}
    }
}
