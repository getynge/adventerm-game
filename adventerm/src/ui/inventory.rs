use adventerm_lib::{AbilityKind, GameState, Stats};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::menu::InventoryTab;
use crate::ui::colors::{menu_block, MenuColors};

/// Target popup size: ~50% of the frame, clamped so it stays readable on
/// small terminals and doesn't dominate large ones.
const TARGET_FRAC: u16 = 2; // 1 / TARGET_FRAC of each axis
const MIN_WIDTH: u16 = 36;
const MIN_HEIGHT: u16 = 12;
const MAX_WIDTH: u16 = 60;
const MAX_HEIGHT: u16 = 22;

/// Height of the tab header row. Single line plus a separating blank line.
const TAB_HEADER_HEIGHT: u16 = 2;

/// Hint shown along the bottom of every tab.
const HINT: &str = "Tab: switch    Enter: use    Esc: close";

pub fn render(
    frame: &mut Frame,
    game: &GameState,
    tab: InventoryTab,
    item_cursor: usize,
    ability_cursor: usize,
    colors: &MenuColors,
) {
    let area = frame.area();
    let popup = popup_rect(area);
    frame.render_widget(Clear, popup);

    let block = menu_block(" Inventory ", colors);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [header, body] = Layout::vertical([
        Constraint::Length(TAB_HEADER_HEIGHT),
        Constraint::Min(0),
    ])
    .areas(inner);

    render_tab_header(frame, tab, header, colors);

    match tab {
        InventoryTab::Items => render_items(frame, game, item_cursor, body, colors),
        InventoryTab::Abilities => {
            render_abilities(frame, game, ability_cursor, body, colors);
        }
        InventoryTab::Stats => render_stats(frame, &game.stats, game.cur_health, body, colors),
    }
}

fn render_tab_header(frame: &mut Frame, active: InventoryTab, area: Rect, colors: &MenuColors) {
    let mut spans: Vec<Span> = Vec::with_capacity(InventoryTab::ALL.len() * 2);
    for (i, tab) in InventoryTab::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", colors.body_style()));
        }
        let label = format!(" {} ", tab.label());
        let style = if *tab == active {
            colors.cursor_style()
        } else {
            colors.body_style()
        };
        spans.push(Span::styled(label, style));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Left),
        area,
    );
}

fn render_items(
    frame: &mut Frame,
    game: &GameState,
    cursor: usize,
    area: Rect,
    colors: &MenuColors,
) {
    let mut lines: Vec<Line> = Vec::new();
    if game.inventory.is_empty() {
        lines.push(Line::from(Span::styled("(empty)", colors.body_style())));
    } else {
        for (i, kind) in game.inventory.iter().enumerate() {
            let marker = if i == cursor { '>' } else { ' ' };
            let style = if i == cursor {
                colors.cursor_style()
            } else {
                colors.body_style()
            };
            lines.push(Line::from(Span::styled(
                format!("{} {}", marker, kind.name()),
                style,
            )));
        }
    }
    push_hint(&mut lines, colors);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(colors.body_style()),
        area,
    );
}

fn render_abilities(
    frame: &mut Frame,
    game: &GameState,
    cursor: usize,
    area: Rect,
    colors: &MenuColors,
) {
    let abilities = &game.abilities;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        "Active slots".to_string(),
        colors.title_style(),
    )));
    for (i, slot) in abilities.active_iter().enumerate() {
        let marker = if i == cursor { '>' } else { ' ' };
        let label = match slot {
            Some(kind) => kind.name().to_string(),
            None => "(empty)".to_string(),
        };
        let style = if i == cursor {
            colors.cursor_style()
        } else {
            colors.body_style()
        };
        lines.push(Line::from(Span::styled(
            format!("{} {}. {}", marker, i + 1, label),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Passive slots".to_string(),
        colors.title_style(),
    )));
    for (i, _slot) in abilities.passive_iter().enumerate() {
        // Passives have no kinds yet, so every slot reads as empty.
        lines.push(Line::from(Span::styled(
            format!("  {}. (empty)", i + 1),
            colors.body_style(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(
            "Learned: {}",
            format_learned_active(&abilities.learned_active)
        ),
        colors.body_style(),
    )));

    push_hint(&mut lines, colors);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(colors.body_style()),
        area,
    );
}

fn format_learned_active(learned: &[AbilityKind]) -> String {
    if learned.is_empty() {
        "(none)".to_string()
    } else {
        learned
            .iter()
            .map(|k| k.name())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn render_stats(
    frame: &mut Frame,
    stats: &Stats,
    cur_health: u8,
    area: Rect,
    colors: &MenuColors,
) {
    let lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!("Health    {} / {}", cur_health, stats.health),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            format!("Attack    {}", stats.attack),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            format!("Defense   {}", stats.defense),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            format!("Speed     {}", stats.speed),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            format!("Attribute {}", stats.attribute.name()),
            colors.body_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(HINT, hint_style(colors))),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(colors.body_style()),
        area,
    );
}

fn push_hint(lines: &mut Vec<Line>, colors: &MenuColors) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(HINT, hint_style(colors))));
}

fn hint_style(colors: &MenuColors) -> Style {
    Style::default().fg(colors.accel).bg(colors.background)
}

fn popup_rect(area: Rect) -> Rect {
    let target_w = (area.width / TARGET_FRAC)
        .clamp(MIN_WIDTH, MAX_WIDTH)
        .min(area.width);
    let target_h = (area.height / TARGET_FRAC)
        .clamp(MIN_HEIGHT, MAX_HEIGHT)
        .min(area.height);
    let x = area.x + area.width.saturating_sub(target_w) / 2;
    let y = area.y + area.height.saturating_sub(target_h) / 2;
    Rect {
        x,
        y,
        width: target_w,
        height: target_h,
    }
}
