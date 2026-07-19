use std::path::PathBuf;

use crate::api::IssueSummary;

fn recent_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
        })?;

    Some(base.join("linear-tui").join("recently-viewed.json"))
}

pub fn load_recent() -> Vec<IssueSummary> {
    recent_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_recent(issues: &[IssueSummary]) {
    let Some(path) = recent_path() else {
        return;
    };

    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    if let Ok(json) = serde_json::to_string(issues) {
        let _ = std::fs::write(path, json);
    }
}
