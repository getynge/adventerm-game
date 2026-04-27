use crate::console::command::{CompletionCtx, DevCommand, DevCtx};
use crate::console::commands::give::item_completions;
use crate::game::ENEMY_RNG_SALT;
use crate::items::random::random_item_kind;
use crate::systems::dev::{spawn_enemy_near_player, spawn_item_at_player};
use crate::{EnemyKind, ItemKind};

const SUBCOMMANDS: &[&str] = &["item", "enemy"];

/// `spawn <item|enemy> [name]` — drop an entity into the world. `item`
/// without a name picks a random kind from the same weighted pool dungeon
/// generation uses; with a name it spawns that exact kind. `enemy` always
/// requires a name.
pub struct SpawnCommand;

impl DevCommand for SpawnCommand {
    fn name(&self) -> &'static str {
        "spawn"
    }

    fn help(&self) -> &'static str {
        "spawn <item|enemy> [name] — spawn an entity at/near the player"
    }

    fn arg_completions(
        &self,
        arg_index: usize,
        prior_args: &[&str],
        partial: &str,
        _ctx: &CompletionCtx<'_>,
    ) -> Vec<String> {
        match arg_index {
            0 => SUBCOMMANDS
                .iter()
                .filter(|s| s.starts_with(partial))
                .map(|s| s.to_string())
                .collect(),
            1 => match prior_args.first().copied() {
                Some("item") => item_completions(partial),
                Some("enemy") => enemy_completions(partial),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    fn execute(&self, args: &[String], ctx: &mut DevCtx<'_>) -> Result<String, String> {
        let (subcommand, name) = match args {
            [sub] => (sub.as_str(), None),
            [sub, name] => (sub.as_str(), Some(name.as_str())),
            _ => return Err("usage: spawn <item|enemy> [name]".to_string()),
        };
        let game = ctx
            .game
            .as_deref_mut()
            .ok_or_else(|| "spawn requires an active game".to_string())?;
        match subcommand {
            "item" => {
                let kind = if let Some(name) = name {
                    ItemKind::from_display_name(name)
                        .ok_or_else(|| format!("unknown item {name:?}"))?
                } else {
                    let rng = game
                        .player
                        .enemy_rng_mut(game.dungeon.seed, ENEMY_RNG_SALT);
                    random_item_kind(rng)
                };
                spawn_item_at_player(game, kind);
                Ok(format!("spawned item {}", kind.name()))
            }
            "enemy" => {
                let name = name.ok_or_else(|| "spawn enemy requires a name".to_string())?;
                let kind = EnemyKind::from_display_name(name)
                    .ok_or_else(|| format!("unknown enemy {name:?}"))?;
                let entity = spawn_enemy_near_player(game, kind)
                    .ok_or_else(|| "no walkable tile adjacent to the player".to_string())?;
                let _ = entity;
                Ok(format!("spawned enemy {}", kind.name()))
            }
            other => Err(format!("unknown spawn target {other:?}")),
        }
    }
}

fn enemy_completions(partial: &str) -> Vec<String> {
    let needle = partial.to_lowercase();
    [EnemyKind::Slime]
        .iter()
        .copied()
        .map(|k| k.name().to_lowercase())
        .filter(|n| n.starts_with(&needle))
        .collect()
}
