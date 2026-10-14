use anyhow::Result;
use hermes::{
    config::AppConfig,
    credentials,
    engine::{Command, Engine},
    paths::AppPaths,
};

#[derive(Clone)]
pub struct Preferences {
    pub hotkey: String,
    pub toggle: bool,
    pub device: String,
    pub endpoint: String,
    pub model: String,
    pub language: String,
    pub cleanup: bool,
    pub cleanup_endpoint: String,
    pub cleanup_model: String,
    pub auto_submit: bool,
    pub dictionary: Vec<String>,
    pub key_hint: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            hotkey: "RIGHTCTRL".into(),
            toggle: false,
            device: "System default".into(),
            endpoint: "https://api.groq.com/openai/v1".into(),
            model: "whisper-large-v3-turbo".into(),
            language: String::new(),
            cleanup: true,
            cleanup_endpoint: "https://api.groq.com/openai/v1".into(),
            cleanup_model: "openai/gpt-oss-20b".into(),
            auto_submit: false,
            dictionary: Vec::new(),
            key_hint: "Enter API key".into(),
        }
    }
}

pub struct Binding {
    config: AppConfig,
    paths: Option<AppPaths>,
    pub engine: Option<Engine>,
}

impl Binding {
    pub fn demo() -> Self {
        Self {
            config: AppConfig::default(),
            paths: None,
            engine: None,
        }
    }

    pub fn load() -> Result<(Self, Preferences)> {
        let paths = AppPaths::discover()?;
        let config = AppConfig::load(&paths)?;
        let preferences = Preferences {
            hotkey: config.primary_shortcut.clone(),
            toggle: config.recording_mode == "toggle",
            device: config
                .selected_device_name
                .clone()
                .unwrap_or_else(|| "System default".into()),
            endpoint: config
                .rest_endpoint_url
                .clone()
                .unwrap_or_else(|| Preferences::default().endpoint),
            model: config.model.clone(),
            language: config.language.clone().unwrap_or_default(),
            auto_submit: config.auto_submit,
            cleanup: config.filter_filler_words,
            dictionary: config.word_overrides.keys().cloned().collect(),
            key_hint: credentials::get_credential(&paths, "groq")?
                .map(|key| mask_key(&key))
                .unwrap_or_else(|| "Enter API key".into()),
            ..Preferences::default()
        };
        Ok((
            Self {
                config,
                paths: Some(paths),
                engine: None,
            },
            preferences,
        ))
    }

    pub fn config(&self) -> AppConfig {
        self.config.clone()
    }

    pub fn save(&mut self, preferences: &Preferences, key: &str) -> Result<()> {
        let Some(paths) = &self.paths else {
            return Ok(());
        };
        self.config.primary_shortcut.clone_from(&preferences.hotkey);
        self.config.recording_mode = if preferences.toggle {
            "toggle"
        } else {
            "push_to_talk"
        }
        .into();
        self.config.selected_device_name =
            (preferences.device != "System default").then(|| preferences.device.clone());
        self.config.rest_endpoint_url = Some(preferences.endpoint.clone());
        self.config.model.clone_from(&preferences.model);
        self.config.language =
            (!preferences.language.is_empty()).then(|| preferences.language.clone());
        self.config.auto_submit = preferences.auto_submit;
        self.config.filter_filler_words = preferences.cleanup;
        self.config.word_overrides = preferences
            .dictionary
            .iter()
            .map(|word| (word.clone(), word.clone()))
            .collect();
        if !key.is_empty() {
            credentials::save_credential(paths, "groq", key)?;
        }
        self.config.save(paths)?;
        if let Some(engine) = &self.engine {
            engine.send(Command::Reload(self.config.clone()));
        }
        Ok(())
    }
}

pub fn mask_key(key: &str) -> String {
    if key.chars().count() < 9 {
        return "••••••••".into();
    }
    let first: String = key.chars().take(4).collect();
    let last: String = key
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{first}••••••••{last}")
}
