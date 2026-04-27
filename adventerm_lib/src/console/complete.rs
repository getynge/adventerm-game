//! Completion engine: given an input line and an optional view of the
//! game state, yields a list of completion candidates plus a "best"
//! suggestion. The renderer shows the best as ghost text after the input;
//! Tab accepts it (or cycles through `candidates` when the input is
//! ambiguous).

use crate::GameState;

use super::command::{find as find_command, registry, CompletionCtx};
use super::parse::{analyze, quote_if_needed, InputPosition, Token};

/// Result of a completion query.
#[derive(Debug, Clone, Default)]
pub struct Completion {
    /// Every candidate at the cursor position. Empty when nothing
    /// applies. Sorted alphabetically so cycle order is stable.
    pub candidates: Vec<String>,
    /// Longest common prefix across `candidates` *minus* what the user
    /// has already typed at the cursor position. Empty when there is
    /// nothing to extend or no candidates apply. Renderers paint this as
    /// ghost text right after the user's caret.
    pub ghost: String,
    /// What the cursor is currently completing. Empty at a token
    /// boundary, otherwise the partial token text.
    pub partial: String,
}

impl Completion {
    /// Compute the completion set for `input`, drawing arg-completion
    /// candidates from the active game state when one is available.
    pub fn from_input(input: &str, game: Option<&GameState>) -> Self {
        let pos = analyze(input);
        let ctx = CompletionCtx { game };
        match pos {
            InputPosition::Boundary { tokens } => Self::for_boundary(&tokens, &ctx),
            InputPosition::Inside {
                tokens,
                partial,
                partial_quoted,
            } => Self::for_inside(&tokens, &partial, partial_quoted, &ctx),
        }
    }

    /// User finished typing a token and pressed space (or hasn't typed
    /// anything yet). Suggest *all* candidates for the next slot — no
    /// ghost, since a boundary has no in-progress token to extend.
    fn for_boundary(prior_tokens: &[Token], ctx: &CompletionCtx<'_>) -> Self {
        if prior_tokens.is_empty() {
            // No command typed yet; show every command name.
            let mut candidates: Vec<String> = registry().iter().map(|c| c.name().to_string()).collect();
            candidates.sort();
            return Completion {
                candidates,
                ghost: String::new(),
                partial: String::new(),
            };
        }
        let Some(cmd) = find_command(&prior_tokens[0].text) else {
            return Completion::default();
        };
        let arg_index = prior_tokens.len() - 1;
        let prior_args: Vec<&str> = prior_tokens[1..].iter().map(|t| t.text.as_str()).collect();
        let mut candidates = cmd.arg_completions(arg_index, &prior_args, "", ctx);
        candidates.sort();
        Completion {
            candidates,
            ghost: String::new(),
            partial: String::new(),
        }
    }

    /// User is in the middle of a token. Filter candidates to those that
    /// start with the partial, then derive the ghost as `lcp - partial`
    /// when the candidates share a longer common prefix than the partial.
    fn for_inside(
        tokens: &[Token],
        partial: &str,
        partial_quoted: bool,
        ctx: &CompletionCtx<'_>,
    ) -> Self {
        let mut candidates: Vec<String> = if tokens.is_empty() {
            registry()
                .iter()
                .map(|c| c.name().to_string())
                .filter(|n| n.starts_with(partial))
                .collect()
        } else {
            let Some(cmd) = find_command(&tokens[0].text) else {
                return Completion::default();
            };
            let arg_index = tokens.len() - 1;
            let prior_args: Vec<&str> = tokens[1..].iter().map(|t| t.text.as_str()).collect();
            cmd.arg_completions(arg_index, &prior_args, partial, ctx)
                .into_iter()
                .filter(|c| c.starts_with(partial))
                .collect()
        };
        candidates.sort();
        candidates.dedup();

        let lcp = longest_common_prefix(&candidates);
        let ghost = if lcp.len() > partial.len() {
            // Candidates with whitespace need a closing quote when accepted —
            // but the *ghost* is just the suffix; quoting wraps when Tab
            // commits the completion (see `accept_into`).
            lcp[partial.len()..].to_string()
        } else {
            String::new()
        };

        // If exactly one candidate matches and we want to wrap it in quotes,
        // the ghost should still be only the suffix; the wrapper is added on
        // accept. The ghost text itself never includes a leading quote.
        let _ = partial_quoted;

        Completion {
            candidates,
            ghost,
            partial: partial.to_string(),
        }
    }

    /// Apply a Tab keystroke. If there's a unique completion and a ghost,
    /// extend the input. If there are multiple matches and Tab is pressed
    /// again, cycle through them by index.
    ///
    /// Returns the new input string. Wraps multi-word candidates in
    /// double quotes so the result re-tokenizes the same way.
    pub fn accept_into(&self, input: &str, cycle_index: usize) -> Option<String> {
        if self.candidates.is_empty() {
            return None;
        }
        let chosen = if self.candidates.len() == 1 {
            self.candidates[0].clone()
        } else if !self.ghost.is_empty() {
            // Multiple candidates but they share a longer prefix — extend
            // to the LCP and stop. Cycling kicks in only after that.
            let extended = format!("{}{}", self.partial, self.ghost);
            return Some(replace_last_token(input, &extended, false));
        } else {
            self.candidates[cycle_index % self.candidates.len()].clone()
        };
        let needs_quoting = chosen.chars().any(char::is_whitespace);
        let replacement = if needs_quoting {
            quote_if_needed(&chosen)
        } else {
            chosen
        };
        Some(replace_last_token(input, &replacement, needs_quoting))
    }
}

fn longest_common_prefix(strings: &[String]) -> String {
    let Some(first) = strings.first() else {
        return String::new();
    };
    let mut prefix = first.clone();
    for s in &strings[1..] {
        while !s.starts_with(&prefix) {
            prefix.pop();
            if prefix.is_empty() {
                return prefix;
            }
        }
    }
    prefix
}

/// Replace the final token of `input` with `replacement`. Preserves any
/// leading whitespace separators and the rest of the input.
fn replace_last_token(input: &str, replacement: &str, _replacement_was_quoted: bool) -> String {
    let trimmed_end = input.trim_end_matches(|c: char| c.is_whitespace());
    if trimmed_end.len() != input.len() {
        // User was at a boundary; the replacement starts a new token.
        return format!("{input}{replacement}");
    }
    let last_break = input.rfind(|c: char| c.is_whitespace() || c == '"');
    match last_break {
        Some(pos) => {
            let head = &input[..=pos];
            let head = head.trim_end_matches('"');
            format!("{head}{replacement}")
        }
        None => replacement.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_lists_every_command() {
        let c = Completion::from_input("", None);
        assert!(c.candidates.iter().any(|s| s == "fullbright"));
        assert!(c.candidates.iter().any(|s| s == "spawn"));
        assert!(c.candidates.iter().any(|s| s == "give"));
        assert!(c.ghost.is_empty());
    }

    #[test]
    fn partial_command_extends_to_lcp() {
        let c = Completion::from_input("ful", None);
        assert_eq!(c.candidates, vec!["fullbright"]);
        assert_eq!(c.ghost, "lbright");
    }

    #[test]
    fn unique_command_with_space_lists_subcommands() {
        let c = Completion::from_input("spawn ", None);
        assert!(c.candidates.iter().any(|s| s == "item"));
        assert!(c.candidates.iter().any(|s| s == "enemy"));
    }

    #[test]
    fn give_partial_arg_completes_item_name() {
        let c = Completion::from_input("give torc", None);
        assert_eq!(c.candidates, vec!["torch"]);
        assert_eq!(c.ghost, "h");
    }

    #[test]
    fn accept_unique_completion_appends_extension() {
        let c = Completion::from_input("ful", None);
        let extended = c.accept_into("ful", 0).unwrap();
        assert_eq!(extended, "fullbright");
    }

    #[test]
    fn accept_multi_word_wraps_in_quotes() {
        let c = Completion::from_input("give scroll", None);
        let extended = c.accept_into("give scroll", 0).unwrap();
        assert_eq!(extended, "give \"scroll of fire\"");
    }

    #[test]
    fn accept_at_boundary_with_one_candidate_appends() {
        let c = Completion::from_input("spawn ", None);
        // multiple candidates -> cycling kicks in; cycle 0 picks first sorted
        let extended = c.accept_into("spawn ", 0).unwrap();
        assert!(extended == "spawn enemy" || extended == "spawn item");
    }
}
