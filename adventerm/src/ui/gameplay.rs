use adventerm_lib::{GameState, Tile};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::config::{rgb_to_color, ColorScheme};

pub fn render(frame: &mut Frame, state: &GameState, status: Option<&str>, scheme: &ColorScheme) {
    let area = frame.area();

    let [main_area, right_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(20)]).areas(area);

    let [center_area, dialog_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(5)]).areas(main_area);

    render_world(frame, state, center_area, scheme);
    render_dialog(frame, state, status, dialog_area, scheme);
    render_actions(frame, right_area, scheme);
}

fn render_world(frame: &mut Frame, state: &GameState, area: ratatui::layout::Rect, scheme: &ColorScheme) {
    let world_bg = rgb_to_color(scheme.world.background);
    let menu_bg = rgb_to_color(scheme.menu.background);
    let menu_text = rgb_to_color(scheme.menu.text);
    let title_color = rgb_to_color(scheme.menu.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" World ")
        .title_style(Style::default().fg(title_color).bg(menu_bg))
        .style(Style::default().fg(menu_text).bg(menu_bg));
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

    let player = rgb_to_color(scheme.world.player);
    let interactive = rgb_to_color(scheme.world.interactive);
    let floor = rgb_to_color(scheme.world.floor);
    let wall = rgb_to_color(scheme.world.wall);

    let mut lines: Vec<Line> = Vec::with_capacity(view_h as usize);
    for y in off_y..off_y + view_h {
        let mut spans: Vec<Span> = Vec::with_capacity(view_w as usize);
        for x in off_x..off_x + view_w {
            let (glyph, fg) = match state.tile_at(x as usize, y as usize) {
                Tile::Player => ('@', player),
                Tile::Door => ('+', interactive),
                Tile::Floor => ('.', floor),
                Tile::Wall => ('#', wall),
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
    area: ratatui::layout::Rect,
    scheme: &ColorScheme,
) {
    let bg = rgb_to_color(scheme.menu.background);
    let text = rgb_to_color(scheme.menu.text);
    let title = rgb_to_color(scheme.menu.title);
    let status_color = rgb_to_color(scheme.menu.status);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Dialog ")
        .title_style(Style::default().fg(title).bg(bg))
        .style(Style::default().fg(text).bg(bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    if state.player_on_door().is_some() {
        lines.push(Line::from(Span::styled(
            "Press Enter to open door",
            Style::default().fg(text).bg(bg),
        )));
    }
    if let Some(msg) = status {
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(status_color).bg(bg),
        )));
    }
    if !lines.is_empty() {
        frame.render_widget(
            Paragraph::new(lines).style(Style::default().fg(text).bg(bg)),
            inner,
        );
    }
}

fn render_actions(frame: &mut Frame, area: ratatui::layout::Rect, scheme: &ColorScheme) {
    let bg = rgb_to_color(scheme.menu.background);
    let text = rgb_to_color(scheme.menu.text);
    let title = rgb_to_color(scheme.menu.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Actions ")
        .title_style(Style::default().fg(title).bg(bg))
        .style(Style::default().fg(text).bg(bg));
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
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(text).bg(bg)),
        inner,
    );
}

fn center_rect(area: ratatui::layout::Rect, width: u16, height: u16) -> ratatui::layout::Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    ratatui::layout::Rect {
        x,
        y,
        width,
        height,
    }
}
