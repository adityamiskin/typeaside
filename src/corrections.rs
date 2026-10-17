use crate::{config::AppConfig, engine::Dictation, paths::AppPaths};
use anyhow::Context;
use std::{collections::HashSet, time::SystemTime};

pub fn candidates(old: &str, new: &str, dictionary: &[String]) -> Vec<String> {
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
    let mut seen = dictionary
        .iter()
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
    phrase
        .chars()
        .any(|c| c.is_uppercase() || c.is_ascii_digit() || matches!(c, '.' | '\'' | '-' | '_'))
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
        .filter(|word| {
            !config
                .dictionary
                .iter()
                .any(|existing| existing.to_lowercase() == word.to_lowercase())
        })
        .cloned()
        .collect::<Vec<_>>();
    config.dictionary.extend(added.iter().cloned());
    config.save(paths)?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

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
