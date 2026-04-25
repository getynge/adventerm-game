use adventerm_lib::{Tile, World};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(frame: &mut Frame, world: &World) {
    let area = frame.area();

    let [main_area, right_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(20)]).areas(area);

    let [center_area, dialog_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(5)]).areas(main_area);

    render_world(frame, world, center_area);
    render_dialog(frame, dialog_area);
    render_actions(frame, right_area);
}

fn render_world(frame: &mut Frame, world: &World, area: ratatui::layout::Rect) {
    let block = Block::default().borders(Borders::ALL).title(" World ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::with_capacity(world.height());
    for y in 0..world.height() {
        let mut row = String::with_capacity(world.width() * 2);
        for x in 0..world.width() {
            let glyph = match world.tile_at(x, y) {
                Tile::Player => 'P',
                Tile::Ground => 'O',
            };
            row.push(glyph);
            if x + 1 < world.width() {
                row.push(' ');
            }
        }
        lines.push(Line::from(row));
    }

    let map_width = (world.width() * 2 - 1) as u16;
    let map_height = world.height() as u16;
    let centered = center_rect(inner, map_width, map_height);

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), centered);
}

fn render_dialog(frame: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Dialog ");
    frame.render_widget(block, area);
}

fn render_actions(frame: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Actions ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from("Move: WASD"),
        Line::from("      / arrows"),
        Line::from(""),
        Line::from("Pause: Esc"),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
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
