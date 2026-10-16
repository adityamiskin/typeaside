use anyhow::{Context, Result, anyhow};
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
    if let Some(key) = cache().lock().expect("credential cache").get(host) {
        return Ok(Some(key.clone()));
    }
    match entry(host)?.get_password() {
        Ok(key) => {
            cache()
                .lock()
                .expect("credential cache")
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
        .expect("credential cache")
        .insert(host.into(), key.into());
    Ok(())
}

pub fn endpoint_host(base_url: &str) -> Result<String> {
    reqwest::Url::parse(base_url)
        .context("invalid endpoint URL")?
        .host_str()
        .map(str::to_owned)
        .context("endpoint has no host")
}

pub(crate) fn endpoint_key(base_url: &str) -> Result<String> {
    let host = endpoint_host(base_url)?;
    let stored = get_credential(&host);
    if let Ok(Some(key)) = &stored
        && !key.trim().is_empty()
    {
        return Ok(key.clone());
    }
    if let Ok(key) = std::env::var("TYPEASIDE_API_KEY")
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
    cache().lock().expect("credential cache").remove(host);
    Ok(())
}
