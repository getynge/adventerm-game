use std::collections::HashSet;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

const RESERVED: &[char] = &['w', 'a', 's', 'd', 'h', 'j', 'k', 'l'];

pub fn assign(labels: &[&str]) -> Vec<Option<usize>> {
    let mut used: HashSet<char> = RESERVED.iter().copied().collect();
    labels
        .iter()
        .map(|label| {
            for (i, c) in label.char_indices() {
                if !c.is_ascii_alphabetic() {
                    continue;
                }
                let lower = c.to_ascii_lowercase();
                if used.insert(lower) {
                    return Some(i);
                }
            }
            None
        })
        .collect()
}

pub fn matches(label: &str, accel: Option<usize>, key: char) -> bool {
    let Some(idx) = accel else {
        return false;
    };
    let Some(c) = label[idx..].chars().next() else {
        return false;
    };
    c.eq_ignore_ascii_case(&key)
}

pub fn line(label: &str, accel: Option<usize>, selected: bool) -> Line<'static> {
    let base = if selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    let (lead, trail) = if selected { ("> ", " <") } else { ("  ", "  ") };

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(5);
    spans.push(Span::styled(lead.to_string(), base));

    match accel {
        Some(idx) => {
            let accel_char = label[idx..].chars().next().expect("accelerator index in label");
            let after_idx = idx + accel_char.len_utf8();
            let before = &label[..idx];
            let after = &label[after_idx..];

            if !before.is_empty() {
                spans.push(Span::styled(before.to_string(), base));
            }
            spans.push(Span::styled(
                accel_char.to_string(),
                base.add_modifier(Modifier::UNDERLINED),
            ));
            if !after.is_empty() {
                spans.push(Span::styled(after.to_string(), base));
            }
        }
        None => spans.push(Span::styled(label.to_string(), base)),
    }

    spans.push(Span::styled(trail.to_string(), base));
    Line::from(spans)
}

pub fn find_by_hotkey<F: Fn(usize) -> &'static str>(
    count: usize,
    label: F,
    key: char,
) -> Option<usize> {
    let labels: Vec<&str> = (0..count).map(&label).collect();
    let accels = assign(&labels);
    accels
        .iter()
        .enumerate()
        .find(|(i, a)| matches(labels[*i], **a, key))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_letter_when_unique() {
        assert_eq!(assign(&["Play", "Quit"]), vec![Some(0), Some(0)]);
    }

    #[test]
    fn skips_reserved_letters() {
        // 'S' is reserved (Down), 'a' is reserved (Left), 'v' is free.
        assert_eq!(assign(&["Save"]), vec![Some(2)]);
    }

    #[test]
    fn skips_letters_taken_by_earlier_options() {
        // Second "Play" can't reuse 'p'; falls to 'l' — reserved — to 'a'? 'a' reserved too. To 'y'.
        assert_eq!(assign(&["Play", "Play"]), vec![Some(0), Some(3)]);
    }

    #[test]
    fn returns_none_when_no_letter_available() {
        // All letters reserved.
        assert_eq!(assign(&["wash"]), vec![None]);
    }

    #[test]
    fn matches_is_case_insensitive() {
        assert!(matches("Play", Some(0), 'P'));
        assert!(matches("Play", Some(0), 'p'));
        assert!(!matches("Play", Some(0), 'q'));
        assert!(!matches("Play", None, 'p'));
    }
}
