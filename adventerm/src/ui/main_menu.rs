use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::menu::MainMenuOption;
use crate::ui::accel;
use crate::ui::colors::MenuColors;
use crate::ui::layout::MAIN_MENU_OPTIONS_WIDTH;

pub fn render(
    frame: &mut Frame,
    options: &[MainMenuOption],
    cursor: usize,
    status: Option<&str>,
    colors: &MenuColors,
) {
    let area = frame.area();

    let status_height: u16 = if status.is_some() { 1 } else { 0 };

    let [_, title_area, _, options_area, _, status_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(options.len() as u16),
        Constraint::Length(2),
        Constraint::Length(status_height),
        Constraint::Fill(1),
    ])
    .areas(area);

    let title = Paragraph::new("Adventerm")
        .alignment(Alignment::Center)
        .style(colors.title_style());
    frame.render_widget(title, title_area);

    let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
    let accels = accel::assign(&labels);

    let lines: Vec<Line> = labels
        .iter()
        .zip(accels.iter())
        .enumerate()
        .map(|(i, (label, a))| accel::line(label, *a, i == cursor, colors))
        .collect();

    let [options_centered] = Layout::horizontal([Constraint::Length(MAIN_MENU_OPTIONS_WIDTH)])
        .flex(Flex::Center)
        .areas(options_area);

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(colors.body_style()),
        options_centered,
    );

    if let Some(msg) = status {
        frame.render_widget(
            Paragraph::new(msg.to_string())
                .alignment(Alignment::Center)
                .style(Style::default().fg(colors.status).bg(colors.background)),
            status_area,
        );
    }
}
