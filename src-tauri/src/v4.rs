//! Tailwind v4: read `@theme` blocks from a CSS entry file and its local imports.

use crate::theme::{Group, Kind, Theme, Token};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const MAX_IMPORT_DEPTH: usize = 4;

static IMPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"@import\s+(?:url\(\s*)?["']([^"']+)["']\s*\)?([^;]*);"#).unwrap()
});
static PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"prefix\(\s*([\w-]+)\s*\)").unwrap());
static THEME_START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@theme\b[^{;]*\{").unwrap());
static DECL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(--[A-Za-z0-9_-]+)\s*:\s*([^;{}]+?)\s*(?:;|\})").unwrap());
static VAR_REF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var\(\s*(--[A-Za-z0-9_-]+)\s*(?:,\s*([^()]*(?:\([^()]*\))?[^()]*))?\)").unwrap());

/// Theme namespace → group key. Longest / most specific prefixes first.
const NAMESPACES: &[(&str, &str)] = &[
    ("color-", "colors"),
    ("font-weight-", "fontWeight"),
    ("font-size-", ""),
    ("font-", "fontFamily"),
    ("text-", "fontSize"),
    ("tracking-", "letterSpacing"),
    ("leading-", "lineHeight"),
    ("breakpoint-", "screens"),
    ("container-", "containers"),
    ("spacing-", "spacing"),
    ("radius-", "borderRadius"),
    ("inset-shadow-", "insetShadow"),
    ("drop-shadow-", "dropShadow"),
    ("text-shadow-", "textShadow"),
    ("shadow-", "boxShadow"),
    ("blur-", "blur"),
    ("perspective-", "perspective"),
    ("aspect-", "aspectRatio"),
    ("ease-", "transitionTimingFunction"),
    ("animate-", "animation"),
];

struct Decl {
    name: String,
    value: String,
    file: String,
    line: u32,
}

#[derive(Default)]
struct Collected {
    theme: Vec<Decl>,
    vars: HashMap<String, String>,
    files: Vec<String>,
    prefix: Option<String>,
}

/// Replace comments with spaces, keeping newlines so offsets map to lines.
fn strip_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            let end = src[i + 2..].find("*/").map(|e| i + 2 + e + 2).unwrap_or(bytes.len());
            for ch in src[i..end].chars() {
                out.push(if ch == '\n' { '\n' } else { ' ' });
            }
            i = end;
        } else {
            let ch = src[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn line_at(src: &str, offset: usize) -> u32 {
    src[..offset].bytes().filter(|&b| b == b'\n').count() as u32 + 1
}

/// Byte ranges of the bodies of every `@theme { … }` block.
fn theme_blocks(src: &str) -> Vec<(usize, usize)> {
    let mut blocks = vec![];
    for m in THEME_START.find_iter(src) {
        let start = m.end();
        let mut depth = 1;
        let mut end = src.len();
        for (i, b) in src[start..].bytes().enumerate() {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        blocks.push((start, end));
    }
    blocks
}

fn resolve_import(from: &Path, target: &str) -> Option<PathBuf> {
    if target.starts_with("http:") || target.starts_with("https:") || target.starts_with("//") {
        return None;
    }
    let dir = from.parent()?;
    let base = if target.starts_with('/') {
        PathBuf::from(target)
    } else if target.starts_with('.') {
        dir.join(target)
    } else {
        // Bare package specifiers (tw-animate-css…) are third-party: not the project's tokens.
        return None;
    };
    [base.clone(), base.with_extension("css")]
        .into_iter()
        .find(|p| p.is_file())
}

fn collect(file: &Path, depth: usize, seen: &mut HashSet<PathBuf>, out: &mut Collected) {
    let Ok(canonical) = file.canonicalize() else { return };
    if !seen.insert(canonical) {
        return;
    }
    let Ok(raw) = std::fs::read_to_string(file) else { return };
    let src = strip_comments(&raw);
    let file_str = file.to_string_lossy().into_owned();
    let mut contributed = false;

    for cap in IMPORT.captures_iter(&src) {
        let target = &cap[1];
        if target == "tailwindcss" || target.starts_with("tailwindcss/") {
            if let Some(p) = PREFIX.captures(&cap[2]) {
                out.prefix = Some(p[1].to_string());
            }
            continue;
        }
        if depth < MAX_IMPORT_DEPTH {
            if let Some(next) = resolve_import(file, target) {
                collect(&next, depth + 1, seen, out);
            }
        }
    }

    let blocks = theme_blocks(&src);
    for &(start, end) in &blocks {
        let body = &src[start..end];
        for cap in DECL.captures_iter(body) {
            let m = cap.get(1).unwrap();
            let name = cap[1].to_string();
            let value = cap[2].trim().to_string();
            out.vars.insert(name.clone(), value.clone());
            out.theme.push(Decl {
                name,
                value,
                file: file_str.clone(),
                line: line_at(&src, start + m.start()),
            });
            contributed = true;
        }
    }

    // Variables defined outside @theme (:root…) — used to resolve var() references.
    for cap in DECL.captures_iter(&src) {
        let pos = cap.get(0).unwrap().start();
        if blocks.iter().any(|&(s, e)| pos >= s && pos < e) {
            continue;
        }
        out.vars.entry(cap[1].to_string()).or_insert_with(|| cap[2].trim().to_string());
    }

    if contributed || depth == 0 {
        out.files.push(file_str);
    }
}

fn resolve_vars(value: &str, vars: &HashMap<String, String>) -> String {
    let mut current = value.to_string();
    for _ in 0..6 {
        if !current.contains("var(") {
            break;
        }
        let next = VAR_REF
            .replace_all(&current, |c: &regex::Captures| {
                vars.get(&c[1])
                    .cloned()
                    .or_else(|| c.get(2).map(|f| f.as_str().trim().to_string()))
                    .unwrap_or_else(|| c[0].to_string())
            })
            .into_owned();
        if next == current {
            break;
        }
        current = next;
    }
    current
}

fn classify(var_name: &str) -> (String, String) {
    let bare = var_name.trim_start_matches("--");
    if bare == "spacing" {
        return ("spacing".into(), "base".into());
    }
    for (ns, group) in NAMESPACES {
        if let Some(rest) = bare.strip_prefix(ns) {
            if group.is_empty() {
                break;
            }
            return (group.to_string(), rest.to_string());
        }
    }
    ("other".into(), bare.to_string())
}

fn project_tailwind_version(entry: &Path) -> Option<String> {
    let mut dir = entry.parent();
    while let Some(d) = dir {
        let pkg = d.join("node_modules/tailwindcss/package.json");
        if let Ok(s) = std::fs::read_to_string(&pkg) {
            let v: serde_json::Value = serde_json::from_str(&s).ok()?;
            return v.get("version").and_then(|v| v.as_str()).map(String::from);
        }
        if d.join("package.json").is_file() {
            break;
        }
        dir = d.parent();
    }
    None
}

pub fn parse(entry: &Path) -> Theme {
    let entry_str = entry.to_string_lossy().into_owned();
    if !entry.is_file() {
        return Theme::failed(Kind::V4, &entry_str, "Fichier introuvable");
    }

    let mut collected = Collected::default();
    collect(entry, 0, &mut HashSet::new(), &mut collected);

    // Later declarations override earlier ones, like the CSS cascade.
    let mut order: Vec<String> = vec![];
    let mut last: HashMap<String, Decl> = HashMap::new();
    for decl in collected.theme {
        if decl.value == "initial" {
            last.remove(&decl.name);
            continue;
        }
        if !last.contains_key(&decl.name) {
            order.push(decl.name.clone());
        }
        last.insert(decl.name.clone(), decl);
    }

    let mut groups: Vec<Group> = vec![];
    for name in order {
        let Some(decl) = last.remove(&name) else { continue };
        let bare = name.trim_start_matches("--");
        // Sub-properties like --text-xs--line-height belong to their parent token.
        if bare.contains("--") {
            continue;
        }
        let (group_key, token_name) = classify(&name);
        let resolved = resolve_vars(&decl.value, &collected.vars);
        let token = Token {
            name: token_name,
            resolved: (resolved != decl.value).then_some(resolved),
            value: decl.value,
            custom: true,
            var_name: Some(name.clone()),
            file: Some(decl.file),
            line: Some(decl.line),
            path: vec![],
        };
        match groups.iter_mut().find(|g| g.key == group_key) {
            Some(g) => g.tokens.push(token),
            None => groups.push(Group { key: group_key, tokens: vec![token] }),
        }
    }

    Theme {
        kind: Kind::V4,
        config_path: entry_str,
        version: project_tailwind_version(entry),
        source: None,
        prefix: collected.prefix.unwrap_or_default(),
        groups,
        files: collected.files,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_namespaces() {
        assert_eq!(classify("--color-ivory-100"), ("colors".into(), "ivory-100".into()));
        assert_eq!(classify("--font-weight-bold"), ("fontWeight".into(), "bold".into()));
        assert_eq!(classify("--font-sans"), ("fontFamily".into(), "sans".into()));
        assert_eq!(classify("--font-size-xxs"), ("other".into(), "font-size-xxs".into()));
        assert_eq!(classify("--spacing"), ("spacing".into(), "base".into()));
        assert_eq!(classify("--z-index-ui"), ("other".into(), "z-index-ui".into()));
    }

    #[test]
    fn resolves_var_references() {
        let mut vars = HashMap::new();
        vars.insert("--brand".to_string(), "#ff0000".to_string());
        assert_eq!(resolve_vars("var(--brand)", &vars), "#ff0000");
        assert_eq!(resolve_vars("var(--missing, 4px)", &vars), "4px");
    }

    #[test]
    fn parses_theme_blocks_and_prefix() {
        let dir = std::env::temp_dir().join(format!("tailtip-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vars.css"), ":root { --brand: #123456; }\n@theme { --color-extra: #000; }").unwrap();
        std::fs::write(
            dir.join("app.css"),
            "@import 'tailwindcss' prefix(tw);\n@import './vars.css';\n/* --color-ignored: red; */\n@theme {\n  --color-brand: var(--brand);\n  --text-xs--line-height: 1;\n  --breakpoint-xl: 1400px;\n}\n",
        )
        .unwrap();
        let theme = parse(&dir.join("app.css"));
        assert_eq!(theme.prefix, "tw");
        let colors = theme.groups.iter().find(|g| g.key == "colors").unwrap();
        let names: Vec<_> = colors.tokens.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["extra", "brand"]);
        assert_eq!(colors.tokens[1].resolved.as_deref(), Some("#123456"));
        assert_eq!(colors.tokens[1].line, Some(5));
        assert!(theme.groups.iter().any(|g| g.key == "screens"));
        assert!(!theme.groups.iter().any(|g| g.tokens.iter().any(|t| t.name.contains("--"))));
        std::fs::remove_dir_all(&dir).ok();
    }
}
