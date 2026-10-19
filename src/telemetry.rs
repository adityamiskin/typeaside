use crate::{
    config::{AppConfig, Source},
    engine::{Dictation, Mode},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

const DAY_MS: u64 = 86_400_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Day {
    day: u64,
    dictations: u64,
    words: u64,
    commands: u64,
    latency_ms: u64,
}

fn aggregate(history: &[Dictation], now: u64) -> Vec<Day> {
    let today = now / DAY_MS * DAY_MS;
    let oldest = today.saturating_sub(6 * DAY_MS);
    let mut buckets = BTreeMap::<u64, Vec<&Dictation>>::new();
    for entry in history {
        let Ok(at) = entry.at.duration_since(UNIX_EPOCH) else {
            continue;
        };
        let at = at.as_millis() as u64;
        let day = at / DAY_MS * DAY_MS;
        if day >= oldest && at <= now {
            buckets.entry(day).or_default().push(entry);
        }
    }
    buckets
        .into_iter()
        .map(|(day, entries)| {
            let mut latencies: Vec<_> = entries
                .iter()
                .map(|entry| u64::from(entry.latency_ms))
                .collect();
            latencies.sort_unstable();
            let middle = latencies.len() / 2;
            Day {
                day,
                dictations: entries
                    .iter()
                    .filter(|entry| entry.mode == Mode::Dictate)
                    .count() as u64,
                commands: entries
                    .iter()
                    .filter(|entry| entry.mode == Mode::Command)
                    .count() as u64,
                words: entries
                    .iter()
                    .map(|entry| entry.cleaned.split_whitespace().count() as u64)
                    .sum(),
                latency_ms: (latencies[middle] + latencies[(latencies.len() - 1) / 2]) / 2,
            }
        })
        .collect()
}

pub fn send(config: &AppConfig, history: &[Dictation]) {
    if !config.share_usage {
        return;
    }
    let Some(install_id) = &config.install_id else {
        return;
    };
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return;
    };
    let days = aggregate(history, now.as_millis() as u64);
    if days.is_empty() {
        return;
    }
    let _ = crate::http::client()
        .post(format!("{}/v1/ping", crate::account::hosted_url()))
        .json(&serde_json::json!({
            "installId": install_id,
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "source": match config.transcription { Source::Hosted => "hosted", Source::OwnKey => "own", Source::Local => "local" },
            "days": days,
        }))
        .send();
}
