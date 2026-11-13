//! Tailwind v3 (and v2): run the bundled node resolver and locate tokens in the config file.

use crate::theme::{Group, Kind, Theme};
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Deserialize)]
struct ResolverOutput {
    version: Option<String>,
    source: Option<String>,
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    groups: Vec<Group>,
    error: Option<String>,
}

/// Find a usable `node` binary. GUI apps don't inherit the shell PATH, so ask a login shell.
pub fn detect_node() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    if let Ok(out) = Command::new(&shell).args(["-lc", "command -v node"]).output() {
        let found = String::from_utf8_lossy(&out.stdout);
        if let Some(line) = found.lines().map(str::trim).filter(|l| l.starts_with('/')).last() {
            return Some(line.to_string());
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let mut candidates: Vec<PathBuf> = vec![
        "/opt/homebrew/bin/node".into(),
        "/usr/local/bin/node".into(),
        format!("{home}/.volta/bin/node").into(),
        format!("{home}/.local/share/fnm/aliases/default/bin/node").into(),
    ];
    if let Ok(entries) = std::fs::read_dir(format!("{home}/.nvm/versions/node")) {
        let mut versions: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path().join("bin/node")).collect();
        versions.sort();
        candidates.extend(versions.into_iter().rev());
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

fn run_resolver(node: &str, resolver: &Path, config: &Path) -> Result<String, String> {
    let mut child = Command::new(node)
        .arg(resolver)
        .arg(config)
        .current_dir(config.parent().unwrap_or(Path::new("/")))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Impossible de lancer node ({node}) : {e}"))?;

    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut buf = String::new();
        stdout.read_to_string(&mut buf).map(|_| buf)
    });

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > TIMEOUT => {
                let _ = child.kill();
                return Err("Délai dépassé en lisant la config".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }
    reader
        .join()
        .map_err(|_| "Lecture interrompue".to_string())?
        .map_err(|e| e.to_string())
}

/// Best-effort line lookup: the key as an object property, searched after its section name.
fn find_line(lines: &[&str], section: &str, path: &[String]) -> Option<u32> {
    let key = path.last()?;
    let patterns = [format!("'{key}'"), format!("\"{key}\""), format!("{key}:"), format!("{key} :")];
    let is_key_line = |l: &str| {
        let t = l.trim_start();
        patterns.iter().any(|p| {
            t.starts_with(p.as_str())
                || (p.starts_with('\'') || p.starts_with('"')) && t.contains(p.as_str())
        })
    };
    let section_start = lines
        .iter()
        .position(|l| l.trim_start().starts_with(&format!("{section}:")) || l.contains(&format!("{section}:")))
        .unwrap_or(0);
    lines[section_start..]
        .iter()
        .position(|l| is_key_line(l))
        .map(|i| section_start + i)
        .or_else(|| lines.iter().position(|l| is_key_line(l)))
        .map(|i| i as u32 + 1)
}

pub fn resolve(node: Option<&str>, resolver: &Path, config: &Path) -> Theme {
    let config_str = config.to_string_lossy().into_owned();
    let Some(node) = node else {
        return Theme::failed(
            Kind::V3,
            &config_str,
            "node introuvable — renseigne son chemin dans les réglages",
        );
    };
    if !resolver.is_file() {
        return Theme::failed(Kind::V3, &config_str, format!("Résolveur manquant : {}", resolver.display()));
    }

    let raw = match run_resolver(node, resolver, config) {
        Ok(s) => s,
        Err(e) => return Theme::failed(Kind::V3, &config_str, e),
    };
    let out: ResolverOutput = match serde_json::from_str(&raw) {
        Ok(o) => o,
        Err(e) => return Theme::failed(Kind::V3, &config_str, format!("Réponse illisible du résolveur : {e}")),
    };
    if let Some(err) = out.error {
        return Theme::failed(Kind::V3, &config_str, err);
    }

    let source = std::fs::read_to_string(config).unwrap_or_default();
    let lines: Vec<&str> = source.lines().collect();
    let mut groups = out.groups;
    for group in &mut groups {
        for token in group.tokens.iter_mut().filter(|t| t.custom) {
            token.line = find_line(&lines, &group.key, &token.path);
            token.file = token.line.map(|_| config_str.clone());
        }
    }

    Theme {
        kind: Kind::V3,
        config_path: config_str.clone(),
        version: out.version,
        source: out.source,
        prefix: out.prefix,
        groups,
        files: vec![config_str],
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_line_after_section() {
        let src = "module.exports = {\n  theme: {\n    extend: {\n      colors: {\n        orange: '#F9AF26',\n        'beige-500': '#ede9df',\n      },\n    },\n  },\n};";
        let lines: Vec<&str> = src.lines().collect();
        assert_eq!(find_line(&lines, "colors", &["orange".into()]), Some(5));
        assert_eq!(find_line(&lines, "colors", &["beige-500".into()]), Some(6));
        assert_eq!(find_line(&lines, "colors", &["nope".into()]), None);
    }
}
