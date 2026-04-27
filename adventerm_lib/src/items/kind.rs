use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Torch,
    Flare,
    Goggles,
    Shirt,
    Gauntlets,
    Trousers,
    Boots,
    ScrollOfFire,
}

impl ItemKind {
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Torch => "Torch",
            ItemKind::Flare => "Flare",
            ItemKind::Goggles => "Goggles of Seeing",
            ItemKind::Shirt => "Woven Shirt",
            ItemKind::Gauntlets => "Woven Gauntlets",
            ItemKind::Trousers => "Woven Trousers",
            ItemKind::Boots => "Old Boots",
            ItemKind::ScrollOfFire => "Scroll of Fire",
        }
    }

    pub fn glyph(self) -> char {
        match self {
            ItemKind::Torch => 'i',
            ItemKind::Flare => '!',
            ItemKind::Goggles => 'g',
            ItemKind::Shirt => 's',
            ItemKind::Gauntlets => 'a',
            ItemKind::Trousers => 't',
            ItemKind::Boots => 'b',
            ItemKind::ScrollOfFire => '?',
        }
    }
}
