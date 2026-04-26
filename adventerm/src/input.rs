use crossterm::event::{Event, KeyCode, KeyEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Escape,
    Hotkey(char),
}

pub fn translate(event: &Event) -> Option<Action> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Up => Some(Action::Up),
        KeyCode::Down => Some(Action::Down),
        KeyCode::Left => Some(Action::Left),
        KeyCode::Right => Some(Action::Right),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Confirm),
        KeyCode::Esc => Some(Action::Escape),
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'w' | 'k' => Some(Action::Up),
            's' | 'j' => Some(Action::Down),
            'a' | 'h' => Some(Action::Left),
            'd' | 'l' => Some(Action::Right),
            _ => Some(Action::Hotkey(c)),
        },
        _ => None,
    }
}
