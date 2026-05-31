//! Task 2 (layout fidelity): the neutral preamble picks up scalar layout
//! overrides derived from `\documentclass[opts]{class}` — font size
//! (`10pt`/`11pt`/`12pt`) and paper size (`a4paper`/`letterpaper`/...). When an
//! option is absent the neutral defaults (us-letter, 11pt) are kept, so this
//! layers onto the Task 1 base without changing documents that don't specify it.

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
    assert!(t.contains("size: 11pt"), "default 11pt font expected; got:\n{t}");
    assert!(
        t.contains("paper: \"us-letter\""),
        "default us-letter paper expected; got:\n{t}"
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
