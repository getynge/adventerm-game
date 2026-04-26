use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{options_row_label, App};
use crate::config::{rgb_to_color, BoundAction, MenuPalette};
use crate::menu::OptionsRow;
use crate::ui::accel;

pub fn render(frame: &mut Frame, app: &App, palette: &MenuPalette) {
    let area = frame.area();
    let bg = rgb_to_color(palette.background);
    let text = rgb_to_color(palette.text);
    let title_color = rgb_to_color(palette.title);
    let status_color = rgb_to_color(palette.status);

    let rows = OptionsRow::all();
    let labels: Vec<String> = rows.iter().map(|r| options_row_label(app, *r)).collect();
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let accels = accel::assign(&label_refs);
    let cursor = app.options_cursor().min(rows.len().saturating_sub(1));

    let widest = labels.iter().map(|s| s.len()).max().unwrap_or(20);
    let panel_w = (widest as u16 + 8).min(area.width.max(20));
    let panel_h = (rows.len() as u16 + 4).min(area.height.max(6));

    let panel = popup_rect(area, panel_w, panel_h);
    frame.render_widget(Clear, panel);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Options ")
        .title_style(
            Style::default()
                .fg(title_color)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(text).bg(bg));
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
        .map(|(i, (label, a))| accel::line(label, *a, i == cursor, palette))
        .collect();

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(text).bg(bg)),
        list_area,
    );

    let footer_text = "Enter: select   Esc: back";
    frame.render_widget(
        Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(text).bg(bg)),
        footer_area,
    );

    if let Some(action) = app.rebind_target() {
        render_capture_overlay(frame, area, action, palette);
    } else if let Some(msg) = app.status() {
        // Lightly highlight any pending status (e.g. "Defaults restored").
        let status_w = (msg.len() as u16 + 4).min(area.width.max(8));
        let status_area = popup_rect(area, status_w, 3);
        // Place under the panel — just nudge the rect down to avoid overlap.
        let nudged = Rect {
            x: status_area.x,
            y: status_area.y.saturating_add(panel_h / 2 + 2).min(area.bottom().saturating_sub(3)),
            width: status_area.width,
            height: status_area.height,
        };
        if nudged.bottom() <= area.bottom() {
            frame.render_widget(Clear, nudged);
            let sblock = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(status_color).bg(bg));
            let sinner = sblock.inner(nudged);
            frame.render_widget(sblock, nudged);
            frame.render_widget(
                Paragraph::new(msg.to_string())
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(status_color).bg(bg)),
                sinner,
            );
        }
    }
}

fn render_capture_overlay(
    frame: &mut Frame,
    area: Rect,
    action: BoundAction,
    palette: &MenuPalette,
) {
    let bg = rgb_to_color(palette.background);
    let text = rgb_to_color(palette.text);
    let title = rgb_to_color(palette.title);

    let prompt = format!("Press a key for '{}'   (Esc to cancel)", action.label());
    let width = (prompt.len() as u16 + 4).min(area.width.max(8));
    let popup = popup_rect(area, width, 3);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Rebind ")
        .title_style(Style::default().fg(title).bg(bg))
        .style(Style::default().fg(text).bg(bg));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(prompt)
            .alignment(Alignment::Center)
            .style(Style::default().fg(text).bg(bg)),
        inner,
    );
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
