use adventerm_lib::PauseMenuOption;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::ui::menu;

pub fn render(frame: &mut Frame, options: &[PauseMenuOption], cursor: usize) {
    let area = frame.area();
    let popup = popup_rect(area, 24, 7);

    frame.render_widget(Clear, popup);

    let block = Block::default().borders(Borders::ALL).title(" Paused ");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
    let accels = menu::assign(&labels);

    let lines: Vec<Line> = labels
        .iter()
        .zip(accels.iter())
        .enumerate()
        .map(|(i, (label, accel))| menu::line(label, *accel, i == cursor))
        .collect();

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
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
