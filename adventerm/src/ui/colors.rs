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

/// Pre-computed ratatui Colors for the world palette.
pub struct WorldColors {
    pub player: Color,
    pub interactive: Color,
    pub floor: Color,
    pub wall: Color,
    pub background: Color,
}

impl WorldColors {
    pub fn from_palette(p: &WorldPalette) -> Self {
        Self {
            player: rgb_to_color(p.player),
            interactive: rgb_to_color(p.interactive),
            floor: rgb_to_color(p.floor),
            wall: rgb_to_color(p.wall),
            background: rgb_to_color(p.background),
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
