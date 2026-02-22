use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PerformanceConfig {
    pub max_parallel: Option<usize>,
    pub timeout_sec: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeymapConfig {
    pub run_parallel: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub max_parallel: Option<usize>,
    pub timeout_sec: Option<u64>,
    pub auto_yes: Option<bool>,
    pub dangerous_bypass: Option<bool>,
    pub keymap: Option<KeymapConfig>,
    pub ai: Option<AiConfig>,
    pub performance: Option<PerformanceConfig>,
}

impl AppConfig {
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|e| format!("failed to read config {}: {}", path.display(), e))?;
        serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse yaml: {}", e))
    }

    pub fn default_max_parallel(&self) -> usize {
        self.max_parallel
            .or_else(|| self.performance.as_ref().and_then(|v| v.max_parallel))
            .unwrap_or(10)
    }

    pub fn default_timeout_sec(&self) -> u64 {
        self.timeout_sec
            .or_else(|| self.performance.as_ref().and_then(|v| v.timeout_sec))
            .unwrap_or(1800)
    }

    pub fn run_parallel_key(&self) -> &str {
        self.keymap
            .as_ref()
            .and_then(|k| k.run_parallel.as_deref())
            .unwrap_or("p")
    }

    pub fn auto_yes_enabled(&self) -> bool {
        self.auto_yes.unwrap_or(true)
    }

    pub fn dangerous_bypass_enabled(&self) -> bool {
        self.dangerous_bypass.unwrap_or(true)
    }
}
