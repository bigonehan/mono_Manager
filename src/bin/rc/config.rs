use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_debug")]
    pub debug: bool,
    #[serde(default)]
    pub headed: bool,
    #[serde(default = "default_browser_url")]
    pub browser_url: String,
    #[serde(default = "default_agent_browser_command")]
    pub agent_browser_command: String,
}

fn default_debug() -> bool {
    true
}
fn default_browser_url() -> String {
    "http://127.0.0.1:3000".to_string()
}
fn default_agent_browser_command() -> String {
    "agent-browser".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            debug: true,
            headed: false,
            browser_url: default_browser_url(),
            agent_browser_command: default_agent_browser_command(),
        }
    }
}

pub fn load_config() -> Result<Config> {
    let path = config_path();
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let config: Config = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(config)
}

pub fn debug_enabled(config: &Config) -> bool {
    config.debug
}

pub fn config_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("configs")
        .join("configs.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_debug_is_true() {
        let config = load_config().expect("config");
        assert!(debug_enabled(&config));
    }
}
