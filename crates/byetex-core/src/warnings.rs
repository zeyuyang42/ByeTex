//! Structured warnings emitted during conversion.
//!
//! Public Serde shape is the source of truth for the sidecar `warnings.json`.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub range: Range,
    pub category: Category,
    pub severity: Severity,
    pub message: String,
    pub snippet: String,
    pub suggested_skill: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_start: u32,
    pub byte_end: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Reason a region could not be deterministically converted.
///
/// `#[serde(tag = "kind")]` makes this a tagged union with shape
/// `{ "kind": "<snake_case_name>", ...fields }` in JSON, e.g.
/// `{ "kind": "unsupported_command", "name": "\\marginpar" }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Category {
    UnsupportedCommand { name: String },
    UnsupportedEnvironment { name: String },
    CustomMacro { name: String },
    Tikz,
    ParseError { tree_sitter_node: String },
    AmbiguousMath { reason: String },
    UnknownPackage { name: String },
    DropOnly,
    NeedsManualReview { reason: String },
}
