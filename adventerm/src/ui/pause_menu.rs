use adventerm_lib::PauseMenuOption;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(frame: &mut Frame, options: &[PauseMenuOption], cursor: usize) {
    let area = frame.area();
    let popup = popup_rect(area, 24, 7);

    frame.render_widget(Clear, popup);

    let block = Block::default().borders(Borders::ALL).title(" Paused ");
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            let label = option.label();
            if i == cursor {
                Line::from(Span::styled(
                    format!("> {label} <"),
                    Style::default().add_modifier(Modifier::REVERSED),
                ))
            } else {
                Line::from(format!("  {label}  "))
            }
        })
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
