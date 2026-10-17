#[cfg(any(target_os = "macos", target_os = "windows", test))]
use std::collections::HashMap;
use std::collections::HashSet;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

// Leave headroom for names that tokenize less efficiently than English prose.
const PROMPT_BYTES: usize = 672;
#[cfg(any(target_os = "macos", target_os = "windows", test))]
const COMMON: &[&str] = &[
    "the",
    "a",
    "an",
    "and",
    "or",
    "but",
    "this",
    "that",
    "these",
    "those",
    "please",
    "hello",
    "hi",
    "thanks",
    "thank",
    "you",
    "your",
    "we",
    "our",
    "they",
    "their",
    "it",
    "its",
    "is",
    "are",
    "was",
    "were",
    "can",
    "could",
    "would",
    "should",
    "will",
    "have",
    "has",
    "how",
    "what",
    "when",
    "where",
    "why",
    "who",
    "for",
    "from",
    "to",
    "in",
    "on",
    "of",
    "at",
    "with",
    "new",
    "view",
    "edit",
    "file",
    "search",
    "send",
    "reply",
    "today",
    "yesterday",
];

pub(crate) fn names() -> Vec<String> {
    #[cfg(target_os = "macos")]
    return extract(&macos::text());
    #[cfg(target_os = "windows")]
    return extract(&windows::text());
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Vec::new()
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn extract(text: &str) -> Vec<String> {
    fn candidate(word: &str) -> bool {
        word.chars().count() >= 2
            && word.chars().count() <= 40
            && word
                .chars()
                .all(|c| c.is_alphabetic() || matches!(c, '-' | '\'' | '’'))
            && word.chars().any(char::is_uppercase)
            && !COMMON.contains(&word.to_lowercase().as_str())
    }
    let mut counts: HashMap<String, (String, usize, usize)> = HashMap::new();
    let mut phrase = Vec::new();
    let mut order = 0;
    let mut flush = |phrase: &mut Vec<&str>| {
        if !phrase.is_empty() {
            let name = phrase.join(" ");
            let entry = counts
                .entry(name.to_lowercase())
                .or_insert((name, 0, order));
            entry.1 += 1;
            order += 1;
            phrase.clear();
        }
    };
    for line in text.lines() {
        for token in line.split_whitespace() {
            let word = token.trim_matches(|c: char| {
                matches!(
                    c,
                    ',' | '.' | '!' | '?' | ':' | ';' | '(' | ')' | '[' | ']' | '"'
                )
            });
            if candidate(word) {
                phrase.push(word);
                if phrase.len() == 3 || token.ends_with(['.', '!', '?', ':', ';', ',']) {
                    flush(&mut phrase);
                }
            } else {
                flush(&mut phrase);
            }
        }
        flush(&mut phrase);
    }
    let mut ranked: Vec<_> = counts.into_values().collect();
    ranked.sort_by_key(|(_, count, order)| (std::cmp::Reverse(*count), *order));
    ranked
        .into_iter()
        .take(30)
        .map(|(name, _, _)| name)
        .collect()
}

pub(crate) fn prompt(dictionary: &[String], names: &[String]) -> String {
    let mut seen = HashSet::new();
    let mut prompt = String::new();
    for term in dictionary.iter().chain(names.iter().take(30)) {
        let term = term.trim();
        if term.is_empty() || !seen.insert(term.to_lowercase()) {
            continue;
        }
        let separator = if prompt.is_empty() { "" } else { ", " };
        if prompt.len() + separator.len() + term.len() <= PROMPT_BYTES {
            prompt.push_str(separator);
            prompt.push_str(term);
        }
    }
    prompt
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) struct Text {
    started: std::time::Instant,
    value: String,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl Text {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            value: String::new(),
        }
    }

    fn expired(&self) -> bool {
        self.started.elapsed() >= std::time::Duration::from_millis(150)
            || self.value.len() >= 50_000
    }

    fn push(&mut self, value: &str) {
        let remaining = 50_000 - self.value.len();
        if remaining <= 1 {
            return;
        }
        self.value
            .push_str(&value[..value.floor_char_boundary(remaining - 1)]);
        self.value.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_prompt_cases() {
        for (text, expected) in [
            (
                "The team met Aditya Rao today. Please review TypeAside.",
                vec!["Aditya Rao", "TypeAside"],
            ),
            (
                "https://Acme.com /Users/Aditya C:\\Users\\Alice Alice@example.com Project42 123",
                vec![],
            ),
            (
                "TypeAside\ntypeAside\nTypeAside\nAlice",
                vec!["TypeAside", "Alice"],
            ),
            ("Zoë Chen met Élodie", vec!["Zoë Chen", "Élodie"]),
            ("Alice met Bob met Bob", vec!["Bob", "Alice"]),
        ] {
            assert_eq!(extract(text), expected, "{text}");
        }
        for (dictionary, names, expected) in [
            (
                vec!["Aditya".into()],
                vec!["aditya".into(), "TypeAside".into()],
                "Aditya, TypeAside".into(),
            ),
            (
                vec!["x".repeat(PROMPT_BYTES + 1)],
                vec!["Zoë".into()],
                "Zoë".into(),
            ),
            (
                vec!["x".repeat(PROMPT_BYTES - 2)],
                vec!["Zoë".into()],
                "x".repeat(PROMPT_BYTES - 2),
            ),
            (vec![], vec![], String::new()),
        ] {
            let actual = prompt(&dictionary, &names);
            assert_eq!(actual, expected);
            assert!(actual.len() <= PROMPT_BYTES);
        }
        let text = (0..40)
            .map(|i| {
                format!(
                    "Name{}{}",
                    char::from(b'A' + i / 26),
                    char::from(b'A' + i % 26)
                )
            })
            .collect::<Vec<_>>()
            .join(" met ");
        assert!(extract(&text).len() <= 30);
    }
}
