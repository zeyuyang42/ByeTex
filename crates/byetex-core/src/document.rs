//! Structured document representation extracted from a LaTeX preamble.
//!
//! The `Emitter` captures this during the AST walk; class-specific
//! renderers in `class_map.rs` populate per-template `#show:..with(...)`
//! calls from it. Keeping these fields in one struct (rather than a
//! handful of `pending_*` fields) makes the extraction → render boundary
//! explicit and gives per-class author parsers a typed place to land
//! their output.
//!
//! Some fields (`corresponding`, `footnote`, `bibliography.path/style`,
//! `is_empty`) are populated by the parsers but not yet read by any
//! template renderer. They're carried so future per-template enhancements
//! can use them without re-extracting from source.
#![allow(dead_code)]

use std::collections::HashMap;

/// Everything we know about a LaTeX document's preamble after the
/// extraction pass. Body content stays as Typst tokens in `Emitter::out`
/// — this struct only carries metadata that templates need as
/// arguments.
#[derive(Debug, Default, Clone)]
pub(crate) struct DocumentMetadata {
    pub title: Option<Content>,
    pub subtitle: Option<Content>,
    pub authors: Vec<Author>,
    pub r#abstract: Option<Content>,
    pub keywords: Vec<String>,
    pub date: Option<String>,
    pub bibliography: Option<BibSpec>,
    /// Class-specific metadata that doesn't fit other fields:
    /// `\acmDOI{...}`, `\IEEEpubid{...}`, `\conference{...}`, etc.
    /// Keyed by the LaTeX command name (without backslash).
    pub class_metadata: HashMap<String, String>,
}

impl DocumentMetadata {
    /// True when the user hasn't supplied a title or any authors —
    /// `build_template_preamble` should fall through to the hand-rolled
    /// path in that case.
    pub fn is_title_block_empty(&self) -> bool {
        self.title.is_none() && self.authors.is_empty()
    }

    /// Merge another `DocumentMetadata` (e.g. from a `\input`ed file)
    /// into self, taking the included file's values only for fields
    /// the parent doesn't already own.
    pub fn merge_from(&mut self, other: &mut DocumentMetadata) {
        if self.title.is_none() {
            self.title = other.title.take();
        }
        if self.subtitle.is_none() {
            self.subtitle = other.subtitle.take();
        }
        if self.authors.is_empty() {
            self.authors = std::mem::take(&mut other.authors);
        }
        if self.r#abstract.is_none() {
            self.r#abstract = other.r#abstract.take();
        }
        if self.keywords.is_empty() {
            self.keywords = std::mem::take(&mut other.keywords);
        }
        if self.date.is_none() {
            self.date = other.date.take();
        }
        if self.bibliography.is_none() {
            self.bibliography = other.bibliography.take();
        }
        for (k, v) in other.class_metadata.drain() {
            self.class_metadata.entry(k).or_insert(v);
        }
    }
}

/// One author, with whatever structured detail the per-class parser
/// was able to pull out. Most fields are `Option` because LaTeX
/// preambles vary wildly — IEEE papers carry department + organization
/// +
/// location + email, but a plain arxiv preprint might give only a
/// name and email.
#[derive(Debug, Default, Clone)]
pub(crate) struct Author {
    pub name: Content,
    pub email: Option<String>,
    pub affiliation: Option<Affiliation>,
    pub orcid: Option<String>,
    pub equal_contribution: bool,
    pub corresponding: bool,
    /// `\thanks{...}` content attached to this author.
    pub footnote: Option<Content>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Affiliation {
    pub department: Option<Content>,
    pub institution: Option<Content>,
    pub city: Option<String>,
    pub country: Option<String>,
    /// When the affiliation came in as one unstructured blob
    /// (e.g. IEEE `\IEEEauthorblockA{Dept of X, Y, Z}` without per-field
    /// markers), keep the raw text here so the renderer can fall back
    /// to it.
    pub raw: Option<Content>,
}

impl Affiliation {
    pub fn from_raw(raw: Content) -> Self {
        Self {
            raw: Some(raw),
            ..Default::default()
        }
    }
}

/// `\bibliography{path}` + `\bibliographystyle{style}` rolled into one.
#[derive(Debug, Default, Clone)]
pub(crate) struct BibSpec {
    pub path: String,
    pub style: Option<String>,
}

/// Rendered content destined for a Typst `[content]` slot or a
/// `"string"` slot, depending on the template field. `Typst` preserves
/// any markup the AST walk has already turned into Typst syntax
/// (italics, bold, math), while `Plain` is safe to embed inside `"..."`
/// literals.
#[derive(Debug, Clone)]
pub(crate) enum Content {
    Plain(String),
    Typst(String),
}

impl Default for Content {
    fn default() -> Self {
        Content::Plain(String::new())
    }
}

impl Content {
    /// Render for embedding inside a Typst `[ ... ]` content block.
    /// Both variants are valid Typst content in that position.
    pub fn as_content(&self) -> &str {
        match self {
            Content::Plain(s) | Content::Typst(s) => s,
        }
    }

    /// Render for embedding inside a Typst `"..."` string literal.
    /// Strips markup-ish characters from the `Typst` variant since
    /// they'd otherwise show up as literal `_foo_` text inside a
    /// string.
    pub fn as_string_literal(&self) -> String {
        let raw = match self {
            Content::Plain(s) => s.clone(),
            Content::Typst(s) => {
                // Strip Typst inline markup so it doesn't appear as
                // literal `_`/`*` characters inside a string slot.
                strip_inline_markup(s)
            }
        };
        escape_for_typst_string(&raw)
    }

    pub fn is_empty(&self) -> bool {
        self.as_content().trim().is_empty()
    }
}

/// Best-effort strip of Typst inline markup that would appear as
/// literal characters when embedded in a string slot. Drops single
/// `_` and `*` runs (italic/bold delimiters) but leaves text intact.
fn strip_inline_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Pass through backslash-escapes literally.
                out.push('\\');
                if let Some(&nx) = chars.peek() {
                    out.push(nx);
                    chars.next();
                }
            }
            '_' | '*' => {
                // Drop single markup delimiters (italic/bold). Multi-
                // letter `_foo_` is treated as `foo`. Naïve but
                // acceptable for title-block strings.
            }
            other => out.push(other),
        }
    }
    out
}

/// Escape for embedding inside a Typst `"..."` string literal.
pub(crate) fn escape_for_typst_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
