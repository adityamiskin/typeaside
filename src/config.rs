use crate::paths::AppPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingMode {
    #[default]
    Hold,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub base_url: String,
    pub model: String,
}

impl Endpoint {
    fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Formal,
    #[default]
    Casual,
    VeryCasual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCategory {
    Personal,
    Work,
    Email,
    Other,
}

impl AppCategory {
    pub const ALL: [Self; 4] = [Self::Personal, Self::Work, Self::Email, Self::Other];

    pub fn of(app_id: Option<&str>) -> Self {
        let id = app_id.unwrap_or_default().to_ascii_lowercase();
        let has = |names: &[&str]| names.iter().any(|name| id.contains(name));
        if has(&[
            "telegram", "whatsapp", "signal", "discord", "element", "beeper",
        ]) {
            Self::Personal
        } else if has(&["slack", "teams", "linear", "mattermost", "zulip"]) {
            Self::Work
        } else if has(&["thunderbird", "geary", "evolution", "mail", "betterbird"]) {
            Self::Email
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Styles {
    pub personal: Tone,
    pub work: Tone,
    pub email: Tone,
    pub other: Tone,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            personal: Tone::Casual,
            work: Tone::Casual,
            email: Tone::Formal,
            other: Tone::Casual,
        }
    }
}

impl Styles {
    pub fn get_mut(&mut self, category: AppCategory) -> &mut Tone {
        match category {
            AppCategory::Personal => &mut self.personal,
            AppCategory::Work => &mut self.work,
            AppCategory::Email => &mut self.email,
            AppCategory::Other => &mut self.other,
        }
    }

    pub fn get(mut self, category: AppCategory) -> Tone {
        *self.get_mut(category)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub trigger: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cleanup {
    pub enabled: bool,
    pub endpoint: Endpoint,
    /// Sent as `reasoning_effort`; "none" keeps reasoning models at chat latency.
    pub reasoning_effort: Option<String>,
}

impl Default for Cleanup {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: Endpoint::new("https://api.groq.com/openai/v1", "qwen/qwen3.8-27b"),
            reasoning_effort: Some("none".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Hosted,
    #[default]
    OwnKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey: String,
    pub mode: RecordingMode,
    pub cancel_hotkey: Option<String>,
    pub input_device: Option<String>,
    pub transcription: Source,
    pub stt: Endpoint,
    pub language: Option<String>,
    pub cleanup: Cleanup,
    pub dictionary: Vec<String>,
    pub auto_submit: bool,
    /// Rewrites the selected text by voice instruction.
    pub command_hotkey: Option<String>,
    pub paste_last_hotkey: Option<String>,
    /// Target language name for translation; None keeps the spoken language.
    pub translate_to: Option<String>,
    pub styles: Styles,
    pub snippets: Vec<Snippet>,
    pub sounds: bool,
    pub duck_media: bool,
    pub launch_at_login: bool,
    pub theme: Theme,
    pub onboarded: bool,
    pub share_usage: bool,
    pub install_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "RIGHTCTRL".into(),
            mode: RecordingMode::Hold,
            cancel_hotkey: Some("ESCAPE".into()),
            input_device: None,
            transcription: Source::OwnKey,
            stt: Endpoint::new("https://api.groq.com/openai/v1", "whisper-large-v3-turbo"),
            language: None,
            cleanup: Cleanup::default(),
            dictionary: Vec::new(),
            auto_submit: false,
            command_hotkey: Some("RIGHTALT".into()),
            paste_last_hotkey: Some("LEFTALT+LEFTSHIFT+V".into()),
            translate_to: None,
            styles: Styles::default(),
            snippets: Vec::new(),
            sounds: true,
            duck_media: true,
            launch_at_login: true,
            theme: Theme::System,
            onboarded: false,
            share_usage: false,
            install_id: None,
        }
    }
}

impl AppConfig {
    pub fn set_share_usage(&mut self, enabled: bool) -> Result<()> {
        if enabled && self.install_id.is_none() {
            let mut bytes = [0; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| anyhow::anyhow!("Could not generate install ID"))?;
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            self.install_id = Some(format!(
                "{}-{}-{}-{}-{}",
                &hex[..8],
                &hex[8..12],
                &hex[12..16],
                &hex[16..20],
                &hex[20..]
            ));
        }
        self.share_usage = enabled;
        Ok(())
    }

    pub fn effective(&self) -> Self {
        let mut config = self.clone();
        if self.transcription == Source::Hosted {
            let base = format!("{}/openai/v1", crate::account::hosted_url());
            config.stt.base_url = base.clone();
            config.cleanup.endpoint.base_url = base;
        }
        config
    }

    pub fn load(paths: &AppPaths) -> Result<Self> {
        match std::fs::read(&paths.config_file) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("invalid config JSON"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(paths)?;
                Ok(config)
            }
            Err(error) => Err(error).context("failed to read config"),
        }
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        paths.ensure()?;
        let temporary = paths.config_file.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temporary, &paths.config_file).context("failed to save config")
    }
}
