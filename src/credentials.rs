use anyhow::{Context, Result, anyhow, ensure};
use keyring_core::{Entry, Error};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

/// Keyring lookups can be IPC round trips (D-Bus on Linux), so keys are cached per host for the process.
fn cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn entry(host: &str) -> Result<Entry> {
    static STORE: OnceLock<Result<(), String>> = OnceLock::new();
    STORE
        .get_or_init(|| keyring::use_named_store(store_name()).map_err(|error| error.to_string()))
        .clone()
        .map_err(|error| anyhow!("failed to open {} keyring: {error}", store_name()))?;
    Entry::new("typeaside", host).with_context(|| format!("failed to open credential for {host}"))
}

pub fn get_credential(host: &str) -> Result<Option<String>> {
    if let Some(key) = cache()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(host)
    {
        return Ok(Some(key.clone()));
    }
    match entry(host)?.get_password() {
        Ok(key) => {
            cache()
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .insert(host.into(), key.clone());
            Ok(Some(key))
        }
        Err(Error::NoEntry) => Ok(None),
        Err(error) => Err(anyhow!("failed to read credential for {host}: {error}")),
    }
}

pub fn save_credential(host: &str, key: &str) -> Result<()> {
    entry(host)?
        .set_password(key)
        .with_context(|| format!("failed to save credential for {host}"))?;
    cache()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(host.into(), key.into());
    Ok(())
}

pub fn endpoint_host(base_url: &str) -> Result<String> {
    let url = reqwest::Url::parse(base_url).context("invalid endpoint URL")?;
    let host = url.host_str().context("endpoint has no host")?;
    ensure!(
        url.scheme() == "https"
            || (url.scheme() == "http" && matches!(host, "localhost" | "127.0.0.1" | "[::1]")),
        "endpoint must use HTTPS (HTTP is allowed only for loopback hosts)"
    );
    Ok(host.to_owned())
}

pub(crate) fn endpoint_key(base_url: &str) -> Result<String> {
    let host = endpoint_host(base_url)?;
    let stored = get_credential(&host);
    if let Ok(Some(key)) = &stored
        && !key.trim().is_empty()
    {
        return Ok(key.clone());
    }
    if host == "api.typeaside.app"
        && let Ok(key) = std::env::var("TYPEASIDE_API_KEY")
        && !key.trim().is_empty()
    {
        return Ok(key);
    }
    stored?;
    Err(anyhow!("no API key saved for {host}"))
}

fn store_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "secret-service"
    }
    #[cfg(target_os = "macos")]
    {
        "keychain"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
}

pub fn delete_credential(host: &str) -> Result<()> {
    match entry(host)?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => {}
        Err(error) => return Err(anyhow!("failed to delete credential for {host}: {error}")),
    }
    cache()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(host);
    Ok(())
}
