use anyhow::{Context, Result};
use std::{
    io::{BufRead, Read},
    net::{TcpStream, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tungstenite::{Message, client::IntoClientRequest, stream::MaybeTlsStream};

const TIMEOUT: Duration = Duration::from_secs(30);

enum Command {
    Audio(Vec<u8>),
    Release(String),
}

#[derive(Clone)]
pub struct AudioSink {
    commands: mpsc::SyncSender<Command>,
    cancelled: Arc<AtomicBool>,
}

impl AudioSink {
    pub fn send(&self, samples: &[f32]) {
        if self.cancelled.load(Ordering::Relaxed) {
            return;
        }
        let bytes = samples
            .iter()
            .flat_map(|sample| ((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes())
            .collect();
        if matches!(
            self.commands.try_send(Command::Audio(bytes)),
            Err(mpsc::TrySendError::Full(_))
        ) {
            self.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

pub struct Session {
    sink: AudioSink,
    result: mpsc::Receiver<Result<crate::pipeline::HostedResponse>>,
}

impl Session {
    pub fn start() -> Result<Self> {
        let (commands, incoming) = mpsc::sync_channel(128);
        let (result, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let transport_cancelled = cancelled.clone();
        std::thread::Builder::new()
            .name("typeaside-stream".into())
            .spawn(move || {
                let response = transport(incoming, &transport_cancelled);
                let _ = result.send(response);
            })?;
        Ok(Self {
            sink: AudioSink {
                commands,
                cancelled,
            },
            result: receiver,
        })
    }

    pub fn audio(&self) -> AudioSink {
        self.sink.clone()
    }

    // Only structured server errors are definitive. All transport failures retain POST fallback.
    pub(crate) fn finish(self, context: String) -> Option<Result<crate::pipeline::HostedResponse>> {
        if self.sink.cancelled.load(Ordering::Relaxed) {
            return None;
        }
        if matches!(
            self.sink.commands.try_send(Command::Release(context)),
            Err(mpsc::TrySendError::Full(_))
        ) {
            return None;
        }
        match self.result.recv_timeout(TIMEOUT + Duration::from_secs(3)) {
            Ok(Ok(value)) => Some(Ok(value)),
            Ok(Err(error)) if error.downcast_ref::<crate::http::HostedError>().is_some() => {
                Some(Err(error))
            }
            _ => None,
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.sink.cancelled.store(true, Ordering::Relaxed);
    }
}

fn transport(
    incoming: mpsc::Receiver<Command>,
    cancelled: &AtomicBool,
) -> Result<crate::pipeline::HostedResponse> {
    let base = crate::account::hosted_url();
    let mut url = reqwest::Url::parse(&format!("{base}/v1/stream"))?;
    url.set_scheme(if url.scheme() == "https" { "wss" } else { "ws" })
        .map_err(|()| anyhow::anyhow!("invalid streaming URL"))?;
    let mut request = url.as_str().into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", crate::credentials::endpoint_key(&base)?).parse()?,
    );
    let host = url.host_str().context("missing streaming host")?;
    let port = url
        .port_or_known_default()
        .context("missing streaming port")?;
    let tcp = (host, port)
        .to_socket_addrs()?
        .find_map(|address| TcpStream::connect_timeout(&address, Duration::from_secs(2)).ok())
        .context("cannot connect dictation stream")?;
    tcp.set_read_timeout(Some(Duration::from_secs(2)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut connection = if url.scheme() == "wss" {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from(host.to_owned())?;
        MaybeTlsStream::Rustls(rustls::StreamOwned::new(
            rustls::ClientConnection::new(Arc::new(config), name)?,
            tcp,
        ))
    } else {
        MaybeTlsStream::Plain(tcp)
    };
    let mut socket = match tungstenite::client::client_with_config(request, &mut connection, None) {
        Ok((socket, _)) => socket,
        Err(tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response))) => {
            let body = upgrade_body(&response, &mut connection).unwrap_or_default();
            crate::http::check_status(response.status().as_u16(), &body)?;
            anyhow::bail!("stream upgrade rejected");
        }
        Err(error) => anyhow::bail!("stream upgrade failed: {error}"),
    };
    set_read_timeout(socket.get_mut(), Duration::from_millis(1))?;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = socket.close(None);
            anyhow::bail!("stream cancelled");
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                decode_response(&text)?;
                anyhow::bail!("stream result arrived before release");
            }
            Ok(Message::Ping(_) | Message::Pong(_)) => {}
            Ok(_) => anyhow::bail!("stream closed before release"),
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(error.into()),
        }
        match incoming.recv_timeout(Duration::from_millis(100)) {
            Ok(Command::Audio(bytes)) => socket.send(Message::Binary(bytes.into()))?,
            Ok(Command::Release(context)) => {
                socket.send(Message::Text(context.into()))?;
                set_read_timeout(socket.get_mut(), TIMEOUT)?;
                let deadline = std::time::Instant::now() + TIMEOUT;
                loop {
                    anyhow::ensure!(
                        std::time::Instant::now() < deadline,
                        "stream result timed out"
                    );
                    match socket.read()? {
                        Message::Text(text) => {
                            let response = decode_response(&text)?;
                            let _ = socket.close(None);
                            return Ok(response);
                        }
                        Message::Ping(_) | Message::Pong(_) => {}
                        _ => anyhow::bail!("stream closed before result"),
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = socket.close(None);
                anyhow::bail!("stream discarded");
            }
        }
    }
}

fn decode_response(text: &str) -> Result<crate::pipeline::HostedResponse> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    if let Some(status) = value["status"].as_u64() {
        crate::http::check_status(u16::try_from(status)?, text.as_bytes())?;
    }
    Ok(serde_json::from_value(value)?)
}

fn set_read_timeout(connection: &mut MaybeTlsStream<TcpStream>, timeout: Duration) -> Result<()> {
    match connection {
        MaybeTlsStream::Plain(tcp) => tcp.set_read_timeout(Some(timeout))?,
        MaybeTlsStream::Rustls(tls) => tls.get_mut().set_read_timeout(Some(timeout))?,
        _ => anyhow::bail!("unsupported stream TLS"),
    }
    Ok(())
}

// Tungstenite returns only the body bytes already buffered with the response headers.
fn upgrade_body(
    response: &tungstenite::handshake::client::Response,
    connection: &mut impl Read,
) -> Result<Vec<u8>> {
    const LIMIT: u64 = 65_536;
    let buffered = response.body().as_deref().unwrap_or_default();
    let mut reader = std::io::BufReader::new(buffered.chain(connection.take(LIMIT)));
    let mut body = Vec::new();
    if response
        .headers()
        .get("transfer-encoding")
        .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"chunked"))
    {
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let size =
                usize::from_str_radix(line.trim().split(';').next().unwrap_or_default(), 16)?;
            anyhow::ensure!(
                body.len().saturating_add(size) <= LIMIT as usize,
                "stream error body too large"
            );
            if size == 0 {
                break;
            }
            let start = body.len();
            body.resize(start + size, 0);
            reader.read_exact(&mut body[start..])?;
            let mut crlf = [0; 2];
            reader.read_exact(&mut crlf)?;
            anyhow::ensure!(crlf == *b"\r\n", "invalid stream error body");
        }
    } else if let Some(length) = response.headers().get("content-length") {
        let length: u64 = length.to_str()?.parse()?;
        anyhow::ensure!(length <= LIMIT, "stream error body too large");
        reader.take(length).read_to_end(&mut body)?;
    } else {
        reader.take(LIMIT).read_to_end(&mut body)?;
    }
    Ok(body)
}
