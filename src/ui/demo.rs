use hermes::engine::{Dictation, Event, Phase};
use std::time::{Duration, Instant, SystemTime};

pub fn history() -> Vec<Dictation> {
    [
        ("Let's move the sync to Thursday at 3 and keep the agenda short: launch checklist, pricing page, and the onboarding email.", "com.slack.Slack", 0),
        ("Can you check whether the overlay steals focus on niri when the main window is closed?", "com.mitchellh.ghostty", 180),
        ("Thanks for the quick turnaround. I'll review the PR tonight and leave comments by morning.", "gmail", 660),
        ("Groceries: oat milk, eggs, spinach, and the good coffee beans from the place on 5th.", "notes", 86400),
        ("Refactor the transcribe module so the HTTP client is created once and reused across dictations.", "t3-code", 87300),
    ].into_iter().map(|(text, app, ago)| Dictation {
        at: SystemTime::now() - Duration::from_secs(ago), raw: text.into(), cleaned: text.into(),
        app_id: Some(app.into()), audio_ms: 3800, latency_ms: 640,
    }).collect()
}

pub fn spawn() -> async_channel::Receiver<Event> {
    let (tx, rx) = async_channel::unbounded();
    std::thread::spawn(move || {
        let started = Instant::now();
        let send = |event| {
            match &event {
                Event::Phase(phase) => eprintln!(
                    "hermes demo @{:.3}s: {phase:?}",
                    started.elapsed().as_secs_f32()
                ),
                Event::Done(_) => {
                    eprintln!("hermes demo @{:.3}s: Done", started.elapsed().as_secs_f32())
                }
                Event::Level(_) => {}
            }
            tx.send_blocking(event).is_ok()
        };
        std::thread::sleep(Duration::from_secs(2));
        let mut cycle = 0;
        loop {
            if !send(Event::Phase(Phase::Listening)) {
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
                let mut dictation = history().remove(cycle % 5);
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
