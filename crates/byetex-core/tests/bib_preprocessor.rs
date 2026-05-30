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
fn drops_bare_at_with_no_entry_type() {
    // 2605.22724 pattern: a lone `@` left behind after an entry was
    // deleted — the `@` has no type and no `{`, so it's not a valid
    // entry.  Typst's parser aborts with `expected identifier`.
    let src = "@article{good, year = 2024}\n\n@\n\n@article{also_good, year = 2025}\n";
    let out = preprocess_bib(src);
    // The bare `@` must be silently dropped, not passed through.
    assert!(
        !out.contains("\n@\n"),
        "bare `@` must be dropped; got:\n{}",
        out
    );
    // The flanking valid entries must survive.
    assert!(out.contains("@article{good,"), "first entry lost: {}", out);
    assert!(
        out.contains("@article{also_good,"),
        "third entry lost: {}",
        out
    );
}

#[test]
fn resolves_biblatex_hash_concatenation() {
    // 2605.22817 pattern: `month = "oct" # "-" # nov` uses BibTeX's
    // `#` string-concatenation operator. Typst's BibLaTeX parser does
    // not support `#`. For month fields the range is truncated to the
    // first component (hayagriva rejects range strings like "oct-nov"
    // with "missing number"). For other fields the terms are joined.
    let src = r#"@inproceedings{yang2018hotpotqa,
    title = {HotpotQA},
    author = {Yang, Zhilin},
    month = "oct" # "-" # nov,
    year = "2018",
    booktitle = {EMNLP 2018},
    note = {Version} # { 2}
}
"#;
    let out = preprocess_bib(src);
    // `#` concatenation must be gone.
    assert!(
        !out.contains(" # "),
        "hash concatenation must be eliminated; got:\n{}",
        out
    );
    // Month range: only the first component survives.
    assert!(
        out.contains("month = \"oct\""),
        "month must be first component only; got:\n{}",
        out
    );
    // Other fields: parts are joined into one quoted string.
    assert!(
        out.contains("\"Version 2\""),
        "non-month # concat must be joined; got:\n{}",
        out
    );
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

#[test]
fn drops_bibtex_bdsk_fields() {
    // 2605.22724 pattern: BibDesk-specific `Bdsk-Url-*` fields whose
    // URLs contain `$` or `%7D` (URL-encoded chars). Typst's BibLaTeX
    // parser chokes on `$` inside a braced field value (treats it as
    // math mode). These fields carry no bibliographic information and
    // must be dropped by the preprocessor. Also covers `OPTBdsk-*`
    // (BibDesk's "optional" variant).
    let src = "@article{x,\n\
               \tyear = {2017},\n\
               \tBdsk-Url-1 = {https://doi.org/10.1162/neco%7D$}}\n";
    let out = preprocess_bib(src);
    assert!(
        !out.contains("Bdsk-Url-1"),
        "Bdsk-Url-1 must be dropped; got:\n{}",
        out
    );
    assert!(
        out.contains("year = {2017}"),
        "year field must survive; got:\n{}",
        out
    );
    // OPTBdsk-* variant (BibDesk marks optional fields with OPT prefix)
    let src2 = "@article{y,\n\
                \tyear = {2020},\n\
                \tOPTBdsk-Url-1 = {https://example.com}}\n";
    let out2 = preprocess_bib(src2);
    assert!(
        !out2.contains("OPTBdsk-Url-1"),
        "OPTBdsk-Url-1 must be dropped; got:\n{}",
        out2
    );
    assert!(
        out2.contains("year = {2020}"),
        "year field must survive in OPTBdsk test; got:\n{}",
        out2
    );
}

#[test]
fn normalizes_year_field_with_month_prefix() {
    // 2605.22507 pattern: `Year = {February, 1993}` — hayagriva rejects
    // non-numeric year values with "wrong number of digits". When the
    // year field contains a 4-digit year embedded in other text, keep
    // only that year. When there is no recognizable year (e.g. "to
    // appear"), drop the field entirely to prevent hayagriva from aborting.
    let src = "@article{x, Year = {February, 1993}}\n";
    let out = preprocess_bib(src);
    // Field name case is preserved in output (Year not year)
    assert!(
        out.contains("{1993}"),
        "year with month prefix must be normalized to just the year; got:\n{}",
        out
    );
    assert!(
        !out.contains("February"),
        "month name must be stripped from year field; got:\n{}",
        out
    );

    // "to appear" — no year number, field must be dropped
    let src2 = "@article{y, title = {Foo}, Year = {to appear}}\n";
    let out2 = preprocess_bib(src2);
    assert!(
        !out2.contains("to appear"),
        "non-numeric 'to appear' year must be dropped; got:\n{}",
        out2
    );

    // "September, 1989" variant
    let src3 = "@article{z, Year = {September, 1989}}\n";
    let out3 = preprocess_bib(src3);
    assert!(
        out3.contains("{1989}"),
        "year with month name must be normalized; got:\n{}",
        out3
    );
}

#[test]
fn normalizes_month_field_with_range_or_number() {
    // 2605.22507 pattern: `Month = {May-June}`, `Month = {May 13}`,
    // `Month = {October 7--10}`. hayagriva rejects these with "missing
    // number". Keep only the first alphabetic word from the month value.
    let cases = [
        ("@article{a, month = {May-June}}", "month = {May}"),
        ("@article{b, month = {May 13}}", "month = {May}"),
        ("@article{c, month = {October 7--10}}", "month = {October}"),
        ("@article{d, month = {nov.}}", "month = {nov}"),
        // Pure month names should be unchanged
        ("@article{e, month = {January}}", "month = {January}"),
        ("@article{f, month = \"Sep\"}", "month = \"Sep\""),
    ];
    for (src, expected_fragment) in &cases {
        let out = preprocess_bib(src);
        assert!(
            out.contains(expected_fragment),
            "month normalization: expected '{}' in output; got:\n{}",
            expected_fragment,
            out
        );
    }
}

#[test]
fn normalizes_day_field_with_range() {
    // 2605.22507 pattern: `Day = {11--15}` — hayagriva expects a plain
    // integer for the day field. Keep only the first digit sequence.
    let src = "@article{a, day = {11--15}, year = {2020}}\n";
    let out = preprocess_bib(src);
    assert!(
        out.contains("day = {11}"),
        "day range must be normalized to first number; got:\n{}",
        out
    );
    // Pure day value should be unchanged
    let src2 = "@article{b, day = {15}}\n";
    let out2 = preprocess_bib(src2);
    assert!(
        out2.contains("day = {15}"),
        "plain day value must be unchanged; got:\n{}",
        out2
    );
}
