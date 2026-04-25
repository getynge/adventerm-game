use crossterm::event::{Event, KeyCode, KeyEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Escape,
}

pub fn translate(event: &Event) -> Option<Action> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') => Some(Action::Left),
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') => Some(Action::Right),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Confirm),
        KeyCode::Esc => Some(Action::Escape),
        _ => None,
    }
}
