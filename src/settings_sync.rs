use crate::config::{AppConfig, Snippet, Styles};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncState {
    pub updated_at: u64,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fields {
    pub dictionary: Vec<String>,
    pub snippets: Vec<Snippet>,
    pub styles: Styles,
}

impl Fields {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            dictionary: config.dictionary.clone(),
            snippets: config.snippets.clone(),
            styles: config.styles,
        }
    }

    pub fn hash(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("settings serialize"))
        )
    }

    pub fn apply(self, config: &mut AppConfig) {
        config.dictionary = self.dictionary;
        config.snippets = self.snippets;
        config.styles = self.styles;
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

pub fn merge(local: Fields, mut remote: Fields) -> Fields {
    let mut words = std::collections::HashSet::new();
    remote
        .dictionary
        .retain(|word| words.insert(word.to_lowercase()));
    for word in local.dictionary {
        if words.insert(word.to_lowercase()) {
            remote.dictionary.push(word);
        }
    }
    let mut triggers = std::collections::HashSet::new();
    remote
        .snippets
        .retain(|snippet| triggers.insert(snippet.trigger.clone()));
    for snippet in local.snippets {
        if triggers.insert(snippet.trigger.clone()) {
            remote.snippets.push(snippet);
        }
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
    let dirty = config
        .settings_sync
        .as_ref()
        .is_none_or(|state| state.hash != local.hash());
    if let Some(remote) = &remote
        && !dirty
    {
        let known = config
            .settings_sync
            .as_ref()
            .expect("clean settings have sync state");
        return Ok(if remote.updated_at > known.updated_at {
            remote.clone()
        } else {
            Settings {
                fields: local,
                updated_at: known.updated_at,
            }
        });
    }
    let fields = if config.settings_sync.is_none() {
        remote.map_or(local.clone(), |remote| merge(local, remote.fields))
    } else {
        local
    };
    let updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis()
        .try_into()?;
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
    fn merge_preserves_words_and_remote_conflicts() {
        let fields = |words: &[&str], snippets: &[(&str, &str)], styles| Fields {
            dictionary: words.iter().map(|word| (*word).into()).collect(),
            snippets: snippets
                .iter()
                .map(|(trigger, text)| Snippet {
                    trigger: (*trigger).into(),
                    text: (*text).into(),
                })
                .collect(),
            styles,
        };
        let local = fields(
            &["HELLO", "Rust", "rust", "世界"],
            &[("sig", "local"), ("new", "retained")],
            Styles::default(),
        );
        let remote_styles = Styles {
            work: crate::config::Tone::Formal,
            ..Styles::default()
        };
        let remote = fields(
            &["Hello", "hello", "Convex"],
            &[("sig", "remote"), ("sig", "duplicate")],
            remote_styles,
        );
        let merged = merge(local, remote);
        assert_eq!(merged.dictionary, ["Hello", "Convex", "Rust", "世界"]);
        assert_eq!(
            merged
                .snippets
                .iter()
                .map(|s| (s.trigger.as_str(), s.text.as_str()))
                .collect::<Vec<_>>(),
            [("sig", "remote"), ("new", "retained")]
        );
        assert_eq!(merged.styles, remote_styles);
        assert_eq!(
            merge(merged.clone(), fields(&[], &[], Styles::default())).dictionary,
            merged.dictionary
        );
    }
}
