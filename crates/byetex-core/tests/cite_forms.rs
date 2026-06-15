//! Tests for natbib/biblatex citation-form mapping (Unit 3).
//!
//! When a `.bib` resolves on disk (`#bibliography(.bib)` is emitted, so the
//! cite keys are real Typst bibliography entries), `\citet`/`\citeauthor`/…
//! map to the matching Typst `#cite(<key>, form: ...)` forms. When the bib is
//! NOT authoritative (inlined `.bbl`, `thebibliography`, bare convert) or the
//! citation sits inside math, every key keeps today's `@key` output because
//! `#cite(...)` would abort the compile.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-citeform-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Convert a `paper.tex` body with an authoritative `refs.bib` on disk.
fn convert_authoritative(name: &str, bib: &str, body: &str) -> (String, PathBuf) {
    let dir = tmpdir(name);
    fs::write(dir.join("refs.bib"), bib).unwrap();
    let tex = format!(
        "\\documentclass{{article}}\\begin{{document}}\n{}\n\\bibliography{{refs}}\\end{{document}}\n",
        body
    );
    fs::write(dir.join("paper.tex"), &tex).unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    (out.typst, dir)
}

const TWO_KEY_BIB: &str =
    "@article{Smith.2024, author={S}, year={2024}}\n@article{Jones.2023, author={J}, year={2023}}\n";

#[test]
fn citet_emits_prose_form() {
    let (typ, dir) = convert_authoritative(
        "citet",
        "@article{Smith.2024, year={2024}}\n",
        "See \\citet{Smith.2024}.",
    );
    assert!(
        typ.contains("#cite(<Smith.2024>, form: \"prose\")"),
        "expected prose #cite; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn citep_stays_at_form() {
    let (typ, dir) = convert_authoritative(
        "citep",
        "@article{Smith.2024, year={2024}}\n",
        "See \\citep{Smith.2024}.",
    );
    assert!(
        typ.contains("@Smith.2024"),
        "expected @Smith.2024; got:\n{typ}"
    );
    assert!(
        !typ.contains("form: \"prose\""),
        "citep must not become prose; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn citeauthor_and_citeyear_forms() {
    let (typ, dir) = convert_authoritative(
        "author-year",
        "@article{Smith.2024, year={2024}}\n",
        "\\citeauthor{Smith.2024} \\citeyear{Smith.2024}",
    );
    assert!(
        typ.contains("#cite(<Smith.2024>, form: \"author\")"),
        "expected author form; got:\n{typ}"
    );
    assert!(
        typ.contains("#cite(<Smith.2024>, form: \"year\")"),
        "expected year form; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn citeyearpar_wraps_in_parens() {
    let (typ, dir) = convert_authoritative(
        "yearpar",
        "@article{Smith.2024, year={2024}}\n",
        "\\citeyearpar{Smith.2024}",
    );
    assert!(
        typ.contains("(#cite(<Smith.2024>, form: \"year\"))"),
        "expected parenthesized year form; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn nocite_emits_form_none_not_bare_at() {
    let (typ, dir) = convert_authoritative(
        "nocite",
        "@article{Smith.2024, year={2024}}\n",
        "\\nocite{Smith.2024}",
    );
    assert!(
        typ.contains("#cite(<Smith.2024>, form: none)"),
        "expected form: none; got:\n{typ}"
    );
    assert!(
        !typ.contains("@Smith.2024"),
        "nocite must not render a bare @Smith.2024; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn multi_key_citet_emits_two_prose_tokens() {
    let (typ, dir) = convert_authoritative("multi", TWO_KEY_BIB, "\\citet{Smith.2024,Jones.2023}");
    assert!(
        typ.contains("#cite(<Smith.2024>, form: \"prose\")"),
        "missing Smith prose token; got:\n{typ}"
    );
    assert!(
        typ.contains("#cite(<Jones.2023>, form: \"prose\")"),
        "missing Jones prose token; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn supplement_on_citep_uses_bracket_form() {
    let (typ, dir) = convert_authoritative(
        "supp-citep",
        "@article{Smith.2024, year={2024}}\n",
        "\\citep[p.~5]{Smith.2024}",
    );
    assert!(
        typ.contains("@Smith.2024[p. 5]"),
        "expected @Smith.2024[p. 5]; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn supplement_on_citet_uses_supplement_arg() {
    let (typ, dir) = convert_authoritative(
        "supp-citet",
        "@article{Smith.2024, year={2024}}\n",
        "\\citet[p.~5]{Smith.2024}",
    );
    assert!(
        typ.contains("#cite(<Smith.2024>, form: \"prose\", supplement: [p. 5])"),
        "expected prose with supplement; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn thebibliography_guard_keeps_at_form() {
    // No resolvable .bib on disk (empty dir, base_dir set); a manual
    // thebibliography defines the key. Forms must be DISABLED → `@key`.
    let dir = tmpdir("thebib-guard");
    let tex = "\\documentclass{article}\\begin{document}\n\
        See \\citet{Smith.2024}.\n\
        \\begin{thebibliography}{99}\n\
        \\bibitem{Smith.2024} S. Author. Title. 2024.\n\
        \\end{thebibliography}\n\
        \\end{document}\n";
    fs::write(dir.join("paper.tex"), tex).unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("@Smith.2024"),
        "non-authoritative bib must keep @-form; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("form: \"prose\""),
        "forms must be disabled without a resolvable .bib; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bbl_only_paper_keeps_at_form() {
    // Regression (corpus 2605.30609): `\bibliography{refs}` is present but only
    // a `refs.bbl` ships on disk — no `refs.bib`. `emit_bibliography` inlines
    // the `.bbl` as `#figure ... <key>` labels; NO real `#bibliography(.bib)`
    // is emitted, so `#cite(<key>, form: …)` would abort with "the document
    // does not contain a bibliography". Forms MUST stay `@key`. The key harvest
    // reads the `.bbl` (so `had_bib_file` is true) — proving the form gate must
    // be `bib_will_render` (a real `.bib` resolved), not `bib_file_is_authoritative`.
    let dir = tmpdir("bbl-only");
    fs::write(
        dir.join("refs.bbl"),
        "\\begin{thebibliography}{1}\n\
         \\bibitem{Smith.2024} S. Author. Title. 2024.\n\
         \\end{thebibliography}\n",
    )
    .unwrap();
    let tex = "\\documentclass{article}\\begin{document}\n\
        See \\citet{Smith.2024}.\n\
        \\bibliography{refs}\\end{document}\n";
    fs::write(dir.join("paper.tex"), tex).unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("@Smith.2024"),
        "a .bbl-only paper must keep @-form (no real #bibliography); got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("#cite("),
        "no #cite forms without a resolvable .bib; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("#bibliography("),
        "sanity: this fixture must NOT emit a real #bibliography; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn math_mode_citation_stays_at_form() {
    // `\citet` inside `$...$` must keep @-form: `#cite(...)` function syntax
    // is unsafe in math. (If tree-sitter doesn't parse the citation node in
    // math, the assertion still holds trivially.)
    let (typ, dir) = convert_authoritative(
        "math-cite",
        "@article{Smith.2024, year={2024}}\n",
        "$x = \\citet{Smith.2024}$",
    );
    assert!(
        !typ.contains("#cite("),
        "no #cite( inside math; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_key_still_emits_placeholder_in_form_path() {
    let (typ, dir) = convert_authoritative(
        "missing",
        "@article{Smith.2024, year={2024}}\n",
        "\\citet{Ghost}",
    );
    assert!(
        typ.contains("[cite: missing key"),
        "missing-key placeholder must survive the form path; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(&dir);
}
