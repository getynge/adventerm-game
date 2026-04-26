use std::time::SystemTime;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use adventerm_lib::SaveSlot;

use crate::config::{rgb_to_color, MenuPalette};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Load,
    SavePicker,
}

pub fn render(
    frame: &mut Frame,
    saves: &[SaveSlot],
    cursor: usize,
    mode: Mode,
    pending_delete: Option<usize>,
    palette: &MenuPalette,
) {
    let area = frame.area();
    let bg = rgb_to_color(palette.background);
    let text = rgb_to_color(palette.text);
    let title_color = rgb_to_color(palette.title);
    let cursor_fg = rgb_to_color(palette.cursor_fg);
    let cursor_bg = rgb_to_color(palette.cursor_bg);

    let (title, footer) = match mode {
        Mode::Load => (" Load Game ", "Enter: load   x/Del: delete   Esc: back"),
        Mode::SavePicker => (" Save ", "Enter: save   Esc: back"),
    };

    let mut rows: Vec<String> = Vec::new();
    if matches!(mode, Mode::SavePicker) {
        rows.push("+ New save...".to_string());
    }
    for slot in saves {
        rows.push(format!("{}  ({})", slot.name, format_modified(slot.modified)));
    }
    if rows.is_empty() {
        rows.push("(no saves yet)".to_string());
    }

    let row_width = rows.iter().map(|s| s.len()).max().unwrap_or(20);
    let popup_width = (row_width.max(footer.len()) as u16 + 6).min(area.width.max(8));
    let popup_height = (rows.len() as u16 + 4).min(area.height.max(6));

    let popup = popup_rect(area, popup_width, popup_height);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(title_color).bg(bg))
        .style(Style::default().fg(text).bg(bg));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let [list_area, _gap, footer_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let selected = i == cursor;
            let style = if selected {
                Style::default().fg(cursor_fg).bg(cursor_bg)
            } else {
                Style::default().fg(text).bg(bg)
            };
            let prefix = if selected { "> " } else { "  " };
            let suffix = if selected { " <" } else { "  " };
            Line::from(vec![
                Span::styled(prefix.to_string(), style),
                Span::styled(row.clone(), style),
                Span::styled(suffix.to_string(), style),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(text).bg(bg)),
        list_area,
    );
    frame.render_widget(
        Paragraph::new(footer.to_string())
            .alignment(Alignment::Center)
            .style(Style::default().fg(text).bg(bg)),
        footer_area,
    );

    if let Some(idx) = pending_delete
        && let Some(slot) = saves.get(idx)
    {
        let prompt = format!("Delete '{}'?  Enter: confirm   Esc: cancel", slot.name);
        let cwidth = (prompt.len() as u16 + 4).min(area.width.max(8));
        let confirm_area = popup_rect(area, cwidth, 3);
        frame.render_widget(Clear, confirm_area);
        let confirm_block = Block::default()
            .borders(Borders::ALL)
            .title(" Confirm ")
            .title_style(Style::default().fg(title_color).bg(bg))
            .style(Style::default().fg(text).bg(bg));
        let cinner = confirm_block.inner(confirm_area);
        frame.render_widget(confirm_block, confirm_area);
        frame.render_widget(
            Paragraph::new(prompt)
                .alignment(Alignment::Center)
                .style(Style::default().fg(text).bg(bg)),
            cinner,
        );
    }
}

fn format_modified(t: SystemTime) -> String {
    let now = SystemTime::now();
    match now.duration_since(t) {
        Ok(d) => {
            let secs = d.as_secs();
            if secs < 60 {
                format!("{secs}s ago")
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else if secs < 86_400 {
                format!("{}h ago", secs / 3600)
            } else {
                format!("{}d ago", secs / 86_400)
            }
        }
        Err(_) => "now".to_string(),
    }
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
