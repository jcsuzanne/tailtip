//! Typography utilities declared in a project's `typography.css`.
//!
//! Reads both styles:
//! - v4: `@utility u-title-100 { … }`
//! - v3: `.u-title-100 { … }` (usually inside `@layer components`)

use serde::Serialize;
use std::path::Path;
use walkdir::WalkDir;

const FILE_NAME: &str = "typography.css";
const MAX_DEPTH: usize = 8;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Utility {
    pub name: String,
    pub line: u32,
    /// Rule body, dedented.
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypoFile {
    pub path: String,
    pub utilities: Vec<Utility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Every `typography.css` under the project folder, parsed.
pub fn load(project_dir: &Path) -> Vec<TypoFile> {
    let mut paths: Vec<_> = WalkDir::new(project_dir)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !crate::scan::should_prune(e))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case(FILE_NAME))
        .map(|e| e.into_path())
        .collect();
    paths.sort_by_key(|p| p.as_os_str().len());

    paths
        .into_iter()
        .map(|p| {
            let path = p.to_string_lossy().into_owned();
            match std::fs::read_to_string(&p) {
                Ok(src) => TypoFile { path, utilities: parse(&src), error: None },
                Err(e) => TypoFile { path, utilities: vec![], error: Some(e.to_string()) },
            }
        })
        .filter(|f| f.error.is_some() || !f.utilities.is_empty())
        .collect()
}

/// Replace `/* … */` with spaces, keeping newlines so offsets and lines stay valid.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find("*/").map(|e| e + 2).unwrap_or(after.len());
        out.extend(after[..end].chars().map(|c| if c == '\n' { '\n' } else { ' ' }));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn is_class_selector(s: &str) -> bool {
    let Some(name) = s.strip_prefix('.') else { return false };
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '-')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Names a rule prelude declares as utilities, if any.
fn utility_names(prelude: &str) -> Option<Vec<String>> {
    if let Some(rest) = prelude.strip_prefix("@utility") {
        let name = rest.trim();
        return (!name.is_empty() && rest.starts_with(char::is_whitespace)).then(|| vec![name.to_string()]);
    }
    if prelude.starts_with('@') {
        return None;
    }
    let parts: Vec<&str> = prelude.split(',').map(str::trim).collect();
    parts
        .iter()
        .all(|p| is_class_selector(p))
        .then(|| parts.iter().map(|p| p[1..].to_string()).collect())
}

fn dedent(body: &str) -> String {
    let lines: Vec<&str> = body.lines().map(str::trim_end).collect();
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let last = lines.iter().rposition(|l| !l.trim().is_empty());
    let (Some(first), Some(last)) = (first, last) else { return String::new() };
    let lines = &lines[first..=last];
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| if l.len() >= indent { &l[indent..] } else { l.trim_start() })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse(src: &str) -> Vec<Utility> {
    let text = strip_comments(src);
    let line_starts: Vec<usize> =
        std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect();
    let line_of = |offset: usize| line_starts.partition_point(|&s| s <= offset) as u32;

    // Frames: Some((names, body start, line)) for a captured utility, None otherwise.
    let mut stack: Vec<Option<(Vec<String>, usize, u32)>> = vec![];
    let mut prelude_start = 0;
    let mut out = vec![];

    for (i, b) in text.bytes().enumerate() {
        match b {
            b'{' => {
                let raw = &text[prelude_start..i];
                let frame = if stack.iter().any(Option::is_some) {
                    None
                } else {
                    let at = line_of(prelude_start + (raw.len() - raw.trim_start().len()));
                    utility_names(raw.trim()).map(|names| (names, i + 1, at))
                };
                stack.push(frame);
                prelude_start = i + 1;
            }
            b'}' => {
                if let Some(Some((names, start, at))) = stack.pop() {
                    let body = dedent(&text[start..i]);
                    for name in names {
                        out.push(Utility { name, line: at, body: body.clone() });
                    }
                }
                prelude_start = i + 1;
            }
            b';' => prelude_start = i + 1,
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_utilities() {
        let css = "@utility u-title-100 {\n  font-family: var(--font-sans);\n  @apply text-large lg:text-giant;\n}\n\n/* @utility u-old { color: red } */\n@utility u-quote {\n  @apply text-huge;\n}\n";
        let u = parse(css);
        assert_eq!(u.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["u-title-100", "u-quote"]);
        assert_eq!(u[0].line, 1);
        assert_eq!(u[0].body, "font-family: var(--font-sans);\n@apply text-large lg:text-giant;");
        assert_eq!(u[1].line, 7);
    }

    #[test]
    fn v3_classes_in_layer_with_nested_media() {
        let css = "@tailwind components;\n\n@layer components {\n  .u-nav-100 {\n    font-size: 14px;\n\n    @media (--to-lg) {\n      font-size: 13px;\n    }\n  }\n\n  .u-a, .u-b { color: red; }\n  .body p { margin: 0; }\n}\n:root { --x: 1; }\n";
        let u = parse(css);
        assert_eq!(u.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["u-nav-100", "u-a", "u-b"]);
        assert_eq!(u[0].line, 4);
        assert_eq!(u[0].body, "font-size: 14px;\n\n@media (--to-lg) {\n  font-size: 13px;\n}");
        assert_eq!(u[1].line, 12);
    }
}

#[cfg(test)]
mod probe {
    /// `TYPO_DIRS=a:b cargo test probe_typography -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn probe_typography() {
        for dir in std::env::var("TYPO_DIRS").unwrap_or_default().split(':').filter(|d| !d.is_empty()) {
            for f in super::load(std::path::Path::new(dir)) {
                let names: Vec<_> = f.utilities.iter().map(|u| format!("{}@{}", u.name, u.line)).collect();
                println!("{} → {} [{}]", f.path, names.len(), names.iter().take(5).cloned().collect::<Vec<_>>().join(" "));
            }
        }
    }
}
