//! Local-backend verification only. Never accesses credentials, capture or clipboard.
use anyhow::{Result, ensure};
use typeaside::{
    config::{AppConfig, Snippet, Tone},
    paths::AppPaths,
    settings_sync::{self, Fields, Settings, SyncState},
};

fn sync(config: &mut AppConfig, label: &str) -> Result<()> {
    let result = settings_sync::synchronize(
        config,
        &std::env::var("TYPEASIDE_API_URL")?,
        &std::env::var("TYPEASIDE_SYNC_TEST_TOKEN")?,
    )?;
    println!("{label}: {}", serde_json::to_string(&result)?);
    config.settings_sync = Some(SyncState {
        updated_at: result.updated_at,
        hash: result.fields.hash(),
    });
    result.fields.apply(config);
    let root = std::path::PathBuf::from(std::env::var("TYPEASIDE_SYNC_ARTIFACTS")?);
    let paths = AppPaths {
        config_dir: root.clone(),
        config_file: root.join(format!("{label}.json")),
        data_dir: root.clone(),
        history_file: root.join("history.jsonl"),
        failed_file: root.join("failed.wav"),
    };
    config.save(&paths)
}

fn main() -> Result<()> {
    let mut a = AppConfig {
        dictionary: vec!["TypeAside".into()],
        snippets: vec![Snippet {
            trigger: "sig".into(),
            text: "remote".into(),
        }],
        ..AppConfig::default()
    };
    a.styles.work = Tone::Formal;
    sync(&mut a, "a-initial")?;
    let mut b = AppConfig {
        dictionary: vec!["typeaside".into(), "Bun".into()],
        snippets: vec![
            Snippet {
                trigger: "sig".into(),
                text: "local".into(),
            },
            Snippet {
                trigger: "new".into(),
                text: "preserved".into(),
            },
        ],
        ..AppConfig::default()
    };
    sync(&mut b, "b-first-union")?;
    ensure!(
        b.dictionary == ["TypeAside", "Bun"]
            && b.snippets[0].text == "remote"
            && b.snippets.len() == 2
            && b.styles.work == Tone::Formal
    );
    sync(&mut a, "a-pull-union")?;
    a.dictionary.push("Convex".into());
    std::thread::sleep(std::time::Duration::from_millis(5));
    sync(&mut a, "a-edit")?;
    sync(&mut b, "b-pull")?;
    ensure!(a.dictionary == b.dictionary && b.dictionary.contains(&"Convex".into()));
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/v1/settings", std::env::var("TYPEASIDE_API_URL")?);
    let token = std::env::var("TYPEASIDE_SYNC_TEST_TOKEN")?;
    let stale = Settings {
        fields: Fields::from_config(&AppConfig::default()),
        updated_at: 1,
    };
    let response = client.put(&url).bearer_auth(&token).json(&stale).send()?;
    let body: serde_json::Value = response.error_for_status()?.json()?;
    println!("stale-put: {body}");
    ensure!(body["settings"]["dictionary"] == serde_json::to_value(&b.dictionary)?);
    b.dictionary.push("stale-local".into());
    // A future timestamp simulates a competing device winning while this device pushes.
    let mut newer = Settings {
        fields: Fields::from_config(&a),
        updated_at: a.settings_sync.as_ref().unwrap().updated_at + 60_000,
    };
    newer.fields.dictionary.push("Winner".into());
    client
        .put(&url)
        .bearer_auth(&token)
        .json(&newer)
        .send()?
        .error_for_status()?;
    sync(&mut b, "b-stale-converged")?;
    ensure!(b.dictionary == newer.fields.dictionary);
    for (name, payload) in [
        ("body-oversize", "x".repeat(256 * 1024 + 1)),
        (
            "word-oversize",
            serde_json::to_string(&Settings {
                fields: Fields {
                    dictionary: vec!["x".repeat(101)],
                    ..newer.fields.clone()
                },
                updated_at: newer.updated_at + 1,
            })?,
        ),
    ] {
        let response = client.put(&url).bearer_auth(&token).body(payload).send()?;
        let status = response.status();
        println!("{name}: {status} {}", response.text()?);
        ensure!(status.as_u16() == 400);
    }
    println!(
        "PASS: two-device pull, first union, stale rejection and convergence, size validation"
    );
    Ok(())
}
