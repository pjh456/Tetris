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
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("config: failed to read {}: {e}", config_path.display());
                return CliConfig::default();
            }
        };
        let mut cfg: CliConfig = match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("config: failed to parse {}: {e}", config_path.display());
                return CliConfig::default();
            }
        };
        cfg.das_ms = cfg.das_ms.clamp(50, 500);
        cfg.arr_ms = cfg.arr_ms.clamp(0, 100);
        const VALID_THEMES: &[&str] = &["cyberpunk", "retro", "minimal"];
        if !VALID_THEMES.contains(&cfg.theme.as_str()) {
            cfg.theme = "cyberpunk".into();
        }
        cfg
    } else {
        CliConfig::default()
    }
}

#[allow(dead_code)]
pub fn save_config(config: &CliConfig) -> Result<()> {
    let config_dir = dirs::config_dir().context("No config dir")?.join("tetris");
    std::fs::create_dir_all(&config_dir)?;
    let content = toml::to_string_pretty(config)?;
    std::fs::write(config_dir.join("config.toml"), content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = CliConfig::default();
        assert_eq!(cfg.das_ms, 133);
        assert_eq!(cfg.arr_ms, 10);
        assert_eq!(cfg.theme, "cyberpunk");
    }

    #[test]
    fn test_load_config_no_file_returns_defaults() {
        let cfg = load_config();
        assert_eq!(cfg.das_ms, 133);
        assert_eq!(cfg.arr_ms, 10);
    }
}
