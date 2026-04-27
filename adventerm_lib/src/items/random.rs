use crate::items::ItemKind;
use crate::rng::Rng;

/// Weighted distribution used when a ground-item draw fires during dungeon
/// generation. Torches stay the most common find; equipment fills the
/// middle band; flares and the Scroll of Fire are intentionally rare.
/// Adding a new variant: append a `(kind, weight)` row.
const ITEM_WEIGHTS: &[(ItemKind, u32)] = &[
    (ItemKind::Torch, 30),
    (ItemKind::Flare, 4),
    (ItemKind::Shirt, 10),
    (ItemKind::Gauntlets, 10),
    (ItemKind::Trousers, 10),
    (ItemKind::Boots, 10),
    (ItemKind::Goggles, 6),
    (ItemKind::ScrollOfFire, 2),
];

/// Pick a random `ItemKind` using the dungeon-generation weights. Same
/// distribution the dungeon uses for ground items, exposed so the dev
/// console (and any future spawner) reproduces the live behavior.
pub fn random_item_kind(rng: &mut Rng) -> ItemKind {
    weighted_pick(rng, ITEM_WEIGHTS)
}

/// Walk a `(value, weight)` table and pick a value with probability
/// proportional to its weight. Panics on an empty / all-zero table.
fn weighted_pick<T: Copy>(rng: &mut Rng, weights: &[(T, u32)]) -> T {
    let total: u32 = weights.iter().map(|(_, w)| *w).sum();
    assert!(total > 0, "weighted_pick called with empty/zero table");
    let mut roll = rng.range(0, total as usize) as u32;
    for (value, w) in weights {
        if roll < *w {
            return *value;
        }
        roll -= *w;
    }
    weights.last().expect("non-empty checked above").0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(random_item_kind(&mut a), random_item_kind(&mut b));
        }
    }

    #[test]
    fn produces_every_weighted_kind_eventually() {
        let mut rng = Rng::new(7);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10_000 {
            seen.insert(random_item_kind(&mut rng));
        }
        for (kind, _) in ITEM_WEIGHTS {
            assert!(seen.contains(kind), "never produced {:?}", kind);
        }
    }
}
