use crate::{config::AppConfig, credentials::endpoint_key};
use reqwest::blocking::Client;
use std::{sync::OnceLock, time::Duration};

pub(crate) fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("HTTP client initialization")
    })
}

pub(crate) fn prewarm(config: &AppConfig) {
    let endpoints = match config.transcription {
        crate::config::Source::Local => return,
        crate::config::Source::Hosted => {
            let url = crate::account::hosted_url();
            vec![(format!("{url}/v1/me"), url)]
        }
        crate::config::Source::OwnKey => [&config.stt, &config.cleanup.endpoint]
            .into_iter()
            .map(|endpoint| {
                (
                    format!("{}/models", endpoint.base_url.trim_end_matches('/')),
                    endpoint.base_url.clone(),
                )
            })
            .collect(),
    };
    for (url, base_url) in endpoints {
        let _ = std::thread::Builder::new()
            .name("typeaside-prewarm".into())
            .spawn(move || {
                if let Ok(key) = endpoint_key(&base_url) {
                    let _ = client()
                        .get(url)
                        .bearer_auth(key)
                        .timeout(Duration::from_secs(2))
                        .send()
                        .and_then(|response| response.bytes());
                }
            });
    }
}

pub(crate) fn checked(
    response: reqwest::blocking::Response,
) -> anyhow::Result<reqwest::blocking::Response> {
    if response.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
        let body: serde_json::Value = response.json()?;
        if body["error"]["type"] == "limit" {
            anyhow::bail!("Weekly free limit reached. Upgrade in TypeAside.");
        }
        anyhow::bail!("Hosted request failed (402)");
    }
    Ok(response.error_for_status()?)
}

/// Model IDs from an OpenAI-compatible `GET /models`, split into (speech, chat).
pub fn models(base_url: &str) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    #[derive(serde::Deserialize)]
    struct Model {
        id: String,
    }
    #[derive(serde::Deserialize)]
    struct Models {
        data: Vec<Model>,
    }
    let models: Models = client()
        .get(format!("{}/models", base_url.trim_end_matches('/')))
        .bearer_auth(endpoint_key(base_url)?)
        .timeout(Duration::from_secs(5))
        .send()?
        .error_for_status()?
        .json()?;
    let mut ids: Vec<_> = models.data.into_iter().map(|m| m.id).collect();
    ids.sort();
    let (speech, other): (Vec<_>, Vec<_>) = ids
        .into_iter()
        .partition(|id| id.contains("whisper") || id.contains("transcribe"));
    let chat = other
        .into_iter()
        .filter(|id| {
            !["tts", "orpheus", "embed", "guard", "moderation", "dall-e"]
                .iter()
                .any(|n| id.contains(n))
        })
        .collect();
    Ok((speech, chat))
}
