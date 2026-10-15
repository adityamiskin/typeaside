use crate::{
    config::{AppConfig, Source},
    credentials,
};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub fn hosted_url() -> String {
    std::env::var("HERMES_API_URL")
        .unwrap_or_else(|_| {
            option_env!("HERMES_API_URL")
                .unwrap_or("https://doting-rhinoceros-418.convex.site")
                .into()
        })
        .trim_end_matches('/')
        .into()
}
pub fn web_url() -> String {
    std::env::var("HERMES_WEB_URL")
        .unwrap_or_else(|_| {
            option_env!("HERMES_WEB_URL")
                .unwrap_or("https://hermes.adityamiskin98.workers.dev")
                .into()
        })
        .trim_end_matches('/')
        .into()
}
/// Spawned, not awaited: explorer.exe exits 1 even when it opens the target.
pub fn open(target: impl AsRef<std::ffi::OsStr>) -> Result<()> {
    std::process::Command::new(if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer.exe"
    } else {
        "xdg-open"
    })
    .arg(target)
    .spawn()?;
    Ok(())
}
#[derive(Clone, Deserialize)]
pub struct Account {
    pub email: String,
    pub plan: Plan,
    pub usage: Usage,
}
#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    Free,
    Pro,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub words: u64,
    pub cap: Option<u64>,
    pub resets_at: i64,
}
impl Account {
    pub fn at_limit(&self) -> bool {
        self.usage.cap.is_some_and(|cap| self.usage.words >= cap)
    }
}
pub fn me() -> Result<Option<Account>> {
    let url = hosted_url();
    let Some(token) = credentials::get_credential(&credentials::endpoint_host(&url)?)? else {
        return Ok(None);
    };
    let response = crate::http::client()
        .get(format!("{url}/v1/me"))
        .bearer_auth(token)
        .send()?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Ok(None);
    }
    Ok(Some(response.error_for_status()?.json()?))
}
pub fn sign_in(cancel: &AtomicBool) -> Result<Account> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut random = [0; 32];
    getrandom::fill(&mut random)
        .map_err(|_| anyhow::anyhow!("Could not generate sign-in state"))?;
    let state: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut url = reqwest::Url::parse(&format!("{}/desktop/connect", web_url()))?;
    url.query_pairs_mut()
        .append_pair("port", &listener.local_addr()?.port().to_string())
        .append_pair("state", &state);
    open(url.as_str())?;
    let token = receive_callback(listener, &state, Duration::from_secs(300), cancel)?;
    anyhow::ensure!(!cancel.load(Ordering::SeqCst), "Sign in cancelled");
    credentials::save_credential(&credentials::endpoint_host(&hosted_url())?, &token)?;
    me()?.context("Could not load connected account")
}
pub fn sign_out(config: &mut AppConfig) -> Result<()> {
    let hosted = credentials::endpoint_host(&hosted_url())?;
    credentials::delete_credential(&hosted)?;
    config.transcription = Source::Hosted;
    if let Ok(own) = credentials::endpoint_host(&config.stt.base_url)
        && own != hosted
        && credentials::get_credential(&own).ok().flatten().is_some()
    {
        config.transcription = Source::OwnKey;
    }
    Ok(())
}
fn receive_callback(
    listener: TcpListener,
    state: &str,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        anyhow::ensure!(!cancel.load(Ordering::SeqCst), "Sign in cancelled");
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(1)))?;
        let mut line = String::new();
        let _ = BufReader::new(&stream).take(8192).read_line(&mut line);
        let token = callback_token(&line, state);
        let (status, body) = match token {
            Some(_) => ("200 OK", CONNECTED_PAGE),
            None => ("404 Not Found", "Not found"),
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
            body.len()
        );
        if let Some(token) = token {
            return Ok(token);
        }
    }
    bail!("Sign in timed out")
}

const CONNECTED_PAGE: &str = "<!doctype html><meta charset=utf-8><meta name=referrer content=no-referrer><style>body{margin:0;min-height:100vh;display:grid;place-items:center;background:#f9f9f8;color:#16171a;font-family:Inter,system-ui,sans-serif}</style><p>Hermes is connected. You can close this tab.</p>";

fn callback_token(request_line: &str, state: &str) -> Option<String> {
    let target = request_line
        .strip_prefix("GET /callback?")?
        .split(' ')
        .next()?;
    let url = reqwest::Url::parse(&format!("http://localhost/callback?{target}")).ok()?;
    let (mut states, mut tokens) = (Vec::new(), Vec::new());
    for (key, value) in url.query_pairs() {
        match &*key {
            "state" => states.push(value),
            "token" => tokens.push(value),
            _ => {}
        }
    }
    let ([got_state], [token]) = (&states[..], &tokens[..]) else {
        return None;
    };
    let secret = token.strip_prefix("hm_")?;
    (got_state == state
        && secret.len() == 43
        && secret
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'))
    .then(|| token.to_string())
}

#[cfg(test)]
mod wrong_state_malformed_token_other_paths_valid_callback_and_timeout {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpStream,
        thread,
    };

    #[test]
    fn rejects_invalid_requests_and_keeps_waiting_until_valid_callback() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let worker = thread::spawn(move || {
            receive_callback(
                listener,
                "expected",
                Duration::from_secs(3),
                &AtomicBool::new(false),
            )
        });
        let token = format!("hm_{}", "a".repeat(43));
        for path in [
            format!("/callback?state=wrong&token={token}"),
            "/callback?state=expected&token=hm_short".into(),
            "/favicon.ico".into(),
            format!("/other?state=expected&token={token}"),
            format!("/callback?state=expected&token={token}"),
        ] {
            let mut stream = TcpStream::connect(address)?;
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
            let mut response = String::new();
            stream.read_to_string(&mut response)?;
            assert!(response.starts_with(
                if path.starts_with("/callback?state=expected&token=hm_aaa") {
                    "HTTP/1.1 200"
                } else {
                    "HTTP/1.1 404"
                }
            ));
        }
        assert_eq!(worker.join().expect("listener thread")?, token);
        Ok(())
    }

    #[test]
    fn timeout_returns_error() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        assert!(
            receive_callback(
                listener,
                "expected",
                Duration::from_millis(30),
                &AtomicBool::new(false)
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn curl_connects_to_the_sign_in_listener() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!(
            "http://{}/callback?state=curl&token=hm_{}",
            listener.local_addr()?,
            "b".repeat(43)
        );
        let worker = thread::spawn(move || {
            receive_callback(
                listener,
                "curl",
                Duration::from_secs(3),
                &AtomicBool::new(false),
            )
        });
        let response = std::process::Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--fail",
                "--max-time",
                "2",
                &url,
            ])
            .output()?;
        assert!(response.status.success());
        assert!(String::from_utf8(response.stdout)?.contains("Hermes is connected."));
        assert_eq!(
            worker.join().expect("listener thread")?,
            format!("hm_{}", "b".repeat(43))
        );
        Ok(())
    }
}
