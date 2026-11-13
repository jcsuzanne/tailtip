use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Folders scanned for Tailwind projects.
    pub roots: Vec<String>,
    /// Project paths pinned at the top of the list.
    pub pinned: Vec<String>,
    /// Project paths hidden from the list.
    pub hidden: Vec<String>,
    /// Explicit path to `node` (auto-detected when empty).
    pub node_path: Option<String>,
    /// Global shortcut, Tauri accelerator syntax.
    pub shortcut: String,
    /// Recently opened project paths, most recent first.
    pub recent: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_default();
        let candidates = [
            PathBuf::from("/Volumes/DEV"),
            PathBuf::from("/Volumes/HOME/DEV_HOME"),
            home.join("Developer"),
            home.join("Projects"),
            home.join("Sites"),
            home.join("dev"),
            home.join("code"),
        ];
        Settings {
            roots: candidates
                .iter()
                .filter(|p| p.is_dir())
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            pinned: vec![],
            hidden: vec![],
            node_path: None,
            shortcut: "Alt+Super+T".into(),
            recent: vec![],
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }
}
