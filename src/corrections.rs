use crate::{config::AppConfig, engine::Dictation, paths::AppPaths};
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
            if !phrase.is_empty() && seen.insert(phrase.to_lowercase()) {
                result.push(phrase);
            }
        }
    }
    result
}

fn capitalized(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase)
}

pub fn save(
    paths: &AppPaths,
    history: &mut [Dictation],
    config: &mut AppConfig,
    at: SystemTime,
    text: &str,
    selected: &[String],
) -> anyhow::Result<Vec<String>> {
    let index = history
        .iter()
        .position(|item| item.at == at)
        .ok_or_else(|| anyhow::anyhow!("Dictation no longer exists"))?;
    let mut updated = history.to_vec();
    updated[index].cleaned = text.to_owned();
    crate::engine::save_history(paths, &updated)?;
    history[index].cleaned = text.to_owned();
    let mut next_config = config.clone();
    let mut added = Vec::new();
    for word in selected {
        if !next_config
            .dictionary
            .iter()
            .any(|existing| existing.to_lowercase() == word.to_lowercase())
        {
            next_config.dictionary.push(word.clone());
            added.push(word.clone());
        }
    }
    next_config.save(paths)?;
    *config = next_config;
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
                &["Hugging Face", "new"],
            ),
            (
                "use old and older",
                "use Convex and Hugging Face",
                &["CONVEX", "hugging face"],
                &[],
            ),
            ("say old then old", "say old then Convex", &[], &["Convex"]),
            ("some text", "", &[], &[]),
            ("", "Convex", &[], &[]),
            ("same text", "same text", &[], &[]),
            ("word", "a ! x", &[], &[]),
            ("old and older", "Éclair and éclair", &[], &["Éclair"]),
            (
                "use old word",
                "use Node.js today",
                &[],
                &["Node.js", "today"],
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

    #[test]
    fn correction_persists_selected_terms_and_preserves_raw() -> anyhow::Result<()> {
        use crate::{
            engine::{Dictation, Mode, load_history},
            paths::AppPaths,
        };
        let root = std::path::PathBuf::from("/tmp/typeaside-learn-corrections/persistence");
        let paths = AppPaths {
            config_dir: root.join("config/typeaside"),
            config_file: root.join("config/typeaside/config.json"),
            data_dir: root.join("data/typeaside"),
            history_file: root.join("data/typeaside/history.jsonl"),
            failed_file: root.join("data/typeaside/failed.wav"),
        };
        paths.ensure()?;
        let at = std::time::SystemTime::UNIX_EPOCH;
        let mut history = vec![Dictation {
            at,
            raw: "use huggingface and con vex".into(),
            cleaned: "Use huggingface and con vex".into(),
            app_id: None,
            audio_ms: 1000,
            latency_ms: 200,
            mode: Mode::Dictate,
        }];
        let mut config = crate::config::AppConfig::default();
        crate::engine::save_history(&paths, &history)?;
        config.save(&paths)?;
        std::fs::copy(&paths.history_file, root.join("before-history.jsonl"))?;
        std::fs::copy(&paths.config_file, root.join("before-config.json"))?;
        let added = save(
            &paths,
            &mut history,
            &mut config,
            at,
            "Use Hugging Face and Convex",
            &["Hugging Face".into()],
        )?;
        assert_eq!(added, ["Hugging Face"]);
        let loaded = load_history(&paths);
        assert_eq!(loaded[0].cleaned, "Use Hugging Face and Convex");
        assert_eq!(loaded[0].raw, "use huggingface and con vex");
        assert_eq!(
            crate::config::AppConfig::load(&paths)?.dictionary,
            ["Hugging Face"]
        );
        std::fs::copy(&paths.history_file, root.join("after-history.jsonl"))?;
        std::fs::copy(&paths.config_file, root.join("after-config.json"))?;
        assert!(
            save(
                &paths,
                &mut history,
                &mut config,
                at,
                "Use Hugging Face and Convex",
                &["Hugging Face".into()]
            )?
            .is_empty()
        );
        let bad_paths = AppPaths {
            history_file: root.join("missing/history.jsonl"),
            ..paths
        };
        assert!(save(&bad_paths, &mut history, &mut config, at, "changed", &[]).is_err());
        assert_eq!(history[0].cleaned, "Use Hugging Face and Convex");
        Ok(())
    }
}
