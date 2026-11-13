use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    V3,
    V4,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub name: String,
    /// Value as written in the config (v4) or resolved by Tailwind (v3).
    pub value: String,
    /// v4 only: value with `var(--x)` references resolved, when it differs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<String>,
    /// Defined by the project (vs. Tailwind defaults).
    pub custom: bool,
    /// v4 only: CSS variable name, e.g. `--color-ivory`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// v3 only: key path inside the theme section (used to locate the line).
    #[serde(default, skip_serializing)]
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub key: String,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub kind: Kind,
    pub config_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// v3: "project" | "bundled" — which Tailwind resolved the theme.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub prefix: String,
    pub groups: Vec<Group>,
    /// Files that contributed tokens (v4 follows local @import).
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Theme {
    pub fn failed(kind: Kind, config_path: &str, error: impl Into<String>) -> Self {
        Theme {
            kind,
            config_path: config_path.to_string(),
            version: None,
            source: None,
            prefix: String::new(),
            groups: vec![],
            files: vec![config_path.to_string()],
            error: Some(error.into()),
        }
    }
}
