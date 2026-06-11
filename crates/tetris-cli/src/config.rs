use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub das_ms: u32,
    pub arr_ms: u32,
    pub theme: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            das_ms: 133,
            arr_ms: 10,
            theme: "cyberpunk".into(),
        }
    }
}

pub fn load_config() -> CliConfig {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let config_path = config_dir.join("tetris").join("config.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        let mut cfg: CliConfig = toml::from_str(&content).unwrap_or_default();
        cfg.das_ms = cfg.das_ms.clamp(50, 500);
        cfg.arr_ms = cfg.arr_ms.clamp(0, 100);
        cfg
    } else {
        CliConfig::default()
    }
}

pub fn save_config(config: &CliConfig) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("No config dir")?
        .join("tetris");
    std::fs::create_dir_all(&config_dir)?;
    let content = toml::to_string_pretty(config)?;
    std::fs::write(config_dir.join("config.toml"), content)?;
    Ok(())
}
