use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::config::{BoundAction, KeybindMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    QuickUp,
    QuickDown,
    QuickLeft,
    QuickRight,
    Confirm,
    Escape,
    Delete,
    Inventory,
    Hotkey(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputAction {
    Char(char),
    Backspace,
    Confirm,
    Escape,
}

pub fn translate(event: &Event, binds: &KeybindMap) -> Option<Action> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if let Some(action) = binds.lookup(&key.code) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        return Some(match action {
            BoundAction::Up if shift => Action::QuickUp,
            BoundAction::Down if shift => Action::QuickDown,
            BoundAction::Left if shift => Action::QuickLeft,
            BoundAction::Right if shift => Action::QuickRight,
            BoundAction::Up => Action::Up,
            BoundAction::Down => Action::Down,
            BoundAction::Left => Action::Left,
            BoundAction::Right => Action::Right,
            BoundAction::Confirm => Action::Confirm,
            BoundAction::Escape => Action::Escape,
            BoundAction::Delete => Action::Delete,
        });
    }
    if matches!(key.code, KeyCode::Tab) {
        return Some(Action::Inventory);
    }
    if let KeyCode::Char(c) = key.code {
        return Some(Action::Hotkey(c));
    }
    None
}

pub fn translate_text(event: &Event) -> Option<TextInputAction> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Char(c) => Some(TextInputAction::Char(c)),
        KeyCode::Backspace => Some(TextInputAction::Backspace),
        KeyCode::Enter => Some(TextInputAction::Confirm),
        KeyCode::Esc => Some(TextInputAction::Escape),
        _ => None,
    }
}
