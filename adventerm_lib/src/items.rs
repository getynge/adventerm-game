use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Torch,
    Flare,
}

impl ItemKind {
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Torch => "Torch",
            ItemKind::Flare => "Flare",
        }
    }

    pub fn glyph(self) -> char {
        match self {
            ItemKind::Torch => 'i',
            ItemKind::Flare => '!',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
}
