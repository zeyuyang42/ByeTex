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

#[test]
fn thesection_emits_counter() {
    check(
        r"See \S\thesection\ for details.",
        "counter(heading.1).display()",
    );
}

#[test]
fn thesubsection_emits_counter() {
    check(r"\thesubsection", "counter(heading.2).display()");
}

#[test]
fn thechapter_emits_counter() {
    check(r"Chapter \thechapter.", "counter(heading.1).display()");
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

// Compound: \thesection.\thesubsection style numbering
#[test]
fn compound_counter_numbering() {
    let src = r"\thesection.\thesubsection";
    let out = convert(src, &ConvertOptions::default());
    assert!(out.typst.contains("counter(heading.1)"));
    assert!(out.typst.contains("counter(heading.2)"));
}
