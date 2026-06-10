//! ByeTex core: LaTeX -> Typst conversion library.
//!
//! Public entry point is [`convert`]. The minimum M1 surface is intentionally tiny:
//! plain-text paragraphs round-trip identically and every backslash command produces
//! an [`Unknown`](Category::UnsupportedCommand) warning so the agent handoff is wired.

#![deny(rust_2018_idioms)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub mod bib;
pub mod parser;
pub mod project;
pub mod skills;
pub mod warnings;

mod class_map;
mod document;
pub(crate) mod emit;
pub(crate) mod package_macros;
mod typst_diag;
pub use typst_diag::{parse_typst_errors, TypstError};

/// Test-support surface: thin wrappers over internal functions exposed for
/// integration tests in `tests/`. Not part of the public API.
#[doc(hidden)]
pub mod __test_support {
    pub fn lookup_math_symbol(name: &str) -> Option<&'static str> {
        super::emit::lookup_math_symbol(name)
    }
    pub fn wrap_for_command_name(name: &str) -> Option<(&'static str, &'static str)> {
        super::emit::wrap_for_command_name(name)
    }
    /// Returns true if the name is seeded as an always-on KATEX_BUILTIN macro.
    pub fn is_katex_builtin(name: &str) -> bool {
        super::package_macros::KATEX_BUILTIN
            .iter()
            .any(|(n, _)| *n == name)
    }
}

mod skill_map;
pub use skill_map::default_skill_for;

mod source_map;
pub use source_map::{resolve_error_line, NodeOutput};

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
    /// Assets (images, bibliography files) that the emitter successfully
    /// resolved on disk. The project layer uses this list to copy files
    /// into the output directory. Only populated when `base_dir` is set.
    pub asset_refs: Vec<AssetRef>,
    /// Class-specific metadata captured from the LaTeX preamble (e.g. ACM
    /// author fields like `institution`, `email`, `orcid`). Keys are command
    /// names without the leading backslash; values are the rendered content of
    /// the first argument. Populated regardless of whether a class template was
    /// detected — callers can read or forward to other tools.
    pub class_metadata: HashMap<String, String>,
    /// Content-anchored provenance map (`.typ` text → LaTeX source span) per
    /// emitted node. Empty unless produced via `convert_capturing_source_map`.
    pub source_map: Vec<source_map::NodeOutput>,
}

/// A single asset that the emitter resolved on disk during conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRef {
    pub kind: AssetKind,
    /// Path string as written in the emitted Typst source (e.g. `"fig/foo.pdf"`).
    pub typst_path: String,
    /// Absolute or base-dir-relative path of the asset on the host filesystem.
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Image,
    Bibliography,
}

pub fn convert(source: &str, opts: &ConvertOptions) -> ConvertOutput {
    convert_with_macros(source, opts, HashMap::new(), HashSet::new(), false)
}

/// Like [`convert`], but also records the content-anchored source map on the
/// returned `ConvertOutput.source_map`. Used by `byetex diagnose`.
pub fn convert_capturing_source_map(source: &str, opts: &ConvertOptions) -> ConvertOutput {
    convert_with_macros(source, opts, HashMap::new(), HashSet::new(), true)
}

/// Internal variant of [`convert`] that lets the project layer pre-seed
/// the emitter's `\newcommand` table before parsing. Used by
/// [`project::plan_project_from_dir`] to make project-wide macro
/// definitions visible no matter which file declares them and whether
/// the entry file reaches them via `\input`.
///
/// Not public: macro records are an internal crate detail. Callers
/// outside the crate use `plan_project_from_dir` which constructs the
/// table for them.
pub(crate) fn convert_with_macros(
    source: &str,
    opts: &ConvertOptions,
    preseeded_macros: HashMap<String, emit::MacroDef>,
    preseeded_refs: HashSet<String>,
    record_source_map: bool,
) -> ConvertOutput {
    let tree = parser::parse(source);
    let source_name = opts.source_name.as_deref().unwrap_or("<input>");
    let visited: HashSet<PathBuf> = HashSet::new();
    let mut emitter = emit::Emitter::with_includes_and_macros(
        source,
        source_name,
        opts.base_dir.clone(),
        visited,
        preseeded_macros,
    );
    emitter.record_source_map = record_source_map;
    // Seed project-wide referenced labels before the prepass so a `\ref` in
    // one file informs multi-label attachment for a section in another.
    emitter.seed_referenced_labels(preseeded_refs);
    let root = tree.root_node();
    emitter.prepass_collect(root);
    emitter.emit_root(root);
    let emit::FinishOutput { typst, warnings, asset_refs, class_metadata, source_map } =
        emitter.finish();
    ConvertOutput {
        typst,
        warnings,
        asset_refs,
        class_metadata,
        source_map,
    }
}
