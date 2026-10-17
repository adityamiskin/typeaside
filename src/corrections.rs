use crate::{config::AppConfig, engine::Dictation, paths::AppPaths};
use anyhow::Context;
use std::{collections::HashSet, time::SystemTime};

pub fn candidates<'a>(
    old: &str,
    new: &str,
    known: impl IntoIterator<Item = &'a String>,
) -> Vec<String> {
    let old = old.split_whitespace().collect::<Vec<_>>();
    let new = new.split_whitespace().collect::<Vec<_>>();
    let mut lcs = vec![vec![0; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lcs[i][j] = if old[i] == new[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut seen = known
        .into_iter()
        .map(|word| word.to_lowercase())
        .collect::<HashSet<_>>();
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < old.len() || j < new.len() {
        if i < old.len() && j < new.len() && old[i] == new[j] {
            i += 1;
            j += 1;
            continue;
        }
        let (start_i, start_j) = (i, j);
        while i < old.len() || j < new.len() {
            if i < old.len() && j < new.len() && old[i] == new[j] {
                break;
            }
            if i < old.len() && (j == new.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
                i += 1;
            } else {
                j += 1;
            }
        }
        if i == start_i || j == start_j {
            continue;
        }
        let replaced = old[start_i..i].join(" ").to_lowercase();
        let words = new[start_j..j]
            .iter()
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
            .collect::<Vec<_>>();
        let mut k = 0;
        while k < words.len() {
            let start = k;
            k += 1;
            if capitalized(words[start]) {
                while k < words.len() && capitalized(words[k]) {
                    k += 1;
                }
            }
            let phrase = words[start..k]
                .iter()
                .filter(|word| word.chars().count() >= 2)
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            if term_like(&phrase)
                && !replaced.contains(&phrase.to_lowercase())
                && seen.insert(phrase.to_lowercase())
            {
                result.push(phrase);
            }
        }
    }
    result
}

fn capitalized(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase)
}

/// Plain lowercase words are ordinary edits, not vocabulary worth teaching.
fn term_like(phrase: &str) -> bool {
    phrase.split(' ').all(|word| !machine_text(word))
        && phrase
            .chars()
            .any(|c| c.is_uppercase() || c.is_ascii_digit() || matches!(c, '.' | '\'' | '-'))
}

/// URLs, paths, emails, numbers, dates and code are typed, not dictated, so they never teach STT.
fn machine_text(word: &str) -> bool {
    const TLDS: &[&str] = &[
        "com", "org", "net", "io", "app", "dev", "ai", "co", "me", "xyz",
    ];
    !word.chars().any(char::is_alphabetic)
        || word.contains(['/', '\\', '@', ':', '=', '_', '(', ')', '<', '>', '#', '`'])
        || word
            .rsplit_once('.')
            .is_some_and(|(_, tld)| TLDS.contains(&tld.to_lowercase().as_str()))
}

pub fn save(
    paths: &AppPaths,
    history: &mut [Dictation],
    config: &mut AppConfig,
    at: SystemTime,
    text: &str,
    selected: &[String],
) -> anyhow::Result<Vec<String>> {
    let item = history
        .iter_mut()
        .find(|item| item.at == at)
        .context("Dictation no longer exists")?;
    item.cleaned = text.to_owned();
    crate::engine::save_history(paths, history)?;
    let added = selected
        .iter()
        .filter(|word| config.add_word((*word).clone()))
        .cloned()
        .collect::<Vec<_>>();
    config.save(paths)?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_words_are_not_suggested_until_added_again() {
        let mut config = AppConfig::default();
        assert!(config.add_word("Convex".into()));
        config.remove_word("Convex");
        assert!(config.dictionary.is_empty());
        assert!(candidates("use old", "use CONVEX", config.known_words()).is_empty());
        assert!(config.add_word("convex".into()));
        assert!(config.removed_words.is_empty());
        assert!(!config.add_word("CONVEX".into()));
    }

    #[test]
    fn replacement_candidates() {
        let cases: &[(&str, &str, &[&str], &[&str])] = &[
            (
                "use nope and person",
                "use (Node.js), and O'Brien!",
                &[],
                &["Node.js", "O'Brien"],
            ),
            (
                "one and two and three",
                "Convex and convex and CONVEX",
                &[],
                &["Convex"],
            ),
            ("use this", "use Hugging Face this", &[], &[]),
            ("remove this word", "remove word", &[], &[]),
            (
                "use huggingface with old",
                "use Hugging Face with new",
                &[],
                &["Hugging Face"],
            ),
            (
                "use old and older",
                "use Convex and Hugging Face",
                &["CONVEX", "hugging face"],
                &[],
            ),
            ("say old then old", "say old then Convex", &[], &["Convex"]),
            ("send it", "Send it", &[], &[]),
            ("ship it on monday", "ship it on Tuesday", &[], &["Tuesday"]),
            ("some text", "", &[], &[]),
            ("", "Convex", &[], &[]),
            ("same text", "same text", &[], &[]),
            ("word", "a ! x", &[], &[]),
            ("old and older", "Éclair and éclair", &[], &["Éclair"]),
            ("use old word", "use Node.js today", &[], &["Node.js"]),
            (
                "see a b c d e f g h i",
                "see example.com https://typeaside.app/docs src/main.rs adi@x.com 10:30 2026-10-17 3.5 user_id foo()",
                &[],
                &[],
            ),
            (
                "try old and old on old",
                "try GPT-5 and S3 on Typeaside.app",
                &[],
                &["GPT-5", "S3"],
            ),
        ];
        for (old, new, dictionary, expected) in cases {
            let dictionary = dictionary
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<Vec<_>>();
            assert_eq!(
                candidates(old, new, &dictionary),
                *expected,
                "{old:?} -> {new:?}"
            );
        }
    }
}
