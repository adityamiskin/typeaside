use super::{AVAILABLE, assets::Asset, inventory};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, str::FromStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelId {
    #[default]
    ParakeetV3,
    BarkClean,
    Bark,
    BarkPlus,
    Cohere,
    WhisperTurbo,
}
impl ModelId {
    pub const ALL: [Self; 6] = [
        Self::ParakeetV3,
        Self::BarkClean,
        Self::Bark,
        Self::BarkPlus,
        Self::Cohere,
        Self::WhisperTurbo,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::ParakeetV3 => "parakeet-v3",
            Self::BarkClean => "bark-3-clean",
            Self::Bark => "bark-3",
            Self::BarkPlus => "bark-3-plus",
            Self::Cohere => "cohere",
            Self::WhisperTurbo => "whisper-turbo",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::ParakeetV3 => "Parakeet v3",
            Self::BarkClean => "Bark 3.0 Clean",
            Self::Bark => "Bark 3.0",
            Self::BarkPlus => "Bark 3.0 Plus",
            Self::Cohere => "Cohere Transcribe",
            Self::WhisperTurbo => "Whisper large-v3-turbo",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::ParakeetV3 => "25 European languages",
            Self::BarkClean => "English, writes what you meant: no um, no false starts",
            Self::Bark => "English, every word",
            Self::BarkPlus => "English, every word, fewer mistakes",
            Self::Cohere => "14 languages incl. CJK, select spoken language",
            Self::WhisperTurbo => "99 languages",
        }
    }
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|id| *id == self).unwrap()
    }
    pub(super) fn assets(self) -> &'static [Asset] {
        use super::assets::*;
        match self {
            Self::ParakeetV3 => &PARAKEET_V3,
            Self::BarkClean => &BARK_3_CLEAN,
            Self::Bark => &BARK_3,
            Self::BarkPlus => &BARK_3_PLUS,
            Self::Cohere => &COHERE,
            Self::WhisperTurbo => &WHISPER_TURBO,
        }
    }
    pub fn size_mb(self) -> u64 {
        self.assets()
            .iter()
            .map(|a| a.size)
            .sum::<u64>()
            .div_ceil(1_000_000)
    }
    pub fn directory(self) -> Result<PathBuf> {
        Ok(super::models_directory()?.join(self.key()))
    }
    pub fn installed(self) -> bool {
        AVAILABLE
            && self.directory().is_ok_and(|dir| {
                fs::read_to_string(dir.join("installed"))
                    .ok()
                    .zip(inventory(self, &dir).ok())
                    .is_some_and(|(saved, current)| saved == current)
            })
    }
    pub fn available(self) -> Option<Self> {
        std::iter::once(self)
            .chain(Self::ALL)
            .find(|model| model.installed())
    }
}
impl FromStr for ModelId {
    type Err = anyhow::Error;
    fn from_str(key: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|id| id.key() == key)
            .ok_or_else(|| anyhow::anyhow!("Unknown on-device model: {key}"))
    }
}
