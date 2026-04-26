use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::config::{rgb_to_color, MenuPalette};
use crate::menu::MainMenuOption;
use crate::ui::accel;

pub fn render(
    frame: &mut Frame,
    cursor: usize,
    any_saves: bool,
    status: Option<&str>,
    palette: &MenuPalette,
) {
    let area = frame.area();
    let options = MainMenuOption::available(any_saves);

    let bg = rgb_to_color(palette.background);
    let text = rgb_to_color(palette.text);
    let title_color = rgb_to_color(palette.title);
    let status_color = rgb_to_color(palette.status);

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

    let title = Paragraph::new("Adventerm").alignment(Alignment::Center).style(
        Style::default()
            .fg(title_color)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title, title_area);

    let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
    let accels = accel::assign(&labels);

    let lines: Vec<Line> = labels
        .iter()
        .zip(accels.iter())
        .enumerate()
        .map(|(i, (label, a))| accel::line(label, *a, i == cursor, palette))
        .collect();

    let [options_centered] = Layout::horizontal([Constraint::Length(14)])
        .flex(Flex::Center)
        .areas(options_area);

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(text).bg(bg)),
        options_centered,
    );

    if let Some(msg) = status {
        frame.render_widget(
            Paragraph::new(msg.to_string())
                .alignment(Alignment::Center)
                .style(Style::default().fg(status_color).bg(bg)),
            status_area,
        );
    }
}
