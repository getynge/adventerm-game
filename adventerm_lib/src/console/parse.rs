//! Console line tokenizer. Splits an input string on whitespace, but
//! treats `"…"` as a single token so multi-word item names round-trip
//! through the command line. Used by both the executor and the completer
//! so completion candidates produce executable input verbatim.

/// A consumed token from the input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    /// True if the token was originally enclosed in double quotes. The
    /// renderer / completer use this to know whether to wrap a completion
    /// suffix in matching quotes.
    pub quoted: bool,
}

/// Tokenize an input line into already-unquoted tokens. Trailing whitespace
/// is treated as "user finished typing the previous token"; an unterminated
/// quote consumes through end-of-input (lenient — the user is still typing).
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = input.chars().peekable();
    loop {
        while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
            chars.next();
        }
        let Some(&first) = chars.peek() else {
            break;
        };
        if first == '"' {
            chars.next();
            let mut buf = String::new();
            while let Some(&c) = chars.peek() {
                if c == '"' {
                    chars.next();
                    break;
                }
                buf.push(c);
                chars.next();
            }
            out.push(Token {
                text: buf,
                quoted: true,
            });
        } else {
            let mut buf = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                buf.push(c);
                chars.next();
            }
            out.push(Token {
                text: buf,
                quoted: false,
            });
        }
    }
    out
}

/// Where the cursor is, relative to the token stream:
/// `Inside { idx, partial }` — caret sits in the middle of an active token.
/// `Boundary { args }` — caret sits after a separator; the next character
///   typed will start a new token. `args` is what is already complete.
#[derive(Debug, PartialEq, Eq)]
pub enum InputPosition {
    Inside { tokens: Vec<Token>, partial: String, partial_quoted: bool },
    Boundary { tokens: Vec<Token> },
}

/// Inspect input and report whether the user is mid-token or at a boundary.
/// This drives whether Tab should complete the current word or suggest a
/// new argument.
pub fn analyze(input: &str) -> InputPosition {
    let trailing_space = input
        .chars()
        .last()
        .map(|c| c.is_whitespace())
        .unwrap_or(true);
    let tokens = tokenize(input);
    if trailing_space || tokens.is_empty() {
        InputPosition::Boundary { tokens }
    } else {
        let mut tokens = tokens;
        let last = tokens.pop().unwrap();
        InputPosition::Inside {
            tokens,
            partial: last.text,
            partial_quoted: last.quoted,
        }
    }
}

/// Quote `s` only if it contains whitespace (or is empty). Used by the
/// completer when emitting an argument suffix that must round-trip through
/// the tokenizer.
pub fn quote_if_needed(s: &str) -> String {
    if s.is_empty() || s.chars().any(char::is_whitespace) {
        format!("\"{s}\"")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare(s: &str) -> Token {
        Token {
            text: s.to_string(),
            quoted: false,
        }
    }
    fn quoted(s: &str) -> Token {
        Token {
            text: s.to_string(),
            quoted: true,
        }
    }

    #[test]
    fn bare_tokens() {
        assert_eq!(
            tokenize("give torch"),
            vec![bare("give"), bare("torch")]
        );
    }

    #[test]
    fn quoted_multi_word() {
        assert_eq!(
            tokenize("give \"goggles of seeing\""),
            vec![bare("give"), quoted("goggles of seeing")]
        );
    }

    #[test]
    fn unterminated_quote_consumes_rest() {
        assert_eq!(
            tokenize("give \"goggles of seeing"),
            vec![bare("give"), quoted("goggles of seeing")]
        );
    }

    #[test]
    fn trailing_space_means_boundary() {
        match analyze("give ") {
            InputPosition::Boundary { tokens } => assert_eq!(tokens, vec![bare("give")]),
            other => panic!("expected Boundary, got {other:?}"),
        }
    }

    #[test]
    fn no_trailing_space_means_inside() {
        match analyze("giv") {
            InputPosition::Inside {
                tokens,
                partial,
                partial_quoted,
            } => {
                assert!(tokens.is_empty());
                assert_eq!(partial, "giv");
                assert!(!partial_quoted);
            }
            other => panic!("expected Inside, got {other:?}"),
        }
    }

    #[test]
    fn quote_if_needed_wraps_when_required() {
        assert_eq!(quote_if_needed("torch"), "torch");
        assert_eq!(quote_if_needed("scroll of fire"), "\"scroll of fire\"");
    }
}
