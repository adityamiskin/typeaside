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

impl AppCategory {
    pub const ALL: [Self; 5] = [
        Self::Personal,
        Self::Work,
        Self::Email,
        Self::Code,
        Self::Other,
    ];

    pub fn of(app_id: Option<&str>) -> Self {
        let id = app_id.unwrap_or_default().to_ascii_lowercase();
        let has = |names: &[&str]| names.iter().any(|name| id.contains(name));
        let name = id.strip_suffix(".exe").unwrap_or(&id);
        let code = name.starts_with("jetbrains-")
            || name.starts_with("com.jetbrains.")
            || CODE_APPS.contains(&name);
        if code {
            Self::Code
        } else if has(&[
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub hotkey: String,
    pub mode: RecordingMode,
    pub input_device: Option<String>,
    pub transcription: Source,
    pub stt: Endpoint,
    pub language: Option<String>,
    pub cleanup: Cleanup,
    pub dictionary: Vec<String>,
    /// Words the user removed; corrections never suggest them again.
    pub removed_words: Vec<String>,
    pub settings_sync: Option<crate::settings_sync::SyncState>,
    pub auto_submit: bool,
    pub auto_learn: bool,
    pub screen_context: bool,
    pub command_hotkey: Option<String>,
    pub translate_hotkey: Option<String>,
    pub translate_to: String,
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
            hotkey: if cfg!(target_os = "macos") {
                "FN"
            } else {
                "RIGHTCTRL"
            }
            .into(),
            mode: RecordingMode::Hold,
            input_device: None,
            transcription: Source::OwnKey,
            stt: Endpoint::new("https://api.groq.com/openai/v1", "whisper-large-v3-turbo"),
            language: None,
            cleanup: Cleanup::default(),
            dictionary: Vec::new(),
            removed_words: Vec::new(),
            settings_sync: None,
            auto_submit: false,
            auto_learn: true,
            screen_context: true,
            command_hotkey: Some("RIGHTALT".into()),
            translate_hotkey: Some("RIGHTALT+RIGHTSHIFT".into()),
            translate_to: "English".into(),
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

#[cfg(test)]
mod tests {
    use super::{AppCategory, Styles, Tone};

    #[test]
    fn app_categories_match_real_ids_without_code_substrings() {
        use AppCategory::*;
        for id in [
            "code",
            "Code.exe",
            "code-oss",
            "Code - OSS.exe",
            "com.visualstudio.code",
            "com.visualstudio.code.oss",
            "com.visualstudio.code.insiders",
            "code-insiders",
            "VSCodium",
            "VSCodium.exe",
            "com.vscodium.codium",
            "codium",
            "Cursor",
            "Cursor.exe",
            "com.todesktop.230313mzl4w4u92",
            "cursor",
            "dev.zed.Zed",
            "Zed",
            "com.jetbrains.intellij",
            "jetbrains-idea-ce",
            "idea64.exe",
            "pycharm64.exe",
            "com.jetbrains.pycharm.ce",
            "jetbrains-rustrover",
            "com.sublimetext.4",
            "sublime_text.exe",
            "sublime_text",
            "neovide",
            "dev.neovide.neovide",
            "com.neovide.neovide",
            "Alacritty",
            "org.alacritty.Alacritty",
            "org.alacritty",
            "net.kovidgoyal.kitty",
            "kitty",
            "com.mitchellh.ghostty",
            "Ghostty",
            "org.wezfurlong.wezterm",
            "com.github.wez.wezterm",
            "wezterm-gui.exe",
            "foot",
            "footclient",
            "org.kde.konsole",
            "org.gnome.Terminal",
            "gnome-terminal-server",
            "org.gnome.Ptyxis",
            "com.apple.Terminal",
            "com.googlecode.iterm2",
            "dev.warp.Warp-Stable",
            "Warp.exe",
            "WindowsTerminal.exe",
            "wt.exe",
        ] {
            assert_eq!(AppCategory::of(Some(id)), Code, "{id}");
        }
        for (id, expected) in [
            (Some("com.slack.Slack"), Work),
            (Some("org.telegram.desktop"), Personal),
            (Some("org.mozilla.Thunderbird"), Email),
            (Some("org.mozilla.firefox"), Other),
            (Some("com.example.code-review"), Other),
            (Some("com.example.decode"), Other),
            (Some("com.example.cursor"), Other),
            (Some("com.example.kitty"), Other),
            (Some("codepen"), Other),
            (Some("linear-code-review"), Work),
            (Some("org.gnome.TerminalPreferences"), Other),
            (None, Other),
        ] {
            assert_eq!(AppCategory::of(id), expected, "{id:?}");
        }
        let old: Styles = serde_json::from_str(r#"{"email":"very_casual"}"#).unwrap();
        assert_eq!(old.code, Tone::Casual);
        assert_eq!(old.email, Tone::VeryCasual);
    }
}
