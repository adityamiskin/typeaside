use atspi::zbus::{self, proxy::CacheProperties};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    future::Future,
    process::{Command, Output, Stdio},
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_millis(400);
const DUCKED: f64 = 0.3;

#[derive(Debug, Clone, PartialEq)]
struct Playback {
    status: String,
    track: Option<String>,
}

struct Player {
    owner: String,
    track: Option<String>,
}

/// MPRIS cannot tell us who paused a player, so a player still paused on the
/// track we paused is the best evidence the user left it alone.
fn resumable<'a>(paused: &'a [Player], now: &HashMap<String, Playback>) -> Vec<&'a str> {
    paused
        .iter()
        .filter(|player| {
            now.get(&player.owner).is_some_and(|playback| {
                playback.status == "Paused" && playback.track == player.track
            })
        })
        .map(|player| player.owner.as_str())
        .collect()
}

async fn bounded<T>(future: impl Future<Output = zbus::Result<T>>) -> anyhow::Result<T> {
    Ok(tokio::time::timeout(TIMEOUT, future).await??)
}

async fn proxy<'a>(
    connection: &'a zbus::Connection,
    owner: &'a str,
) -> anyhow::Result<zbus::Proxy<'a>> {
    bounded(
        zbus::proxy::Builder::<zbus::Proxy>::new(connection)
            .destination(owner)?
            .path("/org/mpris/MediaPlayer2")?
            .interface("org.mpris.MediaPlayer2.Player")?
            .cache_properties(CacheProperties::No)
            .build(),
    )
    .await
}

async fn playback(proxy: &zbus::Proxy<'_>) -> anyhow::Result<Playback> {
    let status = bounded(proxy.get_property("PlaybackStatus")).await?;
    let metadata: Option<HashMap<String, zbus::zvariant::OwnedValue>> =
        bounded(proxy.get_property("Metadata")).await.ok();
    let track = metadata.as_ref().and_then(|metadata| {
        let id = metadata.get("mpris:trackid")?;
        <&zbus::zvariant::ObjectPath<'_>>::try_from(id)
            .ok()
            .map(ToString::to_string)
    });
    Ok(Playback { status, track })
}

pub(super) struct Paused {
    runtime: tokio::runtime::Runtime,
    connection: zbus::Connection,
    players: Vec<Player>,
}

impl Paused {
    pub(super) fn start(stopped: impl Fn() -> bool) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (connection, players) = runtime.block_on(async {
            let connection = bounded(
                zbus::connection::Builder::session()?
                    .method_timeout(TIMEOUT)
                    .build(),
            )
            .await?;
            let bus = bounded(zbus::fdo::DBusProxy::new(&connection)).await?;
            let names = tokio::time::timeout(TIMEOUT, bus.list_names()).await??;
            let mut players: Vec<Player> = Vec::new();
            for name in names
                .into_iter()
                .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
            {
                if stopped() {
                    break;
                }
                // Address the unique owner so a restarted player cannot inherit a resume.
                let Ok(Ok(owner)) =
                    tokio::time::timeout(TIMEOUT, bus.get_name_owner(name.into())).await
                else {
                    continue;
                };
                let owner = owner.to_string();
                if players.iter().any(|player| player.owner == owner) {
                    continue;
                }
                let Ok(proxy) = proxy(&connection, &owner).await else {
                    continue;
                };
                let Ok(playback) = playback(&proxy).await else {
                    continue;
                };
                if playback.status != "Playing" {
                    continue;
                }
                if bounded(proxy.call::<_, _, ()>("Pause", &())).await.is_ok() {
                    players.push(Player {
                        owner,
                        track: playback.track,
                    });
                }
            }
            Ok::<_, anyhow::Error>((connection, players))
        })?;
        Ok(Self {
            runtime,
            connection,
            players,
        })
    }
}

impl Drop for Paused {
    fn drop(&mut self) {
        self.runtime.block_on(async {
            let mut now = HashMap::new();
            for player in &self.players {
                if let Ok(proxy) = proxy(&self.connection, &player.owner).await
                    && let Ok(playback) = playback(&proxy).await
                {
                    now.insert(player.owner.clone(), playback);
                }
            }
            for owner in resumable(&self.players, &now) {
                if let Ok(proxy) = proxy(&self.connection, owner).await {
                    let _ = bounded(proxy.call::<_, _, ()>("Play", &())).await;
                }
            }
        });
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Backend {
    Wpctl,
    Pactl,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Volume {
    backend: Backend,
    sink: String,
    original: Vec<String>,
}

fn command(program: &str, args: &[&str]) -> Option<Output> {
    Command::new("timeout")
        .arg("0.5s")
        .arg(program)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()
}

fn stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = command(program, args)?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())?
}

fn marker() -> Option<std::path::PathBuf> {
    Some(
        crate::paths::AppPaths::discover()
            .ok()?
            .data_dir
            .join("media-volume.json"),
    )
}

impl Volume {
    fn wpctl() -> Option<(Self, Vec<String>)> {
        let text = stdout("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
        if text.contains("MUTED") {
            return None;
        }
        let original = text.split_whitespace().nth(1)?.to_owned();
        let level = original.parse::<f64>().ok()?;
        if !level.is_finite() || level < 0. {
            return None;
        }
        // wpctl only addresses nodes by id, which is stable while PipeWire runs.
        let sink = stdout("wpctl", &["inspect", "@DEFAULT_AUDIO_SINK@"])?
            .strip_prefix("id ")?
            .split(',')
            .next()?
            .to_owned();
        let volume = Self {
            backend: Backend::Wpctl,
            sink,
            original: vec![original],
        };
        Some((volume, vec![format!("{:.3}", level * DUCKED)]))
    }

    fn pactl() -> Option<(Self, Vec<String>)> {
        let sink = stdout("pactl", &["get-default-sink"])?.trim().to_owned();
        if stdout("pactl", &["get-sink-mute", &sink])?.contains("yes") {
            return None;
        }
        let original: Vec<f64> = stdout("pactl", &["get-sink-volume", &sink])?
            .split_whitespace()
            .filter_map(|word| word.strip_suffix('%')?.parse().ok())
            .collect();
        if original.is_empty() {
            return None;
        }
        let lowered = original
            .iter()
            .map(|v| format!("{}%", (v * DUCKED).round()))
            .collect();
        let volume = Self {
            backend: Backend::Pactl,
            sink,
            original: original.iter().map(|v| format!("{v}%")).collect(),
        };
        Some((volume, lowered))
    }

    pub(super) fn duck() -> Option<Self> {
        let path = marker()?;
        if path.exists() {
            return None;
        }
        let (volume, lowered) = Self::wpctl().or_else(Self::pactl)?;
        std::fs::create_dir_all(path.parent()?).ok()?;
        std::fs::write(&path, serde_json::to_vec(&volume).ok()?).ok()?;
        volume.set(&lowered);
        Some(volume)
    }

    fn set(&self, values: &[String]) -> bool {
        let (program, operation) = match self.backend {
            Backend::Wpctl => ("wpctl", "set-volume"),
            Backend::Pactl => ("pactl", "set-sink-volume"),
        };
        let mut args = vec![operation, &self.sink];
        args.extend(values.iter().map(String::as_str));
        command(program, &args).is_some_and(|output| output.status.success())
    }
}

impl Drop for Volume {
    fn drop(&mut self) {
        if self.set(&self.original)
            && let Some(path) = marker()
        {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub(super) fn recover() {
    let Some(path) = marker() else { return };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    match serde_json::from_slice::<Volume>(&bytes) {
        Ok(volume) => drop(volume),
        Err(error) => {
            eprintln!("typeaside: discarding unreadable media volume marker: {error}");
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(owner: &str, track: Option<&str>) -> Player {
        Player {
            owner: owner.into(),
            track: track.map(Into::into),
        }
    }

    fn playback(status: &str, track: Option<&str>) -> Playback {
        Playback {
            status: status.into(),
            track: track.map(Into::into),
        }
    }

    #[test]
    fn resumes_only_players_still_paused_where_we_left_them() {
        let paused = [
            player(":1.1", Some("/track/a")),
            player(":1.2", Some("/track/b")),
            player(":1.3", None),
            player(":1.4", Some("/track/d")),
            player(":1.5", Some("/track/e")),
        ];
        let now = HashMap::from([
            (":1.1".to_owned(), playback("Paused", Some("/track/a"))),
            // User resumed it themselves.
            (":1.2".to_owned(), playback("Playing", Some("/track/b"))),
            (":1.3".to_owned(), playback("Paused", None)),
            // User skipped to another track and paused.
            (":1.4".to_owned(), playback("Paused", Some("/track/x"))),
            // :1.5 quit while dictating.
        ]);
        assert_eq!(resumable(&paused, &now), [":1.1", ":1.3"]);
    }
}
