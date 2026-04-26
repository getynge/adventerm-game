use std::mem;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use adventerm_lib::{save, Direction, GameState, Save, SaveSlot};
use crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::config::{
    self, BoundAction, ColorScheme, Config, Key, MenuPalette, SchemeRegistry,
};
use crate::input::{Action, TextInputAction};
use crate::menu::{MainMenuOption, OptionsRow, PauseMenuOption};
use crate::ui::accel;

pub enum Screen {
    MainMenu,
    LoadGame,
    Playing(GameState),
    Paused(GameState),
    SaveSlotPicker(GameState),
    NameEntry(GameState),
    Options,
    RebindCapture(BoundAction),
    Quit,
}

pub struct App {
    screen: Screen,
    main_menu_cursor: usize,
    pause_menu_cursor: usize,
    save_dir: PathBuf,
    saves: Vec<SaveSlot>,
    save_list_cursor: usize,
    pending_delete: Option<usize>,
    name_buffer: String,
    status: Option<String>,
    config: Config,
    config_path: PathBuf,
    scheme_registry: SchemeRegistry,
    options_cursor: usize,
}

impl App {
    pub fn new() -> Self {
        let save_dir = default_save_dir();
        let saves = save::list_saves(&save_dir).unwrap_or_default();
        let config_path = config::config_path_for(&save_dir);
        let config = Config::load(&config_path);
        let scheme_registry = SchemeRegistry::load(&save_dir);
        Self {
            screen: Screen::MainMenu,
            main_menu_cursor: 0,
            pause_menu_cursor: 0,
            save_dir,
            saves,
            save_list_cursor: 0,
            pending_delete: None,
            name_buffer: String::new(),
            status: None,
            config,
            config_path,
            scheme_registry,
            options_cursor: 0,
        }
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn main_menu_cursor(&self) -> usize {
        self.main_menu_cursor
    }

    pub fn pause_menu_cursor(&self) -> usize {
        self.pause_menu_cursor
    }

    pub fn saves(&self) -> &[SaveSlot] {
        &self.saves
    }

    pub fn save_list_cursor(&self) -> usize {
        self.save_list_cursor
    }

    pub fn pending_delete(&self) -> Option<usize> {
        self.pending_delete
    }

    pub fn name_buffer(&self) -> &str {
        &self.name_buffer
    }

    pub fn any_saves(&self) -> bool {
        !self.saves.is_empty()
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn should_quit(&self) -> bool {
        matches!(self.screen, Screen::Quit)
    }

    pub fn options_cursor(&self) -> usize {
        self.options_cursor
    }

    pub fn rebind_target(&self) -> Option<BoundAction> {
        match self.screen {
            Screen::RebindCapture(a) => Some(a),
            _ => None,
        }
    }

    pub fn color_scheme(&self) -> &ColorScheme {
        self.scheme_registry.resolve(&self.config.color_scheme)
    }

    pub fn menu_palette(&self) -> &MenuPalette {
        &self.color_scheme().menu
    }

    pub fn handle_event(&mut self, event: &Event) {
        if matches!(self.screen, Screen::NameEntry(_)) {
            let Some(action) = crate::input::translate_text(event) else {
                return;
            };
            self.handle_name_entry(action);
            return;
        }
        if let Screen::RebindCapture(target) = self.screen {
            self.handle_rebind_capture(target, event);
            return;
        }
        let Some(action) = crate::input::translate(event, &self.config.keybinds) else {
            return;
        };
        match &self.screen {
            Screen::MainMenu => self.handle_main_menu(action),
            Screen::LoadGame => self.handle_load_game(action),
            Screen::Playing(_) => self.handle_playing(action),
            Screen::Paused(_) => self.handle_pause_menu(action),
            Screen::SaveSlotPicker(_) => self.handle_slot_picker(action),
            Screen::Options => self.handle_options(action),
            Screen::NameEntry(_) | Screen::RebindCapture(_) | Screen::Quit => {}
        }
    }

    fn refresh_saves(&mut self) {
        self.saves = save::list_saves(&self.save_dir).unwrap_or_default();
        if self.saves.is_empty() {
            self.save_list_cursor = 0;
        } else if self.save_list_cursor >= self.saves.len() {
            self.save_list_cursor = self.saves.len() - 1;
        }
    }

    fn main_menu_options(&self) -> Vec<MainMenuOption> {
        MainMenuOption::available(self.any_saves())
    }

    fn handle_main_menu(&mut self, action: Action) {
        let options = self.main_menu_options();
        if options.is_empty() {
            return;
        }
        match action {
            Action::Up => {
                self.main_menu_cursor = prev_cursor(self.main_menu_cursor, options.len())
            }
            Action::Down => {
                self.main_menu_cursor = next_cursor(self.main_menu_cursor, options.len())
            }
            Action::Confirm => {
                let cursor = self.main_menu_cursor.min(options.len() - 1);
                self.select_main_option(options[cursor]);
            }
            Action::Hotkey(c) => {
                let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
                if let Some(i) = accel::find_by_hotkey(&labels, c) {
                    self.main_menu_cursor = i;
                    self.select_main_option(options[i]);
                }
            }
            _ => {}
        }
    }

    fn select_main_option(&mut self, option: MainMenuOption) {
        match option {
            MainMenuOption::NewGame => {
                self.status = None;
                self.screen = Screen::Playing(GameState::new_seeded(seed_from_clock()));
            }
            MainMenuOption::LoadGame => {
                self.refresh_saves();
                if self.saves.is_empty() {
                    self.status = Some("No saves available".into());
                    return;
                }
                self.save_list_cursor = 0;
                self.pending_delete = None;
                self.status = None;
                self.screen = Screen::LoadGame;
            }
            MainMenuOption::Options => {
                self.options_cursor = 0;
                self.status = None;
                self.screen = Screen::Options;
            }
            MainMenuOption::Quit => self.screen = Screen::Quit,
        }
    }

    fn handle_options(&mut self, action: Action) {
        let rows = OptionsRow::all();
        match action {
            Action::Up => {
                self.options_cursor = prev_cursor(self.options_cursor, rows.len());
            }
            Action::Down => {
                self.options_cursor = next_cursor(self.options_cursor, rows.len());
            }
            Action::Confirm => {
                let cursor = self.options_cursor.min(rows.len() - 1);
                self.activate_options_row(rows[cursor]);
            }
            Action::Escape => self.return_to_main_menu(),
            Action::Hotkey(c) => {
                let labels: Vec<String> =
                    rows.iter().map(|r| options_row_label(self, *r)).collect();
                let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                if let Some(i) = accel::find_by_hotkey(&label_refs, c) {
                    self.options_cursor = i;
                    self.activate_options_row(rows[i]);
                }
            }
            _ => {}
        }
    }

    fn activate_options_row(&mut self, row: OptionsRow) {
        match row {
            OptionsRow::ColorScheme => {
                let next = self.scheme_registry.next_after(&self.config.color_scheme);
                self.config.color_scheme = next;
                self.persist_config();
            }
            OptionsRow::Keybind(action) => {
                self.screen = Screen::RebindCapture(action);
            }
            OptionsRow::ResetDefaults => {
                self.config = Config::default();
                self.persist_config();
                self.status = Some("Defaults restored".into());
            }
            OptionsRow::Back => self.return_to_main_menu(),
        }
    }

    fn handle_rebind_capture(&mut self, target: BoundAction, event: &Event) {
        let Event::Key(key) = event else {
            return;
        };
        if key.kind != KeyEventKind::Press {
            return;
        }
        if matches!(key.code, KeyCode::Esc) {
            self.screen = Screen::Options;
            return;
        }
        if let Some(k) = Key::from_key_code(key.code) {
            self.config.keybinds.set(target, k);
            self.persist_config();
        }
        self.screen = Screen::Options;
    }

    fn persist_config(&mut self) {
        if let Err(e) = self.config.save(&self.config_path) {
            self.status = Some(format!("Failed to save config: {e}"));
        }
    }

    fn handle_load_game(&mut self, action: Action) {
        if self.saves.is_empty() {
            self.return_to_main_menu();
            return;
        }
        if self.pending_delete.is_some() {
            match action {
                Action::Confirm => self.confirm_delete(),
                Action::Escape => self.pending_delete = None,
                _ => {}
            }
            return;
        }
        match action {
            Action::Up => {
                self.save_list_cursor = prev_cursor(self.save_list_cursor, self.saves.len());
            }
            Action::Down => {
                self.save_list_cursor = next_cursor(self.save_list_cursor, self.saves.len());
            }
            Action::Confirm => {
                let cursor = self.save_list_cursor.min(self.saves.len() - 1);
                let path = self.saves[cursor].path.clone();
                match load_save_file(&path) {
                    Ok(state) => {
                        self.status = None;
                        self.screen = Screen::Playing(state);
                    }
                    Err(msg) => self.status = Some(msg),
                }
            }
            Action::Delete | Action::Hotkey('x') => {
                self.pending_delete = Some(self.save_list_cursor);
            }
            Action::Escape => self.return_to_main_menu(),
            _ => {}
        }
    }

    fn confirm_delete(&mut self) {
        let Some(idx) = self.pending_delete.take() else {
            return;
        };
        if let Some(slot) = self.saves.get(idx) {
            let name = slot.name.clone();
            match save::delete_save(&slot.path) {
                Ok(()) => self.status = Some(format!("Deleted '{name}'")),
                Err(e) => self.status = Some(format!("Delete failed: {e}")),
            }
        }
        self.refresh_saves();
        if self.saves.is_empty() {
            self.return_to_main_menu();
        }
    }

    fn return_to_main_menu(&mut self) {
        self.refresh_saves();
        self.main_menu_cursor = 0;
        self.pending_delete = None;
        self.screen = Screen::MainMenu;
    }

    fn handle_playing(&mut self, action: Action) {
        match action {
            Action::Up => self.with_state(|s| {
                s.move_player(Direction::Up);
            }),
            Action::Down => self.with_state(|s| {
                s.move_player(Direction::Down);
            }),
            Action::Left => self.with_state(|s| {
                s.move_player(Direction::Left);
            }),
            Action::Right => self.with_state(|s| {
                s.move_player(Direction::Right);
            }),
            Action::Confirm => self.with_state(|s| {
                s.interact();
            }),
            Action::Escape => {
                self.pause_menu_cursor = 0;
                self.transition_screen(|s| match s {
                    Screen::Playing(g) => Screen::Paused(g),
                    other => other,
                });
            }
            _ => {}
        }
    }

    fn handle_pause_menu(&mut self, action: Action) {
        let options = &PauseMenuOption::ALL;
        match action {
            Action::Up => {
                self.pause_menu_cursor = prev_cursor(self.pause_menu_cursor, options.len())
            }
            Action::Down => {
                self.pause_menu_cursor = next_cursor(self.pause_menu_cursor, options.len())
            }
            Action::Confirm => self.select_pause_option(options[self.pause_menu_cursor]),
            Action::Escape => self.select_pause_option(PauseMenuOption::Resume),
            Action::Hotkey(c) => {
                let labels: Vec<&str> = options.iter().map(|o| o.label()).collect();
                if let Some(i) = accel::find_by_hotkey(&labels, c) {
                    self.pause_menu_cursor = i;
                    self.select_pause_option(options[i]);
                }
            }
            _ => {}
        }
    }

    fn select_pause_option(&mut self, option: PauseMenuOption) {
        match option {
            PauseMenuOption::Resume => self.transition_screen(|s| match s {
                Screen::Paused(g) => Screen::Playing(g),
                other => other,
            }),
            PauseMenuOption::Save => {
                self.refresh_saves();
                self.save_list_cursor = 0;
                self.status = None;
                self.transition_screen(|s| match s {
                    Screen::Paused(g) => Screen::SaveSlotPicker(g),
                    other => other,
                });
            }
            PauseMenuOption::Quit => {
                self.refresh_saves();
                self.main_menu_cursor = 0;
                self.screen = Screen::MainMenu;
            }
        }
    }

    fn handle_slot_picker(&mut self, action: Action) {
        let total = self.saves.len() + 1;
        match action {
            Action::Up => self.save_list_cursor = prev_cursor(self.save_list_cursor, total),
            Action::Down => self.save_list_cursor = next_cursor(self.save_list_cursor, total),
            Action::Confirm => {
                let cursor = self.save_list_cursor.min(total - 1);
                if cursor == 0 {
                    self.name_buffer.clear();
                    self.transition_screen(|s| match s {
                        Screen::SaveSlotPicker(g) => Screen::NameEntry(g),
                        other => other,
                    });
                } else {
                    let slot = self.saves[cursor - 1].clone();
                    self.write_save(&slot.name, &slot.path);
                }
            }
            Action::Escape => {
                self.transition_screen(|s| match s {
                    Screen::SaveSlotPicker(g) => Screen::Paused(g),
                    other => other,
                });
            }
            _ => {}
        }
    }

    fn handle_name_entry(&mut self, action: TextInputAction) {
        match action {
            TextInputAction::Char(c) => {
                if !c.is_control() && self.name_buffer.chars().count() < 32 {
                    self.name_buffer.push(c);
                }
            }
            TextInputAction::Backspace => {
                self.name_buffer.pop();
            }
            TextInputAction::Confirm => {
                let name = self.name_buffer.trim().to_string();
                if name.is_empty() {
                    self.status = Some("Save name cannot be empty".into());
                    return;
                }
                let path = save::slot_path(&self.save_dir, &name);
                self.write_save(&name, &path);
            }
            TextInputAction::Escape => {
                self.transition_screen(|s| match s {
                    Screen::NameEntry(g) => Screen::SaveSlotPicker(g),
                    other => other,
                });
            }
        }
    }

    fn write_save(&mut self, name: &str, path: &Path) {
        let state = match &self.screen {
            Screen::SaveSlotPicker(g) | Screen::NameEntry(g) => g.clone(),
            _ => return,
        };
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            self.status = Some(format!("Failed to create save dir: {e}"));
            return;
        }
        let bytes = Save::new(name.to_string(), state).to_bytes();
        match std::fs::write(path, &bytes) {
            Ok(()) => {
                self.status = Some(format!("Saved as '{name}'"));
                self.transition_screen(|s| match s {
                    Screen::SaveSlotPicker(g) | Screen::NameEntry(g) => Screen::Paused(g),
                    other => other,
                });
                self.refresh_saves();
            }
            Err(e) => self.status = Some(format!("Save failed: {e}")),
        }
    }

    fn with_state(&mut self, f: impl FnOnce(&mut GameState)) {
        if let Screen::Playing(state) = &mut self.screen {
            f(state);
        }
    }

    fn transition_screen(&mut self, f: impl FnOnce(Screen) -> Screen) {
        let prev = mem::replace(&mut self.screen, Screen::Quit);
        self.screen = f(prev);
    }
}

fn load_save_file(path: &Path) -> Result<GameState, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read save: {e}"))?;
    let save = Save::from_bytes(&bytes).map_err(|e| format!("{e}"))?;
    Ok(save.state)
}

fn next_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (cursor + 1) % len }
}

fn prev_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (cursor + len - 1) % len }
}

fn seed_from_clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xDEADBEEF)
}

fn default_save_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("com", "adventerm", "adventerm") {
        return dirs.data_dir().to_path_buf();
    }
    PathBuf::from("saves")
}

pub fn options_row_label(app: &App, row: OptionsRow) -> String {
    match row {
        OptionsRow::ColorScheme => format!("Color Scheme: {}", app.config.color_scheme),
        OptionsRow::Keybind(action) => {
            let keys = app.config.keybinds.keys_for(action);
            let joined = if keys.is_empty() {
                "<unbound>".to_string()
            } else {
                keys.iter()
                    .map(|k| k.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!("{}: {}", action.label(), joined)
        }
        OptionsRow::ResetDefaults => "Reset Defaults".to_string(),
        OptionsRow::Back => "Back".to_string(),
    }
}
