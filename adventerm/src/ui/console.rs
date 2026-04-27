use log::Level;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::console::log_sink::{snapshot, LogEntry};
use crate::console::ConsoleState;
use crate::ui::colors::{menu_block, MenuColors};
use crate::ui::layout::{
    CONSOLE_HORIZONTAL_MARGIN, CONSOLE_INPUT_ROWS, CONSOLE_MIN_HEIGHT, CONSOLE_MIN_WIDTH,
    CONSOLE_VERTICAL_MARGIN,
};

const PROMPT: &str = "> ";
const FOOTER_HINT: &str = "Tab: complete   Enter: run   Esc/`: close";

pub fn render(frame: &mut Frame, console: &ConsoleState, colors: &MenuColors) {
    let area = frame.area();
    let popup = console_rect(area);
    frame.render_widget(Clear, popup);

    let block = menu_block(" Developer Console ", colors);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let [log_area, input_area, footer_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    render_log(frame, log_area, colors);
    render_input(frame, console, input_area, colors);
    frame.render_widget(
        Paragraph::new(FOOTER_HINT).style(footer_style(colors)),
        footer_area,
    );
}

fn console_rect(area: Rect) -> Rect {
    let width = area
        .width
        .saturating_sub(CONSOLE_HORIZONTAL_MARGIN * 2)
        .max(CONSOLE_MIN_WIDTH.min(area.width));
    let height = area
        .height
        .saturating_sub(CONSOLE_VERTICAL_MARGIN * 2)
        .max(CONSOLE_MIN_HEIGHT.min(area.height));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn render_log(frame: &mut Frame, area: Rect, colors: &MenuColors) {
    let lines: Vec<Line> = snapshot(area.height as usize)
        .into_iter()
        .map(|entry| log_line(&entry, colors))
        .collect();
    frame.render_widget(Paragraph::new(lines).style(colors.body_style()), area);
}

fn log_line(entry: &LogEntry, colors: &MenuColors) -> Line<'static> {
    let (level_text, level_style) = match entry.level {
        Level::Error => ("ERROR ", Style::default().fg(Color::Red).bg(colors.background)),
        Level::Warn => (
            "WARN  ",
            Style::default().fg(Color::Yellow).bg(colors.background),
        ),
        Level::Info => ("INFO  ", colors.body_style()),
        Level::Debug => (
            "DEBUG ",
            colors
                .body_style()
                .add_modifier(Modifier::DIM),
        ),
        Level::Trace => (
            "TRACE ",
            colors
                .body_style()
                .add_modifier(Modifier::DIM),
        ),
    };
    Line::from(vec![
        Span::styled(level_text.to_string(), level_style),
        Span::styled(entry.message.clone(), colors.body_style()),
    ])
}

fn render_input(frame: &mut Frame, console: &ConsoleState, area: Rect, colors: &MenuColors) {
    let typed = console.input.clone();
    let ghost = console.completion().ghost.clone();
    let cycle_hint = if console.completion().candidates.len() > 1 && ghost.is_empty() {
        format!(" ({} matches)", console.completion().candidates.len())
    } else {
        String::new()
    };
    let line = Line::from(vec![
        Span::styled(PROMPT, colors.title_style()),
        Span::styled(typed, colors.body_style()),
        Span::styled(
            ghost,
            Style::default()
                .fg(colors.text)
                .bg(colors.background)
                .add_modifier(Modifier::DIM),
        ),
        Span::styled("_", colors.cursor_style()),
        Span::styled(cycle_hint, footer_style(colors)),
    ]);
    frame.render_widget(Paragraph::new(line).style(colors.body_style()), area);
    let _ = CONSOLE_INPUT_ROWS;
}

fn footer_style(colors: &MenuColors) -> Style {
    Style::default()
        .fg(colors.status)
        .bg(colors.background)
        .add_modifier(Modifier::ITALIC)
}
