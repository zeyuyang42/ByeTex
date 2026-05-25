//! End-to-end test of the `.bib` preprocessor on the real-world
//! patterns that block Typst's BibLaTeX parser.

use byetex_core::bib::preprocess_bib;

#[test]
fn fixes_unresolved_string_with_quote() {
    // 2605.22507 pattern: `Journal = mor,` but no `@string{mor = "..."}`.
    let src = "@article{burnetas1997,\n\tJournal = mor,\n\tYear = 1997\n}\n";
    let out = preprocess_bib(src);
    assert!(
        out.contains("\"mor\""),
        "mor should be quoted; got:\n{}",
        out
    );
    assert!(
        !out.contains("Journal = mor,"),
        "raw `Journal = mor,` should be replaced; got:\n{}",
        out
    );
}

#[test]
fn resolves_at_string_to_literal() {
    let src = "@string{tpami = \"IEEE TPAMI\"}\n\
               @article{x, Journal = tpami, Year = 2024}\n";
    let out = preprocess_bib(src);
    assert!(
        out.contains("\"IEEE TPAMI\""),
        "resolved value missing; got:\n{}",
        out
    );
}

#[test]
fn fixes_newline_between_brace_and_key() {
    // 2605.22738 pattern: `@inproceedings{\n  Key.2025,\n  ...`
    let src = "@inproceedings{\n    Spliethoever.2025,\n    title = \"Foo\"\n}\n";
    let out = preprocess_bib(src);
    assert!(
        out.contains("@inproceedings{Spliethoever.2025,"),
        "key not glued to brace; got:\n{}",
        out
    );
}

#[test]
fn handles_latex_accent_in_braced_value() {
    // The brace matcher must not get confused by `\"` (LaTeX umlaut)
    // inside a `{...}` value group. This was the silent-failure mode
    // that made the preprocessor's transforms invisible on real
    // 22738 inputs (first entry's `author = {Splieth{\"o}ver}`).
    let src = "@inproceedings{good.2024,\n\tauthor = {Splieth{\\\"o}ver}\n}\n\
               @inproceedings{\n    next.2025,\n    title = \"X\"\n}\n";
    let out = preprocess_bib(src);
    assert!(
        out.contains("@inproceedings{next.2025,"),
        "second entry not normalized (brace-matcher confused by `\\\"`); got:\n{}",
        out
    );
}

#[test]
fn strips_stray_at_in_field_position() {
    // 2605.22738 pattern: a stray `@` glued to a field name like
    // `@doi = {10.1109/...}`. Typst reads this as an entry header
    // and aborts.
    let src = "@article{x, title = \"foo\", year = 2024,\n\t@doi = {10.1109/abc}\n}\n";
    let out = preprocess_bib(src);
    assert!(
        !out.contains("@doi"),
        "stray @doi not stripped; got:\n{}",
        out
    );
    assert!(out.contains("doi = {10.1109/abc}"));
}

#[test]
fn drops_duplicate_keys() {
    // 2605.22507 pattern: `BM13` defined twice. Typst aborts with
    // `duplicate key`. Keep the first occurrence.
    let src = "@article{BM13, year = 2013}\n@article{BM13, year = 2014}\n";
    let out = preprocess_bib(src);
    let count = out.matches("@article{BM13").count();
    assert_eq!(
        count, 1,
        "expected 1 BM13 entry; got {} in:\n{}",
        count, out
    );
}
