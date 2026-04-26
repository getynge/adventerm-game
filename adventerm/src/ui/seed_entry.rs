use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::widgets::{Clear, Paragraph};

use crate::ui::colors::{menu_block, MenuColors};
use crate::ui::layout::{popup_rect, SEED_ENTRY_HEIGHT, SEED_ENTRY_WIDTH};

pub fn render(frame: &mut Frame, buffer: &str, colors: &MenuColors) {
    let area = frame.area();
    let popup = popup_rect(area, SEED_ENTRY_WIDTH, SEED_ENTRY_HEIGHT);
    frame.render_widget(Clear, popup);

    let block = menu_block(" New game — seed ", colors);
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
            .style(colors.body_style()),
        input_area,
    );
    frame.render_widget(
        Paragraph::new("Enter: start (blank = random)   Esc: cancel")
            .alignment(Alignment::Center)
            .style(colors.body_style()),
        footer_area,
    );
}
