use byetex_core::{convert, ConvertOptions};

fn check(src: &str, expected_substr: &str) {
    let out = convert(src, &ConvertOptions::default());
    assert!(
        out.typst.contains(expected_substr),
        "expected {:?} in output, got:\n{}",
        expected_substr,
        out.typst
    );
    let counter_warnings: Vec<_> = out
        .warnings
        .iter()
        .filter(|w| {
            matches!(
                &w.category,
                byetex_core::warnings::Category::UnsupportedCommand { name }
                if name.starts_with("\\the")
            )
        })
        .collect();
    assert!(
        counter_warnings.is_empty(),
        "expected no \\the* warnings, got: {:?}",
        counter_warnings
    );
}

#[test]
fn thepage_emits_counter() {
    check(r"Page \thepage.", "counter(page).display()");
}

// `counter(heading.N)` is INVALID Typst (`.N` field access on the element fn →
// "expected comma"); the heading counter is `counter(heading)` and `.display()`
// formats it per the document's own numbering (corpus ctan-memoir,
// gh-sikatikenmogne-report). We display the current heading number rather than a
// hardcoded level slice — simpler, valid in markup+math, respects the doc style.
#[test]
fn thesection_emits_counter() {
    check(
        r"See \S\thesection\ for details.",
        "counter(heading).display()",
    );
}

#[test]
fn thesubsection_emits_counter() {
    check(r"\thesubsection", "counter(heading).display()");
}

#[test]
fn thesubsubsection_emits_counter() {
    check(r"\thesubsubsection", "counter(heading).display()");
}

#[test]
fn thechapter_emits_counter() {
    check(r"Chapter \thechapter.", "counter(heading).display()");
}

#[test]
fn heading_counters_use_no_invalid_level_suffix() {
    // Guard the regression: none of the heading `\the*` commands may emit the
    // invalid `counter(heading.N)` form.
    for src in [
        r"\thesection",
        r"\thesubsection",
        r"\thesubsubsection",
        r"\thechapter",
    ] {
        let out = convert(src, &ConvertOptions::default());
        assert!(
            !out.typst.contains("counter(heading."),
            "invalid `counter(heading.N)` for {src:?}; got:\n{}",
            out.typst
        );
    }
}

#[test]
fn thefigure_emits_counter() {
    check(r"Figure \thefigure.", "counter(figure).display()");
}

#[test]
fn thetable_emits_counter() {
    check(
        r"Table \thetable.",
        "counter(figure.where(kind: table)).display()",
    );
}

#[test]
fn theequation_emits_counter() {
    check(
        r"Equation~(\theequation).",
        "counter(math.equation).display()",
    );
}

// Compound: \thesection.\thesubsection style numbering. Both emit the valid
// `counter(heading).display()`; the literal `.` between them stays markup (a `.`
// before `#context` cannot be a field access, so the first expression ends).
#[test]
fn compound_counter_numbering() {
    let src = r"\thesection.\thesubsection";
    let out = convert(src, &ConvertOptions::default());
    assert!(!out.typst.contains("counter(heading."), "got:\n{}", out.typst);
    assert_eq!(
        out.typst.matches("counter(heading).display()").count(),
        2,
        "both counters present; got:\n{}",
        out.typst
    );
}
