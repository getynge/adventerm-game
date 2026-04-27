use crate::console::command::{CompletionCtx, DevCommand, DevCtx};

/// `fullbright` — toggle the world-illumination override on the active
/// `GameState`. No arguments. Refreshes visibility immediately so the
/// effect is visible the moment the console closes.
pub struct FullbrightCommand;

impl DevCommand for FullbrightCommand {
    fn name(&self) -> &'static str {
        "fullbright"
    }

    fn help(&self) -> &'static str {
        "fullbright — toggle full illumination of every tile in every room"
    }

    fn arg_completions(
        &self,
        _arg_index: usize,
        _prior_args: &[&str],
        _partial: &str,
        _ctx: &CompletionCtx<'_>,
    ) -> Vec<String> {
        Vec::new()
    }

    fn execute(&self, args: &[String], ctx: &mut DevCtx<'_>) -> Result<String, String> {
        if !args.is_empty() {
            return Err(format!(
                "fullbright takes no arguments (got {})",
                args.len()
            ));
        }
        let game = ctx
            .game
            .as_deref_mut()
            .ok_or_else(|| "fullbright requires an active game".to_string())?;
        let new_state = !game.fullbright();
        game.set_fullbright(new_state);
        game.refresh_visibility();
        Ok(format!(
            "fullbright {}",
            if new_state { "on" } else { "off" }
        ))
    }
}
