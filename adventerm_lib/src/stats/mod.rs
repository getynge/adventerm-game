use serde::{Deserialize, Serialize};

pub const STAT_MIN: u8 = 0;
pub const STAT_MAX: u8 = 100;

/// Elemental affinity carried by every combatant. Currently display-only —
/// damage formulas do not factor it in. Hooked here so future elemental
/// matchups slot in without reshaping `Stats`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Attribute {
    Fire,
    Water,
    Earth,
    Light,
    Dark,
}

impl Attribute {
    pub fn name(self) -> &'static str {
        match self {
            Attribute::Fire => "Fire",
            Attribute::Water => "Water",
            Attribute::Earth => "Earth",
            Attribute::Light => "Light",
            Attribute::Dark => "Dark",
        }
    }
}

/// Five-axis stat block used by the player and (later) enemies. Numeric stats
/// are clamped to `[STAT_MIN, STAT_MAX]` on construction; mutators clamp on
/// write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub health: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub attribute: Attribute,
}

impl Stats {
    pub fn new(health: u8, attack: u8, defense: u8, speed: u8, attribute: Attribute) -> Self {
        Self {
            health: clamp_stat(health),
            attack: clamp_stat(attack),
            defense: clamp_stat(defense),
            speed: clamp_stat(speed),
            attribute,
        }
    }
}

fn clamp_stat(v: u8) -> u8 {
    v.clamp(STAT_MIN, STAT_MAX)
}

const STARTER_HEALTH: u8 = 25;
const STARTER_ATTACK: u8 = 10;
const STARTER_DEFENSE: u8 = 5;
const STARTER_SPEED: u8 = 8;
const STARTER_ATTRIBUTE: Attribute = Attribute::Fire;

impl Default for Stats {
    fn default() -> Self {
        Stats::new(
            STARTER_HEALTH,
            STARTER_ATTACK,
            STARTER_DEFENSE,
            STARTER_SPEED,
            STARTER_ATTRIBUTE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_above_max() {
        let s = Stats::new(250, 250, 250, 250, Attribute::Water);
        assert_eq!(s.health, STAT_MAX);
        assert_eq!(s.attack, STAT_MAX);
        assert_eq!(s.defense, STAT_MAX);
        assert_eq!(s.speed, STAT_MAX);
    }

    #[test]
    fn default_is_starter_profile() {
        let s = Stats::default();
        assert_eq!(s.health, STARTER_HEALTH);
        assert_eq!(s.attribute, STARTER_ATTRIBUTE);
    }
}
