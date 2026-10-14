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
    fn groq(model: &str) -> Self {
        Self {
            base_url: "https://api.groq.com/openai/v1".into(),
            model: model.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cleanup {
    pub enabled: bool,
    pub endpoint: Endpoint,
}

impl Default for Cleanup {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: Endpoint::groq("openai/gpt-oss-20b"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey: String,
    pub mode: RecordingMode,
    pub cancel_hotkey: Option<String>,
    pub input_device: Option<String>,
    pub stt: Endpoint,
    pub language: Option<String>,
    pub cleanup: Cleanup,
    pub dictionary: Vec<String>,
    pub auto_submit: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "RIGHTCTRL".into(),
            mode: RecordingMode::Hold,
            cancel_hotkey: Some("ESCAPE".into()),
            input_device: None,
            stt: Endpoint::groq("whisper-large-v3-turbo"),
            language: None,
            cleanup: Cleanup::default(),
            dictionary: Vec::new(),
            auto_submit: false,
        }
    }
}

impl AppConfig {
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
