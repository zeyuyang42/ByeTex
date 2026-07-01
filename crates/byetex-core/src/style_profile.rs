//! Per-class style profiles: front-matter + citation fidelity knobs derived
//! from the detected `DocClass`. Sizes are em relative to the body font size
//! (the conference classes all run 10pt bodies, so e.g. 17pt == 1.7em).
//!
//! Unit 1 consumes only the title fields + `body_font`; the abstract /
//! citation / bibliography fields are set per the same class-file ground
//! truth and consumed in Units 2-4.

use crate::class_map::DocClass;

/// Default in-text citation form for the class (`\cite` rendering).
/// Consumed by Unit 4 bibliography-style resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CiteMode {
    Numeric,
    AuthorYear,
}

/// How the class renders its abstract block. Consumed in Unit 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AbstractStyle {
    Neutral,
    ArticleCentered,
    ConferenceHeading { smallcaps: bool },
    RunInBoldItalic,
    RunInBold,
}

pub(crate) struct StyleProfile {
    /// Title font size (em, relative to the body size).
    pub title_size: &'static str,
    pub title_bold: bool,
    pub title_smallcaps: bool,
    /// Full-width rule ABOVE the title: `(stroke, gap below the rule)`.
    pub title_rule_above: Option<(&'static str, &'static str)>,
    /// Full-width rule BELOW the title:
    /// `(gap above the rule, stroke, gap below the rule)`.
    pub title_rule_below: Option<(&'static str, &'static str, &'static str)>,
    /// How the class renders its abstract block (Unit 2).
    pub abstract_style: AbstractStyle,
    /// Whether the abstract is placed inside the two-column body (Unit 2).
    pub abstract_in_columns: bool,
    pub body_font: &'static str,
    /// Default in-text citation mode for the class (Unit 4 bib-style resolution).
    pub cite_default: CiteMode,
    /// The class's own default bibliography style as a Typst CSL name (Unit 4).
    pub default_bib_style: Option<&'static str>,
    /// Section / subsection / subsubsection heading sizes (em vs the body).
    /// article runs \Large/\large/\normalsize (1.44/1.2/1.0); the compact
    /// conference classes section with \large\bf / \normalsize\bf at a 10pt
    /// body, i.e. 1.2/1.0/1.0.
    pub heading_sizes: [&'static str; 3],
    /// Uppercase the title (amsart's `\MakeUppercase`).
    pub title_uppercase: bool,
    /// Center level-1 (section) headings (amsart, REVTeX).
    pub heading_centered: bool,
    /// Uppercase level-1 (section) headings (REVTeX).
    pub heading_uppercase: bool,
}

impl StyleProfile {
    /// The class-faithful profile for a detected `DocClass`. Ground truth was
    /// verified against the actual class files (see the table in Unit 1):
    /// every size below is the class's own `\maketitle` font size at a 10pt
    /// body. Unprofiled classes (elsarticle — zero corpus papers — RevTeX,
    /// Unknown) keep [`StyleProfile::neutral`] byte-identical output.
    pub fn for_class(class: &DocClass) -> Self {
        match class {
            // article.cls \maketitle is {\LARGE \@title} — NOT bold.
            DocClass::ArxivArticle => Self {
                title_size: "1.728em",
                title_bold: false,
                abstract_style: AbstractStyle::ArticleCentered,
                ..Self::neutral()
            },
            // neurips_2026.sty lines 307-328: 4pt toptitlebar + 0.25in gap,
            // \LARGE(=17pt) bold title, 0.29in gap + 1pt bottomtitlebar +
            // 0.09in gap; authors follow the bottom rule.
            DocClass::Neurips => Self {
                title_size: "1.7em",
                title_bold: true,
                title_rule_above: Some(("4pt", "0.25in")),
                title_rule_below: Some(("0.29in", "1pt", "0.09in")),
                abstract_style: AbstractStyle::ConferenceHeading { smallcaps: false },
                cite_default: CiteMode::AuthorYear,
                heading_sizes: ["1.2em", "1.0em", "1.0em"],
                ..Self::neutral()
            },
            // icml2026.sty toptitlebar/bottomtitlebar: 1pt rule + 0.25in gap
            // above; {\Large\bf}(=14pt) title; 0.22in gap + 1pt rule + 0.3in.
            DocClass::Icml => Self {
                title_size: "1.4em",
                title_bold: true,
                title_rule_above: Some(("1pt", "0.25in")),
                title_rule_below: Some(("0.22in", "1pt", "0.3in")),
                abstract_style: AbstractStyle::ConferenceHeading { smallcaps: false },
                abstract_in_columns: true,
                cite_default: CiteMode::AuthorYear,
                heading_sizes: ["1.2em", "1.0em", "1.0em"],
                ..Self::neutral()
            },
            // iclr_conference.sty: {\LARGE\sc \@title} — small caps, regular.
            DocClass::Iclr => Self {
                title_size: "1.7em",
                title_bold: false,
                title_smallcaps: true,
                abstract_style: AbstractStyle::ConferenceHeading { smallcaps: true },
                cite_default: CiteMode::AuthorYear,
                heading_sizes: ["1.2em", "1.0em", "1.0em"],
                ..Self::neutral()
            },
            // ACL (acl.sty): two-column, in-column abstract, natbib author-year.
            // Conservative profile — exact title size/weight + ACL bib style
            // deferred to a visual re-grade.
            DocClass::Acl => Self {
                // acl.sty:152 — `{\Large\bfseries \@title}` on its 10pt body → 1.44em bold
                // (NOT the neutral 1.5em; the inherited value rendered the title visibly too
                // large vs the truth). Headings: \large/\normalsize → 1.2/1.0/1.0 (acl.sty:234+).
                title_size: "1.44em",
                abstract_style: AbstractStyle::ConferenceHeading { smallcaps: false },
                abstract_in_columns: true,
                cite_default: CiteMode::AuthorYear,
                heading_sizes: ["1.2em", "1.0em", "1.0em"],
                ..Self::neutral()
            },
            // IEEEtran.cls \@maketitle (non-technote): {\Huge ... \@title}.
            // IEEE sets its body in Times (a narrow serif). New Computer Modern is
            // both wrong for IEEE and ~10% wider, over-wrapping lines and inflating
            // the page count (2605.31549 page_ratio 1.40, 2605.22779 1.33). Libertinus
            // Serif is a bundled (reproducible) Times-adjacent serif — denser and far
            // closer to IEEE's Times than Computer Modern.
            DocClass::IeeeTran { .. } => Self {
                title_size: "2.4em",
                title_bold: false,
                abstract_style: AbstractStyle::RunInBoldItalic,
                abstract_in_columns: true,
                body_font: "Libertinus Serif",
                default_bib_style: Some("ieee"),
                ..Self::neutral()
            },
            // acmart truth is sans bold \LARGE; serif approximation (Typst
            // bundles no matching sans). Libertinus Serif matches acmart's
            // Linux Libertine and is bundled with Typst.
            DocClass::AcmArt { .. } => Self {
                title_size: "1.728em",
                title_bold: true,
                abstract_style: AbstractStyle::ConferenceHeading { smallcaps: false },
                abstract_in_columns: true,
                body_font: "Libertinus Serif",
                default_bib_style: Some("association-for-computing-machinery"),
                ..Self::neutral()
            },
            // llncs.cls: {\Large \bfseries\boldmath \@title}; svmult is the
            // same Springer family.
            DocClass::Lncs | DocClass::SvMult => Self {
                title_size: "1.44em",
                title_bold: true,
                abstract_style: AbstractStyle::RunInBold,
                default_bib_style: Some("springer-basic"),
                heading_sizes: ["1.2em", "1.0em", "1.0em"],
                ..Self::neutral()
            },
            // elsarticle is deliberately unprofiled for the title (zero corpus
            // papers) — neutral output, but its bib style is still known.
            DocClass::ElsArticle { .. } => Self {
                default_bib_style: Some("elsevier-with-titles"),
                ..Self::neutral()
            },
            // Beamer: the slide page-size/title-slide styling is applied in the
            // emitter; the title-block knobs stay neutral for now.
            DocClass::Beamer => Self::neutral(),
            // amsart: \MakeUppercase title + centered section headings.
            DocClass::Amsart => Self {
                title_uppercase: true,
                heading_centered: true,
                ..Self::neutral()
            },
            // REVTeX/APS: centered + uppercase section headings (numbering is
            // class-specific too — see `heading_numbering_decl`).
            DocClass::RevTeX => Self {
                heading_centered: true,
                heading_uppercase: true,
                ..Self::neutral()
            },
            DocClass::Unknown => Self::neutral(),
        }
    }

    /// The unprofiled fallback: byte-identical to the historical hardcoded
    /// title line (`1.5em` bold) and body font (`New Computer Modern`).
    pub fn neutral() -> Self {
        Self {
            title_size: "1.5em",
            title_bold: true,
            title_smallcaps: false,
            title_rule_above: None,
            title_rule_below: None,
            abstract_style: AbstractStyle::Neutral,
            abstract_in_columns: false,
            body_font: "New Computer Modern",
            cite_default: CiteMode::Numeric,
            default_bib_style: None,
            // article.cls \Large/\large/\normalsize sectioning — also the
            // historical global default for every class (level-3 kept as the
            // exact historical `1em` literal for byte-identical neutral output).
            heading_sizes: ["1.44em", "1.2em", "1em"],
            title_uppercase: false,
            heading_centered: false,
            heading_uppercase: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_matches_historical_hardcoded_values() {
        let p = StyleProfile::neutral();
        assert_eq!(p.title_size, "1.5em");
        assert!(p.title_bold);
        assert!(!p.title_smallcaps);
        assert!(p.title_rule_above.is_none() && p.title_rule_below.is_none());
        assert_eq!(p.body_font, "New Computer Modern");
    }

    #[test]
    fn compact_conference_classes_have_smaller_headings() {
        for class in [
            DocClass::Neurips,
            DocClass::Icml,
            DocClass::Iclr,
            DocClass::Lncs,
            DocClass::SvMult,
        ] {
            assert_eq!(
                StyleProfile::for_class(&class).heading_sizes,
                ["1.2em", "1.0em", "1.0em"],
                "{class:?} should use compact \\large\\bf/\\normalsize sectioning"
            );
        }
        // article + neutral keep the article-correct \Large/\large/\normalsize.
        assert_eq!(
            StyleProfile::for_class(&DocClass::ArxivArticle).heading_sizes,
            ["1.44em", "1.2em", "1em"]
        );
        assert_eq!(
            StyleProfile::neutral().heading_sizes,
            ["1.44em", "1.2em", "1em"]
        );
    }

    #[test]
    fn ieeetran_uses_times_family_serif() {
        // IEEE sets its body in Times (a narrow serif); New Computer Modern (wide)
        // over-wraps every line and inflates the page count (2605.31549 1.40, 2605.22779
        // 1.33). Libertinus Serif is a bundled, reproducible Times-adjacent serif — both
        // denser and far closer to IEEE's Times than Computer Modern.
        let p = StyleProfile::for_class(&DocClass::IeeeTran {
            paper_type: "conference".to_string(),
        });
        assert_eq!(p.body_font, "Libertinus Serif");
    }

    #[test]
    fn unprofiled_classes_stay_neutral() {
        for class in [
            DocClass::Unknown,
            DocClass::RevTeX,
            DocClass::ElsArticle { format: None },
        ] {
            let p = StyleProfile::for_class(&class);
            assert_eq!(p.title_size, "1.5em", "{class:?} title must stay neutral");
            assert!(p.title_bold, "{class:?} title must stay neutral bold");
            assert_eq!(p.body_font, "New Computer Modern");
            assert!(p.title_rule_above.is_none() && p.title_rule_below.is_none());
        }
    }

    #[test]
    fn rules_only_for_neurips_and_icml() {
        for class in [DocClass::Neurips, DocClass::Icml] {
            let p = StyleProfile::for_class(&class);
            assert!(
                p.title_rule_above.is_some() && p.title_rule_below.is_some(),
                "{class:?} must have title rules"
            );
        }
        for class in [
            DocClass::ArxivArticle,
            DocClass::Iclr,
            DocClass::IeeeTran {
                paper_type: "conference".into(),
            },
            DocClass::AcmArt {
                format: "sigconf".into(),
            },
            DocClass::Lncs,
            DocClass::SvMult,
        ] {
            let p = StyleProfile::for_class(&class);
            assert!(
                p.title_rule_above.is_none() && p.title_rule_below.is_none(),
                "{class:?} must have no title rules"
            );
        }
    }
}
