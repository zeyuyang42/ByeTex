//! Map `\documentclass[opts]{class}` (and class-style `\usepackage{neurips_*}`
//! / `iclr*` / `icml*` packages) into a Typst Universe template binding.
//!
//! The classes we recognize each have a community-maintained Typst template
//! that mimics the LaTeX class's visual identity. By emitting an
//! `#import "@preview/X:V": fn` + `#show: fn.with(...)` pair we get correct
//! page geometry, column count, fonts, heading style, and title block —
//! everything `\documentclass` controls in LaTeX, in one package.
//!
//! Truly unknown classes return `DocClass::Unknown` and ByeTex falls back
//! to the hand-rolled title block (`#align(center)[...]`) and Typst's
//! defaults. Plain `\documentclass{article}` (the common arxiv preprint
//! shape) is routed to `ArxivArticle` and gets the `arkheion` template.

#[allow(unused_imports)]
use crate::document::{Author, Content, DocumentMetadata};

/// Document classes we know how to map to a Typst template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocClass {
    IeeeTran {
        paper_type: String, // "conference" (default), "journal", "technote", ...
    },
    AcmArt {
        format: String, // "sigconf" (default), "sigplan", "sigchi", "acmsmall", ...
    },
    /// `\usepackage{neurips_20XX}` / `iclr*` / `icml*` (the class itself is
    /// usually plain `article`; the conference style is selected by the
    /// `.sty` package).
    Icml,
    Neurips,
    Iclr,
    RevTeX,
    ElsArticle {
        format: Option<String>, // "preprint" (default), "1p", "3p", "5p", "review"
    },
    /// Plain `\documentclass{article}` (or `report` / `book`) with no
    /// conference-style `\usepackage{...}` refinement. Routed to the
    /// `arkheion` Typst template, which is purpose-built for arxiv
    /// preprint look (title block + author affiliations + abstract).
    ArxivArticle,
    /// `\documentclass{llncs}` — Springer Lecture Notes in Computer Science.
    /// Renders as a single-column conference-proceedings layout.
    Lncs,
    /// `\documentclass[graybox]{svmult}` — Springer multi-author / contributed
    /// volume class (`svmult.cls`). Same family as `llncs`; we route both to
    /// the same template binding.
    SvMult,
    /// Unrecognized class with no template binding — emits the hand-rolled
    /// title block fallback.
    Unknown,
}

impl DocClass {
    /// First pass: detect the class purely from `\documentclass[opts]{class}`.
    pub fn from_class(class: &str, opts: &[String]) -> Self {
        match class {
            "IEEEtran" | "IEEEconf" => {
                let paper_type = opts
                    .iter()
                    .find(|o| {
                        matches!(
                            o.as_str(),
                            "conference" | "journal" | "technote" | "peerreview" | "peerreviewca"
                        )
                    })
                    .cloned()
                    .unwrap_or_else(|| "conference".to_string());
                Self::IeeeTran { paper_type }
            }
            "acmart" => {
                let format = opts
                    .iter()
                    .find(|o| {
                        matches!(
                            o.as_str(),
                            "sigconf"
                                | "sigplan"
                                | "sigchi"
                                | "sigchi-a"
                                | "acmtog"
                                | "acmsmall"
                                | "acmlarge"
                                | "manuscript"
                        )
                    })
                    .cloned()
                    .unwrap_or_else(|| "sigconf".to_string());
                Self::AcmArt { format }
            }
            "revtex4" | "revtex4-1" | "revtex4-2" => Self::RevTeX,
            "elsarticle" => {
                let format = opts
                    .iter()
                    .find(|o| matches!(o.as_str(), "preprint" | "review" | "1p" | "3p" | "5p"))
                    .cloned();
                Self::ElsArticle { format }
            }
            "llncs" => Self::Lncs,
            "svmult" => Self::SvMult,
            // Plain article / report / book — the most common arxiv
            // preprint case. Route to `ArxivArticle` so we get a
            // template instead of the hand-rolled fallback.
            // `refine_from_package` may upgrade this to Neurips / Icml /
            // Iclr if a conference style package is later loaded.
            "article" | "report" | "book" => Self::ArxivArticle,
            _ => Self::Unknown,
        }
    }

    /// Second-pass refinement: ML conference papers usually load their style
    /// via `\usepackage{neurips_2024}` / `iclr2025_conference` / etc. on top
    /// of plain `\documentclass{article}`. Override `ArxivArticle` (and
    /// `Unknown`) when we see one of these packages — the conference style
    /// wins over the generic arxiv look.
    pub fn refine_from_package(self, pkg: &str) -> Self {
        if !matches!(self, Self::Unknown | Self::ArxivArticle) {
            return self;
        }
        if pkg.starts_with("neurips_") {
            return Self::Neurips;
        }
        if pkg.starts_with("icml") {
            return Self::Icml;
        }
        if pkg.starts_with("iclr") {
            return Self::Iclr;
        }
        self
    }

    /// `#import "@preview/X:V": fn` line for this class, or `None` if no
    /// template is bound.
    pub fn import_line(&self) -> Option<&'static str> {
        Some(match self {
            Self::IeeeTran { .. } => "#import \"@preview/charged-ieee:0.1.4\": ieee",
            Self::AcmArt { .. } => "#import \"@preview/clean-acmart:0.0.1\": acmart",
            Self::Neurips | Self::Iclr | Self::Icml => {
                "#import \"@preview/lucky-icml:0.7.0\": icml2025 as conf"
            }
            Self::RevTeX => "#import \"@preview/revtyp:0.14.0\": revtyp",
            Self::ElsArticle { .. } => "#import \"@preview/elsearticle:3.1.0\": elsearticle",
            // `arkheion` is purpose-built for arxiv-style preprints —
            // single column, sans-serif title block with affiliations,
            // abstract, keywords. Covers most plain `\documentclass{article}`
            // arxiv papers.
            Self::ArxivArticle => "#import \"@preview/arkheion:0.1.2\": arkheion",
            // `llncs` / `svmult` — Springer LNCS / multi-author classes.
            // No verified Typst Universe template covers them yet, so
            // fall through to the hand-rolled title block. (When a
            // suitable template appears — `lncs` v0.1.x has been
            // proposed but not published — re-bind here.)
            Self::Lncs | Self::SvMult => return None,
            Self::Unknown => return None,
        })
    }

    /// Whether the abstract should be captured into `metadata.r#abstract`
    /// rather than being emitted inline as body content.
    ///
    /// Returns `true` for all classes whose title-block renderer (either a
    /// Typst Universe template or the rich native renderer in
    /// `flush_title_block`) accepts the abstract as a named field.
    /// `AcmArt` and `RevTeX` are the only exceptions: their templates render
    /// the `abstract` environment directly from body content.
    pub fn wants_abstract_field(&self) -> bool {
        !matches!(self, Self::AcmArt { .. } | Self::RevTeX)
    }

    /// Build the `#show: fn.with(...)` call from captured title-block data.
    /// Each template has its own argument shape; we emit only the fields it
    /// actually accepts, in the records it actually expects.
    pub fn show_call(&self, meta: &DocumentMetadata) -> Option<String> {
        let title = meta
            .title
            .as_ref()
            .map(Content::as_content)
            .unwrap_or("")
            .to_string();
        let abstract_ = meta
            .r#abstract
            .as_ref()
            .map(Content::as_content)
            .unwrap_or("")
            .to_string();
        let keywords_csv = meta.keywords.join(", ");
        match self {
            Self::IeeeTran { .. } => Some(ieee_show_call(
                &title,
                &meta.authors,
                &abstract_,
                &keywords_csv,
            )),
            Self::AcmArt { .. } => Some(acmart_show_call(&title, &meta.authors, &keywords_csv)),
            Self::Neurips | Self::Iclr | Self::Icml => Some(icml_show_call(
                &title,
                &meta.authors,
                &abstract_,
                &keywords_csv,
            )),
            Self::RevTeX => Some(revtyp_show_call(&title, &meta.authors)),
            Self::ElsArticle { format } => Some(elsearticle_show_call(
                &title,
                &meta.authors,
                &abstract_,
                &keywords_csv,
                format.as_deref(),
            )),
            Self::ArxivArticle => Some(arkheion_show_call(
                &title,
                &meta.authors,
                &abstract_,
                &keywords_csv,
                meta.date.as_deref(),
            )),
            // Lncs and SvMult reach flush_title_block (import_line returns None).
            Self::Lncs | Self::SvMult | Self::Unknown => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Show-call scaffolding helpers
// ---------------------------------------------------------------------------
//
// Each `*_show_call` builder below emits a Typst `#show: X.with(...)` block.
// The structural skeleton is identical across templates (header, title slot,
// authors block, optional abstract slot, optional keywords slot, closer);
// only the per-author record shape genuinely differs. The helpers in this
// section absorb the skeleton so each builder reads as just its
// distinguishing parts.

/// Start a show-call: `#show: <template>.with(\n  title: [<escaped>],\n`.
fn show_call_open(template_fn: &str, title: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("#show: {}.with(\n", template_fn));
    s.push_str(&format!("  title: [{}],\n", content_escape(title)));
    s
}

/// Emit `  abstract: [<escaped>],\n` when `abstract_` is non-empty.
fn push_abstract_slot(s: &mut String, abstract_: &str) {
    if !abstract_.is_empty() {
        s.push_str(&format!("  abstract: [{}],\n", content_escape(abstract_)));
    }
}

/// Emit `  <slot_name>: (<csv as tuple>,),\n` when `csv` is non-empty.
/// Two slot names are in use: `keywords` (most templates) and `index-terms`
/// (IEEE). Picking the slot name per-class avoids interpolation surprises.
fn push_csv_slot(s: &mut String, slot_name: &str, csv: &str) {
    if !csv.is_empty() {
        s.push_str(&format!("  {}: ({},),\n", slot_name, quote_csv(csv)));
    }
}

/// `charged-ieee` 0.1.4 signature (verified against the cached package):
///   ieee(title, authors: array of records, abstract, index-terms, paper-size,
///        bibliography, figure-supplement, body)
/// Author record: `(name, department?, organization?, location?, email?)`.
fn ieee_show_call(title: &str, authors: &[Author], abstract_: &str, keywords: &str) -> String {
    let mut s = show_call_open("ieee", title);
    s.push_str("  authors: (\n");
    for a in authors {
        s.push_str("    (");
        // charged-ieee uses author.name in `set document(author: ...)` which
        // requires a str, not content. Use a string literal.
        s.push_str(&format!("name: \"{}\"", string_escape(a.name.as_content())));
        if let Some(aff) = &a.affiliation {
            if let Some(dept) = &aff.department {
                s.push_str(&format!(
                    ", department: [{}]",
                    content_escape(dept.as_content())
                ));
            }
            if let Some(inst) = &aff.institution {
                s.push_str(&format!(
                    ", organization: [{}]",
                    content_escape(inst.as_content())
                ));
            }
            let loc = match (&aff.city, &aff.country) {
                (Some(c), Some(co)) => Some(format!("{}, {}", c, co)),
                (Some(c), None) => Some(c.clone()),
                (None, Some(co)) => Some(co.clone()),
                (None, None) => aff.raw.as_ref().map(|c| c.as_content().to_string()),
            };
            if let Some(loc) = loc {
                // charged-ieee expects location as a str, not content.
                s.push_str(&format!(", location: \"{}\"", string_escape(&loc)));
            }
        }
        if let Some(email) = &a.email {
            s.push_str(&format!(", email: \"{}\"", string_escape(email)));
        }
        s.push_str("),\n");
    }
    s.push_str("  ),\n");
    push_abstract_slot(&mut s, abstract_);
    push_csv_slot(&mut s, "index-terms", keywords);
    s.push_str(")\n");
    s
}

/// `clean-acmart` 0.0.1 signature (verified):
///   acmart(title, authors: array, affiliations: array, keywords: array of
///          strings, conference: dict, doi, isbn, price, copyright, review, body)
/// No `abstract` parameter — the abstract goes in the body.
fn acmart_show_call(title: &str, authors: &[Author], keywords: &str) -> String {
    let mut s = show_call_open("acmart", title);
    s.push_str("  authors: (\n");
    for a in authors {
        let aff = a
            .affiliation
            .as_ref()
            .and_then(|aff| aff.institution.as_ref().or(aff.raw.as_ref()))
            .map(|c| c.as_content().to_string())
            .unwrap_or_default();
        let email = a.email.clone().unwrap_or_default();
        s.push_str(&format!(
            "    (name: [{}], affiliation: [{}], email: [{}]),\n",
            content_escape(a.name.as_content()),
            content_escape(&aff),
            content_escape(&email),
        ));
    }
    s.push_str("  ),\n");
    push_csv_slot(&mut s, "keywords", keywords);
    s.push_str(")\n");
    s
}

/// `lucky-icml` 0.7.0 signature: the `authors` arg is a *tuple* of
/// `(authors-array, affls-dict)`. Passing `accepted: none` skips the
/// anonymous-override path that would otherwise replace authors when
/// `accepted: false` (the default).
fn icml_show_call(title: &str, authors: &[Author], abstract_: &str, keywords: &str) -> String {
    let mut s = show_call_open("conf", title);
    s.push_str("  authors: (\n");
    s.push_str("    (\n");
    for a in authors {
        // `affl: ()` (empty array) avoids the template's affls-dict lookup
        // assertion. Same for note/email — empty defaults all the way down.
        s.push_str(&format!(
            "      (name: \"{}\", affl: (), email: \"{}\", equal: {}, note: \"\"),\n",
            a.name.as_string_literal(),
            string_escape(a.email.as_deref().unwrap_or("")),
            a.equal_contribution,
        ));
    }
    s.push_str("    ),\n");
    s.push_str("    (:),\n"); // empty affiliations map
    s.push_str("  ),\n");
    push_abstract_slot(&mut s, abstract_);
    push_csv_slot(&mut s, "keywords", keywords);
    s.push_str("  accepted: none,\n");
    s.push_str(")\n");
    s
}

fn revtyp_show_call(title: &str, authors: &[Author]) -> String {
    let mut s = show_call_open("revtyp", title);
    s.push_str("  authors: (\n");
    for a in authors {
        let aff = a
            .affiliation
            .as_ref()
            .and_then(|aff| aff.institution.as_ref().or(aff.raw.as_ref()))
            .map(|c| c.as_string_literal())
            .unwrap_or_default();
        s.push_str(&format!(
            "    (name: \"{}\", affiliation: \"{}\"),\n",
            a.name.as_string_literal(),
            aff,
        ));
    }
    s.push_str("  ),\n");
    s.push_str(")\n");
    s
}

fn elsearticle_show_call(
    title: &str,
    authors: &[Author],
    abstract_: &str,
    keywords: &str,
    format: Option<&str>,
) -> String {
    let mut s = show_call_open("elsearticle", title);
    s.push_str("  authors: (\n");
    for a in authors {
        s.push_str(&format!(
            "    (name: \"{}\"),\n",
            a.name.as_string_literal()
        ));
    }
    s.push_str("  ),\n");
    push_abstract_slot(&mut s, abstract_);
    push_csv_slot(&mut s, "keywords", keywords);
    if let Some(fmt) = format {
        s.push_str(&format!("  format: \"{}\",\n", string_escape(fmt)));
    }
    s.push_str(")\n");
    s
}

/// `arkheion` 0.1.2 signature (verified against the cached package):
///   arkheion(title, authors: array, abstract, keywords, date)
/// Author record: `(name, email, affiliation, orcid)`.
fn arkheion_show_call(
    title: &str,
    authors: &[Author],
    abstract_: &str,
    keywords: &str,
    date: Option<&str>,
) -> String {
    let mut s = show_call_open("arkheion", title);
    s.push_str("  authors: (\n");
    for a in authors {
        // For string-literal slots, run raw values through string_escape so
        // a `"` or `\` in email/affiliation/orcid doesn't terminate the slot
        // prematurely. Only the `name` slot went through escape before.
        let aff = a
            .affiliation
            .as_ref()
            .and_then(|aff| aff.institution.as_ref().or(aff.raw.as_ref()))
            .map(|c| c.as_string_literal())
            .unwrap_or_default();
        let email = a.email.as_deref().map(string_escape).unwrap_or_default();
        let orcid = a.orcid.as_deref().map(string_escape).unwrap_or_default();
        s.push_str(&format!(
            "    (name: \"{}\", email: \"{}\", affiliation: \"{}\", orcid: \"{}\"),\n",
            a.name.as_string_literal(),
            email,
            aff,
            orcid,
        ));
    }
    s.push_str("  ),\n");
    push_abstract_slot(&mut s, abstract_);
    push_csv_slot(&mut s, "keywords", keywords);
    if let Some(d) = date {
        s.push_str(&format!("  date: \"{}\",\n", string_escape(d)));
    }
    s.push_str(")\n");
    s
}

/// `lncs` 0.1.0 signature: simpler `(title, authors, abstract)` plus
/// optional affiliation tuple. Single column, sans-serif title block.
/// `"foo, bar"` → `"\"foo\", \"bar\""` for embedding as a Typst tuple of strings.
fn quote_csv(s: &str) -> String {
    s.split(',')
        .map(|p| format!("\"{}\"", string_escape(p.trim())))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Escape a string for embedding inside a Typst `"..."` literal.
fn string_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escape a string for embedding inside a Typst `[...]` content block.
/// Closes (`]`) and backslashes are the only characters that can break
/// the surrounding bracket structure or be interpreted as Typst escapes;
/// markup like `_italic_` or `*bold*` is left alone so Content::Typst
/// renderings still display correctly. Hash (`#`) at the start of a token
/// would introduce a code injection, but inside an author-name slot it
/// reads as a literal sigil; escape it conservatively to avoid surprises.
fn content_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Pass through backslash sequences verbatim — `\_`, `\[`, `\#` etc.
                // are already Typst escape sequences produced by the emitter.
                out.push('\\');
                if let Some(&next) = chars.peek() {
                    out.push(next);
                    chars.next();
                }
            }
            '#' => {
                // `#` followed by a letter or `_` is a Typst function-call prefix
                // (`#raw(...)`, `#link(...)`, `#table(...)`) generated by ByeTex.
                // Only escape a bare `#` that is NOT part of such a call.
                if chars
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
                {
                    out.push('#');
                } else {
                    out.push_str("\\#");
                }
            }
            // `[` and `]` are valid Typst content-block delimiters; ByeTex's
            // emitter already escapes user-literal brackets when needed.
            other => out.push(other),
        }
    }
    out
}

/// Public entry point: turn raw `\author{...}` strings (one per call in
/// the source) into structured `Author` records. The generic parser
/// handles the common shape (single-author, `\and`-separated authors,
/// embedded `\email{}` / `\affiliation{}` / `\thanks{}`); per-class
/// hints rewrite IEEE / NeurIPS-style author blocks first.
pub(crate) fn parse_authors(raw: &[String], class: &DocClass) -> Vec<Author> {
    let mut out = Vec::new();
    for s in raw {
        match class {
            DocClass::IeeeTran { .. } => out.extend(parse_ieee_block(s)),
            DocClass::Neurips | DocClass::Icml | DocClass::Iclr => {
                out.extend(parse_neurips_block(s))
            }
            _ => out.extend(parse_generic_block(s)),
        }
    }
    out
}

/// Generic `\author{Alice \and Bob}` parser. Splits on `\and` (and the
/// NeurIPS case-variants `\And` / `\AND`), then for each chunk attempts
/// to pull out `\email{...}`, `\thanks{...}`, `\affiliation{...}`,
/// `\orcid{...}`, etc.
fn parse_generic_block(s: &str) -> Vec<Author> {
    // Normalise case-variants of the \and separator so a single split suffices.
    let normalised = s.replace("\\AND", "\\and").replace("\\And", "\\and");
    let mut authors = Vec::new();
    for piece in normalised.split("\\and") {
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }
        authors.push(parse_one_author(trimmed));
    }
    authors
}

/// Parse a single `\author{...}` chunk that may contain embedded
/// `\email{}` / `\affiliation{}` / `\orcid{}` / `\thanks{}` commands.
/// Pieces not consumed by any of those commands become the author's
/// `name`.
fn parse_one_author(chunk: &str) -> Author {
    let mut name = chunk.to_string();
    let mut email = None;
    let mut affiliation_raw: Option<String> = None;
    let mut orcid = None;
    let mut equal = false;

    // Scan the chunk for `\cmd{...}` patterns and pull each known one out.
    // The leftover name text is what wasn't claimed.
    //
    // Commands silently stripped (no display content):
    //   \corref, \fnref, \authorrefmark, \inst — LaTeX cross-ref markers.
    //   \textbf, \textit, \emph — wrappers whose inner text stays in name.
    //   \textsuperscript — affiliation ref numbers.
    //
    // Commands that produce structure:
    //   \email → Author.email
    //   \affiliation / \institute / \institution / \address → Author.affiliation
    //   \orcid / \orcidID → Author.orcid
    //   \thanks → equal_contribution flag (body consumed)
    for cmd in &[
        "email",
        "affiliation",
        "affil",
        "institute",
        "institution",
        "address",
        "orcidID", // must come before "orcid" — \orcid is a prefix of \orcidID
        "orcid",
        "thanks",
        // strip-only — no structured output
        "corref",
        "fnref",
        "authorrefmark",
        "inst",
        "textbf",
        "textit",
        "emph",
        "textsuperscript",
    ] {
        let pattern = format!("\\{}", cmd);
        // Some commands (\textbf, \textit, \emph) unwrap their body into the
        // name; others are consumed entirely.
        let unwrap_body = matches!(*cmd, "textbf" | "textit" | "emph");
        loop {
            let Some(idx) = name.find(&pattern) else { break };
            let after = idx + pattern.len();
            // Handle optional `[N]` bracket arg before `{body}` (e.g. \affil[1]{text}).
            let body_start = if name[after..].starts_with('[') {
                match name[after..].find(']') {
                    Some(rb) => after + rb + 1,
                    None => after,
                }
            } else {
                after
            };
            if name[body_start..].starts_with('{') {
                if let Some(end) = matched_close_brace(&name, body_start) {
                    let body = name[body_start + 1..end].trim().to_string();
                    let replacement = if unwrap_body { body.clone() } else { String::new() };
                    match *cmd {
                        "email" => email = Some(body),
                        "affiliation" | "affil" | "institute" | "institution" | "address" => {
                            affiliation_raw = Some(body);
                        }
                        "orcid" | "orcidID" => orcid = Some(body),
                        "thanks" => {
                            if body.to_ascii_lowercase().contains("equal") {
                                equal = true;
                            }
                        }
                        _ => {}
                    }
                    name.replace_range(idx..=end, &replacement);
                    continue;
                }
            }
            // No brace group — strip the bare command token.
            name.replace_range(idx..idx + pattern.len(), "");
        }
    }
    // General cleanup: strip remaining `\cmd{body}` patterns whose command
    // name wasn't matched above (e.g. unknown author sub-commands). Emit the
    // body contents so the name stays as clean text. Also strip `\cmd` (no
    // braces) when it's a pure marker.
    let cleaned_name = strip_unknown_author_cmds(name.trim());
    Author {
        name: Content::Typst(latex_text_to_typst(&cleaned_name)),
        email,
        affiliation: affiliation_raw
            .map(|raw| crate::document::Affiliation::from_raw(Content::Typst(raw))),
        orcid,
        equal_contribution: equal,
    }
}

/// IEEEtran-specific block parser. The IEEE author block is
/// `\IEEEauthorblockN{Name1 \and Name2}\IEEEauthorblockA{Affil1}\IEEEauthorblockA{Affil2}`.
/// We split on `\IEEEauthorblockN` boundaries: each segment owns one or
/// more names + one `\IEEEauthorblockA{...}` affiliation. When no
/// IEEE-specific markers are present, fall back to the generic parser.
fn parse_ieee_block(s: &str) -> Vec<Author> {
    if !s.contains("\\IEEEauthorblockN") {
        return parse_generic_block(s);
    }
    let mut authors = Vec::new();
    // Split on `\IEEEauthorblockN`; first piece (before the first
    // marker) is preamble we ignore.
    let mut chunks = s.split("\\IEEEauthorblockN");
    chunks.next();
    for chunk in chunks {
        // Each chunk starts at the `{` after the N marker.
        let chunk = chunk.trim_start();
        let (names_text, rest) = match split_first_braced(chunk) {
            Some(parts) => parts,
            None => continue,
        };
        // Pull the first `\IEEEauthorblockA{...}` from `rest` for the
        // affiliation; ignore additional A's for now (they apply to
        // additional names that LaTeX renders with a footnote marker
        // pointing into them).
        let affil_text = rest
            .find("\\IEEEauthorblockA")
            .and_then(|i| split_first_braced(rest[i + "\\IEEEauthorblockA".len()..].trim_start()))
            .map(|(t, _)| t)
            .unwrap_or_default();
        for name_piece in names_text.split("\\and") {
            let name = name_piece.trim().to_string();
            if name.is_empty() {
                continue;
            }
            let affiliation = if affil_text.is_empty() {
                None
            } else {
                Some(parse_ieee_affiliation(&affil_text))
            };
            authors.push(Author {
                name: Content::Typst(latex_text_to_typst(&strip_textsuperscript(&name))),
                affiliation,
                ..Author::default()
            });
        }
    }
    if authors.is_empty() {
        return parse_generic_block(s);
    }
    authors
}

/// Parse an IEEE affiliation block, which conventionally has the shape
/// `\textit{Dept of CS} \\ \textit{MIT, USA} \\ alice@mit.edu`.
fn parse_ieee_affiliation(raw: &str) -> crate::document::Affiliation {
    // Strip `\textit{}` / `\textbf{}` wrappers and split on `\\`.
    let cleaned = strip_textit(raw);
    let parts: Vec<&str> = cleaned.split("\\\\").map(str::trim).collect();
    let mut dept = None;
    let mut inst = None;
    let mut loc = None;
    let mut email_line = None;
    for (i, part) in parts.iter().enumerate() {
        if part.contains('@') {
            email_line = Some(part.to_string());
        } else if i == 0 {
            dept = Some(Content::Typst(latex_text_to_typst(part)));
        } else if i == 1 {
            inst = Some(Content::Typst(latex_text_to_typst(part)));
        } else {
            loc = Some(latex_text_to_typst(part));
        }
    }
    let _ = email_line; // attached to the affiliation isn't ideal; leave for now.
    crate::document::Affiliation {
        department: dept,
        institution: inst,
        city: loc,
        country: None,
        raw: Some(Content::Typst(latex_text_to_typst(raw))),
    }
}

/// NeurIPS / lucky-icml — `\author{Alice\thanks{equal}\\Affil\\\texttt{alice@x.org}}`.
fn parse_neurips_block(s: &str) -> Vec<Author> {
    // Normalise \And / \AND → \and so a single split covers all variants.
    let normalised = s.replace("\\AND", "\\and").replace("\\And", "\\and");
    let s = normalised.as_str();
    let mut authors = Vec::new();
    for piece in s.split("\\and") {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let lines: Vec<&str> = piece.split("\\\\").map(str::trim).collect();
        let mut name = String::new();
        let mut email = None;
        let mut affil = None;
        let mut equal = false;
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                // The first line is the name; strip `\thanks{...}`.
                let (n, t) = extract_thanks(line);
                name = latex_text_to_typst(&strip_textsuperscript(&n));
                if let Some(t) = t {
                    if t.to_ascii_lowercase().contains("equal") {
                        equal = true;
                    }
                }
            } else if line.contains('@') {
                // Email line, often wrapped in `\texttt{...}`.
                let cleaned = line
                    .trim()
                    .trim_start_matches("\\texttt{")
                    .trim_end_matches('}');
                email = Some(cleaned.to_string());
            } else if affil.is_none() {
                affil = Some(crate::document::Affiliation::from_raw(Content::Typst(
                    (*line).to_string(),
                )));
            }
        }
        if name.is_empty() {
            // Couldn't extract a name; fall through to the generic parser.
            authors.extend(parse_generic_block(piece));
            continue;
        }
        authors.push(Author {
            name: Content::Typst(name),
            email,
            affiliation: affil,
            equal_contribution: equal,
            ..Author::default()
        });
    }
    if authors.is_empty() {
        return parse_generic_block(s);
    }
    authors
}

/// Strip `\textit{X}` / `\emph{X}` wrappers from a string, leaving X.
fn strip_textit(s: &str) -> String {
    let mut out = s.to_string();
    for cmd in &["\\textit", "\\emph", "\\textbf"] {
        while let Some(i) = out.find(cmd) {
            let after = i + cmd.len();
            if out[after..].starts_with('{') {
                if let Some(end) = matched_close_brace(&out, after) {
                    let inner = out[after + 1..end].to_string();
                    out.replace_range(i..=end, &inner);
                    continue;
                }
            }
            break;
        }
    }
    out
}

/// Pull `\thanks{...}` off the end (or middle) of a string and return
/// (cleaned_text, thanks_text). `thanks_text` is `None` when no thanks
/// is present.
fn extract_thanks(s: &str) -> (String, Option<String>) {
    if let Some(i) = s.find("\\thanks") {
        let after = i + "\\thanks".len();
        if s[after..].starts_with('{') {
            if let Some(end) = matched_close_brace(s, after) {
                let inner = s[after + 1..end].to_string();
                let cleaned = format!("{}{}", &s[..i], &s[end + 1..]);
                return (cleaned, Some(inner));
            }
        }
    }
    (s.to_string(), None)
}

/// Remove `\textsuperscript{X}` / `${}^{X}$` markers that LaTeX uses
/// to attach author-to-affiliation correspondence numbers. We drop
/// them since the structured `Author` record doesn't carry the linkage.
fn strip_textsuperscript(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(i) = out.find("\\textsuperscript") {
        let after = i + "\\textsuperscript".len();
        if out[after..].starts_with('{') {
            if let Some(end) = matched_close_brace(&out, after) {
                out.replace_range(i..=end, "");
                continue;
            }
        }
        break;
    }
    // Strip simple `${}^X$` markers too.
    out = regex_replace(&out, r"\$\{?\}?\^\{?[^}$]*\}?\$", "");
    out.trim().to_string()
}

/// Tiny regex shim — `regex` isn't in our deps and pulling it in just
/// for this would be heavy. Implements only the patterns we need
/// (single-character replace, anchored bracket strip).
fn regex_replace(s: &str, pattern: &str, repl: &str) -> String {
    // Specialised for the one pattern above: drop `${}^X$` markers.
    if pattern == r"\$\{?\}?\^\{?[^}$]*\}?\$" {
        let mut out = String::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'$' {
                // Look ahead for `${}^X$` or `$^X$` etc.
                let rest = &s[i + 1..];
                if let Some(close) = rest.find('$') {
                    let inner = &rest[..close];
                    let inner_t = inner.trim_start_matches('{').trim_start_matches('}');
                    if inner_t.starts_with('^') {
                        // Skip the whole `$...$` superscript marker.
                        i += 1 + close + 1;
                        out.push_str(repl);
                        continue;
                    }
                }
            }
            // Append the next UTF-8 codepoint as a unit so multi-byte chars
            // (`é`, `ø`, CJK) survive the pass intact. Pushing raw bytes here
            // turned `Møller` into `MÃ¸ller`.
            let ch = s[i..].chars().next().expect("non-empty by loop guard");
            let step = ch.len_utf8();
            out.push(ch);
            i += step;
        }
        return out;
    }
    s.to_string()
}

/// Strip any remaining `\cmd{body}` or bare `\cmd` patterns from an author
/// name fragment whose commands were not consumed by the structured scan.
/// For braced forms the inner body is kept (so `\unknowncmd{text}` → `text`);
/// for bare forms the command token is dropped.
fn strip_unknown_author_cmds(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
            // Skip the command name letters.
            let cmd_start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            // Skip optional whitespace between command and `{`.
            let ws_start = i;
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'{' {
                // Braced form: emit the inner content.
                if let Some(close) = matched_close_brace(s, i) {
                    out.push_str(&s[i + 1..close]);
                    i = close + 1;
                    continue;
                }
            }
            // No brace — restore skipped whitespace but drop command token.
            i = ws_start;
            let _ = cmd_start; // suppress unused warning
            continue;
        }
        // Preserve multi-byte UTF-8 codepoints as a unit.
        let ch = s[i..].chars().next().unwrap_or('?');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Convert a raw LaTeX author-name string to a Typst-safe string.
///
/// Handles the subset of LaTeX that commonly appears in author names:
/// - Accent sequences: `\"u` → ü, `\'e` → é, `` \`a `` → à, `\^o` → ô, `\~n` → ñ.
/// - Curly-group accent: `{\'E}` → É.
/// - Named letter commands: `\ae` → æ, `\oe` → œ, `\ss` → ß, `\o` → ø, etc.
/// - Text-mode escapes: `\&` → &, `\%` → %, `\_` → _, etc.
/// - Display wrappers stripped: `\textbf{X}` → `X` (via strip_textit).
/// - Affiliation ref markers stripped: `\textsuperscript{N}` (via strip_textsuperscript).
fn latex_text_to_typst(s: &str) -> String {
    // Strip display-only wrappers first.
    let s = strip_textsuperscript(&strip_textit(s));
    raw_latex_accents_to_unicode(&s)
}

/// Walk raw LaTeX bytes and convert accent sequences + named letter commands
/// to precomposed Unicode, delegating to `emit::apply_text_accent`.
fn raw_latex_accents_to_unicode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            // Bare `{...}` → unwrap (e.g. `{\'E}ric`).
            if bytes[i] == b'{' {
                if let Some(close) = matched_close_brace(s, i) {
                    out.push_str(&raw_latex_accents_to_unicode(&s[i + 1..close]));
                    i = close + 1;
                    continue;
                }
            }
            let ch = s[i..].chars().next().unwrap_or('?');
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        // We have `\`. Look at the next character.
        if i + 1 >= bytes.len() {
            out.push('\\');
            i += 1;
            continue;
        }
        let next = bytes[i + 1] as char;
        match next {
            // --- accent commands: \' \" \` \^ \~ ---
            '\'' | '"' | '`' | '^' | '~' => {
                i += 2; // skip \ + accent char
                // Skip optional whitespace.
                while i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
                if i < bytes.len() {
                    if bytes[i] == b'{' {
                        // Braced argument: `{u}`.
                        if let Some(close) = matched_close_brace(s, i) {
                            let inner = &s[i + 1..close];
                            let letter = inner.chars().next().unwrap_or(' ');
                            out.push_str(&crate::emit::apply_text_accent(next, letter));
                            i = close + 1;
                            continue;
                        }
                    } else {
                        // Bare letter: `u`.
                        let letter = s[i..].chars().next().unwrap_or(' ');
                        out.push_str(&crate::emit::apply_text_accent(next, letter));
                        i += letter.len_utf8();
                        continue;
                    }
                }
                // Fallback: emit the accent char literally.
                out.push(next);
            }
            // --- named letter commands ---
            'a' if s[i..].starts_with("\\ae") && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('æ'); i += 3;
            }
            'A' if s[i..].starts_with("\\AE") && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('Æ'); i += 3;
            }
            'o' if s[i..].starts_with("\\oe") && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('œ'); i += 3;
            }
            'O' if s[i..].starts_with("\\OE") && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('Œ'); i += 3;
            }
            's' if s[i..].starts_with("\\ss") && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('ß'); i += 3;
            }
            'o' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('ø'); i += 2;
                if i < bytes.len() && bytes[i] == b' ' { i += 1; }
            }
            'O' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('Ø'); i += 2;
                if i < bytes.len() && bytes[i] == b' ' { i += 1; }
            }
            'i' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('ı'); i += 2;
                if i < bytes.len() && bytes[i] == b' ' { i += 1; }
            }
            'l' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('ł'); i += 2;
                if i < bytes.len() && bytes[i] == b' ' { i += 1; }
            }
            'L' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('Ł'); i += 2;
                if i < bytes.len() && bytes[i] == b' ' { i += 1; }
            }
            'a' if s[i..].starts_with("\\aa") && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('å'); i += 3;
            }
            'A' if s[i..].starts_with("\\AA") && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                out.push('Å'); i += 3;
            }
            // Cedilla: \c{x} or \c x
            'c' if s[i..].starts_with("\\c") && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                i += 2;
                while i < bytes.len() && bytes[i] == b' ' { i += 1; }
                if i < bytes.len() {
                    let letter_start = i;
                    if bytes[i] == b'{' {
                        if let Some(close) = matched_close_brace(s, i) {
                            let inner = &s[i + 1..close];
                            let letter = inner.chars().next().unwrap_or(' ');
                            let cedilla: char = match letter {
                                'c' => 'ç', 'C' => 'Ç', 's' => 'ş', 'S' => 'Ş',
                                't' => 'ţ', 'T' => 'Ţ', _ => letter,
                            };
                            out.push(cedilla);
                            i = close + 1;
                            continue;
                        }
                    } else {
                        let letter = s[letter_start..].chars().next().unwrap_or(' ');
                        let cedilla: char = match letter {
                            'c' => 'ç', 'C' => 'Ç', 's' => 'ş', 'S' => 'Ş',
                            't' => 'ţ', 'T' => 'Ţ', _ => letter,
                        };
                        out.push(cedilla);
                        i += letter.len_utf8();
                        continue;
                    }
                }
            }
            // Text-mode special characters
            '&' => { out.push('&'); i += 2; }
            '%' => { out.push('%'); i += 2; }
            '_' => { out.push('_'); i += 2; }
            '$' => { out.push('$'); i += 2; }
            '#' => { out.push('#'); i += 2; }
            '{' => { out.push('{'); i += 2; }
            '}' => { out.push('}'); i += 2; }
            ' ' => { out.push(' '); i += 2; }
            '-' => { i += 2; } // soft hyphen — drop
            _ => {
                // Unknown command: skip command name, emit nothing (or braced body).
                i += 1; // skip `\`
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() { i += 1; }
                while i < bytes.len() && bytes[i] == b' ' { i += 1; }
                if i < bytes.len() && bytes[i] == b'{' {
                    if let Some(close) = matched_close_brace(s, i) {
                        out.push_str(&raw_latex_accents_to_unicode(&s[i + 1..close]));
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
    }
    out
}

/// Find the matching `}` for the `{` at `open_brace` position in `s`.
/// Returns the index of the closing brace (inclusive).
fn matched_close_brace(s: &str, open_brace: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(open_brace) != Some(&b'{') {
        return None;
    }
    let mut depth = 1i32;
    let mut i = open_brace + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

/// Split `s` at the first `{...}` group: return (inner_text, rest).
/// `s` must START with `{` for this to succeed.
fn split_first_braced(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }
    let end = matched_close_brace(s, 0)?;
    Some((s[1..end].to_string(), &s[end + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee_conference_detected() {
        let c = DocClass::from_class("IEEEtran", &["conference".to_string()]);
        assert!(matches!(c, DocClass::IeeeTran { .. }));
        assert!(c.import_line().is_some());
    }

    #[test]
    fn acm_sigconf_detected() {
        let c = DocClass::from_class("acmart", &["sigconf".to_string()]);
        assert!(matches!(c, DocClass::AcmArt { .. }));
        assert!(c.import_line().is_some());
    }

    #[test]
    fn neurips_via_package() {
        let c = DocClass::from_class("article", &[]).refine_from_package("neurips_2024");
        assert!(matches!(c, DocClass::Neurips));
    }

    #[test]
    fn unknown_class_falls_through() {
        let c = DocClass::from_class("foo", &[]);
        assert_eq!(c, DocClass::Unknown);
        assert!(c.import_line().is_none());
    }

    #[test]
    fn show_call_with_ieee_record_shape() {
        let c = DocClass::IeeeTran {
            paper_type: "conference".to_string(),
        };
        let meta = DocumentMetadata {
            title: Some(Content::Typst("The Title".to_string())),
            authors: parse_authors(&["Alice".to_string()], &c),
            ..Default::default()
        };
        let s = c.show_call(&meta).unwrap();
        // `paper-type` is NOT a charged-ieee argument; we only emit the
        // fields the real signature accepts.
        assert!(s.contains("title: [The Title]"));
        // charged-ieee uses author.name in `set document` which requires str.
        assert!(s.contains("name: \"Alice\""));
        assert!(!s.contains("paper-type"));
    }

    #[test]
    fn arxiv_article_routes_to_arkheion() {
        let c = DocClass::from_class("article", &[]);
        assert!(matches!(c, DocClass::ArxivArticle));
        assert!(c.import_line().unwrap().contains("arkheion"));
    }

    #[test]
    fn neurips_package_overrides_article() {
        let c = DocClass::from_class("article", &[]).refine_from_package("neurips_2024");
        assert!(matches!(c, DocClass::Neurips));
    }

    #[test]
    fn generic_author_parser_splits_on_and() {
        let v = parse_authors(
            &["Alice \\and Bob \\and Carol".to_string()],
            &DocClass::Unknown,
        );
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].name.as_content().trim(), "Alice");
        assert_eq!(v[1].name.as_content().trim(), "Bob");
        assert_eq!(v[2].name.as_content().trim(), "Carol");
    }

    #[test]
    fn ieee_author_block_extracts_affiliation() {
        let raw =
            "\\IEEEauthorblockN{Alice}\\IEEEauthorblockA{\\textit{Dept of CS} \\\\ \\textit{MIT, USA} \\\\ alice@mit.edu}".to_string();
        let class = DocClass::IeeeTran {
            paper_type: "conference".to_string(),
        };
        let authors = parse_authors(&[raw], &class);
        assert_eq!(authors.len(), 1);
        let a = &authors[0];
        assert_eq!(a.name.as_content().trim(), "Alice");
        let aff = a.affiliation.as_ref().expect("affiliation");
        assert_eq!(
            aff.department.as_ref().unwrap().as_content().trim(),
            "Dept of CS"
        );
        assert_eq!(
            aff.institution.as_ref().unwrap().as_content().trim(),
            "MIT, USA"
        );
    }

    #[test]
    fn generic_extracts_email_and_thanks() {
        let raw = "Alice\\thanks{equal contribution}\\email{alice@x.org}".to_string();
        let authors = parse_authors(&[raw], &DocClass::ArxivArticle);
        assert_eq!(authors.len(), 1);
        let a = &authors[0];
        assert!(a.equal_contribution, "expected equal_contribution true");
        assert_eq!(a.email.as_deref(), Some("alice@x.org"));
        assert!(a.name.as_content().trim().starts_with("Alice"));
    }

    // ------------------------------------------------------------------
    // Byte-stable show-call snapshots
    // ------------------------------------------------------------------
    //
    // These tests pin the literal `#show: X.with(...)` output of every
    // class's builder. They protect against silent drift when the
    // scaffolding helpers (`show_call_open`, `push_abstract_slot`,
    // `push_csv_slot`) are touched. If you intentionally change the
    // output, update the expected string here AND verify the new
    // form parses against the upstream template.

    fn one_author() -> Vec<Author> {
        vec![Author {
            name: Content::Typst("Alice".to_string()),
            ..Default::default()
        }]
    }

    #[test]
    fn snapshot_ieee_minimal() {
        let s = ieee_show_call("T", &one_author(), "", "");
        assert_eq!(
            s,
            "#show: ieee.with(\n  title: [T],\n  authors: (\n    (name: \"Alice\"),\n  ),\n)\n"
        );
    }

    #[test]
    fn snapshot_acmart_minimal() {
        let s = acmart_show_call("T", &one_author(), "");
        assert_eq!(
            s,
            "#show: acmart.with(\n  title: [T],\n  authors: (\n    (name: [Alice], affiliation: [], email: []),\n  ),\n)\n"
        );
    }

    #[test]
    fn snapshot_icml_minimal() {
        let s = icml_show_call("T", &one_author(), "", "");
        assert_eq!(
            s,
            "#show: conf.with(\n  title: [T],\n  authors: (\n    (\n      (name: \"Alice\", affl: (), email: \"\", equal: false, note: \"\"),\n    ),\n    (:),\n  ),\n  accepted: none,\n)\n"
        );
    }

    #[test]
    fn snapshot_revtyp_minimal() {
        let s = revtyp_show_call("T", &one_author());
        assert_eq!(
            s,
            "#show: revtyp.with(\n  title: [T],\n  authors: (\n    (name: \"Alice\", affiliation: \"\"),\n  ),\n)\n"
        );
    }

    #[test]
    fn snapshot_elsearticle_minimal() {
        let s = elsearticle_show_call("T", &one_author(), "", "", None);
        assert_eq!(
            s,
            "#show: elsearticle.with(\n  title: [T],\n  authors: (\n    (name: \"Alice\"),\n  ),\n)\n"
        );
    }

    #[test]
    fn snapshot_arkheion_minimal() {
        let s = arkheion_show_call("T", &one_author(), "", "", None);
        assert_eq!(
            s,
            "#show: arkheion.with(\n  title: [T],\n  authors: (\n    (name: \"Alice\", email: \"\", affiliation: \"\", orcid: \"\"),\n  ),\n)\n"
        );
    }

    #[test]
    fn snapshot_with_abstract_and_keywords() {
        // Exercise the full slot scaffold on ieee (which uses
        // `index-terms` not `keywords`) and arkheion (with optional
        // date + keywords).
        let ieee = ieee_show_call("T", &one_author(), "An abstract.", "ml, nlp");
        assert!(ieee.contains("  abstract: [An abstract.],\n"));
        assert!(ieee.contains("  index-terms: (\"ml\", \"nlp\",),\n"));

        let ark = arkheion_show_call("T", &one_author(), "abs", "kw", Some("2026-05-24"));
        assert!(ark.contains("  abstract: [abs],\n"));
        assert!(ark.contains("  keywords: (\"kw\",),\n"));
        assert!(ark.contains("  date: \"2026-05-24\",\n"));
    }
}
