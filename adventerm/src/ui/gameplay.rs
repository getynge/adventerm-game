use adventerm_lib::{GameState, Tile};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::ui::colors::{menu_block, SchemeColors};

const ACTIONS_PANEL_WIDTH: u16 = 20;
const DIALOG_PANEL_HEIGHT: u16 = 5;

pub fn render(frame: &mut Frame, state: &GameState, status: Option<&str>, colors: &SchemeColors) {
    let area = frame.area();

    let [main_area, right_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(ACTIONS_PANEL_WIDTH)])
            .areas(area);

    let [center_area, dialog_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(DIALOG_PANEL_HEIGHT)])
            .areas(main_area);

    render_world(frame, state, center_area, colors);
    render_dialog(frame, state, status, dialog_area, &colors.menu);
    render_actions(frame, right_area, &colors.menu);
}

fn render_world(frame: &mut Frame, state: &GameState, area: Rect, colors: &SchemeColors) {
    let block = menu_block(" World ", &colors.menu);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let room = state.current_room();
    let room_w = room.width as u16;
    let room_h = room.height as u16;
    let view_w = room_w.min(inner.width);
    let view_h = room_h.min(inner.height);

    let (px, py) = state.player;
    let off_x = scroll_offset(px as u16, view_w, room_w);
    let off_y = scroll_offset(py as u16, view_h, room_h);

    let world_bg = colors.world.background;
    let mut lines: Vec<Line> = Vec::with_capacity(view_h as usize);
    for y in off_y..off_y + view_h {
        let mut spans: Vec<Span> = Vec::with_capacity(view_w as usize);
        for x in off_x..off_x + view_w {
            let (tx, ty) = (x as usize, y as usize);
            let (glyph, fg) = if state.is_visible(tx, ty) {
                match state.tile_at(tx, ty) {
                    Tile::Player => ('@', colors.world.player),
                    Tile::Door => ('+', colors.world.interactive),
                    Tile::Floor => ('.', colors.world.floor),
                    Tile::Wall => ('#', colors.world.wall),
                }
            } else if state.is_explored(tx, ty) {
                match state.terrain_at(tx, ty) {
                    Tile::Door => ('+', colors.world.memory_interactive),
                    Tile::Floor => ('.', colors.world.memory_floor),
                    Tile::Wall => ('#', colors.world.memory_wall),
                    Tile::Player => (' ', world_bg),
                }
            } else {
                (' ', world_bg)
            };
            spans.push(Span::styled(
                glyph.to_string(),
                Style::default().fg(fg).bg(world_bg),
            ));
        }
        lines.push(Line::from(spans));
    }

    let viewport = center_rect(inner, view_w, view_h);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(Style::default().bg(world_bg)),
        viewport,
    );
}

/// Pick a scroll origin so the player sits near the centre of the viewport,
/// clamped so the viewport never extends past the room edges.
fn scroll_offset(player: u16, view: u16, total: u16) -> u16 {
    if total <= view {
        return 0;
    }
    let half = view / 2;
    player.saturating_sub(half).min(total - view)
}

fn render_dialog(
    frame: &mut Frame,
    state: &GameState,
    status: Option<&str>,
    area: Rect,
    colors: &crate::ui::colors::MenuColors,
) {
    let block = menu_block(" Dialog ", colors);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    if state.player_on_door().is_some() {
        lines.push(Line::from(Span::styled(
            "Press Enter to open door",
            colors.body_style(),
        )));
    }
    if let Some(msg) = status {
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(colors.status).bg(colors.background),
        )));
    }
    if !lines.is_empty() {
        frame.render_widget(Paragraph::new(lines).style(colors.body_style()), inner);
    }
}

fn render_actions(frame: &mut Frame, area: Rect, colors: &crate::ui::colors::MenuColors) {
    let block = menu_block(" Actions ", colors);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from("Move: WASD"),
        Line::from("      / arrows"),
        Line::from(""),
        Line::from("Interact: Enter"),
        Line::from(""),
        Line::from("Pause: Esc"),
    ];
    frame.render_widget(Paragraph::new(lines).style(colors.body_style()), inner);
}

fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}
