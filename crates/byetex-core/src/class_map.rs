//! Recognize `\documentclass[opts]{class}` (and class-style
//! `\usepackage{neurips_*}` / `iclr*` / `icml*` packages) so we can parse the
//! source's author block correctly and retain its layout hints.
//!
//! ByeTex renders every document with one self-generated neutral preamble
//! (see `emit::build_neutral_preamble`); it does NOT bind a Typst Universe
//! template. `DocClass` survives for two reasons:
//!   1. Author-block parsing is class-specific (IEEE `\IEEEauthorblockN`,
//!      NeurIPS multi-line, the generic `\and` form) — `parse_authors`
//!      dispatches on the detected class to populate `DocumentMetadata`.
//!   2. The retained payloads (`paper_type`, `format`, ...) are the
//!      source-derived layout hints that Task 2 (layout fidelity) will read
//!      to reintroduce columns / geometry on top of the neutral base.

#[allow(unused_imports)]
use crate::document::{Author, Content, DocumentMetadata};

/// Document classes we recognize, for author parsing and layout hints.
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
    /// `\usepackage{acl}` (ACL Anthology style; the class is plain `article`).
    /// Two-column in both preprint and final modes.
    Acl,
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
    /// `\documentclass{beamer}` — LaTeX presentation class. Rendered as slides:
    /// each `frame` becomes its own page with a slide title.
    Beamer,
    /// Unrecognized class with no template binding — emits the hand-rolled
    /// title block fallback.
    Unknown,
}

impl DocClass {
    /// First pass: detect the class purely from `\documentclass[opts]{class}`.
    pub fn from_class(class: &str, opts: &[String]) -> Self {
        match class {
            // Match all `IEEEtran*` variants (IEEEtranTCOM, IEEEtranBSTCTL, …),
            // not just the base class — they are all two-column.
            name if name.starts_with("IEEEtran") || name == "IEEEconf" => {
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
            "beamer" => Self::Beamer,
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
        // The package may carry a path prefix, e.g.
        // `\usepackage{style/neurips_2026}` (corpus 2605.22507) — match the
        // basename so the conference style is still detected.
        let base = pkg.rsplit('/').next().unwrap_or(pkg);
        if base.starts_with("neurips_") {
            return Self::Neurips;
        }
        if base.starts_with("icml") {
            return Self::Icml;
        }
        if base.starts_with("iclr") {
            return Self::Iclr;
        }
        // ACL Anthology style (`acl`, `acl_natbib`, `acl20xx`) — two-column.
        if base.starts_with("acl") {
            return Self::Acl;
        }
        self
    }

    /// Whether this class lays out its body in two columns by default (absent an
    /// explicit `onecolumn`/`twocolumn` override). Conservative: only the
    /// classes that are reliably two-column return `true`.
    pub fn default_two_column(&self) -> bool {
        match self {
            // IEEEtran is two-column for conference, journal and technote.
            Self::IeeeTran { .. } => true,
            // acmart: the proceedings formats are two-column; the journal /
            // manuscript formats (acmsmall/acmlarge/acmtog/manuscript) are not.
            Self::AcmArt { format } => {
                matches!(
                    format.as_str(),
                    "sigconf" | "sigplan" | "sigchi" | "sigchi-a"
                )
            }
            // ICML camera-ready is two-column.
            Self::Icml => true,
            // ACL is two-column in both preprint and final modes.
            Self::Acl => true,
            _ => false,
        }
    }
}

/// Scalar, source-derived layout overrides for the neutral preamble (Task 2,
/// layout fidelity). Each `None` field falls back to the neutral default in
/// `emit::build_neutral_preamble`, so a document that doesn't request a size /
/// paper keeps the Task 1 baseline (us-letter, 11pt).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Layout {
    /// Typst paper name from `\documentclass[a4paper|letterpaper|...]`.
    pub paper: Option<&'static str>,
    /// Base font size from `\documentclass[10pt|11pt|12pt]`.
    pub font_size: Option<&'static str>,
    /// Explicit column request from `\documentclass[twocolumn|onecolumn]`.
    /// `Some(true)` = twocolumn, `Some(false)` = onecolumn, `None` = defer to
    /// the class default ([`DocClass::default_two_column`]).
    pub two_column: Option<bool>,
    /// Page margins from the `geometry` package (`\usepackage[...]{geometry}`
    /// and `\geometry{...}`). All `None` → the neutral default margin.
    pub margin: Margin,
}

/// Page margins parsed from `geometry` keys. Each side may be set individually;
/// `uniform` (the `margin=` key) is the fallback for any side left unset.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Margin {
    pub uniform: Option<String>,
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

impl Margin {
    fn is_empty(&self) -> bool {
        *self == Margin::default()
    }

    /// True when no `geometry` value was set, so a class-default margin applies.
    pub(crate) fn is_default(&self) -> bool {
        self.is_empty()
    }

    /// Whether any per-side key (not just `margin=`) was set.
    fn has_sides(&self) -> bool {
        self.top.is_some() || self.bottom.is_some() || self.left.is_some() || self.right.is_some()
    }

    /// The value for Typst's `page(margin: ...)`. With nothing set this is the
    /// neutral default (`(x: 1in, y: 1in)`); a lone `margin=` becomes a uniform
    /// length; per-side keys become a dict, each side falling back to the
    /// `margin=` value and then to `1in`.
    pub fn to_typst_value(&self) -> String {
        if self.is_empty() {
            return "(x: 1in, y: 1in)".to_string();
        }
        if !self.has_sides() {
            return self
                .uniform
                .clone()
                .unwrap_or_else(|| "(x: 1in, y: 1in)".to_string());
        }
        let fallback = self.uniform.as_deref().unwrap_or("1in");
        format!(
            "(top: {}, bottom: {}, left: {}, right: {})",
            self.top.as_deref().unwrap_or(fallback),
            self.bottom.as_deref().unwrap_or(fallback),
            self.left.as_deref().unwrap_or(fallback),
            self.right.as_deref().unwrap_or(fallback),
        )
    }
}

impl Layout {
    /// Derive scalar overrides from the `\documentclass[opts]` option list.
    /// Class-specific options (e.g. `conference`, `sigconf`) are ignored here —
    /// they are handled by [`DocClass::from_class`].
    pub fn from_class_options(opts: &[String]) -> Self {
        let mut layout = Layout::default();
        for opt in opts {
            if let Some(p) = map_paper_option(opt) {
                layout.paper = Some(p);
            } else if let Some(p) = map_beamer_aspectratio(opt) {
                // beamer `[aspectratio=…]` selects the slide page shape.
                layout.paper = Some(p);
            } else if let Some(s) = map_font_size_option(opt) {
                layout.font_size = Some(s);
            } else if opt == "twocolumn" {
                layout.two_column = Some(true);
            } else if opt == "onecolumn" {
                layout.two_column = Some(false);
            }
        }
        layout
    }

    /// Apply venue page geometry that a `\usepackage`-based style hard-codes over
    /// the document class. ACL's `acl.sty` forces a4 paper, 2.5cm margins and a
    /// 10pt body (`\PassOptionsToPackage{a4paper,margin=2.5cm}{geometry}` + `\xpt`)
    /// regardless of the `\documentclass[11pt]{article}` option — keeping the
    /// article defaults (us-letter, 11pt) inflated the page count ~50%. Call this
    /// after class/package detection and any user `geometry` are resolved.
    pub fn apply_venue_style(&mut self, class: &DocClass) {
        if matches!(class, DocClass::Acl) {
            // Style-forced over the class options.
            self.paper = Some("a4");
            self.font_size = Some("10pt");
            // Don't clobber an explicit user `geometry` margin.
            if self.margin.is_default() {
                self.margin.uniform = Some("2.5cm".to_string());
            }
        }
    }

    /// Resolve the effective column count given the detected class: an explicit
    /// `twocolumn`/`onecolumn` option wins, otherwise fall back to the class's
    /// own default.
    pub fn is_two_column(&self, class: &DocClass) -> bool {
        // Beamer slides are never page-level two-column. The presentation preamble
        // omits `columns: 2`, so honoring an explicit `[twocolumn]` option here would
        // make finish() emit a parent-scoped float title with no column context
        // (Typst errors). Beamer uses the `columns` environment for side-by-side.
        if matches!(class, DocClass::Beamer) {
            return false;
        }
        self.two_column
            .unwrap_or_else(|| class.default_two_column())
    }

    /// Merge `geometry` options (from `\usepackage[...]{geometry}` or
    /// `\geometry{...}`) into this layout. Called in source order, so a later
    /// call overrides an earlier key. Keys whose value isn't a Typst-expressible
    /// absolute length (e.g. `0.8\textwidth`) are skipped silently.
    pub fn apply_geometry(&mut self, opts: &str) {
        for token in split_top_level_commas(opts) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            match token.split_once('=') {
                Some((key, val)) => {
                    let key = key.trim();
                    let len = match sanitize_length(val.trim()) {
                        Some(l) => l,
                        None => continue,
                    };
                    match key {
                        "margin" => self.margin.uniform = Some(len),
                        "top" | "tmargin" => self.margin.top = Some(len),
                        "bottom" | "bmargin" => self.margin.bottom = Some(len),
                        "left" | "lmargin" | "inner" => self.margin.left = Some(len),
                        "right" | "rmargin" | "outer" => self.margin.right = Some(len),
                        "hmargin" => {
                            self.margin.left = Some(len.clone());
                            self.margin.right = Some(len);
                        }
                        "vmargin" => {
                            self.margin.top = Some(len.clone());
                            self.margin.bottom = Some(len);
                        }
                        // textwidth/textheight/paperwidth/... need arithmetic
                        // against the paper size — not yet supported.
                        _ => {}
                    }
                }
                // A bare flag: paper size (landscape etc. not yet supported).
                None => {
                    if let Some(p) = map_paper_option(token) {
                        self.paper = Some(p);
                    }
                }
            }
        }
    }
}

/// Split a geometry option string on top-level commas. (Geometry values don't
/// nest braces in practice, so a plain split is sufficient and keeps `=` values
/// intact.)
fn split_top_level_commas(s: &str) -> impl Iterator<Item = &str> {
    s.split(',')
}

/// Validate a LaTeX length and return it in a Typst-compatible form, or `None`
/// if it isn't an absolute length Typst understands. Accepts a decimal number
/// followed by one of Typst's length units (in/cm/mm/pt/em); rejects relative
/// or macro-based lengths like `0.8\textwidth` or `2\baselineskip`.
fn sanitize_length(v: &str) -> Option<String> {
    let v = v.trim();
    for unit in ["in", "cm", "mm", "pt", "em"] {
        if let Some(num) = v.strip_suffix(unit) {
            let num = num.trim();
            if !num.is_empty() && num.parse::<f64>().is_ok() {
                return Some(format!("{num}{unit}"));
            }
        }
    }
    None
}

/// Map a LaTeX paper-size class option to its Typst `page(paper: ...)` name.
fn map_paper_option(opt: &str) -> Option<&'static str> {
    Some(match opt {
        "a4paper" => "a4",
        "a5paper" => "a5",
        "b5paper" => "iso-b5",
        "letterpaper" => "us-letter",
        "legalpaper" => "us-legal",
        "executivepaper" => "us-executive",
        _ => return None,
    })
}

/// Map a beamer `aspectratio=<N>` class option to a Typst presentation paper.
/// Widescreen ratios (16:9, 16:10, 14:9) → `presentation-16-9`; everything else
/// (4:3, 5:4, 3:2, …) → `presentation-4-3` (beamer's default shape). `None` if `opt`
/// isn't an `aspectratio=` option at all.
fn map_beamer_aspectratio(opt: &str) -> Option<&'static str> {
    let val = opt.trim().strip_prefix("aspectratio=")?.trim();
    Some(match val {
        "169" | "1610" | "149" => "presentation-16-9",
        _ => "presentation-4-3",
    })
}

/// Map a LaTeX base font-size class option to a Typst `text(size: ...)` value.
fn map_font_size_option(opt: &str) -> Option<&'static str> {
    Some(match opt {
        "10pt" => "10pt",
        "11pt" => "11pt",
        "12pt" => "12pt",
        _ => return None,
    })
}

/// Public entry point: turn raw `\author{...}` strings (one per call in
/// the source) into structured `Author` records. The generic parser
/// handles the common shape (single-author, `\and`-separated authors,
/// embedded `\email{}` / `\affiliation{}` / `\thanks{}`); per-class
/// hints rewrite IEEE / NeurIPS-style author blocks first.
pub(crate) fn parse_authors(raw: &[String], class: &DocClass) -> Vec<Author> {
    let mut out = Vec::new();
    for s_raw in raw {
        // The NeurIPS multi-`\textbf{Name}$^{n}$ \quad …` pattern must be detected on
        // the RAW string: `sanitize_author_block` unwraps `\textbf` and drops `\quad`,
        // destroying the author boundaries before the line-based parser can see them.
        if matches!(class, DocClass::Neurips | DocClass::Icml | DocClass::Iclr) {
            if let Some(authors) = parse_neurips_textbf_authors(s_raw) {
                out.extend(authors);
                continue;
            }
        }
        let s = sanitize_author_block(s_raw);
        let s = s.as_str();
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
    let normalised = s.replace("\\AND", "\\and").replace("\\And", "\\and");

    // Pattern 1: `\and`-separated self-contained authors.
    if normalised.contains("\\and") {
        return normalised
            .split("\\and")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(parse_one_author)
            .collect();
    }

    // Pattern 2: comma-separated names followed by shared `\\` lines.
    if let Some((head, tail)) = normalised.split_once("\\\\") {
        let (shared_affil, shared_email) = parse_shared_lines(tail);
        let names = split_top_level_commas_owned(head.trim());
        let attach = |mut a: Author| -> Author {
            if a.affiliation.is_none() {
                a.affiliation = shared_affil.clone();
            }
            if a.email.is_none() {
                a.email = shared_email.clone();
            }
            a
        };
        if names.len() > 1 {
            return names
                .iter()
                .map(|n| attach(parse_one_author(n.trim())))
                .collect();
        }
        return vec![attach(parse_one_author(head.trim()))];
    }

    // Pattern 3: `\quad`/`\qquad`-separated grouped names (post-sanitize the
    // `\textbf{...}` is unwrapped, leaving `A \quad B`).
    if normalised.contains("\\quad") || normalised.contains("\\qquad") {
        return normalised
            .replace("\\qquad", "\\quad")
            .split("\\quad")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(parse_one_author)
            .collect();
    }

    // Single author.
    vec![parse_one_author(normalised.trim())]
}

/// Split on top-level commas — commas inside `{...}` are NOT separators.
/// Returns owned trimmed, non-empty parts (the author-block variant; distinct
/// from the geometry `split_top_level_commas` which yields `&str` slices).
fn split_top_level_commas_owned(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(s[start..i].to_string());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(s[start..].to_string());
    parts
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// From the `\\`-separated lines that follow the name list, derive a shared
/// affiliation (first non-email line) and email (first line containing `@` or
/// an `\email{}`), applied to every author in the block.
fn parse_shared_lines(tail: &str) -> (Option<crate::document::Affiliation>, Option<String>) {
    let mut affil = None;
    let mut email = None;
    for line in tail.split("\\\\").map(str::trim).filter(|l| !l.is_empty()) {
        // `\email{x}` or a bare `x@y` token.
        if let Some(e) = extract_email_token(line) {
            if email.is_none() {
                email = Some(e);
            }
            continue;
        }
        if affil.is_none() {
            affil = Some(crate::document::Affiliation::from_raw(Content::Typst(
                latex_text_to_typst(line),
            )));
        }
    }
    (affil, email)
}

/// Pull an email from a line: `\email{x@y}` body, or the first `@`-containing
/// whitespace token. Returns `None` if the line has no email.
fn extract_email_token(line: &str) -> Option<String> {
    if let Some(i) = line.find("\\email") {
        let after = i + "\\email".len();
        if line[after..].trim_start().starts_with('{') {
            let bpos = after + line[after..].find('{').unwrap();
            if let Some(end) = matched_close_brace(line, bpos) {
                return Some(line[bpos + 1..end].trim().to_string());
            }
        }
    }
    line.split_whitespace().find(|t| t.contains('@')).map(|t| {
        t.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-'
        })
        .to_string()
    })
}

/// Remove an `\email{...}` command or a bare `x@y` token from a line, leaving
/// the rest (used to separate a \thanks affiliation from its email).
fn strip_email_token(line: &str) -> String {
    let mut out = line.to_string();
    if let Some(i) = out.find("\\email") {
        let after = i + "\\email".len();
        if let Some(rel) = out[after..].find('{') {
            let bpos = after + rel;
            if let Some(end) = matched_close_brace(&out, bpos) {
                out.replace_range(i..=end, "");
            }
        }
    }
    out.split_whitespace()
        .filter(|t| !t.contains('@'))
        .collect::<Vec<_>>()
        .join(" ")
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
        while let Some(idx) = name.find(&pattern) {
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
                    let replacement = if unwrap_body {
                        body.clone()
                    } else {
                        String::new()
                    };
                    match *cmd {
                        "email" => email = Some(body),
                        "affiliation" | "affil" | "institute" | "institution" | "address" => {
                            affiliation_raw = Some(body);
                        }
                        "orcid" | "orcidID" => orcid = Some(body),
                        "thanks"
                            if body.to_ascii_lowercase().contains("equal")
                                || body.to_ascii_lowercase().contains("contribut") =>
                        {
                            equal = true;
                        }
                        "thanks" => {
                            // Substantive \thanks (article affiliation idiom): pull
                            // an email out, the rest becomes the affiliation.
                            if email.is_none() {
                                email = extract_email_token(&body);
                            }
                            if affiliation_raw.is_none() {
                                let aff = strip_email_token(&body);
                                if !aff.trim().is_empty() {
                                    affiliation_raw = Some(aff.trim().to_string());
                                }
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
    let despaced = name.replace("\\qquad", " ").replace("\\quad", " ");
    let cleaned_name = strip_unknown_author_cmds(despaced.trim());
    Author {
        name: Content::Typst(latex_text_to_typst(&cleaned_name)),
        email,
        affiliation: affiliation_raw.map(|raw| {
            crate::document::Affiliation::from_raw(Content::Typst(latex_text_to_typst(&raw)))
        }),
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
/// Find the digits of the first `^{n}` / `^n` superscript in `s` (e.g. an
/// affiliation ref after an author name). Returns `None` if there's no numeric
/// superscript.
fn first_superscript_digits(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'^' {
            let mut j = i + 1;
            if j < bytes.len() && bytes[j] == b'{' {
                j += 1;
            }
            let start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                return Some(s[start..j].to_string());
            }
        }
        i += 1;
    }
    None
}

/// A NeurIPS affiliation-legend token like `$^{1}$Rensselaer Polytechnic Institute`
/// → `("1", "Rensselaer Polytechnic Institute")`. Returns `None` for non-legend
/// tokens (no leading superscript ref).
fn parse_affil_legend(tok: &str) -> Option<(String, String)> {
    let t = tok.trim().trim_start_matches('$').trim_start();
    let after = t.strip_prefix('^')?;
    let (refnum, rest) = if let Some(after_brace) = after.strip_prefix('{') {
        let close = after_brace.find('}')?;
        (after_brace[..close].trim().to_string(), &after_brace[close + 1..])
    } else {
        let end = after
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after.len());
        (after[..end].to_string(), &after[end..])
    };
    let inst = rest.trim().trim_start_matches('$').trim().to_string();
    if refnum.is_empty() || inst.is_empty() {
        return None;
    }
    Some((refnum, inst))
}

/// NeurIPS multi-author pattern with NO `\and`: `\textbf{Name}$^{n}$` entries
/// separated by `\quad` / `\\`, followed by a `$^{n}$Institution` legend (corpus
/// 2605.22786). Splits each `\textbf` into its own author and attaches the legend
/// affiliation by ref. Returns `None` (fall through to the line-based parser) unless
/// the pattern clearly matches (≥2 `\textbf` authors, no `\and`).
fn parse_neurips_textbf_authors(s: &str) -> Option<Vec<Author>> {
    // Require `\quad` — the in-row author separator that defines this pattern. Its
    // presence distinguishes a multi-author row from `\textbf{Name}\\\textbf{Affil}`
    // (a bold affiliation on its own `\\` line), which must NOT be split.
    if s.contains("\\and") || !s.contains("\\quad") || s.matches("\\textbf{").count() < 2 {
        return None;
    }
    let tokens: Vec<String> = s
        .split("\\quad")
        .flat_map(|t| t.split("\\\\"))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    let mut specs: Vec<(String, Option<String>)> = Vec::new();
    let mut legend: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for tok in &tokens {
        if let Some(tb) = tok.find("\\textbf{") {
            let brace = tb + "\\textbf".len();
            if let Some(close) = matched_close_brace(tok, brace) {
                let raw_name = &tok[brace + 1..close];
                let name = latex_text_to_typst(&strip_textsuperscript(raw_name))
                    .trim()
                    .to_string();
                let refnum = first_superscript_digits(&tok[close..]);
                if !name.is_empty() {
                    specs.push((name, refnum));
                }
            }
        } else if let Some((r, inst)) = parse_affil_legend(tok) {
            legend.entry(r).or_insert(inst);
        }
    }
    if specs.len() < 2 {
        return None;
    }
    Some(
        specs
            .into_iter()
            .map(|(name, refnum)| Author {
                affiliation: refnum.and_then(|r| legend.get(&r)).map(|inst| {
                    crate::document::Affiliation::from_raw(Content::Typst(latex_text_to_typst(inst)))
                }),
                name: Content::Typst(name),
                ..Author::default()
            })
            .collect(),
    )
}

fn parse_neurips_block(s: &str) -> Vec<Author> {
    // (The multi-`\textbf` no-`\and` pattern is handled pre-sanitize in
    // `parse_authors`, since sanitize strips `\textbf`/`\quad`.)
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
                    latex_text_to_typst(line),
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
            'a' if s[i..].starts_with("\\ae")
                && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('æ');
                i += 3;
            }
            'A' if s[i..].starts_with("\\AE")
                && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('Æ');
                i += 3;
            }
            'o' if s[i..].starts_with("\\oe")
                && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('œ');
                i += 3;
            }
            'O' if s[i..].starts_with("\\OE")
                && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('Œ');
                i += 3;
            }
            's' if s[i..].starts_with("\\ss")
                && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('ß');
                i += 3;
            }
            'o' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('ø');
                i += 2;
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
            }
            'O' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('Ø');
                i += 2;
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
            }
            'i' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('ı');
                i += 2;
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
            }
            'l' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('ł');
                i += 2;
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
            }
            'L' if i + 2 <= s.len()
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('Ł');
                i += 2;
                if i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
            }
            'a' if s[i..].starts_with("\\aa")
                && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('å');
                i += 3;
            }
            'A' if s[i..].starts_with("\\AA")
                && !s[i + 3..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                out.push('Å');
                i += 3;
            }
            // Cedilla: \c{x} or \c x
            'c' if s[i..].starts_with("\\c")
                && !s[i + 2..].starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                i += 2;
                while i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
                if i < bytes.len() {
                    let letter_start = i;
                    if bytes[i] == b'{' {
                        if let Some(close) = matched_close_brace(s, i) {
                            let inner = &s[i + 1..close];
                            let letter = inner.chars().next().unwrap_or(' ');
                            let cedilla: char = match letter {
                                'c' => 'ç',
                                'C' => 'Ç',
                                's' => 'ş',
                                'S' => 'Ş',
                                't' => 'ţ',
                                'T' => 'Ţ',
                                _ => letter,
                            };
                            out.push(cedilla);
                            i = close + 1;
                            continue;
                        }
                    } else {
                        let letter = s[letter_start..].chars().next().unwrap_or(' ');
                        let cedilla: char = match letter {
                            'c' => 'ç',
                            'C' => 'Ç',
                            's' => 'ş',
                            'S' => 'Ş',
                            't' => 'ţ',
                            'T' => 'Ţ',
                            _ => letter,
                        };
                        out.push(cedilla);
                        i += letter.len_utf8();
                        continue;
                    }
                }
            }
            // Text-mode special characters
            '&' => {
                out.push('&');
                i += 2;
            }
            '%' => {
                out.push('%');
                i += 2;
            }
            '_' => {
                out.push('_');
                i += 2;
            }
            '$' => {
                out.push('$');
                i += 2;
            }
            '#' => {
                out.push('#');
                i += 2;
            }
            '{' => {
                out.push('{');
                i += 2;
            }
            '}' => {
                out.push('}');
                i += 2;
            }
            ' ' => {
                out.push(' ');
                i += 2;
            }
            '-' => {
                i += 2;
            } // soft hyphen — drop
            _ => {
                // Unknown command: skip command name, emit nothing (or braced body).
                i += 1; // skip `\`
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                while i < bytes.len() && bytes[i] == b' ' {
                    i += 1;
                }
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

/// Commands the sanitizer PRESERVES verbatim (the parsers consume them later,
/// or they're author separators). Their `{...}` body flows through and is
/// itself sanitized as text.
// `quad`/`qquad` are KEPT (not dropped as spacing) so `parse_generic_block`
// Pattern 3 can split `\textbf{A \quad B}` grouped authors on them; any residual
// is normalized to a space in `parse_one_author`. A KEEP command's `{...}` body
// flows back through the sanitizer as text, so `~`/`&` inside one become spaces
// (harmless for the emails/affiliations these carry).
const AUTHOR_KEEP_CMDS: &[&str] = &[
    "and",
    "And",
    "AND",
    "quad",
    "qquad",
    "email",
    "affiliation",
    "affil",
    "institute",
    "institution",
    "address",
    "orcid",
    "orcidID",
    "thanks",
    "texttt",
    "IEEEauthorblockN",
    "IEEEauthorblockA",
    "corref",
    "fnref",
    "authorrefmark",
    "inst",
    "textsuperscript",
];
/// Font-style/size commands whose inner text is KEPT (the command stripped) —
/// e.g. affiliation lines wrapped in `\small{University}` keep "University".
const AUTHOR_UNWRAP_CMDS: &[&str] = &[
    "textbf",
    "textit",
    "emph",
    "text",
    "textnormal",
    "textrm",
    "textsf",
    "textsc",
    "small",
    "footnotesize",
    "scriptsize",
    "tiny",
    "large",
    "Large",
    "LARGE",
    "huge",
    "Huge",
    "normalsize",
    "bfseries",
    "mdseries",
    "itshape",
    "scshape",
    "upshape",
    "sffamily",
    "rmfamily",
    "ttfamily",
];

/// Strip `%`…end-of-line LaTeX comments, honoring an escaped `\%`.
fn strip_latex_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Escaped char (incl. \%) — keep both, UTF-8 safe.
            out.push('\\');
            let ch = s[i + 1..].chars().next().unwrap();
            out.push(ch);
            i += 1 + ch.len_utf8();
            continue;
        }
        if bytes[i] == b'%' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Sanitize a raw `\author{...}` block into clean LaTeX text for the structure
/// parsers: drop comments, non-displaying spacing/format macros, and unknown
/// braced commands (unwrapping only the font-style set), while preserving `\\`
/// separators, `\quad`/`\qquad` author separators, and the structured commands
/// the parsers consume. UTF-8 safe; idempotent.
fn sanitize_author_block(raw: &str) -> String {
    let s = strip_latex_comments(raw);
    let out = sanitize_macros(&s);
    // Collapse whitespace runs (tabs/newlines/multi-space) to single spaces.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_macros(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // `\\` line break — keep, then drop an optional `[len]`.
            if i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                out.push_str("\\\\");
                i += 2;
                if i < bytes.len() && bytes[i] == b'[' {
                    if let Some(rb) = s[i..].find(']') {
                        i += rb + 1;
                    }
                }
                continue;
            }
            // `\` + non-letter control symbol.
            if i + 1 < bytes.len() && !bytes[i + 1].is_ascii_alphabetic() {
                let ch = s[i + 1..].chars().next().unwrap();
                match ch {
                    ',' | ';' | '!' | ':' | '>' | ' ' => out.push(' '), // thin/neg spaces
                    // Stray escaped brace, delimiter (`\|`), italic correction
                    // (`\/`), discretionary hyphen (`\-`) — no name content, drop.
                    '{' | '}' | '|' | '/' | '-' => {}
                    // Keep everything else with the backslash: the text escapes
                    // (`\&` `\%` `\_` `\#` `\$`) AND the accent commands
                    // (`\~n` `\"u` `\'e` `` \`a `` `\^o` `\=` `\.`) which
                    // `latex_text_to_typst` resolves to accented characters.
                    _ => {
                        out.push('\\');
                        out.push(ch);
                    }
                }
                i += 1 + ch.len_utf8();
                continue;
            }
            // `\command` — read the name.
            let name_start = i + 1;
            let mut j = name_start;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            let name = &s[name_start..j];
            // optional `*`
            let mut k = j;
            if k < bytes.len() && bytes[k] == b'*' {
                k += 1;
            }
            if AUTHOR_KEEP_CMDS.contains(&name) {
                // Re-emit the command verbatim; its `{...}` body (if any) flows
                // through the loop and is sanitized as normal text.
                out.push_str(&s[i..k]);
                i = k;
                continue;
            }
            // Skip an optional `[..]` arg then an optional `{..}` body.
            let mut a = k;
            while a < bytes.len() && bytes[a] == b' ' {
                a += 1;
            }
            if a < bytes.len() && bytes[a] == b'[' {
                if let Some(rb) = s[a..].find(']') {
                    a += rb + 1;
                }
            }
            let mut b = a;
            while b < bytes.len() && bytes[b] == b' ' {
                b += 1;
            }
            if b < bytes.len() && bytes[b] == b'{' {
                match matched_close_brace(s, b) {
                    Some(close) => {
                        if AUTHOR_UNWRAP_CMDS.contains(&name) {
                            out.push_str(&sanitize_macros(&s[b + 1..close]));
                        }
                        // else: drop the command AND its body entirely.
                        i = close + 1;
                    }
                    // Unterminated `{` — the tail is malformed and unrecoverable;
                    // drop the orphan command + rest so the stray `{…` never leaks.
                    None => i = bytes.len(),
                }
                continue;
            }
            // Bare unknown command (no body) — drop the token AND its optional
            // `[..]` arg (which `a` already skipped past). Using `k` here left the
            // `[1]` of e.g. `\footnotemark[1]` to leak next to the author name.
            i = a;
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        match ch {
            '&' | '~' => out.push(' '),
            _ => out.push(ch),
        }
        i += ch.len_utf8();
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
    }

    #[test]
    fn acm_sigconf_detected() {
        let c = DocClass::from_class("acmart", &["sigconf".to_string()]);
        assert!(matches!(c, DocClass::AcmArt { .. }));
    }

    #[test]
    fn neurips_via_package() {
        let c = DocClass::from_class("article", &[]).refine_from_package("neurips_2024");
        assert!(matches!(c, DocClass::Neurips));
    }

    fn opts(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn layout_from_options_maps_font_and_paper() {
        let l = Layout::from_class_options(&opts(&["12pt", "a4paper"]));
        assert_eq!(l.font_size, Some("12pt"));
        assert_eq!(l.paper, Some("a4"));
    }

    #[test]
    fn layout_ignores_class_specific_options() {
        // `conference` / `sigconf` are class options, not layout scalars.
        let l = Layout::from_class_options(&opts(&["conference", "sigconf"]));
        assert_eq!(l, Layout::default());
    }

    #[test]
    fn layout_paper_aliases() {
        assert_eq!(
            Layout::from_class_options(&opts(&["letterpaper"])).paper,
            Some("us-letter")
        );
        assert_eq!(
            Layout::from_class_options(&opts(&["b5paper"])).paper,
            Some("iso-b5")
        );
    }

    #[test]
    fn layout_default_when_empty() {
        assert_eq!(Layout::from_class_options(&[]), Layout::default());
    }

    #[test]
    fn sanitize_length_accepts_typst_units_rejects_relative() {
        assert_eq!(sanitize_length("1.5in").as_deref(), Some("1.5in"));
        assert_eq!(sanitize_length(" 25mm ").as_deref(), Some("25mm"));
        assert_eq!(sanitize_length("2cm").as_deref(), Some("2cm"));
        assert_eq!(sanitize_length("0.8\\textwidth"), None);
        assert_eq!(sanitize_length("3"), None); // no unit
        assert_eq!(sanitize_length("10bp"), None); // unit Typst lacks
    }

    #[test]
    fn apply_geometry_uniform_then_default_value() {
        let mut l = Layout::default();
        l.apply_geometry("margin=1in");
        assert_eq!(l.margin.to_typst_value(), "1in");
    }

    #[test]
    fn apply_geometry_command_merges_over_package() {
        let mut l = Layout::default();
        l.apply_geometry("margin=1in"); // package
        l.apply_geometry("top=2cm"); // \geometry command
        assert_eq!(
            l.margin.to_typst_value(),
            "(top: 2cm, bottom: 1in, left: 1in, right: 1in)"
        );
    }

    #[test]
    fn apply_geometry_paper_flag_and_empty_default() {
        let mut l = Layout::default();
        l.apply_geometry("a4paper");
        assert_eq!(l.paper, Some("a4"));
        // No margin keys → the neutral default value.
        assert_eq!(l.margin.to_typst_value(), "(x: 1in, y: 1in)");
    }

    #[test]
    fn unknown_class_falls_through() {
        let c = DocClass::from_class("foo", &[]);
        assert_eq!(c, DocClass::Unknown);
    }

    #[test]
    fn arxiv_article_detected() {
        let c = DocClass::from_class("article", &[]);
        assert!(matches!(c, DocClass::ArxivArticle));
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
    fn neurips_quad_separated_textbf_authors_split() {
        // NeurIPS pattern with NO \and: `\textbf{Name}$^{n}$` separated by \quad /
        // \\, then a `$^{n}$Institution` legend. Used to collapse all names into one
        // (corpus 2605.22786). Each \textbf author must become its own entry, and the
        // legend lines must NOT be parsed as authors.
        let raw = "\\textbf{Sadia Asif}$^{1}$ \\quad \\textbf{Mohammad Amiri}$^{1}$ \\quad \\textbf{Momin Abbas}$^{2}$ \\\\ $^{1}$Rensselaer Polytechnic Institute \\\\ $^{2}$IBM Research".to_string();
        let authors = parse_authors(&[raw], &DocClass::Neurips);
        let names: Vec<String> = authors
            .iter()
            .map(|a| a.name.as_content().trim().to_string())
            .collect();
        assert_eq!(authors.len(), 3, "expected 3 authors, got: {names:?}");
        assert_eq!(names[0], "Sadia Asif");
        assert_eq!(names[1], "Mohammad Amiri");
        assert_eq!(names[2], "Momin Abbas");
    }

    #[test]
    fn bold_affiliation_not_split_as_author() {
        // Code-review guard: a single author that bolds BOTH name and affiliation on
        // `\\` lines (no `\quad`) must NOT be split into two authors.
        let raw = "\\textbf{Alice}\\\\\\textbf{MIT}\\\\alice@x.org".to_string();
        let authors = parse_authors(&[raw], &DocClass::Neurips);
        assert_eq!(authors.len(), 1, "must stay 1 author; got: {:?}",
            authors.iter().map(|a| a.name.as_content()).collect::<Vec<_>>());
        assert_eq!(authors[0].name.as_content().trim(), "Alice");
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
}

#[cfg(test)]
mod author_sanitize_tests {
    use super::*;

    #[test]
    fn strips_comments_keeps_escaped_percent() {
        assert_eq!(sanitize_author_block("% lead comment\nAlice"), "Alice");
        assert_eq!(sanitize_author_block(r"50\% done"), r"50\% done");
    }

    #[test]
    fn drops_control_symbols_and_spacing() {
        // \, \; \! and a stray \} vanish; words keep single spaces.
        assert_eq!(sanitize_author_block(r"Alice \, Bob\}"), "Alice Bob");
        // \hspace{..} drops command AND body; & and ~ become spaces.
        assert_eq!(sanitize_author_block(r"A\hspace{0.5cm}& B~C"), "A B C");
        // An UNTERMINATED braced command drops the orphan `{…` too (never leak it).
        assert_eq!(sanitize_author_block(r"Alice \hspace{0.5cm B C"), "Alice");
    }

    #[test]
    fn unwraps_font_styles_drops_unknown_braced() {
        assert_eq!(sanitize_author_block(r"\textbf{Alice}"), "Alice");
        assert_eq!(
            sanitize_author_block(r"\emph{Bob} \unknown{drop me}"),
            "Bob"
        );
    }

    #[test]
    fn preserves_structure_and_separators() {
        // \\ kept (with [len] dropped); \and, \email{}, \quad preserved verbatim.
        assert_eq!(sanitize_author_block(r"A\\[2pt]B"), r"A\\B");
        assert_eq!(
            sanitize_author_block(r"Alice \and Bob \email{b@x} \quad C"),
            r"Alice \and Bob \email{b@x} \quad C"
        );
    }

    #[test]
    fn utf8_safe_and_idempotent() {
        let once = sanitize_author_block("M\\\"uller \\, Gra\\ss e");
        // accents are NOT resolved here (that is latex_text_to_typst's job) —
        // only spacing is removed; multibyte input is never split.
        assert_eq!(once, sanitize_author_block(&once));
        assert!(once.contains("ller"));
    }
}
