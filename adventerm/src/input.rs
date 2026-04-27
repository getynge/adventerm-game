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
    /// Open or close the developer console overlay. Bound to backtick when
    /// `Config::dev_console_enabled()` is true; otherwise the keystroke
    /// falls through as a literal character via `Hotkey`.
    ToggleConsole,
    Hotkey(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputAction {
    Char(char),
    Backspace,
    Confirm,
    Escape,
}

/// Console-screen-specific actions. Expressive enough that the console
/// handler does not need raw `KeyCode`s. Anything not enumerated here is
/// ignored while the console is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleInputAction {
    Char(char),
    Backspace,
    Submit,
    Cancel,
    Tab,
    HistoryUp,
    HistoryDown,
    /// Backtick — closes the console (the same key that opened it).
    Toggle,
}

pub fn translate(event: &Event, binds: &KeybindMap, console_enabled: bool) -> Option<Action> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if console_enabled && matches!(key.code, KeyCode::Char('`')) {
        return Some(Action::ToggleConsole);
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

/// Translate a key event for the developer console. Backtick closes,
/// Tab completes, Enter submits, Esc cancels, arrows navigate history.
/// Other keys feed character/backspace edits to the input buffer.
pub fn translate_console(event: &Event) -> Option<ConsoleInputAction> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Char('`') => Some(ConsoleInputAction::Toggle),
        KeyCode::Char(c) => Some(ConsoleInputAction::Char(c)),
        KeyCode::Backspace => Some(ConsoleInputAction::Backspace),
        KeyCode::Enter => Some(ConsoleInputAction::Submit),
        KeyCode::Esc => Some(ConsoleInputAction::Cancel),
        KeyCode::Tab => Some(ConsoleInputAction::Tab),
        KeyCode::Up => Some(ConsoleInputAction::HistoryUp),
        KeyCode::Down => Some(ConsoleInputAction::HistoryDown),
        _ => None,
    }
}
