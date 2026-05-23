//! ByeTex core: LaTeX -> Typst conversion library.
//!
//! Public entry point is [`convert`]. The minimum M1 surface is intentionally tiny:
//! plain-text paragraphs round-trip identically and every backslash command produces
//! an [`Unknown`](Category::UnsupportedCommand) warning so the agent handoff is wired.

#![deny(rust_2018_idioms)]

use std::collections::HashSet;
use std::path::PathBuf;

pub mod parser;
pub mod skills;
pub mod warnings;

mod class_map;
mod emit;

pub use warnings::{Category, Range, Severity, Warning};

#[derive(Debug, Default)]
pub struct ConvertOptions {
    pub source_name: Option<String>,
    /// Directory used to resolve `\input{...}` / `\include{...}` paths
    /// relative to. When set, ByeTex expands those directives inline by
    /// reading and converting the referenced files. When `None`, includes
    /// are dropped with a `needs_manual_review` warning (the v0.1 behavior).
    pub base_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct ConvertOutput {
    pub typst: String,
    pub warnings: Vec<Warning>,
}

pub fn convert(source: &str, opts: &ConvertOptions) -> ConvertOutput {
    let tree = parser::parse(source);
    let source_name = opts.source_name.as_deref().unwrap_or("<input>");
    let visited: HashSet<PathBuf> = HashSet::new();
    let mut emitter =
        emit::Emitter::with_includes(source, source_name, opts.base_dir.clone(), visited);
    emitter.emit_root(tree.root_node());
    let (typst, warnings) = emitter.finish();
    ConvertOutput { typst, warnings }
}
