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
        let (config_dir, data_dir) = if let Some(root) = std::env::var_os("TYPEASIDE_HOME") {
            let root = PathBuf::from(root);
            anyhow::ensure!(
                root.is_absolute(),
                "TYPEASIDE_HOME must be an absolute path"
            );
            (root.join("config"), root.join("data"))
        } else {
            let dirs =
                ProjectDirs::from("", "", "typeaside").context("cannot resolve app directories")?;
            (
                dirs.config_dir().to_path_buf(),
                dirs.data_dir().to_path_buf(),
            )
        };
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
