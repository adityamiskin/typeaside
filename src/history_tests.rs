use super::*;

fn entry(index: u64, text: &str) -> Dictation {
    Dictation {
        at: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
        raw: text.into(),
        cleaned: text.into(),
        app_id: None,
        audio_ms: 0,
        latency_ms: 0,
        mode: Mode::Dictate,
    }
}

#[test]
fn off_preserves_existing_entries_but_caps_oldest() {
    let mut history = (0..2005).rev().map(|i| entry(i, "text")).collect();
    assert!(prune_history(
        &mut history,
        HistoryRetention::Off,
        SystemTime::now()
    ));
    assert_eq!(history.len(), 2000);
    assert_eq!(history.iter().map(|d| d.at).min(), Some(entry(5, "").at));
    assert!(!prune_history(
        &mut history,
        HistoryRetention::Off,
        SystemTime::now()
    ));
}

#[test]
fn bounds_utf8_and_serialized_bytes_including_escaping() {
    let mut history = (0..1000)
        .map(|i| entry(i, &"\u{0001}🦀".repeat(5000)))
        .collect();
    assert!(prune_history(
        &mut history,
        HistoryRetention::Forever,
        SystemTime::now()
    ));
    assert!(
        history
            .iter()
            .all(|d| d.raw.len() + d.cleaned.len() <= 16 * 1024)
    );
    let bytes: usize = history
        .iter()
        .map(|d| serde_json::to_vec(d).unwrap().len() + 1)
        .sum();
    assert!(bytes <= 4 * 1024 * 1024);
    assert_eq!(history.last().unwrap().at, entry(999, "").at);
}

#[test]
fn retention_still_expires_old_entries() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(40 * 86400);
    let mut history = vec![entry(0, "old"), entry(39 * 86400, "recent")];
    assert!(prune_history(&mut history, HistoryRetention::Week, now));
    assert_eq!(history.len(), 1);
}

#[test]
fn audio_budget_evicts_oldest_and_oversized_files() {
    let sizes = vec![1, 100, 100, 201];
    assert_eq!(failed_evictions(&sizes, 20, 200), 4);
    assert_eq!(failed_evictions(&[100, 50, 50], 20, 100), 1);
    assert_eq!(failed_evictions(&[1; 21], 20, 200), 1);
}
