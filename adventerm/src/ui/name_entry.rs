use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::config::{rgb_to_color, MenuPalette};

pub fn render(frame: &mut Frame, buffer: &str, palette: &MenuPalette) {
    let area = frame.area();
    let popup = popup_rect(area, 44, 5);
    frame.render_widget(Clear, popup);

    let bg = rgb_to_color(palette.background);
    let text = rgb_to_color(palette.text);
    let title = rgb_to_color(palette.title);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Save name ")
        .title_style(Style::default().fg(title).bg(bg))
        .style(Style::default().fg(text).bg(bg));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let [input_area, _gap, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let input = format!("{buffer}_");
    frame.render_widget(
        Paragraph::new(input)
            .alignment(Alignment::Center)
            .style(Style::default().fg(text).bg(bg)),
        input_area,
    );
    frame.render_widget(
        Paragraph::new("Enter: save   Esc: cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(text).bg(bg)),
        footer_area,
    );
}

fn popup_rect(area: Rect, width: u16, height: u16) -> Rect {
    let [_, row, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);

    let [centered] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(row);
    centered
}
