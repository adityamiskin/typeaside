use crate::paths::AppPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaControl {
    Off,
    LowerVolume,
    PauseMedia,
}

impl Default for MediaControl {
    fn default() -> Self {
        if cfg!(target_os = "linux") {
            Self::PauseMedia
        } else {
            Self::Off
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingMode {
    #[default]
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryRetention {
    #[default]
    Forever,
    Month,
    Week,
    Off,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalUnload {
    FiveMinutes,
    #[default]
    ThirtyMinutes,
    Never,
}

impl LocalUnload {
    pub fn after(self) -> Option<std::time::Duration> {
        match self {
            Self::FiveMinutes => Some(std::time::Duration::from_secs(5 * 60)),
            Self::ThirtyMinutes => Some(std::time::Duration::from_secs(30 * 60)),
            Self::Never => None,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Personal,
    Work,
    Email,
    Code,
    Other,
}

/// Exact IDs, since substrings like "code" or "kitty" also match unrelated apps.
const CODE_APPS: &[&str] = &[
    // Editors
    "code",
    "code - oss",
    "code-oss",
    "code-insiders",
    "code - insiders",
    "codium",
    "vscodium",
    "com.visualstudio.code",
    "com.visualstudio.code.oss",
    "com.visualstudio.code.insiders",
    "com.vscodium.codium",
    "cursor",
    "com.todesktop.230313mzl4w4u92",
    "zed",
    "dev.zed.zed",
    "dev.zed.zed-preview",
    "sublime_text",
    "sublime-text",
    "com.sublimetext.3",
    "com.sublimetext.4",
    "neovide",
    "dev.neovide.neovide",
    "com.neovide.neovide",
    "idea64",
    "pycharm64",
    "webstorm64",
    "goland64",
    "clion64",
    "rider64",
    "rustrover64",
    // Terminals
    "alacritty",
    "org.alacritty",
    "org.alacritty.alacritty",
    "kitty",
    "net.kovidgoyal.kitty",
    "ghostty",
    "com.mitchellh.ghostty",
    "wezterm",
    "wezterm-gui",
    "org.wezfurlong.wezterm",
    "com.github.wez.wezterm",
    "foot",
    "footclient",
    "konsole",
    "org.kde.konsole",
    "gnome-terminal-server",
    "org.gnome.terminal",
    "org.gnome.ptyxis",
    "ptyxis",
    "org.gnome.console",
    "kgx",
    "tilix",
    "terminator",
    "xfce4-terminal",
    "blackbox",
    "com.apple.terminal",
    "com.googlecode.iterm2",
    "dev.warp.warp-stable",
    "warp",
    "windowsterminal",
    "wt",
];

/// Case-insensitive app id without a Windows ".exe" suffix.
pub fn app_key(app_id: &str) -> String {
    let id = app_id.to_ascii_lowercase();
    match id.strip_suffix(".exe") {
        Some(name) => name.to_owned(),
        None => id,
    }
}

impl AppCategory {
    pub const ALL: [Self; 5] = [
        Self::Personal,
        Self::Work,
        Self::Email,
        Self::Code,
        Self::Other,
    ];

    pub fn of(app_id: Option<&str>) -> Self {
        let id = app_key(app_id.unwrap_or_default());
        let has = |names: &[&str]| names.iter().any(|name| id.contains(name));
        let code = id.starts_with("jetbrains-")
            || id.starts_with("com.jetbrains.")
            || CODE_APPS.contains(&id.as_str());
        if code {
            Self::Code
        } else if has(&[
            "telegram", "whatsapp", "signal", "discord", "element", "beeper",
        ]) {
            Self::Personal
        } else if has(&["slack", "teams", "linear", "mattermost", "zulip"]) {
            Self::Work
        } else if has(&[
            "thunderbird",
            "geary",
            "evolution",
            "mail",
            "betterbird",
            "outlook",
        ]) {
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
    pub code: Tone,
    pub other: Tone,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            personal: Tone::Casual,
            work: Tone::Casual,
            email: Tone::Formal,
            code: Tone::Casual,
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
            AppCategory::Code => &mut self.code,
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
pub struct AppInstruction {
    pub app: String,
    pub text: String,
}

pub const MAX_APP_INSTRUCTIONS: usize = 20;
/// UTF-16 units, the unit of the Worker's zod `max`.
pub const MAX_INSTRUCTION_LEN: usize = 300;

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
    Local,
    Hosted,
    #[default]
    OwnKey,
}

fn known_or_default<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<crate::local::ModelId, D::Error> {
    Ok(String::deserialize(deserializer)?
        .parse()
        .unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey: String,
    pub mode: RecordingMode,
    pub input_devices: Vec<String>,
    pub keep_microphone_ready: bool,
    pub transcription: Source,
    // A removed model falls back to the default instead of invalidating the whole config.
    #[serde(deserialize_with = "known_or_default")]
    pub local_model: crate::local::ModelId,
    pub local_unload: LocalUnload,
    pub stt: Endpoint,
    pub language: Option<String>,
    pub cleanup: Cleanup,
    pub dictionary: Vec<String>,
    pub replacements: Vec<crate::replacements::Rule>,
    /// Words the user removed; corrections never suggest them again.
    pub removed_words: Vec<String>,
    pub settings_sync: Option<crate::settings_sync::SyncState>,
    pub usage_notice_week: Option<i64>,
    pub auto_submit: bool,
    pub send_command: bool,
    pub on_dictation: Option<String>,
    pub auto_learn: bool,
    pub screen_context: bool,
    pub command_hotkey: Option<String>,
    pub translate_hotkey: Option<String>,
    pub pause_hotkey: Option<String>,
    pub note_hotkey: Option<String>,
    pub paste_last_hotkey: Option<String>,
    pub translate_to: String,
    pub styles: Styles,
    pub snippets: Vec<Snippet>,
    pub app_instructions: Vec<AppInstruction>,
    pub sounds: bool,
    pub media_control: MediaControl,
    pub launch_at_login: bool,
    pub theme: Theme,
    pub onboarded: bool,
    pub share_usage: bool,
    pub history_retention: HistoryRetention,
    pub install_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: if cfg!(target_os = "macos") {
                "FN"
            } else {
                "RIGHTCTRL"
            }
            .into(),
            mode: RecordingMode::Hold,
            input_devices: Vec::new(),
            keep_microphone_ready: false,
            transcription: Source::OwnKey,
            local_model: crate::local::ModelId::default(),
            local_unload: LocalUnload::default(),
            stt: Endpoint::new("https://api.groq.com/openai/v1", "whisper-large-v3-turbo"),
            language: None,
            cleanup: Cleanup::default(),
            dictionary: Vec::new(),
            replacements: Vec::new(),
            removed_words: Vec::new(),
            settings_sync: None,
            usage_notice_week: None,
            auto_submit: false,
            send_command: true,
            on_dictation: None,
            auto_learn: true,
            screen_context: true,
            command_hotkey: Some("RIGHTALT".into()),
            translate_hotkey: Some("RIGHTALT+RIGHTSHIFT".into()),
            pause_hotkey: Some("PAUSE".into()),
            note_hotkey: Some("LEFTSHIFT*2".into()),
            paste_last_hotkey: Some("LEFTCTRL+LEFTALT+V".into()),
            translate_to: "English".into(),
            styles: Styles::default(),
            snippets: Vec::new(),
            app_instructions: Vec::new(),
            sounds: true,
            media_control: MediaControl::default(),
            launch_at_login: true,
            theme: Theme::System,
            onboarded: false,
            share_usage: false,
            history_retention: HistoryRetention::Forever,
            install_id: None,
        }
    }
}

impl AppConfig {
    /// Account (hosted) users always get AI editing; the toggle only exists for their own keys.
    /// Returns false when the word is already in the dictionary.
    pub fn add_word(&mut self, word: String) -> bool {
        let key = word.to_lowercase();
        self.removed_words
            .retain(|removed| removed.to_lowercase() != key);
        if self
            .dictionary
            .iter()
            .any(|existing| existing.to_lowercase() == key)
        {
            return false;
        }
        self.dictionary.push(word);
        true
    }

    pub fn remove_word(&mut self, word: &str) {
        self.dictionary.retain(|existing| existing != word);
        if !self
            .removed_words
            .iter()
            .any(|removed| removed.eq_ignore_ascii_case(word))
        {
            self.removed_words.push(word.to_owned());
        }
    }

    /// Dictionary words plus removed ones: neither should be suggested again.
    pub fn known_words(&self) -> impl Iterator<Item = &String> {
        self.dictionary.iter().chain(&self.removed_words)
    }

    pub fn instructions_for(&self, app_id: Option<&str>) -> Option<&str> {
        let key = app_key(app_id?);
        self.app_instructions
            .iter()
            .find(|rule| app_key(&rule.app) == key)
            .map(|rule| rule.text.trim())
            .filter(|text| !text.is_empty())
            .map(|text| {
                let mut units = 0;
                let end = text.char_indices().find(|(_, c)| {
                    units += c.len_utf16();
                    units > MAX_INSTRUCTION_LEN
                });
                end.map_or(text, |(end, _)| &text[..end])
            })
    }

    pub fn editing(&self) -> bool {
        self.cleanup.enabled || self.transcription == Source::Hosted
    }

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

    pub fn load(paths: &AppPaths) -> Result<(Self, Option<String>)> {
        match std::fs::read(&paths.config_file) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(config) => Ok((config, None)),
                Err(error) => {
                    let backup = paths.config_file.with_extension("json.bad");
                    std::fs::rename(&paths.config_file, &backup)
                        .context("cannot preserve corrupt config")?;
                    let notice = format!(
                        "Invalid config saved as config.json.bad. Started with defaults: {error}"
                    );
                    eprintln!("typeaside: {notice}");
                    Ok((Self::default(), Some(notice)))
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(paths)?;
                Ok((config, None))
            }
            Err(error) => Err(error).context("failed to read config"),
        }
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        paths.ensure()?;
        let temporary = paths.config_file.with_extension("json.tmp");
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec_pretty(self)?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(temporary, &paths.config_file).context("failed to save config")
    }
}
