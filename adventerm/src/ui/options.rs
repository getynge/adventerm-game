use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{options_row_label, App};
use crate::config::BoundAction;
use crate::menu::{MenuState, OptionsRow};
use crate::ui::accel;
use crate::ui::colors::{menu_block, MenuColors};
use crate::ui::layout::{
    popup_rect, PANEL_HORIZONTAL_PAD, PANEL_MIN_HEIGHT, PANEL_MIN_WIDTH, PANEL_VERTICAL_PAD,
    POPUP_BORDER_PAD, POPUP_MIN_WIDTH, STATUS_POPUP_HEIGHT,
};

pub fn render(
    frame: &mut Frame,
    app: &App,
    menu: &MenuState<OptionsRow>,
    status: Option<&str>,
    rebind_target: Option<BoundAction>,
    colors: &MenuColors,
) {
    let area = frame.area();

    let rows = menu.options();
    let labels: Vec<String> = rows
        .iter()
        .map(|r| options_row_label(app.config(), *r))
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

    let block = menu_block(" Options ", colors);
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

    let footer_text = "Enter: select   Esc: back";
    frame.render_widget(
        Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(colors.body_style()),
        footer_area,
    );

    if let Some(action) = rebind_target {
        render_capture_overlay(frame, area, action, colors);
    } else if let Some(msg) = status {
        // Lightly highlight any pending status (e.g. "Defaults restored").
        let status_w = (msg.len() as u16 + POPUP_BORDER_PAD).min(area.width.max(POPUP_MIN_WIDTH));
        let status_area = popup_rect(area, status_w, STATUS_POPUP_HEIGHT);
        // Place under the panel — just nudge the rect down to avoid overlap.
        let nudged = Rect {
            x: status_area.x,
            y: status_area
                .y
                .saturating_add(panel_h / 2 + 2)
                .min(area.bottom().saturating_sub(3)),
            width: status_area.width,
            height: status_area.height,
        };
        if nudged.bottom() <= area.bottom() {
            frame.render_widget(Clear, nudged);
            let sblock = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(colors.status).bg(colors.background));
            let sinner = sblock.inner(nudged);
            frame.render_widget(sblock, nudged);
            frame.render_widget(
                Paragraph::new(msg.to_string())
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(colors.status).bg(colors.background)),
                sinner,
            );
        }
    }
}

fn render_capture_overlay(
    frame: &mut Frame,
    area: Rect,
    action: BoundAction,
    colors: &MenuColors,
) {
    let prompt = format!("Press a key for '{}'   (Esc to cancel)", action.label());
    let width = (prompt.len() as u16 + POPUP_BORDER_PAD).min(area.width.max(POPUP_MIN_WIDTH));
    let popup = popup_rect(area, width, STATUS_POPUP_HEIGHT);
    frame.render_widget(Clear, popup);
    let block = menu_block(" Rebind ", colors);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(prompt)
            .alignment(Alignment::Center)
            .style(colors.body_style()),
        inner,
    );
}
