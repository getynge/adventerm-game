use std::time::{Duration, SystemTime};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use adventerm_lib::SaveSlot;

use crate::ui::accel::{CURSOR_BLANK, CURSOR_LEAD, CURSOR_TRAIL};
use crate::ui::colors::{menu_block, MenuColors};
use crate::ui::layout::{
    popup_rect, PANEL_MIN_HEIGHT, PANEL_VERTICAL_PAD, POPUP_BORDER_PAD, POPUP_MIN_WIDTH,
    SAVE_BROWSER_HORIZONTAL_PAD, STATUS_POPUP_HEIGHT,
};

const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(60 * 60 * 24);

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
    colors: &MenuColors,
) {
    let area = frame.area();

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

    let row_width = rows.iter().map(|s| s.len()).max().unwrap_or(0);
    let popup_width = (row_width.max(footer.len()) as u16 + SAVE_BROWSER_HORIZONTAL_PAD)
        .min(area.width.max(POPUP_MIN_WIDTH));
    let popup_height =
        (rows.len() as u16 + PANEL_VERTICAL_PAD).min(area.height.max(PANEL_MIN_HEIGHT));

    let popup = popup_rect(area, popup_width, popup_height);
    frame.render_widget(Clear, popup);
    let block = menu_block(title, colors);
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
                colors.cursor_style()
            } else {
                colors.body_style()
            };
            let prefix = if selected { CURSOR_LEAD } else { CURSOR_BLANK };
            let suffix = if selected { CURSOR_TRAIL } else { CURSOR_BLANK };
            Line::from(vec![
                Span::styled(prefix.to_string(), style),
                Span::styled(row.clone(), style),
                Span::styled(suffix.to_string(), style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).style(colors.body_style()), list_area);
    frame.render_widget(
        Paragraph::new(footer.to_string())
            .alignment(Alignment::Center)
            .style(colors.body_style()),
        footer_area,
    );

    if let Some(idx) = pending_delete
        && let Some(slot) = saves.get(idx)
    {
        let prompt = format!("Delete '{}'?  Enter: confirm   Esc: cancel", slot.name);
        let cwidth = (prompt.len() as u16 + POPUP_BORDER_PAD).min(area.width.max(POPUP_MIN_WIDTH));
        let confirm_area = popup_rect(area, cwidth, STATUS_POPUP_HEIGHT);
        frame.render_widget(Clear, confirm_area);
        let confirm_block = menu_block(" Confirm ", colors);
        let cinner = confirm_block.inner(confirm_area);
        frame.render_widget(confirm_block, confirm_area);
        frame.render_widget(
            Paragraph::new(prompt)
                .alignment(Alignment::Center)
                .style(colors.body_style()),
            cinner,
        );
    }
}

fn format_modified(t: SystemTime) -> String {
    let now = SystemTime::now();
    match now.duration_since(t) {
        Ok(d) if d < MINUTE => format!("{}s ago", d.as_secs()),
        Ok(d) if d < HOUR => format!("{}m ago", d.as_secs() / MINUTE.as_secs()),
        Ok(d) if d < DAY => format!("{}h ago", d.as_secs() / HOUR.as_secs()),
        Ok(d) => format!("{}d ago", d.as_secs() / DAY.as_secs()),
        Err(_) => "now".to_string(),
    }
}
