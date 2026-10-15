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
            "source": if config.transcription == Source::Hosted { "hosted" } else { "own" },
            "days": days,
        }))
        .send();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Mode;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn buckets_utc_days_and_counts_modes_with_median_latency() {
        let today = 10 * DAY_MS;
        let entry = |at, latency_ms, mode| Dictation {
            at: UNIX_EPOCH + Duration::from_millis(at),
            raw: String::new(),
            cleaned: "two words".into(),
            app_id: None,
            audio_ms: 0,
            latency_ms,
            mode,
        };
        let history = [
            entry(today - 1, 100, Mode::Dictate),
            entry(today - 2, 300, Mode::Command),
            entry(today, 90, Mode::Dictate),
            entry(today + 1, 10, Mode::Dictate),
            entry(today + 2, 50, Mode::Command),
            entry(today - 7 * DAY_MS, 999, Mode::Dictate),
            entry(today + DAY_MS, 999, Mode::Command),
        ];
        let days = aggregate(&history, today + 3);
        assert_eq!(days.len(), 2);
        assert_eq!(
            (
                days[0].day,
                days[0].dictations,
                days[0].commands,
                days[0].words,
                days[0].latency_ms
            ),
            (today - DAY_MS, 1, 1, 4, 200)
        );
        assert_eq!(
            (
                days[1].day,
                days[1].dictations,
                days[1].commands,
                days[1].words,
                days[1].latency_ms
            ),
            (today, 2, 1, 6, 50)
        );
    }
}
