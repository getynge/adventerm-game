use crate::console::command::{CompletionCtx, DevCommand, DevCtx};
use crate::ItemKind;

/// `give <item-name>` — push an `ItemKind` into the player's inventory by
/// the lowercase form of its display name. Multi-word names must be quoted
/// at the call site (the tokenizer already strips the quotes).
pub struct GiveCommand;

impl DevCommand for GiveCommand {
    fn name(&self) -> &'static str {
        "give"
    }

    fn help(&self) -> &'static str {
        "give <name> — add an item to inventory by its display name"
    }

    fn arg_completions(
        &self,
        arg_index: usize,
        _prior_args: &[&str],
        partial: &str,
        _ctx: &CompletionCtx<'_>,
    ) -> Vec<String> {
        if arg_index != 0 {
            return Vec::new();
        }
        item_completions(partial)
    }

    fn execute(&self, args: &[String], ctx: &mut DevCtx<'_>) -> Result<String, String> {
        let [name] = args else {
            return Err("usage: give <item-name>".to_string());
        };
        let kind = ItemKind::from_display_name(name)
            .ok_or_else(|| format!("unknown item {name:?}"))?;
        let game = ctx
            .game
            .as_deref_mut()
            .ok_or_else(|| "give requires an active game".to_string())?;
        game.player.inventory_push(kind);
        Ok(format!("gave {}", kind.name()))
    }
}

/// All item display names (lowercased) that begin with `partial`.
/// Re-used by the spawn command's `item` subcommand.
pub(crate) fn item_completions(partial: &str) -> Vec<String> {
    let needle = partial.to_lowercase();
    ItemKind::ALL
        .iter()
        .copied()
        .map(|k| k.name().to_lowercase())
        .filter(|n| n.starts_with(&needle))
        .collect()
}
