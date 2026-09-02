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

// ── LaTeX spacing inside a citation supplement ────────────────────────────────
//
// `\cite[Sec.\ 2]{k}` uses LaTeX's control space, an ordinary interword space.
// Copied verbatim into a Typst supplement it becomes `\ `, which Typst renders
// as a FORCED LINE BREAK — verified: `Sec.\ 2` renders "Sec." / newline / "2".
// A dogfood agent found ~46 citations on one paper broken into orphan one-word
// lines; removing it alone took that paper from 29 pages to 27 (truth is 22).
// It compiles cleanly and matches no leaked-`\command` pattern, so neither
// `warnings.json` nor `byetex diagnose` says anything.

#[test]
fn control_space_in_a_supplement_becomes_a_real_space() {
    let (typ, dir) = convert_authoritative(
        "ctrlspace",
        "@article{Smith.2024, year={2024}}\n",
        "See \\citep[Sec.\\ 2]{Smith.2024}.",
    );
    assert!(
        typ.contains("@Smith.2024[Sec. 2]"),
        "`\\ ` must become a plain space, not Typst's linebreak; got:\n{typ}"
    );
    assert!(
        !typ.contains("Sec.\\ 2"),
        "a bare `\\ ` in a supplement is a forced line break; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_other_latex_spacings_are_normalised_too() {
    for (latex, want) in [
        ("p.\\,60", "p. 60"),
        ("p.\\;60", "p. 60"),
        ("p.\\:60", "p. 60"),
        ("Ch.\\quad 3", "Ch. 3"),
    ] {
        let (typ, dir) = convert_authoritative(
            "spacings",
            "@article{K, year={2024}}\n",
            &format!("See \\citep[{latex}]{{K}}."),
        );
        assert!(
            typ.contains(&format!("@K[{want}]")),
            "{latex} → {want}; got:\n{typ}"
        );
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn supplement_text_is_otherwise_untouched() {
    // The control: normalising must not eat ordinary content or the `~` handling
    // that already worked.
    let (typ, dir) = convert_authoritative(
        "plainsupp",
        "@article{K, year={2024}}\n",
        "See \\citep[Rem.~4.2, pp.~60--61]{K}.",
    );
    // `--` becomes a real en-dash here, which is the intended LaTeX→Typst
    // typography and not something this change touches.
    assert!(
        typ.contains("@K[Rem. 4.2, pp. 60\u{2013}61]"),
        "plain supplement text and `~` are preserved; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(dir);
}

// ── Regressions found in review ──────────────────────────────────────────────

#[test]
fn an_escaped_backslash_is_not_read_as_a_control_space() {
    // `a\\ b` is an escaped backslash then a space. Scanning naively, the SECOND
    // `\` starts `\ ` and emits a space — leaving `a\` + space, which is the
    // forced line break this whole change removes.
    let (typ, dir) = convert_authoritative(
        "escbs",
        "@article{K, year={2024}}\n",
        r"See \citep[a\\ b]{K}.",
    );
    assert!(
        !typ.contains("@K[a\\ b]"),
        "an escaped backslash must not become a control space; got:\n{typ}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_negative_thin_space_keeps_the_real_space_after_it() {
    // `\!` emits nothing, so the interword space following it is the author's and
    // must survive: `Fig.\! 3` is "Fig. 3", not "Fig.3".
    let (typ, dir) = convert_authoritative(
        "negthin",
        "@article{K, year={2024}}\n",
        r"See \citep[Fig.\! 3]{K}.",
    );
    assert!(typ.contains("@K[Fig. 3]"), "`\\!` must not eat the space; got:\n{typ}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_wider_spacing_set_is_covered() {
    // Anything omitted is copied verbatim and renders as its literal NAME —
    // `p.\thinspace 5` came out as "p.thinspace 5". Match the set the text-mode
    // emitter already handles.
    for latex in [r"p.\thinspace 5", r"p.\enspace 5", r"p.\kern 5", r"p.\linebreak 5"] {
        let (typ, dir) = convert_authoritative(
            "wideset",
            "@article{K, year={2024}}\n",
            &format!("See \\citep[{latex}]{{K}}."),
        );
        assert!(
            typ.contains("@K[p. 5]"),
            "{latex} → `p. 5`, never a literal command name; got:\n{typ}"
        );
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn consecutive_spacing_commands_collapse() {
    let (typ, dir) = convert_authoritative(
        "collapse",
        "@article{K, year={2024}}\n",
        r"See \citep[Ch.\quad\quad 3]{K}.",
    );
    assert!(typ.contains("@K[Ch. 3]"), "one gap, not two spaces; got:\n{typ}");
    let _ = fs::remove_dir_all(dir);
}
