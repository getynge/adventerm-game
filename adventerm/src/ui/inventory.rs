use adventerm_lib::{AbilityKind, EquipSlot, Equipment, GameState, ItemKind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::menu::{InventoryTab, ItemsFocus, PendingConsume, PendingIntent};
use crate::ui::colors::{menu_block, MenuColors};

/// Target popup size: ~50% of the frame, clamped so it stays readable on
/// small terminals and doesn't dominate large ones.
const TARGET_FRAC: u16 = 2; // 1 / TARGET_FRAC of each axis
const MIN_WIDTH: u16 = 48;
const MIN_HEIGHT: u16 = 14;
const MAX_WIDTH: u16 = 72;
const MAX_HEIGHT: u16 = 24;

/// Height of the tab header row. Single line plus a separating blank line.
const TAB_HEADER_HEIGHT: u16 = 2;

/// Width of the equipment sidebar inside the Items tab body.
const EQUIPMENT_SIDEBAR_WIDTH: u16 = 22;

/// Hint shown along the bottom of every tab.
const HINT: &str = "Tab: switch    Enter: use    Esc: close";
const PENDING_CONSUME_HINT: &str = "↑/↓: pick slot    Enter: confirm    Esc: cancel";

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    game: &GameState,
    tab: InventoryTab,
    items_focus: ItemsFocus,
    item_cursor: usize,
    equipment_cursor: usize,
    ability_cursor: usize,
    pending_consume: Option<PendingConsume>,
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
        InventoryTab::Items => render_items(
            frame,
            game,
            items_focus,
            item_cursor,
            equipment_cursor,
            body,
            colors,
        ),
        InventoryTab::Abilities => {
            render_abilities(frame, game, ability_cursor, pending_consume, body, colors);
        }
        InventoryTab::Stats => render_stats(frame, game, body, colors),
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
    focus: ItemsFocus,
    item_cursor: usize,
    equipment_cursor: usize,
    area: Rect,
    colors: &MenuColors,
) {
    let sidebar_w = EQUIPMENT_SIDEBAR_WIDTH.min(area.width.saturating_sub(1));
    let [list_area, sidebar_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(sidebar_w),
    ])
    .areas(area);

    render_items_list(frame, game, focus, item_cursor, list_area, colors);
    render_equipment_sidebar(frame, game.equipment(), focus, equipment_cursor, sidebar_area, colors);
}

fn render_items_list(
    frame: &mut Frame,
    game: &GameState,
    focus: ItemsFocus,
    cursor: usize,
    area: Rect,
    colors: &MenuColors,
) {
    let mut lines: Vec<Line> = Vec::new();
    if game.inventory().is_empty() {
        lines.push(Line::from(Span::styled("(empty)", colors.body_style())));
    } else {
        for (i, kind) in game.inventory().iter().enumerate() {
            let is_cursor = i == cursor && focus == ItemsFocus::List;
            let marker = if is_cursor { '>' } else { ' ' };
            let style = if is_cursor {
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
    push_hint(&mut lines, colors, HINT);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(colors.body_style()),
        area,
    );
}

fn render_equipment_sidebar(
    frame: &mut Frame,
    equipment: &Equipment,
    focus: ItemsFocus,
    cursor: usize,
    area: Rect,
    colors: &MenuColors,
) {
    let mut lines: Vec<Line> = Vec::with_capacity(EquipSlot::ALL.len() + 2);
    lines.push(Line::from(Span::styled("Equipment", colors.title_style())));
    for (i, slot) in EquipSlot::ALL.iter().enumerate() {
        let is_cursor = i == cursor && focus == ItemsFocus::Sidebar;
        let marker = if is_cursor { '>' } else { ' ' };
        let style = if is_cursor {
            colors.cursor_style()
        } else {
            colors.body_style()
        };
        let occupant = equipment
            .slot(*slot)
            .map(short_name)
            .unwrap_or_else(|| "(empty)".to_string());
        lines.push(Line::from(Span::styled(
            format!("{} {:5} {}", marker, slot.name(), occupant),
            style,
        )));
    }
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
    pending_consume: Option<PendingConsume>,
    area: Rect,
    colors: &MenuColors,
) {
    let abilities = game.abilities();
    let mut lines: Vec<Line> = Vec::new();

    if let Some(pending) = pending_consume {
        lines.push(Line::from(Span::styled(
            pending_banner(pending),
            colors.title_style(),
        )));
        lines.push(Line::from(""));
    }

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

    let hint = if pending_consume.is_some() {
        PENDING_CONSUME_HINT
    } else {
        HINT
    };
    push_hint(&mut lines, colors, hint);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(colors.body_style()),
        area,
    );
}

fn pending_banner(pending: PendingConsume) -> String {
    match pending.intent {
        PendingIntent::AbilitySlot => format!(
            "Pick a slot to learn {} into. (overwrites)",
            pending.kind.name()
        ),
    }
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

fn render_stats(frame: &mut Frame, game: &GameState, area: Rect, colors: &MenuColors) {
    let base = *game.stats();
    let effective = game.effective_stats();
    let cur_health = game.cur_health();
    let lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!("Health    {} / {}", cur_health, effective.health),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            stat_row("Attack   ", base.attack, effective.attack),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            stat_row("Defense  ", base.defense, effective.defense),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            stat_row("Speed    ", base.speed, effective.speed),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            format!("Vision    {} tiles", game.vision_radius()),
            colors.body_style(),
        )),
        Line::from(Span::styled(
            format!("Attribute {}", effective.attribute.name()),
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

/// Render `Total (base + bonus)` when bonus is non-zero, otherwise just
/// `Total`. Keeps unmodified rows uncluttered.
fn stat_row(label: &str, base: u8, total: u8) -> String {
    if base == total {
        format!("{} {}", label, total)
    } else if total >= base {
        format!("{} {} ({} +{})", label, total, base, total - base)
    } else {
        format!("{} {} ({} -{})", label, total, base, base - total)
    }
}

fn short_name(kind: ItemKind) -> String {
    // The full equipment names ("Goggles of Seeing", "Woven Trousers") do
    // not fit the sidebar comfortably; trim the "Woven " prefix and the
    // descriptive suffix so the label stays inside `EQUIPMENT_SIDEBAR_WIDTH`.
    let name = kind.name();
    if let Some(rest) = name.strip_prefix("Woven ") {
        return rest.to_string();
    }
    if let Some(idx) = name.find(" of ") {
        return name[..idx].to_string();
    }
    name.to_string()
}

fn push_hint(lines: &mut Vec<Line>, colors: &MenuColors, hint: &str) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(hint.to_string(), hint_style(colors))));
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

