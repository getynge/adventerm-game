use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};

use crate::menu::PauseMenuOption;
use crate::ui::accel;
use crate::ui::colors::{menu_block, MenuColors};
use crate::ui::layout::{popup_rect, PAUSE_MENU_VERTICAL_PAD, PAUSE_MENU_WIDTH};

pub fn render(frame: &mut Frame, cursor: usize, colors: &MenuColors) {
    let area = frame.area();
    let options = &PauseMenuOption::ALL;
    let popup = popup_rect(
        area,
        PAUSE_MENU_WIDTH,
        PAUSE_MENU_VERTICAL_PAD + options.len() as u16,
    );

    frame.render_widget(Clear, popup);

    let block = menu_block(" Paused ", colors);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
    let accels = accel::assign(&labels);

    let lines: Vec<Line> = labels
        .iter()
        .zip(accels.iter())
        .enumerate()
        .map(|(i, (label, a))| accel::line(label, *a, i == cursor, colors))
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(colors.body_style()),
        inner,
    );
}
