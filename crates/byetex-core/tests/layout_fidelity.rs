//! Task 2 (layout fidelity): the neutral preamble picks up scalar layout
//! overrides derived from `\documentclass[opts]{class}` — font size
//! (`10pt`/`11pt`/`12pt`) and paper size (`a4paper`/`letterpaper`/...). When an
//! option is absent the neutral defaults (us-letter, 10pt — LaTeX's `article`
//! default) are kept, so this layers onto the Task 1 base without changing
//! documents that don't specify it.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn font_size_option_overrides_default() {
    for (opt, want) in [("10pt", "size: 10pt"), ("12pt", "size: 12pt")] {
        let src = format!(
            "\\documentclass[{opt}]{{article}}\n\\begin{{document}}\nBody.\n\\end{{document}}"
        );
        let t = typ(&src);
        assert!(
            t.contains(want),
            "[{opt}]: expected `{want}` in `#set text`; got:\n{t}"
        );
    }
}

#[test]
fn paper_size_option_maps_to_typst() {
    for (opt, want) in [
        ("a4paper", "paper: \"a4\""),
        ("letterpaper", "paper: \"us-letter\""),
        ("a5paper", "paper: \"a5\""),
        ("legalpaper", "paper: \"us-legal\""),
    ] {
        let src = format!(
            "\\documentclass[{opt}]{{article}}\n\\begin{{document}}\nBody.\n\\end{{document}}"
        );
        let t = typ(&src);
        assert!(
            t.contains(want),
            "[{opt}]: expected `{want}` in `#set page`; got:\n{t}"
        );
    }
}

#[test]
fn defaults_kept_when_no_layout_options() {
    let src = "\\documentclass{article}\n\\begin{document}\nBody.\n\\end{document}";
    let t = typ(src);
    // LaTeX `\documentclass{article}` with no size option is 10pt.
    assert!(
        t.contains("size: 10pt"),
        "default 10pt font expected; got:\n{t}"
    );
    assert!(
        t.contains("paper: \"us-letter\""),
        "default us-letter paper expected; got:\n{t}"
    );
    // Paragraph spacing matches the line leading (LaTeX `article` is
    // indent-only — no extra inter-paragraph gap).
    assert!(
        t.contains("spacing: 0.65em"),
        "indent-only paragraph spacing expected; got:\n{t}"
    );
}

#[test]
fn font_and_paper_combine_and_coexist_with_class_options() {
    // A class-specific option (`conference` for IEEEtran) must not stop the
    // generic scalar options from being applied.
    let src = "\\documentclass[conference,12pt,a4paper]{IEEEtran}\n\
               \\begin{document}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(t.contains("size: 12pt"), "12pt expected; got:\n{t}");
    assert!(t.contains("paper: \"a4\""), "a4 paper expected; got:\n{t}");
}

#[test]
fn ieeetran_gets_tight_class_default_margins() {
    // IEEEtran's own geometry is far tighter than the neutral 1in; using 1in
    // narrows the two columns and inflates page count (22779 page_ratio 1.38).
    // With no explicit \geometry, IEEEtran picks up a tight class-default margin.
    let src = "\\documentclass[conference]{IEEEtran}\n\\begin{document}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        t.contains("margin: (top: 0.75in, bottom: 1in, x: 0.62in)"),
        "IEEEtran should use a tight class-default margin; got:\n{t}"
    );
}

#[test]
fn article_keeps_neutral_one_inch_margin() {
    // Regression guard: a plain article (no geometry) keeps the neutral 1in —
    // the tight margin is class-specific, not global.
    let src = "\\documentclass{article}\n\\begin{document}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        t.contains("margin: (x: 1in, y: 1in)"),
        "article must keep the neutral 1in margin; got:\n{t}"
    );
}

#[test]
fn explicit_geometry_overrides_ieeetran_default() {
    // A real \usepackage[margin=2cm]{geometry} must still win over the class
    // default.
    let src = "\\documentclass[conference]{IEEEtran}\n\
               \\usepackage[margin=2cm]{geometry}\n\\begin{document}\nB.\n\\end{document}";
    let t = typ(src);
    assert!(
        t.contains("2cm") && !t.contains("0.62in"),
        "explicit geometry must override the IEEEtran default; got:\n{t}"
    );
}
