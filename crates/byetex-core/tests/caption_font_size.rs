//! Caption font size, detected from `\captionsetup`.
//!
//! LaTeX classes almost always set captions smaller than body text; the emitter
//! set caption POSITION but never SIZE, so captions rendered at full body size.
//! Measured against the LaTeX truth, that is a large share of the corpus-wide
//! small-text deficit: `layout_small_tier_share_delta` is negative on 46 of 64
//! papers, mean -0.106 (we render ~10.6 points fewer small glyphs than truth).
//! On 2605.31604 the 9pt text in truth is dominated by figure/table captions.
//!
//! Detected from the source rather than baked in per class: `\captionsetup` is
//! what the author actually wrote, and 21 corpus papers carry a `font=` size.

use byetex_core::{convert, ConvertOptions};

fn conv(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// The emitted caption rule only. Asserting on a bare `1.44em` substring matches
/// the level-1 HEADING rule too, which made a first version of these tests fail
/// for the wrong reason.
fn caption_rule(typst: &str) -> String {
    typst
        .lines()
        .find(|l| l.contains("figure.caption: set text"))
        .unwrap_or("<no caption rule>")
        .to_string()
}

fn doc(preamble: &str) -> String {
    format!(
        "\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n\
         \\begin{{figure}}\\caption{{C}}\\end{{figure}}\n\\end{{document}}"
    )
}

#[test]
fn captionsetup_font_small_sizes_the_caption() {
    let out = conv(&doc("\\usepackage{caption}\n\\captionsetup{font=small}"));
    assert!(
        out.contains("show figure.caption: set text(size: 0.9em)"),
        "font=small must size captions; got:\n{out}"
    );
}

#[test]
fn a_braced_font_list_is_read() {
    // `font={small,it}` is the common multi-key form.
    let out = conv(&doc("\\captionsetup{font={small,it}}"));
    assert!(
        out.contains("size: 0.9em"),
        "font={{small,it}} must still yield the size; got:\n{out}"
    );
}

#[test]
fn footnotesize_captions_use_their_own_ratio() {
    let out = conv(&doc("\\captionsetup{font=footnotesize}"));
    assert!(out.contains("size: 0.8em"), "footnotesize=0.8em; got:\n{out}");
}

#[test]
fn a_float_type_specific_captionsetup_is_honoured() {
    // `\captionsetup[table]{...}` scopes to tables; we take it as the caption
    // size rather than ignoring the declaration entirely.
    let out = conv(&doc("\\captionsetup[table]{font=small}"));
    assert!(out.contains("size: 0.9em"), "the optional arg must not defeat it; got:\n{out}");
}

#[test]
fn font_normalsize_does_not_emit_a_rule() {
    // An explicit reset to body size is a real declaration, but emitting
    // `size: 1em` is a no-op that only adds noise.
    let out = conv(&doc("\\captionsetup{font=normalsize}"));
    assert!(
        !out.contains("figure.caption: set text"),
        "normalsize needs no rule; got:\n{out}"
    );
}

#[test]
fn a_non_size_font_key_is_ignored() {
    // `font=bf` carries weight, not size — it must not be read as a size.
    let out = conv(&doc("\\captionsetup{font=bf}"));
    assert!(
        !out.contains("figure.caption: set text(size"),
        "font=bf is not a size; got:\n{out}"
    );
}

#[test]
fn a_commented_captionsetup_is_ignored() {
    let out = conv(&doc("%\\captionsetup{font=small}"));
    assert!(
        !out.contains("figure.caption: set text(size"),
        "a commented declaration must not fire; got:\n{out}"
    );
}

#[test]
fn no_captionsetup_leaves_output_unchanged() {
    // The control: documents without the package must be byte-identical to before.
    let out = conv(&doc(""));
    assert!(
        !out.contains("figure.caption: set text(size"),
        "must not size captions unbidden; got:\n{out}"
    );
}

#[test]
fn a_declaration_in_a_bundled_style_file_is_found() {
    // Only 3 of 67 corpus papers put `\captionsetup` in the entry file; the rest
    // carry it in an `\input`ed preamble or the class they ship. Scanning the
    // main source alone reached 2 papers.
    use std::fs;
    let dir = std::env::temp_dir().join(format!("byetex-cap-sty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("venue.sty"), "\\captionsetup{font=footnotesize}\n").unwrap();
    let main = "\\documentclass{article}\n\\usepackage{venue}\n\\begin{document}\n\
                \\begin{figure}\\caption{C}\\end{figure}\n\\end{document}";
    fs::write(dir.join("main.tex"), main).unwrap();
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(dir.clone()),
        },
    )
    .typst;
    let _ = fs::remove_dir_all(&dir);
    assert!(
        out.contains("size: 0.8em"),
        "a bundled .sty declaration must be found; got:\n{out}"
    );
}

#[test]
fn the_document_beats_a_bundled_class_default() {
    use std::fs;
    let dir = std::env::temp_dir().join(format!("byetex-cap-pri-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("venue.cls"), "\\captionsetup{font=footnotesize}\n").unwrap();
    let main = "\\documentclass{venue}\n\\captionsetup{font=small}\n\\begin{document}\n\
                \\begin{figure}\\caption{C}\\end{figure}\n\\end{document}";
    fs::write(dir.join("main.tex"), main).unwrap();
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(dir.clone()),
        },
    )
    .typst;
    let _ = fs::remove_dir_all(&dir);
    assert!(out.contains("size: 0.9em"), "the document's own \\captionsetup wins; got:\n{out}");
    assert!(!out.contains("0.8em"), "the class default must not win; got:\n{out}");
}

#[test]
fn a_neighbouring_key_value_is_not_read_as_the_caption_size() {
    // A guard: `font=` already wins here because it is read first. The case that
    // actually distinguishes a precise parser is `a_size_only_in_labelfont_...`
    // below, where `font=` carries NO size.
    let rule = caption_rule(&conv(&doc("\\captionsetup{font=small,labelfont=\\Large}")));
    assert!(rule.contains("size: 0.9em"), "font= is small; got: {rule}");
    assert!(
        !rule.contains("1.44em"),
        "labelfont's value must not become the caption size; got: {rule}"
    );
}

#[test]
fn a_size_only_in_labelfont_does_not_size_the_whole_caption() {
    // `labelfont` styles the "Figure 1:" label alone, not the caption text, so a
    // size there must NOT become the caption size. A guard rather than a
    // regression test: it passes on the looser parser this replaced too.
    let rule = caption_rule(&conv(&doc("\\captionsetup{font={it,bf},labelfont={small}}")));
    assert_eq!(rule, "<no caption rule>", "labelfont is not the caption font; got: {rule}");
}

#[test]
fn only_the_braced_group_of_font_is_read() {
    let rule = caption_rule(&conv(&doc("\\captionsetup{font={small,it},labelfont={\\huge}}")));
    assert!(rule.contains("size: 0.9em"), "font={{small,it}} is small; got: {rule}");
    assert!(!rule.contains("2.074em"), "labelfont must not leak in; got: {rule}");
}

#[test]
fn a_size_in_a_makecaption_definition_is_read() {
    // Classes that predate the `caption` package style captions by redefining
    // `\@makecaption` instead of calling `\captionsetup`. Measured: 8 corpus
    // papers carry a size ONLY there, which `\captionsetup` detection cannot see
    // — that is nearly as many again as the 9 the package form reaches.
    use std::fs;
    let dir = std::env::temp_dir().join(format!("byetex-cap-mk-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("venue.sty"),
        "\\long\\def\\@makecaption#1#2{%\n  \\vskip\\abovecaptionskip\n  \\small\n  \\sbox\\@tempboxa{#1: #2}%\n}\n",
    )
    .unwrap();
    let main = "\\documentclass{article}\n\\usepackage{venue}\n\\begin{document}\n\
                \\begin{figure}\\caption{C}\\end{figure}\n\\end{document}";
    fs::write(dir.join("main.tex"), main).unwrap();
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(dir.clone()),
        },
    )
    .typst;
    let _ = fs::remove_dir_all(&dir);
    assert!(
        caption_rule(&out).contains("size: 0.9em"),
        "\\@makecaption's \\small must size captions; got: {}",
        caption_rule(&out)
    );
}

#[test]
fn captionsetup_wins_over_makecaption() {
    // `\captionsetup` is the document's explicit statement; a class's
    // `\@makecaption` is the fallback.
    let out = conv(&doc("\\captionsetup{font=footnotesize}\n\\long\\def\\@makecaption#1#2{\\small #1: #2}"));
    assert!(
        caption_rule(&out).contains("size: 0.8em"),
        "captionsetup wins; got: {}",
        caption_rule(&out)
    );
}

#[test]
fn a_makecaption_without_a_size_emits_no_rule() {
    let out = conv(&doc("\\long\\def\\@makecaption#1#2{#1: #2}"));
    assert_eq!(
        caption_rule(&out),
        "<no caption rule>",
        "no size in the definition, no rule"
    );
}
