//! Theme cache shared by the project view and cross-project search,
//! plus a file watcher that refreshes themes when configs change.

use crate::scan::{ConfigFile, Project};
use crate::theme::{Kind, Theme, Token};
use crate::{v3, v4};
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const V3_WORKERS: usize = 4;
const MAX_HITS: usize = 80;

/// How to load a theme: node binary + resolver script for v3.
#[derive(Clone)]
pub struct Loader {
    pub node: Option<String>,
    pub resolver: PathBuf,
}

impl Loader {
    fn load(&self, config: &ConfigFile) -> Theme {
        let path = Path::new(&config.path);
        match config.kind {
            Kind::V4 => v4::parse(path),
            Kind::V3 => v3::resolve(self.node.as_deref(), &self.resolver, path),
        }
    }
}

#[derive(Default)]
pub struct Index {
    themes: Mutex<HashMap<String, (u64, Theme)>>,
    building: AtomicBool,
    watcher: Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub project: String,
    pub project_path: String,
    pub config_path: String,
    pub kind: Kind,
    pub prefix: String,
    pub group: String,
    pub token: Token,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    done: usize,
    total: usize,
}

fn mtime(path: &str) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Freshness stamp: latest mtime of every file that contributed to the theme.
fn stamp(theme: &Theme) -> u64 {
    theme
        .files
        .iter()
        .chain(std::iter::once(&theme.config_path))
        .map(|f| mtime(f))
        .max()
        .unwrap_or(0)
}

impl Index {
    pub fn get(&self, loader: &Loader, config: &ConfigFile) -> Theme {
        if let Some((s, theme)) = self.themes.lock().unwrap().get(&config.path) {
            if *s == stamp(theme) {
                return theme.clone();
            }
        }
        let theme = loader.load(config);
        if theme.error.is_none() {
            self.themes
                .lock()
                .unwrap()
                .insert(config.path.clone(), (stamp(&theme), theme.clone()));
        }
        theme
    }

    fn cached(&self, path: &str) -> Option<Theme> {
        self.themes.lock().unwrap().get(path).map(|(_, t)| t.clone())
    }

    /// Load every config in the background (v3 in parallel), then watch their files.
    pub fn build(self: &Arc<Self>, app: AppHandle, loader: Loader, projects: Vec<Project>) {
        if self.building.swap(true, Ordering::SeqCst) {
            return;
        }
        let index = Arc::clone(self);
        std::thread::spawn(move || {
            let configs: Vec<ConfigFile> = projects.iter().flat_map(|p| p.configs.clone()).collect();
            let total = configs.len();
            let done = Mutex::new(0usize);
            let report = |app: &AppHandle| {
                let mut d = done.lock().unwrap();
                *d += 1;
                let _ = app.emit("index-progress", Progress { done: *d, total });
            };

            let (v4s, v3s): (Vec<_>, Vec<_>) = configs.iter().partition(|c| c.kind == Kind::V4);
            for c in v4s {
                index.get(&loader, c);
                report(&app);
            }
            let queue = Mutex::new(v3s);
            std::thread::scope(|s| {
                for _ in 0..V3_WORKERS {
                    s.spawn(|| loop {
                        let next = queue.lock().unwrap().pop();
                        let Some(c) = next else { break };
                        index.get(&loader, c);
                        report(&app);
                    });
                }
            });

            index.watch(&app, &loader, &configs);
            index.building.store(false, Ordering::SeqCst);
            let _ = app.emit("index-ready", total);
        });
    }

    pub fn search(&self, projects: &[Project], hidden: &[String], query: &str) -> Vec<Hit> {
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return vec![];
        }
        let mut hits: Vec<(u8, Hit)> = vec![];
        for project in projects.iter().filter(|p| !hidden.contains(&p.path)) {
            for config in &project.configs {
                let Some(theme) = self.cached(&config.path) else { continue };
                for group in &theme.groups {
                    for token in group.tokens.iter().filter(|t| t.custom) {
                        let name = token.name.to_lowercase();
                        let value = token.resolved.as_deref().unwrap_or(&token.value).to_lowercase();
                        let score = if name == q {
                            4
                        } else if name.starts_with(&q) {
                            3
                        } else if name.contains(&q) {
                            2
                        } else if value.contains(&q) {
                            1
                        } else {
                            continue;
                        };
                        hits.push((
                            score,
                            Hit {
                                project: project.name.clone(),
                                project_path: project.path.clone(),
                                config_path: config.path.clone(),
                                kind: theme.kind,
                                prefix: theme.prefix.clone(),
                                group: group.key.clone(),
                                token: token.clone(),
                            },
                        ));
                    }
                }
            }
        }
        hits.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| (a.1.group != "colors").cmp(&(b.1.group != "colors")))
                .then_with(|| a.1.token.name.cmp(&b.1.token.name))
                .then_with(|| a.1.project.cmp(&b.1.project))
        });
        hits.into_iter().take(MAX_HITS).map(|(_, h)| h).collect()
    }

    /// Watch the folders of every contributing file; reload and notify on change.
    fn watch(self: &Arc<Self>, app: &AppHandle, loader: &Loader, configs: &[ConfigFile]) {
        // file → configs depending on it
        let mut deps: HashMap<PathBuf, Vec<ConfigFile>> = HashMap::new();
        for c in configs {
            let mut files: HashSet<String> = HashSet::from([c.path.clone()]);
            if let Some(theme) = self.cached(&c.path) {
                files.extend(theme.files);
            }
            for f in files {
                deps.entry(PathBuf::from(f)).or_default().push(c.clone());
            }
        }
        let dirs: HashSet<PathBuf> = deps.keys().filter_map(|f| f.parent().map(Path::to_path_buf)).collect();

        let index = Arc::clone(self);
        let app_handle = app.clone();
        let loader = loader.clone();
        let debouncer = new_debouncer(Duration::from_millis(400), move |res: DebounceEventResult| {
            let Ok(events) = res else { return };
            let mut changed: Vec<ConfigFile> = vec![];
            for event in events {
                if let Some(cs) = deps.get(&event.path) {
                    for c in cs {
                        if !changed.iter().any(|x| x.path == c.path) {
                            changed.push(c.clone());
                        }
                    }
                }
            }
            if changed.is_empty() {
                return;
            }
            for c in &changed {
                index.themes.lock().unwrap().remove(&c.path);
                index.get(&loader, c);
            }
            let paths: Vec<String> = changed.into_iter().map(|c| c.path).collect();
            let _ = app_handle.emit("config-changed", paths);
        });

        match debouncer {
            Ok(mut d) => {
                for dir in dirs {
                    let _ = d.watcher().watch(&dir, RecursiveMode::NonRecursive);
                }
                *self.watcher.lock().unwrap() = Some(d);
            }
            Err(e) => eprintln!("[tailtip] watcher: {e}"),
        }
    }
}
