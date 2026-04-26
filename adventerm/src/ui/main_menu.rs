use adventerm_lib::MainMenuOption;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::ui::menu;

pub fn render(frame: &mut Frame, options: &[MainMenuOption], cursor: usize) {
    let area = frame.area();

    let [_, title_area, _, options_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(options.len() as u16),
        Constraint::Fill(1),
    ])
    .areas(area);

    let title = Paragraph::new("Adventerm")
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(title, title_area);

    let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
    let accels = menu::assign(&labels);

    let lines: Vec<Line> = labels
        .iter()
        .zip(accels.iter())
        .enumerate()
        .map(|(i, (label, accel))| menu::line(label, *accel, i == cursor))
        .collect();

    let [options_centered] = Layout::horizontal([Constraint::Length(12)])
        .flex(Flex::Center)
        .areas(options_area);

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        options_centered,
    );
}
