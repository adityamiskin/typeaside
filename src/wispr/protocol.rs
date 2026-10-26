use crate::config::{AppConfig, Tone};
use anyhow::{Result, bail};
use prost::Message;

// Wire fields from Wispr Flow 1.6.1264. Unrecognized response fields are ignored by prost.
#[derive(Clone, PartialEq, Message)]
pub(super) struct Request {
    #[prost(message, optional, tag = "1")]
    init: Option<Init>,
    #[prost(message, optional, tag = "3")]
    payload: Option<Payload>,
    #[prost(int32, tag = "4")]
    commit: i32,
}
#[derive(Clone, PartialEq, Message)]
struct Init {
    #[prost(message, optional, tag = "1")]
    metadata: Option<Metadata>,
    #[prost(message, optional, tag = "2")]
    preferences: Option<Preferences>,
}
#[derive(Clone, PartialEq, Message)]
struct Metadata {
    #[prost(string, tag = "1")]
    user_id: String,
    #[prost(string, tag = "2")]
    session_id: String,
    #[prost(string, tag = "3")]
    request_id: String,
    #[prost(int32, tag = "4")]
    audio_encoding: i32,
    #[prost(int32, tag = "5")]
    environment: i32,
    #[prost(message, optional, tag = "6")]
    client: Option<Client>,
}
#[derive(Clone, PartialEq, Message)]
struct Client {
    #[prost(string, tag = "1")]
    name: String,
    #[prost(int32, tag = "2")]
    platform: i32,
    #[prost(message, optional, tag = "3")]
    version: Option<Version>,
}
#[derive(Clone, PartialEq, Message)]
struct Version {
    #[prost(uint32, tag = "1")]
    major: u32,
    #[prost(uint32, tag = "2")]
    minor: u32,
    #[prost(uint32, tag = "3")]
    patch: u32,
}
#[derive(Clone, PartialEq, Message)]
struct Preferences {
    #[prost(message, optional, tag = "1")]
    user: Option<User>,
    #[prost(int32, repeated, tag = "2")]
    language: Vec<i32>,
    #[prost(message, optional, tag = "3")]
    vocabulary: Option<Vocabulary>,
    #[prost(message, optional, tag = "5")]
    style: Option<Style>,
}
#[derive(Clone, PartialEq, Message)]
struct User {}
#[derive(Clone, PartialEq, Message)]
struct Vocabulary {
    #[prost(string, repeated, tag = "1")]
    words: Vec<String>,
}
#[derive(Clone, PartialEq, Message)]
struct Style {
    #[prost(int32, tag = "2")]
    override_style: i32,
    #[prost(message, optional, tag = "3")]
    tagging: Option<Tagging>,
    #[prost(message, optional, tag = "4")]
    signature: Option<Signature>,
    #[prost(int32, tag = "5")]
    editing_strength: i32,
}
#[derive(Clone, PartialEq, Message)]
struct Tagging {
    #[prost(bool, optional, tag = "1")]
    slack: Option<bool>,
}
#[derive(Clone, PartialEq, Message)]
struct Signature {
    #[prost(bool, optional, tag = "2")]
    hyperlink: Option<bool>,
    #[prost(bool, optional, tag = "3")]
    sign_emails: Option<bool>,
}
#[derive(Clone, PartialEq, Message)]
struct Payload {
    #[prost(message, optional, tag = "2")]
    file: Option<AudioFile>,
}
#[derive(Clone, PartialEq, Message)]
struct AudioFile {
    #[prost(bytes = "vec", tag = "1")]
    data: Vec<u8>,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct Response {
    #[prost(message, optional, tag = "1")]
    pub result: Option<TranscriptionResult>,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct TranscriptionResult {
    #[prost(message, optional, tag = "1")]
    pub output: Option<Output>,
    #[prost(int32, tag = "5")]
    pub status: i32,
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct Output {
    #[prost(string, tag = "2")]
    pub plaintext: String,
}

pub(super) fn requests(
    config: &AppConfig,
    audio: &[f32],
    names: &[String],
    user_id: &str,
    tone: Tone,
) -> Result<Vec<Request>> {
    let language = match config.language.as_deref() {
        None => Vec::new(),
        Some(code) => vec![match code {
            "en" => 1,
            "hi" => 20,
            "es" => 6,
            "fr" => 9,
            "de" => 5,
            "pt" => 11,
            "it" => 18,
            "ja" => 10,
            "ko" => 8,
            "zh" => 4,
            "ar" => 16,
            "ru" => 7,
            _ => bail!("Unsupported Wispr language: {code}. Choose a language in Settings."),
        }],
    };
    Ok(vec![
        Request {
            init: Some(Init {
                metadata: Some(Metadata {
                    user_id: user_id.into(),
                    session_id: super::request_id()?,
                    request_id: super::request_id()?,
                    audio_encoding: 1,
                    environment: 1,
                    client: Some(Client {
                        name: "Wispr Flow".into(),
                        platform: 2,
                        version: Some(Version {
                            major: 1,
                            minor: 6,
                            patch: 1264,
                        }),
                    }),
                }),
                preferences: Some(Preferences {
                    user: Some(User {}),
                    language,
                    vocabulary: Some(Vocabulary {
                        words: config
                            .dictionary
                            .iter()
                            .chain(names)
                            .take(500)
                            .cloned()
                            .collect(),
                    }),
                    style: Some(Style {
                        override_style: match tone {
                            Tone::Formal => 1,
                            Tone::Casual => 2,
                            Tone::VeryCasual => 3,
                        },
                        tagging: Some(Tagging { slack: Some(false) }),
                        signature: Some(Signature {
                            hyperlink: Some(false),
                            sign_emails: Some(false),
                        }),
                        editing_strength: 2,
                    }),
                }),
            }),
            payload: None,
            commit: 2,
        },
        Request {
            init: None,
            payload: Some(Payload {
                file: Some(AudioFile {
                    data: crate::transcribe::encode_wav(audio)?,
                }),
            }),
            commit: 1,
        },
    ])
}
