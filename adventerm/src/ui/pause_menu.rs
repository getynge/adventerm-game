use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::menu::PauseMenuOption;
use crate::ui::accel;

pub fn render(frame: &mut Frame, cursor: usize) {
    let area = frame.area();
    let options = &PauseMenuOption::ALL;
    let popup = popup_rect(area, 24, 2 + options.len() as u16);

    frame.render_widget(Clear, popup);

    let block = Block::default().borders(Borders::ALL).title(" Paused ");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
    let accels = accel::assign(&labels);

    let lines: Vec<Line> = labels
        .iter()
        .zip(accels.iter())
        .enumerate()
        .map(|(i, (label, a))| accel::line(label, *a, i == cursor))
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
