mod app;
mod input;
mod ui;

use app::App;
use ratatui::{DefaultTerminal, Frame};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    ratatui::run(run)?;
    Ok(())
}

fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|frame: &mut Frame| ui::render(frame, &app))?;
        if app.should_quit() {
            return Ok(());
        }
        let event = crossterm::event::read()?;
        app.handle_event(&event);
    }
}
