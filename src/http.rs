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
