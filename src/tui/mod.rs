use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const CONFIG_CANDIDATES: [&str; 2] = ["configs.yaml", "config.yaml"];

pub(crate) fn open_ui() -> Result<String, String> {
    let tui_enabled = crate::action_load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::tui_enabled);
    if !tui_enabled {
        return Err(
            "tui is disabled in config (`client.tui: false`). run `orc activate-tui` first."
                .to_string(),
        );
    }
    crate::action_run_ui()
}

pub(crate) fn activate_tui() -> Result<String, String> {
    let path = action_resolve_writable_config_path();
    let mut doc = action_load_config_doc(&path)?;
    action_set_tui_enabled(&mut doc, true);
    let raw = serde_yaml::to_string(&doc).map_err(|e| format!("failed to encode yaml: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok(format!("tui activated: {}", path.display()))
}

fn action_resolve_writable_config_path() -> PathBuf {
    let root = crate::action_source_root();
    for rel in CONFIG_CANDIDATES {
        let path = root.join(rel);
        if path.exists() {
            return path;
        }
    }
    root.join(CONFIG_CANDIDATES[0])
}

fn action_load_config_doc(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read config {}: {}", path.display(), e))?;
    serde_yaml::from_str(&raw).map_err(|e| format!("failed to parse yaml {}: {}", path.display(), e))
}

fn action_set_tui_enabled(doc: &mut Value, enabled: bool) {
    let root = if let Value::Mapping(map) = doc {
        map
    } else {
        *doc = Value::Mapping(Mapping::new());
        let Value::Mapping(map) = doc else {
            return;
        };
        map
    };

    let client_key = Value::String("client".to_string());
    if !root.contains_key(&client_key) {
        root.insert(client_key.clone(), Value::Mapping(Mapping::new()));
    }
    let Some(client_value) = root.get_mut(&client_key) else {
        return;
    };
    let client = if let Value::Mapping(map) = client_value {
        map
    } else {
        *client_value = Value::Mapping(Mapping::new());
        let Value::Mapping(map) = client_value else {
            return;
        };
        map
    };
    client.insert(Value::String("tui".to_string()), Value::Bool(enabled));
}
