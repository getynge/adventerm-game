use ratatui::layout::{Constraint, Flex, Layout, Rect};

/// Border padding (left + right or top + bottom) added around popup content
/// before clamping. Accounts for the 1-cell border on each side.
pub const POPUP_BORDER_PAD: u16 = 4;

/// Minimum width any popup is allowed to render at, in cells.
pub const POPUP_MIN_WIDTH: u16 = 8;

/// Horizontal breathing room added around panel content (e.g. options panel).
/// Wider than POPUP_BORDER_PAD because panels include footer text.
pub const PANEL_HORIZONTAL_PAD: u16 = 8;

/// Vertical breathing room for panels: top border, bottom border, footer row,
/// and gap row.
pub const PANEL_VERTICAL_PAD: u16 = 4;

/// Minimum width of the options panel, in cells.
pub const PANEL_MIN_WIDTH: u16 = 20;

/// Minimum height of the options panel, in cells.
pub const PANEL_MIN_HEIGHT: u16 = 6;

/// Horizontal breathing room used by the save browser, which renders a wider
/// list (file name + timestamp).
pub const SAVE_BROWSER_HORIZONTAL_PAD: u16 = 6;

/// Fixed dimensions for the pause menu popup.
pub const PAUSE_MENU_WIDTH: u16 = 24;

/// Vertical padding (border top + bottom) added to the pause menu height.
pub const PAUSE_MENU_VERTICAL_PAD: u16 = 2;

/// Fixed dimensions for the save-name entry popup.
pub const NAME_ENTRY_WIDTH: u16 = 44;
pub const NAME_ENTRY_HEIGHT: u16 = 5;

/// Fixed dimensions for the new-game seed entry popup.
pub const SEED_ENTRY_WIDTH: u16 = 44;
pub const SEED_ENTRY_HEIGHT: u16 = 5;

/// Width of the centred main-menu options column.
pub const MAIN_MENU_OPTIONS_WIDTH: u16 = 14;

/// Height used by status/confirm popups (top border + content + bottom border).
pub const STATUS_POPUP_HEIGHT: u16 = 3;

/// Developer-console popup geometry. The console takes most of the frame
/// to give the log pane room; the input row sits at the bottom.
pub const CONSOLE_HORIZONTAL_MARGIN: u16 = 2;
pub const CONSOLE_VERTICAL_MARGIN: u16 = 2;
pub const CONSOLE_MIN_WIDTH: u16 = 40;
pub const CONSOLE_MIN_HEIGHT: u16 = 8;
/// Number of input/footer rows reserved at the bottom of the console.
pub const CONSOLE_INPUT_ROWS: u16 = 2;

pub fn popup_rect(area: Rect, width: u16, height: u16) -> Rect {
    let [_, row, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);

    let [centered] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(row);
    centered
}
