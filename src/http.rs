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
    let config = config.effective();
    for endpoint in [&config.stt, &config.cleanup.endpoint] {
        let endpoint = endpoint.clone();
        let _ = std::thread::Builder::new()
            .name("hermes-prewarm".into())
            .spawn(move || {
                if let Ok(key) = endpoint_key(&endpoint.base_url) {
                    let _ = client()
                        .get(format!(
                            "{}/models",
                            endpoint.base_url.trim_end_matches('/')
                        ))
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
            anyhow::bail!("Weekly free limit reached. Upgrade in Hermes.");
        }
        anyhow::bail!("Hosted request failed (402)");
    }
    Ok(response.error_for_status()?)
}
