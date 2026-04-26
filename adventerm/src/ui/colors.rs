use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};

use crate::config::{rgb_to_color, ColorScheme, MenuPalette, WorldPalette};

/// Pre-computed ratatui Colors for the menu palette. Built once per frame so
/// each renderer doesn't repeat rgb_to_color conversions.
pub struct MenuColors {
    pub background: Color,
    pub text: Color,
    pub title: Color,
    pub cursor_fg: Color,
    pub cursor_bg: Color,
    pub accel: Color,
    pub status: Color,
}

impl MenuColors {
    pub fn from_palette(p: &MenuPalette) -> Self {
        Self {
            background: rgb_to_color(p.background),
            text: rgb_to_color(p.text),
            title: rgb_to_color(p.title),
            cursor_fg: rgb_to_color(p.cursor_fg),
            cursor_bg: rgb_to_color(p.cursor_bg),
            accel: rgb_to_color(p.accel),
            status: rgb_to_color(p.status),
        }
    }

    pub fn body_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.background)
    }

    pub fn cursor_style(&self) -> Style {
        Style::default().fg(self.cursor_fg).bg(self.cursor_bg)
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.title)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }
}

/// Brightness multiplier for tiles the player has previously seen but is no
/// longer in line of sight of. Derived from the scheme's existing world colors
/// so each scheme stays a single source of truth.
pub const MEMORY_DIM_FACTOR: f32 = 0.4;

/// Pre-computed ratatui Colors for the world palette.
pub struct WorldColors {
    pub player: Color,
    pub interactive: Color,
    pub floor: Color,
    pub wall: Color,
    pub background: Color,
    pub memory_floor: Color,
    pub memory_wall: Color,
    pub memory_interactive: Color,
    /// Lit decorations (wall lights, placed torches). Warm so they read as
    /// "light source" against the cooler floor/wall palette.
    pub light: Color,
    pub memory_light: Color,
    /// Items resting on the floor.
    pub item: Color,
    pub memory_item: Color,
}

fn dim(rgb: crate::config::Rgb) -> Color {
    let r = (rgb[0] as f32 * MEMORY_DIM_FACTOR) as u8;
    let g = (rgb[1] as f32 * MEMORY_DIM_FACTOR) as u8;
    let b = (rgb[2] as f32 * MEMORY_DIM_FACTOR) as u8;
    rgb_to_color([r, g, b])
}

/// Warm yellow used for light sources. Hardcoded (not pulled from the scheme
/// JSON) so existing color schemes keep working without a schema bump.
const LIGHT_RGB: crate::config::Rgb = [255, 200, 80];
/// Pale cyan for ground items — distinct from doors/lights.
const ITEM_RGB: crate::config::Rgb = [120, 220, 220];

impl WorldColors {
    pub fn from_palette(p: &WorldPalette) -> Self {
        Self {
            player: rgb_to_color(p.player),
            interactive: rgb_to_color(p.interactive),
            floor: rgb_to_color(p.floor),
            wall: rgb_to_color(p.wall),
            background: rgb_to_color(p.background),
            memory_floor: dim(p.floor),
            memory_wall: dim(p.wall),
            memory_interactive: dim(p.interactive),
            light: rgb_to_color(LIGHT_RGB),
            memory_light: dim(LIGHT_RGB),
            item: rgb_to_color(ITEM_RGB),
            memory_item: dim(ITEM_RGB),
        }
    }
}

pub struct SchemeColors {
    pub menu: MenuColors,
    pub world: WorldColors,
}

impl SchemeColors {
    pub fn from_scheme(scheme: &ColorScheme) -> Self {
        Self {
            menu: MenuColors::from_palette(&scheme.menu),
            world: WorldColors::from_palette(&scheme.world),
        }
    }
}

/// A bordered block styled for menu/popup content. Used by every popup
/// renderer to avoid duplicating the title-style + body-style boilerplate.
pub fn menu_block<'a>(title: &'a str, colors: &MenuColors) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(colors.title_style())
        .style(colors.body_style())
}
