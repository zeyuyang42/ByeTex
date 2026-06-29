//! IEEEtran inline `\IEEEauthorrefmark{n}` author blocks: names carry trailing
//! affiliation-mark superscripts and a legend `\IEEEauthorrefmark{n}Affiliation`
//! follows. Before the fix every author collapsed to one affiliation `[1]`,
//! `X and Y` merged into a single name, and legend entries 2–5 were dropped
//! (dogfood 2605.31499). This is the inline-refmark sibling of the
//! `\IEEEauthorblockN` parser.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
    .typst
}

const DOC: &str = r#"\documentclass[conference]{IEEEtran}
\author{
    Alice Smith\IEEEauthorrefmark{1}\IEEEauthorrefmark{3}, Bob Jones\IEEEauthorrefmark{2} and Carol Lee\IEEEauthorrefmark{1} \\
    \IEEEauthorrefmark{1}Alpha Lab, First University, Town, Country \\
    \IEEEauthorrefmark{2}Beta Department, Second Institute, City, Country \\
    \IEEEauthorrefmark{3}Gamma Group, Third Org, Place, Country \\
}
\begin{document}\maketitle
Body.
\end{document}"#;

#[test]
fn refmark_authors_are_split_and_not_merged() {
    let t = typ(DOC);
    // "Bob Jones and Carol Lee" must be two authors, not one merged name.
    assert!(t.contains("Bob Jones"), "Bob Jones missing:\n{t}");
    assert!(t.contains("Carol Lee"), "Carol Lee missing:\n{t}");
    assert!(
        !t.contains("Bob Jones and Carol Lee") && !t.contains("Jones and Carol"),
        "authors were merged on `and`:\n{t}"
    );
}

#[test]
fn refmark_primary_affiliations_present_and_shared() {
    let t = typ(DOC);
    // Each author's PRIMARY (first) refmark affiliation renders, and authors
    // sharing a primary mark share a superscript (Alice & Carol both at [1]).
    // (Secondary-only affiliations — an index that's never an author's first
    // mark, e.g. Gamma/Third Org here — can't be held by the single-affiliation
    // Author model and are a documented limitation.)
    assert!(t.contains("First University"), "affil 1 (primary) missing:\n{t}");
    assert!(
        t.contains("Second Institute"),
        "affil 2 (primary) missing:\n{t}"
    );
    let n_super1 = t.matches("#super[1]").count();
    assert!(
        n_super1 >= 3,
        "expected Alice, Carol, and the legend to share marker [1] (>=3 #super[1]); got {n_super1}:\n{t}"
    );
}

#[test]
fn refmark_markers_not_left_raw() {
    let t = typ(DOC);
    assert!(
        !t.contains("IEEEauthorrefmark"),
        "raw \\IEEEauthorrefmark leaked:\n{t}"
    );
}
