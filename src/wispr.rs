mod protocol;

use crate::{config::AppConfig, credentials};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tonic::metadata::MetadataValue;

const CREDENTIAL: &str = "wisprflow-session";
const SUPABASE: &str = "https://dodjkfqhwrzqjwkfnthl.supabase.co/auth/v1";
// Public anon key shipped by Wispr's browser and desktop clients, not a backend credential.
const PUBLISHABLE_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImRvZGprZnFod3J6cWp3a2ZudGhsIiwicm9sZSI6ImFub24iLCJpYXQiOjE3MTk4ODQzMDcsImV4cCI6MjAzNTQ2MDMwN30.h6EeQ_6kqFeznH25icVUX0Szn9__kc8HoSXAsxxBWG8";
pub const NEEDS_ACCOUNT: &str = "Connect your Wispr Flow account in Settings first";
static SESSION_LOCK: Mutex<()> = Mutex::new(());

fn lock_session() -> MutexGuard<'static, ()> {
    SESSION_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Serialize, Deserialize)]
struct Session {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
    iss: String,
    email: Option<String>,
    client_id: Option<String>,
    org_id: Option<String>,
    #[serde(rename = "urn:wispr:user_external_id")]
    external_id: Option<String>,
}

impl Claims {
    fn user_id(&self) -> &str {
        self.external_id.as_deref().unwrap_or(&self.sub)
    }

    fn account(self) -> String {
        self.email.unwrap_or_else(|| "Connected".into())
    }
}

impl Session {
    fn claims(&self) -> Result<Claims> {
        let payload = self
            .access_token
            .split('.')
            .nth(1)
            .context("Invalid Wispr session")?;
        let claims: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
        let issuer = reqwest::Url::parse(&claims.iss)?;
        ensure!(
            claims.iss == SUPABASE
                || (issuer.scheme() == "https"
                    && issuer.host_str().is_some_and(
                        |host| host == "api.workos.com" || host.ends_with(".authkit.app")
                    )
                    && claims
                        .client_id
                        .as_deref()
                        .is_some_and(|id| id.starts_with("client_"))),
            "Unrecognized Wispr session issuer"
        );
        ensure!(
            !claims.sub.is_empty() && !self.refresh_token.is_empty(),
            "Incomplete Wispr session"
        );
        Ok(claims)
    }

    fn save(&self) -> Result<()> {
        credentials::save_credential(CREDENTIAL, &serde_json::to_string(self)?)
    }
}

fn stored_session() -> Result<Option<Session>> {
    credentials::get_credential(CREDENTIAL)?
        .map(|session| {
            serde_json::from_str(&session).context("Cannot read Wispr session. Sign in again.")
        })
        .transpose()
}

pub fn account() -> Result<Option<String>> {
    stored_session()?
        .map(|session| Ok(session.claims()?.account()))
        .transpose()
}

pub fn disconnect() -> Result<()> {
    let _guard = lock_session();
    credentials::delete_credential(CREDENTIAL)
}

pub struct Login {
    code: String,
}

impl Login {
    pub fn new() -> Result<Self> {
        Ok(Self {
            code: request_id()?,
        })
    }

    pub fn url(&self) -> String {
        format!(
            "https://wisprflow.ai/login?device_code={}&auto_signin=false&workos_capable=true&skip_default_trial=true",
            self.code
        )
    }

    pub fn wait(self, cancel: &AtomicBool) -> Result<String> {
        #[derive(Deserialize)]
        struct Event {
            status: String,
            #[serde(flatten)]
            session: Option<Session>,
        }
        let deadline = Instant::now() + Duration::from_secs(300);
        while Instant::now() < deadline {
            ensure!(!cancel.load(Ordering::Relaxed), "Wispr sign-in cancelled");
            let event: Event = crate::http::client()
                .get("https://api.wisprflow.ai/device/poll")
                .query(&[("device_code", &self.code)])
                .timeout(Duration::from_secs(2))
                .send()
                .map_err(reqwest::Error::without_url)?
                .error_for_status()
                .map_err(reqwest::Error::without_url)?
                .json()
                .map_err(reqwest::Error::without_url)?;
            ensure!(!cancel.load(Ordering::Relaxed), "Wispr sign-in cancelled");
            match event.status.as_str() {
                "complete" => {
                    let session = event.session.context("Wispr sign-in returned no session")?;
                    let claims = session.claims()?;
                    let _guard = lock_session();
                    ensure!(!cancel.load(Ordering::Relaxed), "Wispr sign-in cancelled");
                    session.save()?;
                    return Ok(claims.account());
                }
                "pending" => std::thread::sleep(Duration::from_secs(1)),
                "timeout" => break,
                _ => bail!("Unexpected Wispr sign-in response. Try again."),
            }
        }
        bail!("Wispr sign-in expired. Try again.")
    }
}

fn session() -> Result<(Session, Claims)> {
    #[derive(Serialize)]
    struct WorkOsRefresh<'a> {
        grant_type: &'static str,
        client_id: &'a str,
        refresh_token: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        organization_id: Option<&'a str>,
    }
    #[derive(Serialize)]
    struct SupabaseRefresh<'a> {
        refresh_token: &'a str,
    }
    let _guard = lock_session();
    let session = stored_session()?.context(NEEDS_ACCOUNT)?;
    let claims = session.claims()?;
    if claims.exp > SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 120 {
        return Ok((session, claims));
    }
    let request = match claims.client_id.as_deref() {
        Some(client_id) => crate::http::client()
            .post("https://api.workos.com/user_management/authenticate")
            .json(&WorkOsRefresh {
                grant_type: "refresh_token",
                client_id,
                refresh_token: &session.refresh_token,
                organization_id: claims.org_id.as_deref(),
            }),
        None => crate::http::client()
            .post(format!("{SUPABASE}/token?grant_type=refresh_token"))
            .header("apikey", PUBLISHABLE_KEY)
            .json(&SupabaseRefresh {
                refresh_token: &session.refresh_token,
            }),
    };
    let response = request.send().map_err(reqwest::Error::without_url)?;
    ensure!(
        response.status().is_success(),
        "Wispr session could not be refreshed (HTTP {}). Sign in again.",
        response.status()
    );
    let refreshed: Session = response.json().map_err(reqwest::Error::without_url)?;
    let refreshed_claims = refreshed.claims()?;
    ensure!(
        refreshed_claims.user_id() == claims.user_id(),
        "Wispr session identity changed"
    );
    refreshed.save()?;
    Ok((refreshed, refreshed_claims))
}

fn request_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| anyhow::anyhow!("Cannot generate Wispr request ID: {e}"))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

pub fn transcribe(
    config: &AppConfig,
    audio: &[f32],
    names: &[String],
    app_id: Option<&str>,
) -> Result<String> {
    let (session, claims) = session()?;
    let requests = protocol::requests(
        config,
        audio,
        names,
        claims.user_id(),
        config.styles.get(crate::config::AppCategory::of(app_id)),
    )?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(60), async {
                let channel =
                    tonic::transport::Endpoint::from_static("https://inference.wisprflow.com")
                        .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())?
                        .connect_timeout(Duration::from_secs(5))
                        .connect()
                        .await
                        .context("Cannot connect to Wispr transcription")?;
                let mut client = tonic::client::Grpc::new(channel)
                    .max_encoding_message_size(20_000_000)
                    .max_decoding_message_size(1_000_000);
                client
                    .ready()
                    .await
                    .context("Wispr transcription is unavailable")?;
                let failed = |e: tonic::Status| {
                    anyhow::anyhow!("Wispr transcription failed ({:?})", e.code())
                };
                let mut request = tonic::Request::new(tokio_stream::iter(requests));
                let metadata = request.metadata_mut();
                metadata.insert(
                    "authorization",
                    format!("Bearer {}", session.access_token).parse()?,
                );
                metadata.insert("flow-debug", MetadataValue::from_static("false"));
                metadata.insert(
                    "disable-formatting",
                    MetadataValue::from_static(if config.editing() { "false" } else { "true" }),
                );
                let response = client
                    .streaming(
                        request,
                        tonic::codegen::http::uri::PathAndQuery::from_static(
                            "/flow_api.v1.TranscriptionService/TranscribeStream",
                        ),
                        tonic_prost::ProstCodec::<protocol::Request, protocol::Response>::default(),
                    )
                    .await
                    .map_err(failed)?;
                let mut stream = response.into_inner();
                let mut result = None;
                while let Some(response) = stream.message().await.map_err(failed)? {
                    result = response.result.or(result);
                }
                let result = result.context("Wispr returned no transcription result")?;
                ensure!(
                    result.status == 1,
                    "Wispr could not transcribe this recording (status {})",
                    result.status
                );
                let output = result.output.context("Wispr returned no transcript")?;
                let text = output.plaintext.trim();
                ensure!(!text.is_empty(), "No speech detected");
                Ok(text.to_owned())
            })
            .await
            .context("Wispr transcription timed out")?
        })
}
