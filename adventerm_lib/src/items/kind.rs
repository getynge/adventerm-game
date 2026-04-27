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

    /// Every concrete kind. Used by reverse lookups and the dev console.
    pub const ALL: &'static [ItemKind] = &[
        ItemKind::Torch,
        ItemKind::Flare,
        ItemKind::Goggles,
        ItemKind::Shirt,
        ItemKind::Gauntlets,
        ItemKind::Trousers,
        ItemKind::Boots,
        ItemKind::ScrollOfFire,
    ];

    /// Reverse of `name()`: case-insensitive lookup against display names.
    pub fn from_display_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|k| k.name().eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_round_trip() {
        for kind in ItemKind::ALL.iter().copied() {
            assert_eq!(ItemKind::from_display_name(kind.name()), Some(kind));
            assert_eq!(
                ItemKind::from_display_name(&kind.name().to_lowercase()),
                Some(kind)
            );
            assert_eq!(
                ItemKind::from_display_name(&kind.name().to_uppercase()),
                Some(kind)
            );
        }
        assert_eq!(ItemKind::from_display_name("not a real item"), None);
    }
}
