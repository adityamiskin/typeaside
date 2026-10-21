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
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub email: Option<String>,
    pub plan: Plan,
    pub current_period_end: Option<i64>,
    pub billing_issue: bool,
    pub usage: Usage,
}
#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    Free,
    Pro,
}
#[derive(Debug, Clone, Deserialize)]
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
    let account: Account = response
        .error_for_status()?
        .json()
        .context("Could not read your TypeAside account. Update the app and try again.")?;
    crate::http::set_limited_until(if account.at_limit() {
        account.usage.resets_at
    } else {
        0
    });
    Ok(Some(account))
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
        let _ = match token {
            Some(_) => write!(
                stream,
                "HTTP/1.1 303 See Other\r\nLocation: {}/desktop/connect?connected=true\r\nContent-Length: 0\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
                web_url()
            ),
            None => write!(
                stream,
                "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot found"
            ),
        };
        if let Some(token) = token {
            return Ok(token);
        }
    }
    bail!("Sign in timed out")
}

const FORM_TYPE: &str = "application/x-www-form-urlencoded";
const MAX_REQUEST_BYTES: u64 = 8192;
const MAX_BODY_BYTES: usize = 1024;

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
