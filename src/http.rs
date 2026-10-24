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
    use crate::config::Source;
    let endpoints: Vec<_> = [
        (config.transcription == Source::OwnKey).then_some(&config.stt),
        (config.cleanup.source == Source::OwnKey).then_some(&config.cleanup.endpoint),
    ]
    .into_iter()
    .flatten()
    .map(|endpoint| {
        (
            format!("{}/models", endpoint.base_url.trim_end_matches('/')),
            endpoint.base_url.clone(),
        )
    })
    .collect();
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
