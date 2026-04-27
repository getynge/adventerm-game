use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use adventerm_lib::{
    battle, category_of, consume_intent_of, dispatch, save, Battle, BattleResult, BattleTurn,
    ConsumeIntent, ConsumeItemAction, ConsumeTarget, DefeatEnemyAction, Direction, EquipItemAction,
    EquipSlot, GameState, InteractAction, ItemCategory, MoveAction, MoveOutcome, PickUpAction,
    PlaceItemAction, QuickMoveAction, Save, SaveSlot, UnequipItemAction,
};
use crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::config::{self, BoundAction, ColorScheme, Config, Key, SchemeRegistry};
use crate::console::ConsoleState;
use crate::input::{Action, ConsoleInputAction, TextInputAction};
use crate::menu::{
    InventoryTab, ItemsFocus, MainMenuOption, MenuState, OptionsAdvancedRow, OptionsRow,
    PauseMenuOption, PendingConsume, PendingIntent, SaveBrowser, Status,
};

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
    Battle {
        game: GameState,
        battle: Battle,
        cursor: usize,
        status: Status,
    },
    Inventory {
        game: GameState,
        tab: InventoryTab,
        items_focus: ItemsFocus,
        item_cursor: usize,
        equipment_cursor: usize,
        ability_cursor: usize,
        pending_consume: Option<PendingConsume>,
        status: Status,
    },
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
    OptionsAdvanced {
        menu: MenuState<OptionsAdvancedRow>,
        status: Status,
    },
    RebindCapture {
        menu: MenuState<OptionsRow>,
        status: Status,
        target: BoundAction,
    },
    /// Developer-console overlay. Wraps whichever screen was active when
    /// the user pressed backtick; closing the console restores the
    /// underlying screen exactly as it was.
    DeveloperConsole {
        underlying: Box<Screen>,
        console: ConsoleState,
    },
    Quit,
}

impl Screen {
    /// Borrow the active `GameState` if the screen carries one. The
    /// developer console uses this to route commands like `give` and
    /// `spawn` to whatever gameplay screen is underneath the overlay.
    pub fn game_mut(&mut self) -> Option<&mut GameState> {
        match self {
            Screen::Playing(game)
            | Screen::Battle { game, .. }
            | Screen::Inventory { game, .. }
            | Screen::Paused { game, .. }
            | Screen::SaveSlotPicker { game, .. }
            | Screen::NameEntry { game, .. } => Some(game),
            Screen::DeveloperConsole { underlying, .. } => underlying.game_mut(),
            _ => None,
        }
    }

    pub fn game(&self) -> Option<&GameState> {
        match self {
            Screen::Playing(game)
            | Screen::Battle { game, .. }
            | Screen::Inventory { game, .. }
            | Screen::Paused { game, .. }
            | Screen::SaveSlotPicker { game, .. }
            | Screen::NameEntry { game, .. } => Some(game),
            Screen::DeveloperConsole { underlying, .. } => underlying.game(),
            _ => None,
        }
    }
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
        // The developer console is its own input mode: it consumes every
        // keystroke (including a second backtick to close) and never falls
        // through to the underlying screen.
        if let Screen::DeveloperConsole { .. } = self.screen {
            if let Some(action) = crate::input::translate_console(event) {
                self.handle_developer_console(action);
            }
            return;
        }
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
        let console_enabled = self.config.dev_console_enabled();
        let Some(action) =
            crate::input::translate(event, &self.config.keybinds, console_enabled)
        else {
            return;
        };
        if matches!(action, Action::ToggleConsole) {
            self.open_developer_console();
            return;
        }
        match &self.screen {
            Screen::MainMenu { .. } => self.handle_main_menu(action),
            Screen::LoadGame { .. } => self.handle_load_game(action),
            Screen::Playing(_) => self.handle_playing(action),
            Screen::Battle { .. } => self.handle_battle(action),
            Screen::Inventory { .. } => self.handle_inventory(action),
            Screen::Paused { .. } => self.handle_pause_menu(action),
            Screen::SaveSlotPicker { .. } => self.handle_slot_picker(action),
            Screen::Options { .. } => self.handle_options(action),
            Screen::OptionsAdvanced { .. } => self.handle_options_advanced(action),
            Screen::NameEntry { .. }
            | Screen::SeedEntry { .. }
            | Screen::RebindCapture { .. }
            | Screen::DeveloperConsole { .. }
            | Screen::Quit => {}
        }
    }

    fn open_developer_console(&mut self) {
        let prev = std::mem::replace(&mut self.screen, Screen::Quit);
        let mut console = ConsoleState::new();
        console.refresh_completion(prev.game());
        self.screen = Screen::DeveloperConsole {
            underlying: Box::new(prev),
            console,
        };
    }

    fn close_developer_console(&mut self) {
        let prev = std::mem::replace(&mut self.screen, Screen::Quit);
        let Screen::DeveloperConsole { underlying, .. } = prev else {
            // Should be unreachable — caller checks the variant first.
            self.screen = prev;
            return;
        };
        self.screen = *underlying;
        // A fullbright toggle while the console was open changes what the
        // renderer shows; force a refresh so the next frame reflects it.
        if let Some(game) = self.screen.game_mut() {
            game.refresh_visibility();
        }
    }

    fn handle_developer_console(&mut self, action: ConsoleInputAction) {
        match action {
            ConsoleInputAction::Toggle | ConsoleInputAction::Cancel => {
                self.close_developer_console();
                return;
            }
            _ => {}
        }
        let Screen::DeveloperConsole { underlying, console } = &mut self.screen else {
            return;
        };
        match action {
            ConsoleInputAction::Char(c) => console.insert_char(c, underlying.game()),
            ConsoleInputAction::Backspace => console.backspace(underlying.game()),
            ConsoleInputAction::Tab => console.tab(underlying.game()),
            ConsoleInputAction::HistoryUp => console.history_up(),
            ConsoleInputAction::HistoryDown => console.history_down(),
            ConsoleInputAction::Submit => console.submit(underlying.game_mut()),
            ConsoleInputAction::Toggle | ConsoleInputAction::Cancel => unreachable!(),
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
            OptionsRow::Advanced => {
                self.screen = Screen::OptionsAdvanced {
                    menu: MenuState::new(OptionsAdvancedRow::ALL.to_vec()),
                    status: Status::None,
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

    fn handle_options_advanced(&mut self, action: Action) {
        let chosen: Option<OptionsAdvancedRow> = {
            let Screen::OptionsAdvanced { menu, .. } = &mut self.screen else {
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
                Action::Escape => Some(OptionsAdvancedRow::Back),
                Action::Hotkey(c) => {
                    let labels: Vec<String> = menu
                        .options()
                        .iter()
                        .map(|r| options_advanced_row_label(&self.config, *r))
                        .collect();
                    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    menu.select_hotkey(c, &label_refs)
                }
                _ => None,
            }
        };
        if let Some(row) = chosen {
            self.activate_options_advanced_row(row);
        }
    }

    fn activate_options_advanced_row(&mut self, row: OptionsAdvancedRow) {
        match row {
            OptionsAdvancedRow::DevConsole => {
                let new_state = !self.config.dev_console_enabled();
                self.config.set_dev_console_enabled(new_state);
                self.persist_config();
            }
            OptionsAdvancedRow::Back => {
                self.screen = Screen::Options {
                    menu: MenuState::new(OptionsRow::all()),
                    status: Status::None,
                };
            }
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
            | Screen::Inventory { status, .. }
            | Screen::Paused { status, .. }
            | Screen::Battle { status, .. }
            | Screen::SaveSlotPicker { status, .. }
            | Screen::NameEntry { status, .. }
            | Screen::SeedEntry { status, .. }
            | Screen::Options { status, .. }
            | Screen::OptionsAdvanced { status, .. }
            | Screen::RebindCapture { status, .. } => *status = new_status,
            _ => {}
        }
    }

    fn handle_playing(&mut self, action: Action) {
        let Screen::Playing(state) = &mut self.screen else {
            return;
        };
        let player = state.player.entity();
        let outcome: Option<MoveOutcome> = match action {
            Action::Up => Some(dispatch(
                state,
                player,
                MoveAction {
                    direction: Direction::Up,
                },
            )),
            Action::Down => Some(dispatch(
                state,
                player,
                MoveAction {
                    direction: Direction::Down,
                },
            )),
            Action::Left => Some(dispatch(
                state,
                player,
                MoveAction {
                    direction: Direction::Left,
                },
            )),
            Action::Right => Some(dispatch(
                state,
                player,
                MoveAction {
                    direction: Direction::Right,
                },
            )),
            Action::QuickUp => Some(dispatch(
                state,
                player,
                QuickMoveAction {
                    direction: Direction::Up,
                },
            )),
            Action::QuickDown => Some(dispatch(
                state,
                player,
                QuickMoveAction {
                    direction: Direction::Down,
                },
            )),
            Action::QuickLeft => Some(dispatch(
                state,
                player,
                QuickMoveAction {
                    direction: Direction::Left,
                },
            )),
            Action::QuickRight => Some(dispatch(
                state,
                player,
                QuickMoveAction {
                    direction: Direction::Right,
                },
            )),
            _ => None,
        };
        // The action layer surfaces an encounter either as a direct
        // `MoveOutcome::Encounter` (walking into an enemy tile) or via
        // the pending-encounter slot (the enemy-tick handler stepped one
        // adjacent during the bus drain).
        let encounter = match outcome {
            Some(MoveOutcome::Encounter(e)) => Some(e),
            _ => state.take_pending_encounter(),
        };
        if let Some(entity) = encounter {
            self.start_battle(entity);
            return;
        }
        match action {
            Action::Confirm => {
                if state.player_on_door().is_some() {
                    let _ = dispatch(state, player, InteractAction);
                } else if state.items_here() {
                    let _ = dispatch(state, player, PickUpAction);
                }
            }
            Action::Inventory => {
                let Screen::Playing(game) = std::mem::replace(&mut self.screen, Screen::Quit)
                else {
                    return;
                };
                self.screen = Screen::Inventory {
                    game,
                    tab: InventoryTab::Items,
                    items_focus: ItemsFocus::List,
                    item_cursor: 0,
                    equipment_cursor: 0,
                    ability_cursor: 0,
                    pending_consume: None,
                    status: Status::None,
                };
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

    fn start_battle(&mut self, entity: adventerm_lib::EntityId) {
        let Screen::Playing(_) = &self.screen else {
            return;
        };
        let Screen::Playing(game) = std::mem::replace(&mut self.screen, Screen::Quit) else {
            return;
        };
        let battle = match battle::start_battle(&game, entity) {
            Some(b) => b,
            None => {
                // Enemy went away between move and start (shouldn't happen);
                // bail back to play.
                self.screen = Screen::Playing(game);
                return;
            }
        };
        self.screen = Screen::Battle {
            game,
            battle,
            cursor: 0,
            status: Status::None,
        };
    }

    fn handle_battle(&mut self, action: Action) {
        let Screen::Battle {
            game,
            battle,
            cursor,
            status,
        } = &mut self.screen
        else {
            return;
        };

        // Resolved → any keypress returns to gameplay (or main menu on defeat).
        if let Some(result) = battle.result() {
            if matches!(action, Action::Confirm | Action::Escape) {
                self.finish_battle(result);
            }
            return;
        }

        match battle.turn() {
            BattleTurn::Player => match action {
                Action::Up => {
                    if *cursor > 0 {
                        *cursor -= 1;
                    }
                }
                Action::Down => {
                    let slots = game.abilities().active_slots.len();
                    if *cursor + 1 < slots {
                        *cursor += 1;
                    }
                }
                Action::Confirm => {
                    let slot = *cursor;
                    match battle::apply_player_ability(game, battle, slot) {
                        Ok(()) => *status = Status::None,
                        Err(_) => {
                            *status = Status::Error("That slot is empty.".into());
                        }
                    }
                }
                Action::Escape => {
                    battle.set_turn(BattleTurn::Resolved(BattleResult::Fled));
                }
                _ => {}
            },
            BattleTurn::Enemy => {
                if matches!(action, Action::Confirm | Action::Up | Action::Down | Action::Escape) {
                    battle::apply_enemy_turn(game, battle);
                }
            }
            BattleTurn::Resolved(_) => {}
        }
    }

    fn finish_battle(&mut self, result: BattleResult) {
        let Screen::Battle { .. } = &self.screen else {
            return;
        };
        let prev = std::mem::replace(&mut self.screen, Screen::Quit);
        let Screen::Battle {
            mut game, battle, ..
        } = prev
        else {
            return;
        };
        match result {
            BattleResult::Victory => {
                game.set_cur_health(battle.player_cur_hp());
                let player = game.player.entity();
                dispatch(
                    &mut game,
                    player,
                    DefeatEnemyAction {
                        room: battle.enemy_room(),
                        entity: battle.enemy_id(),
                    },
                );
                self.screen = Screen::Playing(game);
            }
            BattleResult::Fled => {
                game.set_cur_health(battle.player_cur_hp());
                self.screen = Screen::Playing(game);
            }
            BattleResult::Defeat => {
                let any_saves = !self.list_saves().is_empty();
                self.screen = main_menu_screen(any_saves, Status::Info("You have fallen.".into()));
            }
        }
    }

    fn handle_inventory(&mut self, action: Action) {
        // Pending-consume substate hijacks navigation: the user is locked
        // into the abilities tab picking a slot until they confirm or
        // cancel. Resolves before the regular tab/cursor routing.
        if matches!(action, Action::Confirm | Action::Escape | Action::Up | Action::Down)
            && self.has_pending_consume()
        {
            self.handle_inventory_pending_consume(action);
            return;
        }

        let Screen::Inventory {
            game,
            tab,
            items_focus,
            item_cursor,
            equipment_cursor,
            ability_cursor,
            ..
        } = &mut self.screen
        else {
            return;
        };
        match (action, *tab) {
            (Action::Inventory, InventoryTab::Items) if *items_focus == ItemsFocus::List => {
                *items_focus = ItemsFocus::Sidebar;
            }
            (Action::Inventory, InventoryTab::Items) => {
                // Sidebar focus → advance to the next tab; reset focus
                // when we return to Items so List is the default.
                *items_focus = ItemsFocus::List;
                *tab = tab.next();
            }
            (Action::Inventory, _) => {
                *tab = tab.next();
                if *tab == InventoryTab::Items {
                    *items_focus = ItemsFocus::List;
                }
            }
            (Action::Escape, _) => {
                let Screen::Inventory { game, .. } =
                    std::mem::replace(&mut self.screen, Screen::Quit)
                else {
                    return;
                };
                self.screen = Screen::Playing(game);
            }
            (Action::Up, InventoryTab::Items) => match *items_focus {
                ItemsFocus::List => {
                    if *item_cursor > 0 {
                        *item_cursor -= 1;
                    }
                }
                ItemsFocus::Sidebar => {
                    if *equipment_cursor > 0 {
                        *equipment_cursor -= 1;
                    }
                }
            },
            (Action::Down, InventoryTab::Items) => match *items_focus {
                ItemsFocus::List => {
                    let len = game.inventory().len();
                    if len > 0 && *item_cursor + 1 < len {
                        *item_cursor += 1;
                    }
                }
                ItemsFocus::Sidebar => {
                    if *equipment_cursor + 1 < EquipSlot::ALL.len() {
                        *equipment_cursor += 1;
                    }
                }
            },
            (Action::Up, InventoryTab::Abilities) => {
                if *ability_cursor > 0 {
                    *ability_cursor -= 1;
                }
            }
            (Action::Down, InventoryTab::Abilities) => {
                let slots = game.abilities().active_slots.len();
                if *ability_cursor + 1 < slots {
                    *ability_cursor += 1;
                }
            }
            (Action::Confirm, InventoryTab::Items) => {
                self.confirm_inventory_items_tab();
            }
            // Confirm in Abilities/Stats is a no-op for now — abilities have
            // no in-menu interaction yet (they're used during battle), and the
            // Stats tab is read-only per the approved scope.
            _ => {}
        }
    }

    fn has_pending_consume(&self) -> bool {
        matches!(
            &self.screen,
            Screen::Inventory {
                pending_consume: Some(_),
                ..
            }
        )
    }

    /// Confirm-on-Items dispatches by category: Placeable → place + close,
    /// Equipment → equip in place, Consumable → either fire immediately
    /// (Immediate intent) or enter the pending-pick substate.
    fn confirm_inventory_items_tab(&mut self) {
        let Screen::Inventory {
            game,
            items_focus,
            item_cursor,
            equipment_cursor,
            tab,
            pending_consume,
            status,
            ..
        } = &mut self.screen
        else {
            return;
        };
        match *items_focus {
            ItemsFocus::List => {
                let len = game.inventory().len();
                if len == 0 {
                    return;
                }
                let slot = (*item_cursor).min(len - 1);
                let kind = game.inventory()[slot];
                let category = category_of(kind);
                match category {
                    ItemCategory::Placeable => {
                        let player = game.player.entity();
                        let _ = dispatch(game, player, PlaceItemAction { slot });
                        let Screen::Inventory { game, .. } =
                            std::mem::replace(&mut self.screen, Screen::Quit)
                        else {
                            return;
                        };
                        self.screen = Screen::Playing(game);
                    }
                    ItemCategory::Equipment(_) => {
                        let player = game.player.entity();
                        let _ = dispatch(
                            game,
                            player,
                            EquipItemAction {
                                inventory_slot: slot,
                            },
                        );
                        // Cursor may now point past the new (shorter) list.
                        let new_len = game.inventory().len();
                        if new_len == 0 {
                            *item_cursor = 0;
                        } else if *item_cursor >= new_len {
                            *item_cursor = new_len - 1;
                        }
                        *status = Status::Info(format!("Equipped {}.", kind.name()));
                    }
                    ItemCategory::Consumable => {
                        let intent = consume_intent_of(kind);
                        match intent {
                            ConsumeIntent::Immediate => {
                                let player = game.player.entity();
                                let _ = dispatch(
                                    game,
                                    player,
                                    ConsumeItemAction {
                                        inventory_slot: slot,
                                        target: ConsumeTarget::None,
                                    },
                                );
                                *status = Status::Info(format!("Consumed {}.", kind.name()));
                            }
                            ConsumeIntent::PickAbilitySlot => {
                                *pending_consume = Some(PendingConsume {
                                    inventory_slot: slot,
                                    kind,
                                    intent: PendingIntent::AbilitySlot,
                                });
                                *tab = InventoryTab::Abilities;
                            }
                            // ConsumeIntent is `non_exhaustive`; future
                            // intents must add a UI branch — short-circuit
                            // with an error until they do.
                            _ => {
                                *status = Status::Error(format!(
                                    "Cannot use {} from inventory yet.",
                                    kind.name()
                                ));
                            }
                        }
                    }
                }
            }
            ItemsFocus::Sidebar => {
                let slot = EquipSlot::ALL[(*equipment_cursor).min(EquipSlot::ALL.len() - 1)];
                if let Some(kind) = game.equipment().slot(slot) {
                    let player = game.player.entity();
                    let _ = dispatch(game, player, UnequipItemAction { slot });
                    *status = Status::Info(format!("Unequipped {}.", kind.name()));
                }
            }
        }
    }

    fn handle_inventory_pending_consume(&mut self, action: Action) {
        let Screen::Inventory {
            game,
            ability_cursor,
            pending_consume,
            status,
            tab,
            ..
        } = &mut self.screen
        else {
            return;
        };
        let Some(pending) = *pending_consume else {
            return;
        };
        match action {
            Action::Up => {
                if *ability_cursor > 0 {
                    *ability_cursor -= 1;
                }
            }
            Action::Down => {
                let slots = game.abilities().active_slots.len();
                if *ability_cursor + 1 < slots {
                    *ability_cursor += 1;
                }
            }
            Action::Confirm => match pending.intent {
                PendingIntent::AbilitySlot => {
                    let player = game.player.entity();
                    let chosen = *ability_cursor;
                    let kind = pending.kind;
                    let outcome = dispatch(
                        game,
                        player,
                        ConsumeItemAction {
                            inventory_slot: pending.inventory_slot,
                            target: ConsumeTarget::AbilitySlot(chosen),
                        },
                    );
                    *pending_consume = None;
                    *tab = InventoryTab::Abilities;
                    *status = match outcome {
                        Some(_) => Status::Info(format!(
                            "{} learned into slot {}.",
                            kind.name(),
                            chosen + 1
                        )),
                        None => Status::Error("Could not learn that ability.".into()),
                    };
                }
            },
            Action::Escape => {
                *pending_consume = None;
                *tab = InventoryTab::Items;
                *status = Status::None;
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
        OptionsRow::Advanced => "Advanced...".to_string(),
        OptionsRow::ResetDefaults => "Reset Defaults".to_string(),
        OptionsRow::Back => "Back".to_string(),
    }
}

pub fn options_advanced_row_label(config: &Config, row: OptionsAdvancedRow) -> String {
    match row {
        OptionsAdvancedRow::DevConsole => format!(
            "Developer Console: {}",
            if config.dev_console_enabled() { "On" } else { "Off" }
        ),
        OptionsAdvancedRow::Back => "Back".to_string(),
    }
}
