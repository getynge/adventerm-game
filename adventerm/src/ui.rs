pub mod accel;
pub mod colors;
mod gameplay;
pub mod layout;
mod main_menu;
mod name_entry;
mod options;
mod pause_menu;
mod save_browser;

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::app::{App, Screen};
use crate::menu::MainMenuOption;
use crate::ui::colors::{MenuColors, SchemeColors};

pub fn render(frame: &mut Frame, app: &App) {
    let scheme_colors = SchemeColors::from_scheme(app.color_scheme());
    frame.render_widget(
        Block::default().style(Style::default().bg(scheme_colors.menu.background)),
        frame.area(),
    );

    match app.screen() {
        Screen::MainMenu { menu, status } => {
            main_menu::render(
                frame,
                menu.options(),
                menu.cursor(),
                status.text(),
                &scheme_colors.menu,
            );
        }
        Screen::LoadGame { browser, status } => {
            render_main_menu_underlay(frame, app, MainMenuOption::LoadGame, status.text(), &scheme_colors.menu);
            save_browser::render(
                frame,
                &browser.saves,
                browser.cursor,
                save_browser::Mode::Load,
                browser.pending_delete,
                &scheme_colors.menu,
            );
        }
        Screen::Playing(state) => {
            gameplay::render(frame, state, None, &scheme_colors);
        }
        Screen::Paused { game, menu, status } => {
            gameplay::render(frame, game, status.text(), &scheme_colors);
            pause_menu::render(frame, menu.cursor(), &scheme_colors.menu);
        }
        Screen::SaveSlotPicker {
            game,
            browser,
            status,
        } => {
            gameplay::render(frame, game, status.text(), &scheme_colors);
            save_browser::render(
                frame,
                &browser.saves,
                browser.cursor,
                save_browser::Mode::SavePicker,
                None,
                &scheme_colors.menu,
            );
        }
        Screen::NameEntry {
            game,
            buffer,
            status,
        } => {
            gameplay::render(frame, game, status.text(), &scheme_colors);
            name_entry::render(frame, buffer, &scheme_colors.menu);
        }
        Screen::Options { menu, status } => {
            render_main_menu_underlay(frame, app, MainMenuOption::Options, None, &scheme_colors.menu);
            options::render(frame, app, menu, status.text(), None, &scheme_colors.menu);
        }
        Screen::RebindCapture {
            menu,
            status: _,
            target,
        } => {
            render_main_menu_underlay(frame, app, MainMenuOption::Options, None, &scheme_colors.menu);
            options::render(frame, app, menu, None, Some(*target), &scheme_colors.menu);
        }
        Screen::Quit => {}
    }
}

fn render_main_menu_underlay(
    frame: &mut Frame,
    app: &App,
    highlight: MainMenuOption,
    status: Option<&str>,
    colors: &MenuColors,
) {
    let options = MainMenuOption::available(app.any_saves());
    let cursor = options.iter().position(|o| *o == highlight).unwrap_or(0);
    main_menu::render(frame, &options, cursor, status, colors);
}
