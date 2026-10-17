use std::time::{Duration, Instant, SystemTime};
use typeaside::{
    config::{AppConfig, Snippet},
    engine::{Dictation, Event, FailedRecording, Mode, Phase},
};

pub fn config() -> AppConfig {
    AppConfig {
        transcription: if std::env::var("TYPEASIDE_DEMO_MODEL").as_deref() == Ok("installed") {
            typeaside::config::Source::Local
        } else if account().is_some() {
            typeaside::config::Source::Hosted
        } else {
            typeaside::config::Source::OwnKey
        },
        onboarded: std::env::var("TYPEASIDE_DEMO_PAGE").as_deref() != Ok("onboarding"),
        theme: match std::env::var("TYPEASIDE_DEMO_THEME").as_deref() {
            Ok("light") => typeaside::config::Theme::Light,
            Ok("dark") => typeaside::config::Theme::Dark,
            _ => typeaside::config::Theme::System,
        },
        dictionary: [
            "TypeAside",
            "niri",
            "GPUI",
            "Priya Raman",
            "Zapsight",
            "Tauri",
            "Convex",
            "Groq",
            "CachyOS",
            "Kubernetes",
            "PSS",
            "evdev",
        ]
        .map(str::to_owned)
        .to_vec(),
        snippets: [
            ("my calendar link", "https://cal.com/aditya/30min"),
            (
                "intro blurb",
                "I'm Aditya, I build TypeAside, a voice dictation app for Linux.",
            ),
            ("home address", "221B Baker Street, London"),
        ]
        .into_iter()
        .map(|(trigger, text)| Snippet {
            trigger: trigger.into(),
            text: text.into(),
        })
        .collect(),
        ..Default::default()
    }
}
pub fn history() -> Vec<Dictation> {
    use chrono::{Local, TimeZone};
    let today = Local::now().date_naive();
    [
        ("Let's push the release to Thursday so QA has a full day with the new build. I'll update the ticket.", "let's push the release to thursday so qa has a full day with the new build i'll update the ticket", "slack", 0, 14, 32, 412),
        ("Can you send me the deck before the 3 pm call?", "um can you send me the uh deck before the 2 actually 3 pm call", "slack", 0, 14, 5, 388),
        ("Hi Priya, thanks for the quick turnaround. The numbers look right to me, so feel free to send it to the client.", "hi priya thanks for the quick turnaround the numbers look right to me so feel free to send it to the client", "thunderbird", 0, 11, 48, 440),
        ("fix the off by one in the pagination cursor and add a test for the empty page", "fix the off by one in the pagination cursor and add a test for the empty page", "Alacritty", 1, 22, 10, 412),
    ].into_iter().map(|(cleaned,raw,app,ago,h,m,latency_ms)| {
        let date = today - chrono::Duration::days(ago);
        let at = Local.from_local_datetime(&date.and_hms_opt(h,m,0).unwrap()).earliest().unwrap().into();
        Dictation {at, raw:raw.into(), cleaned:cleaned.into(), app_id:Some(app.into()), audio_ms:4200, latency_ms, mode:Mode::Dictate}
    }).collect()
}

pub fn model() -> typeaside::local::Status {
    use typeaside::local::Status;
    match std::env::var("TYPEASIDE_DEMO_MODEL").as_deref() {
        Ok("downloading") => Status::Downloading(42),
        Ok("installed") => Status::Installed,
        Ok("failed") => Status::Failed,
        _ => Status::NotInstalled,
    }
}

pub fn spawn() -> async_channel::Receiver<Event> {
    let (tx, rx) = async_channel::unbounded();
    std::thread::spawn(move || {
        if std::env::var("TYPEASIDE_DEMO_PAGE").as_deref() == Ok("onboarding")
            || std::env::var_os("TYPEASIDE_DEMO_MODEL").is_some()
            || std::env::var_os("TYPEASIDE_DEMO_UPDATE").is_some()
        {
            // Keep interactive fixtures static and avoid creating an overlay window.
            std::thread::park();
            return;
        }
        let started = Instant::now();
        let send = |event| {
            match &event {
                Event::Phase(phase) => eprintln!(
                    "typeaside demo @{:.3}s: {phase:?}",
                    started.elapsed().as_secs_f32()
                ),
                Event::Done(_) => {
                    eprintln!(
                        "typeaside demo @{:.3}s: Done",
                        started.elapsed().as_secs_f32()
                    )
                }
                Event::Copied(_) => eprintln!("typeaside demo: Copied"),
                Event::Reply { .. }
                | Event::Open(_)
                | Event::DismissReply
                | Event::FileStarted(_)
                | Event::Level(_)
                | Event::Failed(_)
                | Event::Edited { .. } => {}
            }
            tx.send_blocking(event).is_ok()
        };
        if !send(Event::Failed(Some(FailedRecording {
            at: SystemTime::now(),
            audio_ms: 12_000,
            retrying: false,
        }))) {
            return;
        }
        if [
            "TYPEASIDE_DEMO_HISTORY",
            "TYPEASIDE_DEMO_MENU",
            "TYPEASIDE_DEMO_TOAST",
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
        {
            std::thread::park();
            return;
        }
        std::thread::sleep(Duration::from_secs(2));
        if let Ok(visual) = std::env::var("TYPEASIDE_DEMO_OVERLAY") {
            let event = match visual.as_str() {
                "command" => Event::Phase(Phase::Listening(Mode::Command)),
                "translate" => Event::Phase(Phase::Listening(Mode::Translate)),
                "processing" => Event::Phase(Phase::Transcribing),
                "success" => Event::Done(history().remove(0)),
                "copied" => Event::Copied(
                    std::env::var("TYPEASIDE_DEMO_TEXT")
                        .unwrap_or_else(|_| "See you Thursday".into()),
                ),
                "error" => Event::Phase(Phase::Error("No speech detected".into())),
                "reply" => Event::Reply {
                    prompt: "what does this mean".into(),
                    text: "Mitochondria are the parts of a cell that turn food into ATP, the energy the rest of the cell runs on. That is why they are called its powerhouse.".into(),
                },
                _ => Event::Phase(Phase::Listening(Mode::Dictate)),
            };
            if !send(event) {
                return;
            }
            loop {
                if !send(Event::Level(0.7)) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(33));
            }
        }
        let mut cycle = 0;
        loop {
            if !send(Event::Phase(Phase::Listening(if cycle % 2 == 0 {
                Mode::Dictate
            } else {
                Mode::Command
            }))) {
                break;
            }
            for tick in 0..91 {
                let t = tick as f32;
                let level =
                    (0.5 + 0.22 * (t * 0.37).sin() + 0.17 * (t * 1.23).sin()).clamp(0.08, 1.);
                if !send(Event::Level(level)) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(33));
            }
            if cycle % 4 == 3 {
                if !send(Event::Phase(Phase::Error("No speech detected".into()))) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1900));
            } else {
                if !send(Event::Phase(Phase::Transcribing)) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(600));
                if !send(Event::Phase(Phase::Cleaning)) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(400));
                let mut dictation = history().remove(cycle % 4);
                dictation.at = SystemTime::now();
                if !send(Event::Done(dictation)) {
                    break;
                }
            }
            if !send(Event::Phase(Phase::Idle)) {
                break;
            }
            std::thread::sleep(Duration::from_secs(3));
            cycle += 1;
        }
    });
    rx
}

pub fn account() -> Option<typeaside::account::Account> {
    use typeaside::account::{Account, Plan, Usage};
    let value = std::env::var("TYPEASIDE_DEMO_ACCOUNT").ok()?;
    if !["free", "pro", "limit", "signed-in"].contains(&value.as_str()) {
        return None;
    }
    Some(Account {
        email: "aditya@example.com".into(),
        plan: if value == "pro" {
            Plan::Pro
        } else {
            Plan::Free
        },
        usage: Usage {
            words: if value == "limit" { 2000 } else { 1240 },
            cap: if value == "pro" { None } else { Some(2000) },
            resets_at: (chrono::Local::now() + chrono::Duration::days(3)).timestamp_millis(),
        },
    })
}

pub fn intro_time() -> Option<f32> {
    std::env::var("TYPEASIDE_DEMO_INTRO")
        .ok()?
        .parse::<f32>()
        .ok()
        .filter(|t| t.is_finite() && *t >= 0.)
}

pub fn update() -> (Option<typeaside::update::Release>, super::update::Toast) {
    use super::update::Toast;
    let Ok(value) = std::env::var("TYPEASIDE_DEMO_UPDATE") else {
        return (None, Toast::Available);
    };
    let state = match value.as_str() {
        "available" | "managed" => Toast::Available,
        "downloading" => Toast::Downloading(42),
        "failed" => Toast::Failed("Release checksum mismatch".into()),
        _ => return (None, Toast::Available),
    };
    (
        Some(typeaside::update::Release {
            version: "0.3.0".into(),
            installer: (value != "managed").then(|| typeaside::update::Installer {
                file: "TypeAside.AppImage",
                sha256: "a".repeat(64),
            }),
        }),
        state,
    )
}
