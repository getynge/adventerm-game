use adventerm_lib::{Battle, BattleTurn, GameState};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::ui::colors::{menu_block, MenuColors, SchemeColors};

/// Width of the rendered HP bar, in cells. The numeric "cur / max" suffix is
/// printed after the bar so the bar itself can stay short on narrow terms.
const HP_BAR_WIDTH: u16 = 20;

/// Height reserved for the combatant header (player + enemy stat blocks).
const HEADER_HEIGHT: u16 = 6;
/// Height reserved for the action menu (slot list).
const ACTIONS_HEIGHT: u16 = 8;

pub fn render(
    frame: &mut Frame,
    game: &GameState,
    battle: &Battle,
    cursor: usize,
    status: Option<&str>,
    colors: &SchemeColors,
) {
    let area = frame.area();
    frame.render_widget(Clear, area);
    let block = menu_block(" Battle ", &colors.menu);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [header_area, actions_area, log_area] = Layout::vertical([
        Constraint::Length(HEADER_HEIGHT),
        Constraint::Length(ACTIONS_HEIGHT),
        Constraint::Min(0),
    ])
    .areas(inner);

    render_header(frame, game, battle, header_area, &colors.menu);
    render_actions(frame, game, battle, cursor, actions_area, &colors.menu);
    render_log(frame, battle, status, log_area, &colors.menu);
}

fn render_header(
    frame: &mut Frame,
    game: &GameState,
    battle: &Battle,
    area: Rect,
    colors: &MenuColors,
) {
    let enemy_kind = game
        .dungeon
        .room(battle.enemy_room())
        .enemies
        .kind_of(battle.enemy_id());
    let enemy_name = enemy_kind.map(|k| k.name()).unwrap_or("Foe");
    let enemy_max = enemy_kind
        .map(|k| k.base_stats().health)
        .unwrap_or(battle.enemy_cur_hp());

    let lines = vec![
        Line::from(Span::styled(
            "You".to_string(),
            colors.title_style(),
        )),
        Line::from(hp_spans(
            battle.player_cur_hp(),
            game.stats().health,
            colors,
        )),
        Line::from(""),
        Line::from(Span::styled(enemy_name.to_string(), colors.title_style())),
        Line::from(hp_spans(battle.enemy_cur_hp(), enemy_max, colors)),
    ];
    frame.render_widget(
        Paragraph::new(lines).style(colors.body_style()),
        area,
    );
}

fn hp_spans(cur: u8, max: u8, colors: &MenuColors) -> Vec<Span<'static>> {
    let max = max.max(1);
    let filled = (cur as u32 * HP_BAR_WIDTH as u32 / max as u32) as u16;
    let empty = HP_BAR_WIDTH - filled;
    vec![
        Span::styled("[".to_string(), colors.body_style()),
        Span::styled("#".repeat(filled as usize), colors.cursor_style()),
        Span::styled("·".repeat(empty as usize), colors.body_style()),
        Span::styled(
            format!("] {} / {}", cur, max),
            colors.body_style(),
        ),
    ]
}

fn render_actions(
    frame: &mut Frame,
    game: &GameState,
    battle: &Battle,
    cursor: usize,
    area: Rect,
    colors: &MenuColors,
) {
    let mut lines: Vec<Line> = Vec::new();
    let header = match battle.turn() {
        BattleTurn::Player => "Your turn — pick an ability:",
        BattleTurn::Enemy => "Foe's turn...",
        BattleTurn::Resolved(_) => "Battle resolved.",
    };
    lines.push(Line::from(Span::styled(
        header.to_string(),
        colors.title_style(),
    )));
    let player_turn = battle.turn() == BattleTurn::Player;
    for (i, slot) in game.abilities().active_iter().enumerate() {
        let label = match slot {
            Some(kind) => kind.name().to_string(),
            None => "(empty)".to_string(),
        };
        let style = if player_turn && i == cursor {
            colors.cursor_style()
        } else {
            colors.body_style()
        };
        let marker = if player_turn && i == cursor { '>' } else { ' ' };
        lines.push(Line::from(Span::styled(
            format!("{} {}. {}", marker, i + 1, label),
            style,
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).style(colors.body_style()),
        area,
    );
}

fn render_log(
    frame: &mut Frame,
    battle: &Battle,
    status: Option<&str>,
    area: Rect,
    colors: &MenuColors,
) {
    let mut lines: Vec<Line> = battle
        .log()
        .iter()
        .map(|s| Line::from(Span::styled(s.clone(), colors.body_style())))
        .collect();
    if let Some(msg) = status {
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(colors.status).bg(colors.background),
        )));
    }
    lines.push(Line::from(""));
    let hint = match battle.turn() {
        BattleTurn::Player => "Up/Down: choose    Enter: use    Esc: flee",
        BattleTurn::Enemy => "Enter: continue",
        BattleTurn::Resolved(_) => "Enter: continue",
    };
    lines.push(Line::from(Span::styled(
        hint.to_string(),
        Style::default().fg(colors.accel).bg(colors.background),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(colors.body_style()),
        area,
    );
}
