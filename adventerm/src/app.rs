use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use adventerm_lib::{save, Direction, GameState, Save, SaveSlot};
use crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::config::{self, BoundAction, ColorScheme, Config, Key, SchemeRegistry};
use crate::input::{Action, TextInputAction};
use crate::menu::{MainMenuOption, MenuState, OptionsRow, PauseMenuOption, SaveBrowser, Status};

const MAX_SAVE_NAME_CHARS: usize = 32;
const MAX_SEED_CHARS: usize = 32;

pub enum Screen {
    MainMenu {
        menu: MenuState<MainMenuOption>,
        status: Status,
    },
    LoadGame {
        browser: SaveBrowser,
        status: Status,
    },
    Playing(GameState),
    Paused {
        game: GameState,
        menu: MenuState<PauseMenuOption>,
        status: Status,
    },
    SaveSlotPicker {
        game: GameState,
        browser: SaveBrowser,
        status: Status,
    },
    NameEntry {
        game: GameState,
        buffer: String,
        status: Status,
    },
    SeedEntry {
        buffer: String,
        status: Status,
    },
    Options {
        menu: MenuState<OptionsRow>,
        status: Status,
    },
    RebindCapture {
        menu: MenuState<OptionsRow>,
        status: Status,
        target: BoundAction,
    },
    Quit,
}

pub struct App {
    screen: Screen,
    save_dir: PathBuf,
    config: Config,
    config_path: PathBuf,
    scheme_registry: SchemeRegistry,
    /// Cached count of save files on disk. Used to decide whether "Load Game"
    /// appears in the main menu (including in underlay renders). Invalidated
    /// whenever the save directory is written or deleted.
    any_saves: bool,
}

impl App {
    pub fn new() -> Self {
        let save_dir = default_save_dir();
        let config_path = config::config_path_for(&save_dir);
        let config = Config::load(&config_path);
        let scheme_registry = SchemeRegistry::load(&save_dir);
        let any_saves = !save::list_saves(&save_dir).unwrap_or_default().is_empty();
        Self {
            screen: main_menu_screen(any_saves, Status::None),
            save_dir,
            config,
            config_path,
            scheme_registry,
            any_saves,
        }
    }

    pub fn any_saves(&self) -> bool {
        self.any_saves
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn should_quit(&self) -> bool {
        matches!(self.screen, Screen::Quit)
    }

    pub fn color_scheme(&self) -> &ColorScheme {
        self.scheme_registry.resolve(&self.config.color_scheme)
    }

    pub fn handle_event(&mut self, event: &Event) {
        if let Screen::NameEntry { .. } = self.screen {
            if let Some(action) = crate::input::translate_text(event) {
                self.handle_name_entry(action);
            }
            return;
        }
        if let Screen::SeedEntry { .. } = self.screen {
            if let Some(action) = crate::input::translate_text(event) {
                self.handle_seed_entry(action);
            }
            return;
        }
        if let Screen::RebindCapture { .. } = self.screen {
            self.handle_rebind_capture(event);
            return;
        }
        let Some(action) = crate::input::translate(event, &self.config.keybinds) else {
            return;
        };
        match &self.screen {
            Screen::MainMenu { .. } => self.handle_main_menu(action),
            Screen::LoadGame { .. } => self.handle_load_game(action),
            Screen::Playing(_) => self.handle_playing(action),
            Screen::Paused { .. } => self.handle_pause_menu(action),
            Screen::SaveSlotPicker { .. } => self.handle_slot_picker(action),
            Screen::Options { .. } => self.handle_options(action),
            Screen::NameEntry { .. }
            | Screen::SeedEntry { .. }
            | Screen::RebindCapture { .. }
            | Screen::Quit => {}
        }
    }

    fn handle_main_menu(&mut self, action: Action) {
        let chosen: Option<MainMenuOption> = {
            let Screen::MainMenu { menu, .. } = &mut self.screen else {
                return;
            };
            match action {
                Action::Up => {
                    menu.up();
                    None
                }
                Action::Down => {
                    menu.down();
                    None
                }
                Action::Confirm => menu.current(),
                Action::Hotkey(c) => {
                    let labels: Vec<&str> = menu.options().iter().map(|o| o.label()).collect();
                    menu.select_hotkey(c, &labels)
                }
                _ => None,
            }
        };
        if let Some(option) = chosen {
            self.select_main_option(option);
        }
    }

    fn select_main_option(&mut self, option: MainMenuOption) {
        match option {
            MainMenuOption::NewGame => {
                self.screen = Screen::SeedEntry {
                    buffer: String::new(),
                    status: Status::None,
                };
            }
            MainMenuOption::LoadGame => {
                let saves = self.list_saves();
                if saves.is_empty() {
                    self.set_main_menu_status(Status::Info("No saves available".into()));
                    return;
                }
                self.screen = Screen::LoadGame {
                    browser: SaveBrowser::new(saves),
                    status: Status::None,
                };
            }
            MainMenuOption::Options => {
                self.screen = Screen::Options {
                    menu: MenuState::new(OptionsRow::all()),
                    status: Status::None,
                };
            }
            MainMenuOption::Quit => self.screen = Screen::Quit,
        }
    }

    fn handle_load_game(&mut self, action: Action) {
        let Screen::LoadGame { browser, status } = &mut self.screen else {
            return;
        };
        if browser.is_empty() {
            self.return_to_main_menu(Status::None);
            return;
        }
        if browser.pending_delete.is_some() {
            match action {
                Action::Confirm => self.confirm_delete_in_load(),
                Action::Escape => browser.pending_delete = None,
                _ => {}
            }
            return;
        }
        let total = browser.saves.len();
        match action {
            Action::Up => browser.up(total),
            Action::Down => browser.down(total),
            Action::Confirm => {
                let path = browser.saves[browser.cursor].path.clone();
                match load_save_file(&path) {
                    Ok(state) => self.screen = Screen::Playing(state),
                    Err(msg) => *status = Status::Error(msg),
                }
            }
            Action::Delete | Action::Hotkey('x') => {
                browser.pending_delete = Some(browser.cursor);
            }
            Action::Escape => self.return_to_main_menu(Status::None),
            _ => {}
        }
    }

    fn confirm_delete_in_load(&mut self) {
        let Screen::LoadGame { browser, status } = &mut self.screen else {
            return;
        };
        let Some(idx) = browser.pending_delete.take() else {
            return;
        };
        let Some(slot) = browser.saves.get(idx) else {
            return;
        };
        let name = slot.name.clone();
        let path = slot.path.clone();
        *status = match save::delete_save(&path) {
            Ok(()) => Status::Info(format!("Deleted '{name}'")),
            Err(e) => Status::Error(format!("Delete failed: {e}")),
        };
        let fresh = self.list_saves();
        if fresh.is_empty() {
            let last_status = match &mut self.screen {
                Screen::LoadGame { status, .. } => std::mem::replace(status, Status::None),
                _ => Status::None,
            };
            self.return_to_main_menu(last_status);
            return;
        }
        if let Screen::LoadGame { browser, .. } = &mut self.screen {
            browser.saves = fresh;
            browser.clamp(browser.saves.len());
        }
    }

    fn handle_options(&mut self, action: Action) {
        let chosen: Option<OptionsRow> = {
            let Screen::Options { menu, .. } = &mut self.screen else {
                return;
            };
            match action {
                Action::Up => {
                    menu.up();
                    None
                }
                Action::Down => {
                    menu.down();
                    None
                }
                Action::Confirm => menu.current(),
                Action::Escape => {
                    self.return_to_main_menu(Status::None);
                    return;
                }
                Action::Hotkey(c) => {
                    let labels: Vec<String> = menu
                        .options()
                        .iter()
                        .map(|r| options_row_label(&self.config, *r))
                        .collect();
                    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    menu.select_hotkey(c, &label_refs)
                }
                _ => None,
            }
        };
        if let Some(row) = chosen {
            self.activate_options_row(row);
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
                let Screen::Options { menu, status } = std::mem::replace(
                    &mut self.screen,
                    Screen::Quit,
                ) else {
                    return;
                };
                self.screen = Screen::RebindCapture {
                    menu,
                    status,
                    target: action,
                };
            }
            OptionsRow::ResetDefaults => {
                self.config = Config::default();
                self.persist_config();
                if let Screen::Options { status, .. } = &mut self.screen {
                    *status = Status::Info("Defaults restored".into());
                }
            }
            OptionsRow::Back => self.return_to_main_menu(Status::None),
        }
    }

    fn handle_rebind_capture(&mut self, event: &Event) {
        let Screen::RebindCapture { target, .. } = self.screen else {
            return;
        };
        let Event::Key(key) = event else {
            return;
        };
        if key.kind != KeyEventKind::Press {
            return;
        }
        if !matches!(key.code, KeyCode::Esc)
            && let Some(k) = Key::from_key_code(key.code)
        {
            self.config.keybinds.set(target, k);
            self.persist_config();
        }
        let Screen::RebindCapture { menu, status, .. } =
            std::mem::replace(&mut self.screen, Screen::Quit)
        else {
            return;
        };
        self.screen = Screen::Options { menu, status };
    }

    fn persist_config(&mut self) {
        if let Err(e) = self.config.save(&self.config_path) {
            self.set_current_status(Status::Error(format!("Failed to save config: {e}")));
        }
    }

    fn return_to_main_menu(&mut self, status: Status) {
        let any_saves = !self.list_saves().is_empty();
        self.screen = main_menu_screen(any_saves, status);
    }

    fn set_main_menu_status(&mut self, new_status: Status) {
        if let Screen::MainMenu { status, .. } = &mut self.screen {
            *status = new_status;
        }
    }

    fn set_current_status(&mut self, new_status: Status) {
        match &mut self.screen {
            Screen::MainMenu { status, .. }
            | Screen::LoadGame { status, .. }
            | Screen::Paused { status, .. }
            | Screen::SaveSlotPicker { status, .. }
            | Screen::NameEntry { status, .. }
            | Screen::SeedEntry { status, .. }
            | Screen::Options { status, .. }
            | Screen::RebindCapture { status, .. } => *status = new_status,
            _ => {}
        }
    }

    fn handle_playing(&mut self, action: Action) {
        let Screen::Playing(state) = &mut self.screen else {
            return;
        };
        match action {
            Action::Up => {
                state.move_player(Direction::Up);
            }
            Action::Down => {
                state.move_player(Direction::Down);
            }
            Action::Left => {
                state.move_player(Direction::Left);
            }
            Action::Right => {
                state.move_player(Direction::Right);
            }
            Action::QuickUp => {
                state.quick_move(Direction::Up);
            }
            Action::QuickDown => {
                state.quick_move(Direction::Down);
            }
            Action::QuickLeft => {
                state.quick_move(Direction::Left);
            }
            Action::QuickRight => {
                state.quick_move(Direction::Right);
            }
            Action::Confirm => {
                state.interact();
            }
            Action::Escape => {
                let Screen::Playing(game) = std::mem::replace(&mut self.screen, Screen::Quit)
                else {
                    return;
                };
                self.screen = Screen::Paused {
                    game,
                    menu: MenuState::new(PauseMenuOption::ALL.to_vec()),
                    status: Status::None,
                };
            }
            _ => {}
        }
    }

    fn handle_pause_menu(&mut self, action: Action) {
        let chosen: Option<PauseMenuOption> = {
            let Screen::Paused { menu, .. } = &mut self.screen else {
                return;
            };
            match action {
                Action::Up => {
                    menu.up();
                    None
                }
                Action::Down => {
                    menu.down();
                    None
                }
                Action::Confirm => menu.current(),
                Action::Escape => Some(PauseMenuOption::Resume),
                Action::Hotkey(c) => {
                    let labels: Vec<&str> = menu.options().iter().map(|o| o.label()).collect();
                    menu.select_hotkey(c, &labels)
                }
                _ => None,
            }
        };
        if let Some(option) = chosen {
            self.select_pause_option(option);
        }
    }

    fn select_pause_option(&mut self, option: PauseMenuOption) {
        let Screen::Paused { .. } = &self.screen else {
            return;
        };
        let game = match std::mem::replace(&mut self.screen, Screen::Quit) {
            Screen::Paused { game, .. } => game,
            _ => return,
        };
        match option {
            PauseMenuOption::Resume => self.screen = Screen::Playing(game),
            PauseMenuOption::Save => {
                let saves = self.list_saves();
                self.screen = Screen::SaveSlotPicker {
                    game,
                    browser: SaveBrowser::new(saves),
                    status: Status::None,
                };
            }
            PauseMenuOption::Quit => {
                let any_saves = !self.list_saves().is_empty();
                self.screen = main_menu_screen(any_saves, Status::None);
            }
        }
    }

    fn handle_slot_picker(&mut self, action: Action) {
        let Screen::SaveSlotPicker { browser, .. } = &mut self.screen else {
            return;
        };
        let total = browser.saves.len() + 1;
        match action {
            Action::Up => browser.up(total),
            Action::Down => browser.down(total),
            Action::Confirm => {
                let cursor = browser.cursor.min(total - 1);
                if cursor == 0 {
                    let Screen::SaveSlotPicker { game, .. } =
                        std::mem::replace(&mut self.screen, Screen::Quit)
                    else {
                        return;
                    };
                    self.screen = Screen::NameEntry {
                        game,
                        buffer: String::new(),
                        status: Status::None,
                    };
                } else {
                    let slot = browser.saves[cursor - 1].clone();
                    self.write_save(&slot.name, &slot.path);
                }
            }
            Action::Escape => {
                let Screen::SaveSlotPicker { game, .. } =
                    std::mem::replace(&mut self.screen, Screen::Quit)
                else {
                    return;
                };
                self.screen = Screen::Paused {
                    game,
                    menu: MenuState::new(PauseMenuOption::ALL.to_vec()),
                    status: Status::None,
                };
            }
            _ => {}
        }
    }

    fn handle_name_entry(&mut self, action: TextInputAction) {
        let Screen::NameEntry { buffer, status, .. } = &mut self.screen else {
            return;
        };
        match action {
            TextInputAction::Char(c) => {
                if !c.is_control() && buffer.chars().count() < MAX_SAVE_NAME_CHARS {
                    buffer.push(c);
                }
            }
            TextInputAction::Backspace => {
                buffer.pop();
            }
            TextInputAction::Confirm => {
                let name = buffer.trim().to_string();
                if name.is_empty() {
                    *status = Status::Error("Save name cannot be empty".into());
                    return;
                }
                let path = save::slot_path(&self.save_dir, &name);
                self.write_save(&name, &path);
            }
            TextInputAction::Escape => {
                let Screen::NameEntry { game, .. } =
                    std::mem::replace(&mut self.screen, Screen::Quit)
                else {
                    return;
                };
                let saves = self.list_saves();
                self.screen = Screen::SaveSlotPicker {
                    game,
                    browser: SaveBrowser::new(saves),
                    status: Status::None,
                };
            }
        }
    }

    fn handle_seed_entry(&mut self, action: TextInputAction) {
        let Screen::SeedEntry { buffer, .. } = &mut self.screen else {
            return;
        };
        match action {
            TextInputAction::Char(c) => {
                if !c.is_control() && buffer.chars().count() < MAX_SEED_CHARS {
                    buffer.push(c);
                }
            }
            TextInputAction::Backspace => {
                buffer.pop();
            }
            TextInputAction::Confirm => {
                let text = buffer.trim().to_string();
                let seed = if text.is_empty() {
                    seed_from_clock()
                } else {
                    seed_from_text(&text)
                };
                self.screen = Screen::Playing(GameState::new_seeded(seed));
            }
            TextInputAction::Escape => {
                self.return_to_main_menu(Status::None);
            }
        }
    }

    fn write_save(&mut self, name: &str, path: &Path) {
        let game = match &self.screen {
            Screen::SaveSlotPicker { game, .. } | Screen::NameEntry { game, .. } => game.clone(),
            _ => return,
        };
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            self.set_current_status(Status::Error(format!("Failed to create save dir: {e}")));
            return;
        }
        let bytes = Save::new(name.to_string(), game.clone()).to_bytes();
        match std::fs::write(path, &bytes) {
            Ok(()) => {
                self.screen = Screen::Paused {
                    game,
                    menu: MenuState::new(PauseMenuOption::ALL.to_vec()),
                    status: Status::Info(format!("Saved as '{name}'")),
                };
            }
            Err(e) => {
                self.set_current_status(Status::Error(format!("Save failed: {e}")));
            }
        }
    }

    fn list_saves(&mut self) -> Vec<SaveSlot> {
        let saves = save::list_saves(&self.save_dir).unwrap_or_default();
        self.any_saves = !saves.is_empty();
        saves
    }
}

fn main_menu_screen(any_saves: bool, status: Status) -> Screen {
    Screen::MainMenu {
        menu: MenuState::new(MainMenuOption::available(any_saves)),
        status,
    }
}

fn load_save_file(path: &Path) -> Result<GameState, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read save: {e}"))?;
    let save = Save::from_bytes(&bytes).map_err(|e| format!("{e}"))?;
    Ok(save.state)
}

fn seed_from_clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xDEADBEEF)
}

fn seed_from_text(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn default_save_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("com", "adventerm", "adventerm") {
        return dirs.data_dir().to_path_buf();
    }
    PathBuf::from("saves")
}

pub fn options_row_label(config: &Config, row: OptionsRow) -> String {
    match row {
        OptionsRow::ColorScheme => format!("Color Scheme: {}", config.color_scheme),
        OptionsRow::Keybind(action) => {
            let keys = config.keybinds.keys_for(action);
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
