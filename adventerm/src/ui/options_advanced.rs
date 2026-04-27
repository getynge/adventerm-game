use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};

use crate::app::{options_advanced_row_label, App};
use crate::menu::{MenuState, OptionsAdvancedRow};
use crate::ui::accel;
use crate::ui::colors::{menu_block, MenuColors};
use crate::ui::layout::{
    popup_rect, PANEL_HORIZONTAL_PAD, PANEL_MIN_HEIGHT, PANEL_MIN_WIDTH, PANEL_VERTICAL_PAD,
};

pub fn render(
    frame: &mut Frame,
    app: &App,
    menu: &MenuState<OptionsAdvancedRow>,
    colors: &MenuColors,
) {
    let area = frame.area();

    let rows = menu.options();
    let labels: Vec<String> = rows
        .iter()
        .map(|r| options_advanced_row_label(app.config(), *r))
        .collect();
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let accels = accel::assign(&label_refs);
    let cursor = menu.cursor().min(rows.len().saturating_sub(1));

    let widest = labels
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(PANEL_MIN_WIDTH as usize);
    let panel_w = (widest as u16 + PANEL_HORIZONTAL_PAD).min(area.width.max(PANEL_MIN_WIDTH));
    let panel_h = (rows.len() as u16 + PANEL_VERTICAL_PAD).min(area.height.max(PANEL_MIN_HEIGHT));

    let panel = popup_rect(area, panel_w, panel_h);
    frame.render_widget(Clear, panel);

    let block = menu_block(" Advanced ", colors);
    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    let [list_area, _gap, footer_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let lines: Vec<Line> = labels
        .iter()
        .zip(accels.iter())
        .enumerate()
        .map(|(i, (label, a))| accel::line(label, *a, i == cursor, colors))
        .collect();

    frame.render_widget(Paragraph::new(lines).style(colors.body_style()), list_area);
    frame.render_widget(
        Paragraph::new("Enter: toggle   Esc: back")
            .alignment(Alignment::Center)
            .style(colors.body_style()),
        footer_area,
    );
}
