//! `\resizebox{width}{height}{content}` (graphicx) wraps content scaled to a
//! target box. ByeTex had no handler, so the whole node — INCLUDING the wrapped
//! tabular/figure — was dropped (corpus: 21 papers, `\resizebox{\textwidth}{!}{…}`
//! is the standard idiom for fitting a wide table to the text width). The wrapped
//! content must survive; the scale-to-fit sizing is secondary.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn resizebox_preserves_wrapped_tabular() {
    let t = typ(r"\resizebox{\textwidth}{!}{\begin{tabular}{cc}ALPHA & BETA \\ \end{tabular}}");
    assert!(
        t.contains("ALPHA") && t.contains("BETA"),
        "wrapped table content must survive; got:\n{t}"
    );
}

#[test]
fn resizebox_preserves_plain_content() {
    let t = typ(r"x \resizebox{5cm}{!}{KEEPME} y");
    assert!(
        t.contains("KEEPME"),
        "wrapped content must survive; got:\n{t}"
    );
    assert!(
        t.contains('x') && t.contains('y'),
        "surrounding text preserved; got:\n{t}"
    );
}

// --- Scale-to-fit -----------------------------------------------------------
//
// The sizing was the deferred half of the original fix: the wrapped content
// survived, but `\resizebox{\textwidth}{!}{…}` — the standard idiom for fitting
// a wide table to the text width, 22 corpus papers and 142 occurrences — emitted
// a table at its NATURAL width. A table LaTeX shrinks to fit the column then
// overflows in the Typst render, and one LaTeX scales up sits too small; either
// way the float carries the wrong amount of ink, which is also why `ink` shows up
// as an independent layout-drift signal on 25 papers.

#[test]
fn resizebox_to_textwidth_fits_the_available_width() {
    let t = typ(r"\resizebox{\textwidth}{!}{\begin{tabular}{cc}A & B \\ \end{tabular}}");
    assert!(
        t.contains("byetex-fit(100%)"),
        "`\\resizebox{{\\textwidth}}` scales the content to the full measure; got:\n{t}"
    );
    assert!(t.contains("A") && t.contains("B"), "content still survives; got:\n{t}");
}

#[test]
fn columnwidth_and_linewidth_are_also_the_full_measure() {
    for w in [r"\columnwidth", r"\linewidth"] {
        let t = typ(&format!(r"\resizebox{{{w}}}{{!}}{{\begin{{tabular}}{{c}}A\end{{tabular}}}}"));
        assert!(t.contains("byetex-fit(100%)"), "{w} → 100%; got:\n{t}");
    }
}

#[test]
fn a_fraction_of_the_measure_is_carried_through() {
    let t = typ(r"\resizebox{0.5\textwidth}{!}{\begin{tabular}{c}A\end{tabular}}");
    assert!(t.contains("byetex-fit(50%)"), "0.5\\textwidth → 50%; got:\n{t}");
    let t = typ(r"\resizebox{.8\linewidth}{!}{\begin{tabular}{c}A\end{tabular}}");
    assert!(t.contains("byetex-fit(80%)"), ".8\\linewidth → 80%; got:\n{t}");
}

#[test]
fn an_absolute_width_is_used_verbatim() {
    let t = typ(r"\resizebox{5cm}{!}{\begin{tabular}{c}A\end{tabular}}");
    assert!(t.contains("byetex-fit(5cm)"), "5cm → a length target; got:\n{t}");
}

#[test]
fn the_helper_is_defined_when_used_and_absent_when_not() {
    let t = typ(r"\documentclass{article}\begin{document}\resizebox{\textwidth}{!}{X}\end{document}");
    assert!(t.contains("#let byetex-fit"), "helper is defined; got:\n{t}");
    let t = typ(r"\documentclass{article}\begin{document}plain text\end{document}");
    assert!(
        !t.contains("byetex-fit"),
        "no helper when nothing is resized — the preamble stays clean; got:\n{t}"
    );
}

#[test]
fn an_unrecognised_width_still_falls_back_to_bare_content() {
    // `!` means "keep the aspect ratio from the other axis" — there is no target
    // width to fit to, so the pre-existing passthrough must remain rather than
    // guessing a size.
    let t = typ(r"\resizebox{!}{2cm}{\begin{tabular}{c}KEEPME\end{tabular}}");
    assert!(t.contains("KEEPME"), "content survives; got:\n{t}");
    assert!(!t.contains("byetex-fit"), "no fit wrapper without a width; got:\n{t}");
}

// --- The shape real papers actually use --------------------------------------
//
// The inline `\resizebox{W}{!}{\begin{tabular}…}` above is not how papers are
// written. All 110 corpus occurrences (17 papers) look like:
//
//     \resizebox{\linewidth}{!}{
//     \begin{tabular}{lc}
//
// tree-sitter does not nest a multi-line environment inside the command's brace
// group, so the tabular is emitted by the FLOAT path and a wrapper keyed on the
// command's own children never fires. These tests are the ones that matter — the
// command-level tests above pass without them being satisfied.

#[test]
fn a_float_wrapping_its_tabular_in_resizebox_is_fitted() {
    let t = typ(
        "\\begin{table}\n\\resizebox{\\linewidth}{!}{\n\\begin{tabular}{lc}\nA & B \\\\\n\\end{tabular}\n}\n\\caption{Cap}\n\\end{table}",
    );
    assert!(
        t.contains("byetex-fit(100%)"),
        "the float's table is scaled to the measure; got:\n{t}"
    );
    assert!(t.contains("table("), "the table still renders; got:\n{t}");
    assert!(t.contains("Cap"), "caption survives; got:\n{t}");
}

#[test]
fn a_percent_comment_between_the_brace_and_the_tabular_still_counts() {
    // `{%` is the idiom for suppressing the newline's space; it must not hide the
    // wrapper from the scan.
    let t = typ(
        "\\begin{table}\n\\resizebox{0.9\\textwidth}{!}{%\n\\begin{tabular}{lc}\nA & B \\\\\n\\end{tabular}\n}\n\\end{table}",
    );
    assert!(t.contains("byetex-fit(90%)"), "0.9\\textwidth → 90%; got:\n{t}");
}

#[test]
fn a_float_whose_tabular_is_not_wrapped_is_left_alone() {
    // The control: same float, no `\resizebox`. Without this the test above would
    // pass on a change that fitted EVERY table.
    let t = typ(
        "\\begin{table}\n\\begin{tabular}{lc}\nA & B \\\\\n\\end{tabular}\n\\caption{Cap}\n\\end{table}",
    );
    assert!(
        !t.contains("byetex-fit"),
        "an unwrapped table is not scaled; got:\n{t}"
    );
}

#[test]
fn a_resizebox_elsewhere_in_the_float_does_not_capture_the_tabular() {
    // The `\resizebox` here wraps something else and closes before the tabular
    // starts, so the tabular is NOT its content and must not be fitted.
    let t = typ(
        "\\begin{table}\n\\resizebox{\\linewidth}{!}{\\includegraphics{x.png}}\n\\begin{tabular}{lc}\nA & B \\\\\n\\end{tabular}\n\\end{table}",
    );
    assert!(
        !t.contains("byetex-fit"),
        "only a tabular DIRECTLY inside the resizebox is fitted; got:\n{t}"
    );
}

// --- Regressions found in review ---------------------------------------------

#[test]
fn a_commented_out_resizebox_does_not_scale() {
    // Commenting the wrapper out is how an author disables scaling, and the
    // disabled form leaves exactly the whitespace-only gap the scan accepts — so
    // without comment-awareness ByeTex would stretch a table LaTeX renders at its
    // natural size.
    let t = typ(
        "\\begin{table}\n%\\resizebox{\\linewidth}{!}{\n\\begin{tabular}{lc}\nA & B \\\\\n\\end{tabular}\n%}\n\\caption{C}\n\\end{table}",
    );
    assert!(!t.contains("byetex-fit"), "a commented-out wrapper is not applied; got:\n{t}");
    assert!(t.contains("table("), "the table still renders; got:\n{t}");
}

#[test]
fn an_escaped_percent_does_not_hide_a_live_resizebox() {
    // The control for the comment scan: `\%` is a literal percent, not a comment,
    // so a wrapper after one is still live.
    let t = typ(
        "\\begin{table}\n100\\% \\resizebox{\\linewidth}{!}{\n\\begin{tabular}{lc}\nA & B \\\\\n\\end{tabular}\n}\n\\end{table}",
    );
    assert!(t.contains("byetex-fit(100%)"), "`\\%` is not a comment; got:\n{t}");
}

#[test]
fn the_helper_only_shrinks_never_enlarges() {
    // `scale` yields a single unbreakable block, so scaling a long table UP drops
    // every row past the first page break — silently. Content already narrower
    // than the measure must be passed through untouched.
    let t = typ(r"\resizebox{\textwidth}{!}{\begin{tabular}{c}A\end{tabular}}");
    assert!(
        t.contains("natural > target"),
        "the helper guards on natural > target (shrink only); got:\n{t}"
    );
}

#[test]
fn a_degenerate_target_width_is_rejected() {
    // `0\textwidth` would render the table as nothing and `-0.5\textwidth` would
    // mirror it — neither is a size, so fall back to unscaled content.
    for w in ["0\\textwidth", "-0.5\\textwidth", "1e3\\textwidth"] {
        let t = typ(&format!(
            "\\resizebox{{{w}}}{{!}}{{\\begin{{tabular}}{{c}}KEEPME\\end{{tabular}}}}"
        ));
        assert!(!t.contains("byetex-fit"), "{w} is not a usable target; got:\n{t}");
        assert!(t.contains("KEEPME"), "{w}: content still survives; got:\n{t}");
    }
}
