use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub trigger: String,
    pub replacement: String,
}

static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\w$").unwrap());
static SPACELESS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\p{Han}\p{Hiragana}\p{Katakana}\p{Thai}\p{Lao}\p{Khmer}\p{Myanmar}]$").unwrap()
});

/// Scripts written without spaces have no word edges, so only joins between other word characters count.
fn joined(a: Option<char>, b: Option<char>) -> bool {
    let wordy = |c: Option<char>| {
        c.is_some_and(|c| {
            let c = c.to_string();
            WORD.is_match(&c) && !SPACELESS.is_match(&c)
        })
    };
    wordy(a) && wordy(b)
}

pub fn apply(text: &str, rules: &[Rule]) -> String {
    let patterns = rules
        .iter()
        .filter(|rule| !rule.trigger.trim().is_empty())
        .filter_map(|rule| {
            let pattern = Regex::new(&format!("^(?i:{})", regex::escape(rule.trigger.trim())));
            Some((pattern.ok()?, rule.replacement.as_str()))
        })
        .collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut previous = None;
    let mut rest = text;
    while let Some(next) = rest.chars().next() {
        let best = patterns
            .iter()
            .filter_map(|(pattern, replacement)| {
                let matched = pattern.find(rest)?.as_str();
                let edges = !joined(previous, matched.chars().next())
                    && !joined(
                        matched.chars().next_back(),
                        rest[matched.len()..].chars().next(),
                    );
                edges.then_some((matched, *replacement))
            })
            .rev()
            .max_by_key(|(matched, _)| matched.len());
        let (matched, replacement) =
            best.unwrap_or((&rest[..next.len_utf8()], &rest[..next.len_utf8()]));
        output.push_str(replacement);
        previous = matched.chars().next_back();
        rest = &rest[matched.len()..];
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        rules: Vec<(String, String)>,
        input: String,
        expected: String,
    }

    #[test]
    fn literal_phrase_rules() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../tests/fixtures/replacements.json")).unwrap();
        for case in cases {
            let rules = case
                .rules
                .into_iter()
                .map(|(trigger, replacement)| Rule {
                    trigger,
                    replacement,
                })
                .collect::<Vec<_>>();
            assert_eq!(apply(&case.input, &rules), case.expected, "{}", case.name);
        }
    }
}
