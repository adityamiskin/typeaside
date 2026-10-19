use std::collections::HashSet;

pub fn learned<'a>(
    inserted: &str,
    edited: &str,
    known: impl IntoIterator<Item = &'a String>,
) -> Vec<String> {
    let old = inserted.split_whitespace().collect::<Vec<_>>();
    let new = edited.split_whitespace().collect::<Vec<_>>();
    let hunks = diff(&old, &new);
    let short = old.len().max(new.len()) <= 3;
    let changed = hunks
        .iter()
        .map(|(a, b)| a.len().max(b.len()))
        .sum::<usize>();
    if hunks.len() > 3
        || (!short && changed * 5 > old.len().max(new.len()) * 2)
        || hunks.iter().any(|(a, b)| a.len() > 3 || b.len() > 3)
    {
        return Vec::new();
    }
    let mut seen = known
        .into_iter()
        .map(|word| word.to_lowercase())
        .collect::<HashSet<_>>();
    let mut result = Vec::new();
    for (before, after) in hunks {
        if before.is_empty() || after.is_empty() {
            continue;
        }
        if !short && before.start == 0 && before.end == old.len() {
            continue;
        }
        let old_words = &old[before];
        let new_words = &new[after];
        let similar = if old_words.len() == new_words.len() {
            old_words
                .iter()
                .zip(new_words)
                .all(|(a, b)| vocabulary_fix(a, b))
        } else {
            vocabulary_fix(&old_words.join(" "), &new_words.join(" "))
        };
        if !similar {
            continue;
        }
        for phrase in phrases(new_words) {
            if seen.insert(phrase.to_lowercase()) {
                result.push(phrase);
            }
        }
    }
    result
}

fn phrases(words: &[&str]) -> Vec<String> {
    let words = words
        .iter()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect::<Vec<_>>();
    let mut phrases = Vec::new();
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
        if term_like(&phrase) {
            phrases.push(phrase);
        }
    }
    phrases
}

fn distinctive(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        || text
            .split_whitespace()
            .any(|word| word.chars().skip(1).any(char::is_uppercase))
}

fn vocabulary_fix(old: &str, new: &str) -> bool {
    if new.split_whitespace().any(|word| {
        word.rsplit_once(['\'', '’']).is_some_and(|(_, suffix)| {
            matches!(
                suffix.to_lowercase().as_str(),
                "t" | "s" | "m" | "d" | "ll" | "re" | "ve"
            )
        })
    }) {
        return false;
    }
    let letters = |text: &str| {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    };
    let old_letters = letters(old);
    let new_letters = letters(new);
    if old.split_whitespace().count() == new.split_whitespace().count()
        && old_letters.to_lowercase() == new_letters.to_lowercase()
        && (old_letters == new_letters || !distinctive(new))
    {
        return false;
    }
    let normalize = |text: &str| {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<Vec<_>>()
    };
    let old = normalize(old);
    let new = normalize(new);
    if old.is_empty() || new.is_empty() {
        return false;
    }
    let mut distances = (0..=new.len()).collect::<Vec<_>>();
    for (i, a) in old.iter().enumerate() {
        let mut diagonal = distances[0];
        distances[0] = i + 1;
        for (j, b) in new.iter().enumerate() {
            let previous = distances[j + 1];
            distances[j + 1] = (diagonal + usize::from(a != b))
                .min(distances[j] + 1)
                .min(previous + 1);
            diagonal = previous;
        }
    }
    distances[new.len()] * 2 <= old.len().max(new.len())
}

fn diff(old: &[&str], new: &[&str]) -> Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> {
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
    let mut hunks = Vec::new();
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
        hunks.push((start_i..i, start_j..j));
    }
    hunks
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
