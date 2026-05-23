//! Map `\documentclass[opts]{class}` (and class-style `\usepackage{neurips_*}`
//! / `iclr*` / `icml*` packages) into a Typst Universe template binding.
//!
//! The classes we recognize each have a community-maintained Typst template
//! that mimics the LaTeX class's visual identity. By emitting an
//! `#import "@preview/X:V": fn` + `#show: fn.with(...)` pair we get correct
//! page geometry, column count, fonts, heading style, and title block —
//! everything `\documentclass` controls in LaTeX, in one package.
//!
//! Unknown classes return `DocClass::Unknown` and ByeTex falls back to the
//! hand-rolled title block (`#align(center)[...]`) and Typst's defaults.

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
    /// Plain `article`, `report`, `book`, unknown classes — caller falls
    /// back to the hand-rolled title block.
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
            _ => Self::Unknown,
        }
    }

    /// Second-pass refinement: ML conference papers usually load their style
    /// via `\usepackage{neurips_2024}` / `iclr2025_conference` / etc. on top
    /// of plain `\documentclass{article}`. Override `Unknown` (only) when we
    /// see one of these packages.
    pub fn refine_from_package(self, pkg: &str) -> Self {
        if !matches!(self, Self::Unknown) {
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
            Self::Unknown => return None,
        })
    }

    /// Whether the bound template accepts the abstract as a named field. When
    /// `false` (acmart), the caller should leave the abstract in the body.
    pub fn wants_abstract_field(&self) -> bool {
        matches!(
            self,
            Self::IeeeTran { .. }
                | Self::Neurips
                | Self::Iclr
                | Self::Icml
                | Self::ElsArticle { .. }
        )
    }

    /// Build the `#show: fn.with(...)` call from captured title-block data.
    /// Each template has its own argument shape; we emit only the fields it
    /// actually accepts, in the records it actually expects.
    pub fn show_call(
        &self,
        title: &str,
        authors: &[String],
        abstract_: &str,
        keywords: &str,
    ) -> Option<String> {
        match self {
            Self::IeeeTran { .. } => Some(ieee_show_call(title, authors, abstract_, keywords)),
            Self::AcmArt { .. } => Some(acmart_show_call(title, authors, keywords)),
            Self::Neurips | Self::Iclr | Self::Icml => {
                Some(icml_show_call(title, authors, abstract_, keywords))
            }
            Self::RevTeX => Some(revtyp_show_call(title, authors)),
            Self::ElsArticle { format } => Some(elsearticle_show_call(
                title,
                authors,
                abstract_,
                keywords,
                format.as_deref(),
            )),
            Self::Unknown => None,
        }
    }
}

/// `charged-ieee` 0.1.4 signature (verified against the cached package):
///   ieee(title, authors: array of records, abstract, index-terms, paper-size,
///        bibliography, figure-supplement, body)
/// Author record: `(name, department?, organization?, location?, email?)`.
fn ieee_show_call(title: &str, authors: &[String], abstract_: &str, keywords: &str) -> String {
    let mut s = String::new();
    s.push_str("#show: ieee.with(\n");
    s.push_str(&format!("  title: [{}],\n", title));
    s.push_str("  authors: (\n");
    for n in expand_authors(authors) {
        s.push_str(&format!("    (name: [{}]),\n", n));
    }
    s.push_str("  ),\n");
    if !abstract_.is_empty() {
        s.push_str(&format!("  abstract: [{}],\n", abstract_));
    }
    if !keywords.is_empty() {
        s.push_str(&format!("  index-terms: ({},),\n", quote_csv(keywords)));
    }
    s.push_str(")\n");
    s
}

/// `clean-acmart` 0.0.1 signature (verified):
///   acmart(title, authors: array, affiliations: array, keywords: array of
///          strings, conference: dict, doi, isbn, price, copyright, review, body)
/// No `abstract` parameter — the abstract goes in the body.
fn acmart_show_call(title: &str, authors: &[String], keywords: &str) -> String {
    let mut s = String::new();
    s.push_str("#show: acmart.with(\n");
    s.push_str(&format!("  title: [{}],\n", title));
    s.push_str("  authors: (\n");
    for n in expand_authors(authors) {
        s.push_str(&format!("    (name: [{}], email: []),\n", n));
    }
    s.push_str("  ),\n");
    if !keywords.is_empty() {
        s.push_str(&format!("  keywords: ({},),\n", quote_csv(keywords)));
    }
    // Leave conference, doi, etc. at the template defaults — they're
    // submission-specific metadata we don't extract yet.
    s.push_str(")\n");
    s
}

/// `lucky-icml` 0.7.0 signature: the `authors` arg is a *tuple* of
/// `(authors-array, affls-dict)`. Passing `accepted: none` skips the
/// anonymous-override path that would otherwise replace authors when
/// `accepted: false` (the default).
fn icml_show_call(title: &str, authors: &[String], abstract_: &str, keywords: &str) -> String {
    let mut s = String::new();
    s.push_str("#show: conf.with(\n");
    s.push_str(&format!("  title: [{}],\n", title));
    s.push_str("  authors: (\n");
    s.push_str("    (\n");
    for n in expand_authors(authors) {
        // `affl: ()` (empty array) avoids the template's affls-dict lookup
        // assertion. Same for note/email — empty defaults all the way down.
        s.push_str(&format!(
            "      (name: \"{}\", affl: (), email: \"\", equal: false, note: \"\"),\n",
            string_escape(&strip_brackets(&n))
        ));
    }
    s.push_str("    ),\n");
    s.push_str("    (:),\n"); // empty affiliations map
    s.push_str("  ),\n");
    if !abstract_.is_empty() {
        s.push_str(&format!("  abstract: [{}],\n", abstract_));
    }
    if !keywords.is_empty() {
        s.push_str(&format!("  keywords: ({},),\n", quote_csv(keywords)));
    }
    s.push_str("  accepted: none,\n");
    s.push_str(")\n");
    s
}

fn revtyp_show_call(title: &str, authors: &[String]) -> String {
    let mut s = String::new();
    s.push_str("#show: revtyp.with(\n");
    s.push_str(&format!("  title: [{}],\n", title));
    s.push_str("  authors: (\n");
    for n in expand_authors(authors) {
        s.push_str(&format!(
            "    (name: \"{}\"),\n",
            string_escape(&strip_brackets(&n))
        ));
    }
    s.push_str("  ),\n");
    s.push_str(")\n");
    s
}

fn elsearticle_show_call(
    title: &str,
    authors: &[String],
    abstract_: &str,
    keywords: &str,
    format: Option<&str>,
) -> String {
    let mut s = String::new();
    s.push_str("#show: elsearticle.with(\n");
    s.push_str(&format!("  title: [{}],\n", title));
    s.push_str("  authors: (\n");
    for n in expand_authors(authors) {
        s.push_str(&format!(
            "    (name: \"{}\"),\n",
            string_escape(&strip_brackets(&n))
        ));
    }
    s.push_str("  ),\n");
    if !abstract_.is_empty() {
        s.push_str(&format!("  abstract: [{}],\n", abstract_));
    }
    if !keywords.is_empty() {
        s.push_str(&format!("  keywords: ({},),\n", quote_csv(keywords)));
    }
    if let Some(fmt) = format {
        s.push_str(&format!("  format: \"{}\",\n", fmt));
    }
    s.push_str(")\n");
    s
}

/// Many LaTeX templates put multiple authors in a single `\author{... \and ...}`
/// invocation. Split on the literal `\and` separator so each author becomes
/// its own record. We also tolerate authors already separated into multiple
/// `\author{}` calls.
fn expand_authors(authors: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for a in authors {
        for piece in a.split("\\and") {
            let t = piece.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

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

/// Strip a single leading `[` and trailing `]` from a content-block-wrapped
/// string so it embeds cleanly as a Typst string literal.
fn strip_brackets(s: &str) -> String {
    let mut t = s.trim().to_string();
    if t.starts_with('[') && t.ends_with(']') {
        t = t[1..t.len() - 1].to_string();
    }
    // Best-effort escape for embedding inside `"..."`.
    t.replace('\\', "\\\\").replace('"', "\\\"")
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
        let s = c
            .show_call("The Title", &["Alice".to_string()], "", "")
            .unwrap();
        // `paper-type` is NOT a charged-ieee argument; we only emit the
        // fields the real signature accepts.
        assert!(s.contains("title: [The Title]"));
        assert!(s.contains("(name: [Alice])"));
        assert!(!s.contains("paper-type"));
    }

    #[test]
    fn expand_authors_splits_on_and() {
        let v = expand_authors(&["Alice \\and Bob \\and Carol".to_string()]);
        assert_eq!(v, vec!["Alice", "Bob", "Carol"]);
    }
}
