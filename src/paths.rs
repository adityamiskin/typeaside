use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
    pub history_file: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let dirs =
            ProjectDirs::from("", "", "typeaside").context("cannot resolve app directories")?;
        let config_dir = dirs.config_dir().to_path_buf();
        let data_dir = dirs.data_dir().to_path_buf();
        Ok(Self {
            config_file: config_dir.join("config.json"),
            history_file: data_dir.join("history.jsonl"),
            config_dir,
            data_dir,
        })
    }

    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }
}
