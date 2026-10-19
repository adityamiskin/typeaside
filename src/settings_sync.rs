use crate::config::{AppConfig, AppInstruction, MAX_APP_INSTRUCTIONS, Snippet, Styles, app_key};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncState {
    pub updated_at: u64,
    pub base: Fields,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Fields {
    pub dictionary: Vec<String>,
    pub removed_words: Vec<String>,
    pub snippets: Vec<Snippet>,
    pub styles: Styles,
    pub app_instructions: Vec<AppInstruction>,
}

impl Fields {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            dictionary: config.dictionary.clone(),
            removed_words: config.removed_words.clone(),
            snippets: config.snippets.clone(),
            styles: config.styles,
            app_instructions: config.app_instructions.clone(),
        }
    }

    pub fn apply(self, config: &mut AppConfig) {
        config.dictionary = self.dictionary;
        config.removed_words = self.removed_words;
        config.snippets = self.snippets;
        config.styles = self.styles;
        config.app_instructions = self.app_instructions;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(flatten)]
    pub fields: Fields,
    pub updated_at: u64,
}

#[derive(Deserialize)]
struct Response {
    settings: Option<Settings>,
}

fn word_keys(words: &[String]) -> HashSet<String> {
    words.iter().map(|word| word.to_lowercase()).collect()
}

fn merge_words(base: &[String], local: Vec<String>, mut remote: Vec<String>) -> Vec<String> {
    let base = word_keys(base);
    let local_keys = word_keys(&local);
    let mut seen = HashSet::new();
    remote.retain(|word| {
        let key = word.to_lowercase();
        (!base.contains(&key) || local_keys.contains(&key)) && seen.insert(key)
    });
    for word in local {
        let key = word.to_lowercase();
        if !base.contains(&key) && seen.insert(key) {
            remote.push(word);
        }
    }
    remote
}

trait Keyed {
    fn key(&self) -> String;
    fn text(&self) -> &str;
}

impl Keyed for Snippet {
    fn key(&self) -> String {
        self.trigger.clone()
    }
    fn text(&self) -> &str {
        &self.text
    }
}

impl Keyed for AppInstruction {
    fn key(&self) -> String {
        app_key(&self.app)
    }
    fn text(&self) -> &str {
        &self.text
    }
}

/// Three-way merge by key: local adds, edits and deletes beat the remote copy; remote-only changes survive.
fn merge_keyed<T: Keyed>(base: &[T], local: Vec<T>, remote: &mut Vec<T>) {
    let base: HashMap<_, _> = base.iter().map(|item| (item.key(), item.text())).collect();
    let local_keys: HashSet<_> = local.iter().map(Keyed::key).collect();
    let mut seen = HashSet::new();
    remote.retain(|item| {
        let key = item.key();
        (!base.contains_key(&key) || local_keys.contains(&key)) && seen.insert(key)
    });
    for item in local {
        let key = item.key();
        if base.get(&key).copied() != Some(item.text()) {
            if let Some(stored) = remote.iter_mut().find(|stored| stored.key() == key) {
                *stored = item;
            } else {
                remote.push(item);
            }
        }
    }
}

pub fn merge(base: &Fields, local: Fields, mut remote: Fields) -> Fields {
    let dictionary_adds: HashSet<_> = word_keys(&local.dictionary)
        .difference(&word_keys(&base.dictionary))
        .cloned()
        .collect();
    let removed_adds: HashSet<_> = word_keys(&local.removed_words)
        .difference(&word_keys(&base.removed_words))
        .cloned()
        .collect();
    remote.dictionary = merge_words(&base.dictionary, local.dictionary, remote.dictionary);
    remote.removed_words = merge_words(
        &base.removed_words,
        local.removed_words,
        remote.removed_words,
    );
    remote
        .removed_words
        .retain(|word| !dictionary_adds.contains(&word.to_lowercase()));
    let removed = word_keys(&remote.removed_words);
    remote.dictionary.retain(|word| {
        let key = word.to_lowercase();
        !removed_adds.contains(&key) && !removed.contains(&key)
    });

    merge_keyed(&base.snippets, local.snippets, &mut remote.snippets);
    merge_keyed(
        &base.app_instructions,
        local.app_instructions,
        &mut remote.app_instructions,
    );
    remote.app_instructions.truncate(MAX_APP_INSTRUCTIONS);
    if local.styles != base.styles {
        remote.styles = local.styles;
    }
    remote
}

pub fn synchronize(config: &AppConfig, url: &str, token: &str) -> Result<Settings> {
    let client = crate::http::client();
    let remote = client
        .get(format!("{url}/v1/settings"))
        .bearer_auth(token)
        .send()?
        .error_for_status()?
        .json::<Response>()?
        .settings;
    let local = Fields::from_config(config);
    let remote_updated_at = remote.as_ref().map(|remote| remote.updated_at);
    let fields = if let Some(remote) = remote {
        let base = config.settings_sync.as_ref().map(|state| &state.base);
        if base == Some(&local) {
            return Ok(remote);
        }
        let initial_base;
        let base = if let Some(base) = base {
            base
        } else {
            // A stale device's first sync must respect words removed on another device.
            initial_base = Fields {
                dictionary: remote.fields.removed_words.clone(),
                ..Fields::default()
            };
            &initial_base
        };
        merge(base, local, remote.fields)
    } else {
        local
    };
    let now_ms: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis()
        .try_into()?;
    let updated_at = remote_updated_at.map_or(now_ms, |updated_at| now_ms.max(updated_at + 1));
    client
        .put(format!("{url}/v1/settings"))
        .bearer_auth(token)
        .json(&Settings { fields, updated_at })
        .send()?
        .error_for_status()?
        .json::<Response>()?
        .settings
        .context("missing stored settings")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_applies_local_changes_to_remote() {
        let fields = |words: &[&str], removed: &[&str], snippets: &[(&str, &str)], styles| Fields {
            dictionary: words.iter().map(|word| (*word).into()).collect(),
            removed_words: removed.iter().map(|word| (*word).into()).collect(),
            snippets: snippets
                .iter()
                .map(|(trigger, text)| Snippet {
                    trigger: (*trigger).into(),
                    text: (*text).into(),
                })
                .collect(),
            styles,
            ..Fields::default()
        };
        let defaults = Styles::default();
        let changed = Styles {
            work: crate::config::Tone::Formal,
            ..defaults
        };
        let cases = [
            (
                "local removal survives remote edits",
                fields(&["Rust"], &[], &[], defaults),
                fields(&[], &["rust"], &[], defaults),
                fields(&["RUST", "Convex"], &[], &[], defaults),
                fields(&["Convex"], &["rust"], &[], defaults),
            ),
            (
                "remote removal survives unchanged local word",
                fields(&["Rust"], &[], &[], defaults),
                fields(&["Rust", "Bun"], &[], &[], defaults),
                fields(&["Convex"], &["rust"], &[], defaults),
                fields(&["Convex", "Bun"], &["rust"], &[], defaults),
            ),
            (
                "concurrent adds preserve order and dedupe ignoring case",
                Fields::default(),
                fields(
                    &["HELLO", "Rust", "rust", "世界"],
                    &["OLD", "old"],
                    &[],
                    defaults,
                ),
                fields(
                    &["Hello", "hello", "Convex"],
                    &["Old", "old"],
                    &[],
                    defaults,
                ),
                fields(
                    &["Hello", "Convex", "Rust", "世界"],
                    &["Old"],
                    &[],
                    defaults,
                ),
            ),
            (
                "first sync base suppresses stale dictionary words removed remotely",
                fields(&["rust"], &[], &[], defaults),
                fields(&["RUST", "Bun"], &[], &[], defaults),
                fields(&["Convex"], &["rust"], &[], defaults),
                fields(&["Convex", "Bun"], &["rust"], &[], defaults),
            ),
            (
                "explicit re-add overrides remote removal",
                fields(&[], &["Rust"], &[], defaults),
                fields(&["RUST"], &[], &[], defaults),
                fields(&[], &["Rust", "Old"], &[], defaults),
                fields(&["RUST"], &["Old"], &[], defaults),
            ),
            (
                "otherwise dictionary loses remote overlap",
                Fields::default(),
                Fields::default(),
                fields(&["Rust"], &["RUST"], &[], defaults),
                fields(&[], &["RUST"], &[], defaults),
            ),
            (
                "snippet edit beats remote delete and local delete beats remote edit",
                fields(
                    &[],
                    &[],
                    &[("edit", "before"), ("delete", "before")],
                    defaults,
                ),
                fields(&[], &[], &[("edit", "after"), ("add", "local")], defaults),
                fields(
                    &[],
                    &[],
                    &[("delete", "remote edit"), ("remote", "retained")],
                    changed,
                ),
                fields(
                    &[],
                    &[],
                    &[("remote", "retained"), ("edit", "after"), ("add", "local")],
                    changed,
                ),
            ),
            (
                "unchanged snippet adopts remote edit and local styles override",
                fields(&[], &[], &[("sig", "before")], defaults),
                fields(&[], &[], &[("sig", "before")], changed),
                fields(&[], &[], &[("sig", "after")], defaults),
                fields(&[], &[], &[("sig", "after")], changed),
            ),
        ];
        for (name, base, local, remote, expected) in cases {
            assert_eq!(merge(&base, local, remote), expected, "{name}");
        }
    }

    #[test]
    fn app_instructions_merge_by_normalised_app() {
        let rules = |items: &[(&str, &str)]| {
            items
                .iter()
                .map(|(app, text)| AppInstruction {
                    app: (*app).into(),
                    text: (*text).into(),
                })
                .collect::<Vec<_>>()
        };
        let with = |items: &[(&str, &str)]| Fields {
            app_instructions: rules(items),
            ..Fields::default()
        };
        let merged = merge(
            &with(&[("slack", "before"), ("zed", "keep")]),
            with(&[("Slack.exe", "after"), ("kitty", "local")]),
            with(&[("slack", "before"), ("zed", "keep"), ("mail", "remote")]),
        );
        assert_eq!(
            merged.app_instructions,
            rules(&[
                ("Slack.exe", "after"),
                ("mail", "remote"),
                ("kitty", "local")
            ])
        );
    }
}
