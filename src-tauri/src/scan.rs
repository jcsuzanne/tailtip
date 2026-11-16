//! Finds Tailwind projects under a set of root folders.
//!
//! - v3: any `tailwind.config.{js,cjs,mjs,ts,cts,mts}`; the project is its folder.
//! - v4: any `.css` file that imports `tailwindcss`; the project is the nearest
//!   ancestor holding a `package.json` (or the CSS folder itself).

use crate::theme::Kind;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;
use walkdir::{DirEntry, WalkDir};

const MAX_DEPTH: usize = 8;
const MAX_CSS_BYTES: u64 = 512 * 1024;

const PRUNE: &[&str] = &[
    "node_modules",
    "vendor",
    "dist",
    "build",
    "out",
    "target",
    "coverage",
    "tmp",
    "temp",
    "logs",
    "Library",
    "SteamLibrary",
    "wp-admin",
    "wp-includes",
    "bower_components",
    "Pods",
    "DerivedData",
];

static V3_CONFIG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^tailwind\.config\.(js|cjs|mjs|ts|cts|mts)$").unwrap());
static V4_ENTRY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"@import\s+(?:url\(\s*)?["']tailwindcss(?:/theme(?:\.css)?)?["']"#).unwrap()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    pub path: String,
    pub kind: Kind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub path: String,
    /// Scan root the project was found under.
    pub root: String,
    pub configs: Vec<ConfigFile>,
    /// Latest modification time of its config files (unix seconds).
    pub modified: u64,
}

pub(crate) fn should_prune(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') || name.ends_with(".app") || PRUNE.contains(&name.as_ref()) {
        return true;
    }
    // WordPress plugins ship their own Tailwind builds — noise.
    if name == "plugins" {
        if let Some(parent) = entry.path().parent().and_then(|p| p.file_name()) {
            return parent == "wp-content";
        }
    }
    false
}

/// Folder names that say nothing about which project they belong to.
const GENERIC: &[&str] = &[
    "web", "app", "apps", "src", "site", "www", "public", "frontend", "front", "client", "theme", "themes",
    "wp-content", "packages", "resources", "assets", "styles", "css",
];

fn meaningful_ancestor(path: &Path) -> Option<String> {
    path.ancestors()
        .skip(1)
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .find(|n| !GENERIC.contains(&n.as_str()))
}

fn modified(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn project_root_for_css(css: &Path, scan_root: &Path) -> PathBuf {
    let start = css.parent().unwrap_or(css);
    let mut dir = start;
    loop {
        if dir.join("package.json").is_file() {
            return dir.to_path_buf();
        }
        if dir == scan_root {
            break;
        }
        match dir.parent() {
            Some(p) if p.starts_with(scan_root) => dir = p,
            _ => break,
        }
    }
    start.to_path_buf()
}

pub fn scan(roots: &[String]) -> Vec<Project> {
    let mut by_path: BTreeMap<PathBuf, Project> = BTreeMap::new();

    for root in roots {
        let root_path = PathBuf::from(root);
        if !root_path.is_dir() {
            continue;
        }
        let walker = WalkDir::new(&root_path)
            .max_depth(MAX_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !should_prune(e));

        for entry in walker.filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            let path = entry.path();

            let found = if V3_CONFIG.is_match(&name) {
                Some((path.parent().unwrap_or(path).to_path_buf(), Kind::V3))
            } else if name.ends_with(".css") {
                let small = entry.metadata().map(|m| m.len() <= MAX_CSS_BYTES).unwrap_or(false);
                let is_entry = small
                    && std::fs::read_to_string(path)
                        .map(|s| V4_ENTRY.is_match(&s))
                        .unwrap_or(false);
                is_entry.then(|| (project_root_for_css(path, &root_path), Kind::V4))
            } else {
                None
            };

            if let Some((project_dir, kind)) = found {
                let project = by_path.entry(project_dir.clone()).or_insert_with(|| Project {
                    name: project_dir
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| project_dir.to_string_lossy().into_owned()),
                    path: project_dir.to_string_lossy().into_owned(),
                    root: root.clone(),
                    configs: vec![],
                    modified: 0,
                });
                project.modified = project.modified.max(modified(path));
                project.configs.push(ConfigFile {
                    path: path.to_string_lossy().into_owned(),
                    kind,
                });
            }
        }
    }

    let mut projects: Vec<Project> = by_path.into_values().collect();
    for p in &mut projects {
        // Component stylesheets often `@import "tailwindcss"` just to use @apply.
        // When several v4 entries exist, keep only those that actually define tokens.
        let v4: Vec<usize> = (0..p.configs.len()).filter(|&i| p.configs[i].kind == Kind::V4).collect();
        if v4.len() > 1 {
            let with_tokens: Vec<bool> = v4
                .iter()
                .map(|&i| !crate::v4::parse(Path::new(&p.configs[i].path)).groups.is_empty())
                .collect();
            if with_tokens.iter().any(|&t| t) {
                let mut idx = 0;
                p.configs.retain(|c| {
                    if c.kind != Kind::V4 {
                        return true;
                    }
                    let keep = with_tokens[idx];
                    idx += 1;
                    keep
                });
            }
        }
        // Prefer files named like a config first (tailwind-config.css, tailwind.css…).
        p.configs.sort_by_key(|c| {
            let file = Path::new(&c.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            (!file.contains("tailwind"), c.path.len())
        });
    }
    // Disambiguate generic or duplicated folder names: "web" → "modern-index / web".
    let mut counts: std::collections::HashMap<String, usize> = Default::default();
    for p in &projects {
        *counts.entry(p.name.clone()).or_default() += 1;
    }
    for p in &mut projects {
        if counts[&p.name] > 1 || GENERIC.contains(&p.name.as_str()) {
            if let Some(owner) = meaningful_ancestor(Path::new(&p.path)) {
                p.name = format!("{owner} / {}", p.name);
            }
        }
    }
    projects.sort_by_key(|p| p.name.to_lowercase());
    projects
}
