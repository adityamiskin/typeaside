use crate::{
    config::{AppConfig, Source},
    credentials,
};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub fn hosted_url() -> String {
    std::env::var("TYPEASIDE_API_URL")
        .unwrap_or_else(|_| {
            option_env!("TYPEASIDE_API_URL")
                .unwrap_or("https://api.typeaside.app")
                .into()
        })
        .trim_end_matches('/')
        .into()
}
pub fn web_url() -> String {
    std::env::var("TYPEASIDE_WEB_URL")
        .unwrap_or_else(|_| {
            option_env!("TYPEASIDE_WEB_URL")
                .unwrap_or("https://typeaside.app")
                .into()
        })
        .trim_end_matches('/')
        .into()
}
#[cfg(not(target_os = "windows"))]
pub fn open(target: impl AsRef<std::ffi::OsStr>) -> Result<()> {
    std::process::Command::new(if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    })
    .arg(target)
    .spawn()?;
    Ok(())
}
// explorer.exe opens Documents instead of some URLs, so let the shell pick the handler.
#[cfg(target_os = "windows")]
pub fn open(target: impl AsRef<std::ffi::OsStr>) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::{PCWSTR, w},
    };
    let target: Vec<u16> = target.as_ref().encode_wide().chain([0]).collect();
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(target.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    // Values above 32 mean success.
    if result.0 as isize <= 32 {
        bail!(
            "cannot open {}",
            String::from_utf16_lossy(&target[..target.len() - 1])
        );
    }
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
    config.settings_sync = None;
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
        let token = read_callback_body(&stream).and_then(|body| callback_token(&body, state));
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

const FORM_TYPE: &str = "application/x-www-form-urlencoded";
const MAX_REQUEST_BYTES: u64 = 8192;
const MAX_BODY_BYTES: usize = 1024;
const CONNECTED_PAGE: &str = "<!doctype html><meta charset=utf-8><meta name=referrer content=no-referrer><style>body{margin:0;min-height:100vh;display:grid;place-items:center;background:#f9f9f8;color:#16171a;font-family:Inter,system-ui,sans-serif}</style><p>TypeAside is connected. You can close this tab.</p>";

// Only a form POST to /callback carries the token, so it never lands in browser history.
fn read_callback_body(stream: &TcpStream) -> Option<String> {
    let mut reader = BufReader::new(Read::take(stream, MAX_REQUEST_BYTES));
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    if line.trim_end() != "POST /callback HTTP/1.1" {
        return None;
    }
    let (mut length, mut form) = (None, false);
    loop {
        line.clear();
        reader.read_line(&mut line).ok()?;
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':')?;
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "content-length" => length = value.parse::<usize>().ok(),
            "content-type" => {
                form = value
                    .split(';')
                    .next()
                    .is_some_and(|kind| kind.trim().eq_ignore_ascii_case(FORM_TYPE));
            }
            _ => {}
        }
    }
    let length = length.filter(|&length| form && length <= MAX_BODY_BYTES)?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

fn callback_token(body: &str, state: &str) -> Option<String> {
    let (mut states, mut tokens) = (Vec::new(), Vec::new());
    for (key, value) in reqwest::Url::parse(&format!("http://localhost/?{body}"))
        .ok()?
        .query_pairs()
    {
        match &*key {
            "state" => states.push(value.into_owned()),
            "token" => tokens.push(value.into_owned()),
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
    use std::{io::Read, net::TcpStream, thread};

    fn request(method: &str, path: &str, content_type: &str, body: &str) -> String {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    }

    fn post(address: std::net::SocketAddr, request: &str) -> Result<String> {
        let mut stream = TcpStream::connect(address)?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(request.as_bytes())?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

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
        let valid_body = format!("state=expected&token={token}");
        let form = "application/x-www-form-urlencoded";
        for (request, ok) in [
            (
                request(
                    "POST",
                    "/callback",
                    form,
                    &format!("state=wrong&token={token}"),
                ),
                false,
            ),
            (
                request("POST", "/callback", form, "state=expected&token=hm_short"),
                false,
            ),
            (
                request(
                    "POST",
                    "/callback",
                    form,
                    &format!("state=expected&state=expected&token={token}"),
                ),
                false,
            ),
            (
                request(
                    "POST",
                    "/callback",
                    form,
                    &format!("state=expected&token={token}&token={token}"),
                ),
                false,
            ),
            (
                request("POST", "/callback", "text/plain", &valid_body),
                false,
            ),
            (request("POST", "/other", form, &valid_body), false),
            (
                request("POST", "/callback?state=expected", form, &valid_body),
                false,
            ),
            (request("GET", "/callback", form, &valid_body), false),
            (
                format!("GET /callback?{valid_body} HTTP/1.1\r\nHost: localhost\r\n\r\n"),
                false,
            ),
            (request("POST", "/callback", form, &valid_body), true),
        ] {
            let response = post(address, &request)?;
            assert!(
                response.starts_with(if ok { "HTTP/1.1 200" } else { "HTTP/1.1 404" }),
                "{request}"
            );
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
        let url = format!("http://{}/callback", listener.local_addr()?);
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
                "--data-urlencode",
                "state=curl",
                "--data-urlencode",
                &format!("token=hm_{}", "b".repeat(43)),
                &url,
            ])
            .output()?;
        assert!(response.status.success());
        assert!(String::from_utf8(response.stdout)?.contains("TypeAside is connected."));
        assert_eq!(
            worker.join().expect("listener thread")?,
            format!("hm_{}", "b".repeat(43))
        );
        Ok(())
    }
}
