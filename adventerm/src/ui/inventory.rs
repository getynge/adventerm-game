use adventerm_lib::GameState;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::ui::colors::{menu_block, MenuColors};

/// Target popup size: ~50% of the frame, clamped so it stays readable on
/// small terminals and doesn't dominate large ones.
const TARGET_FRAC: u16 = 2; // 1 / TARGET_FRAC of each axis
const MIN_WIDTH: u16 = 30;
const MIN_HEIGHT: u16 = 10;
const MAX_WIDTH: u16 = 60;
const MAX_HEIGHT: u16 = 20;

pub fn render(frame: &mut Frame, game: &GameState, cursor: usize, colors: &MenuColors) {
    let area = frame.area();
    let popup = popup_rect(area);
    frame.render_widget(Clear, popup);

    let block = menu_block(" Inventory ", colors);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    if game.inventory.is_empty() {
        lines.push(Line::from(Span::styled(
            "(empty)",
            Style::default().fg(colors.text).bg(colors.background),
        )));
    } else {
        for (i, item) in game.inventory.iter().enumerate() {
            let marker = if i == cursor { '>' } else { ' ' };
            let style = if i == cursor {
                colors.cursor_style()
            } else {
                colors.body_style()
            };
            lines.push(Line::from(Span::styled(
                format!("{} {}", marker, item.kind.name()),
                style,
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Enter: place    Tab/Esc: close",
        Style::default().fg(colors.accel).bg(colors.background),
    )));

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(colors.body_style()),
        inner,
    );
}

fn popup_rect(area: Rect) -> Rect {
    let target_w = (area.width / TARGET_FRAC).clamp(MIN_WIDTH, MAX_WIDTH).min(area.width);
    let target_h = (area.height / TARGET_FRAC).clamp(MIN_HEIGHT, MAX_HEIGHT).min(area.height);
    let x = area.x + area.width.saturating_sub(target_w) / 2;
    let y = area.y + area.height.saturating_sub(target_h) / 2;
    Rect {
        x,
        y,
        width: target_w,
        height: target_h,
    }
}
