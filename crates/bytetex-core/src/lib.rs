//! ByeTex core: LaTeX -> Typst conversion library.
//!
//! Public entry point is [`convert`]. The minimum M1 surface is intentionally tiny:
//! plain-text paragraphs round-trip identically and every backslash command produces
//! an [`Unknown`](Category::UnsupportedCommand) warning so the agent handoff is wired.

#![deny(rust_2018_idioms)]

pub mod parser;
pub mod skills;
pub mod warnings;

mod emit;

pub use warnings::{Category, Range, Severity, Warning};

#[derive(Debug, Default)]
pub struct ConvertOptions {
    pub source_name: Option<String>,
}

#[derive(Debug)]
pub struct ConvertOutput {
    pub typst: String,
    pub warnings: Vec<Warning>,
}

pub fn convert(source: &str, opts: &ConvertOptions) -> ConvertOutput {
    let tree = parser::parse(source);
    let source_name = opts.source_name.as_deref().unwrap_or("<input>");
    let mut emitter = emit::Emitter::new(source, source_name);
    emitter.emit_root(tree.root_node());
    let (typst, warnings) = emitter.finish();
    ConvertOutput { typst, warnings }
}
