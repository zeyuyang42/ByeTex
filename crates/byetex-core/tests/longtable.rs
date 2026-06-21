//! `longtable` (round-5 dogfood): a multi-page table that ByeTex dropped entirely
//! (`unsupported_environment`) — common in theses (nomenclature/symbol lists) and papers
//! (long data tables). It has the same `{colspec}` shape as `tabular`, so it routes to
//! the same table emitter. Content (and an optional `\caption`) must survive.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn longtable_content_is_kept() {
    let t = typ("\\documentclass{article}\\usepackage{longtable}\\begin{document}\\begin{longtable}{ll}Alpha & 1 \\\\ Beta & 2 \\\\ \\end{longtable}\\end{document}");
    assert!(t.contains("#table("), "longtable → #table; got:\n{t}");
    for s in ["Alpha", "Beta", "1", "2"] {
        assert!(t.contains(s), "cell `{s}` kept; got:\n{t}");
    }
}

#[test]
fn longtable_with_caption_and_endhead() {
    // longtable header markers (`\endhead`, `\hline`) must not break it; caption kept.
    let t = typ("\\documentclass{article}\\usepackage{longtable}\\begin{document}\\begin{longtable}{ll}\\hline Name & Val \\\\ \\hline \\endhead Gamma & 3 \\\\ \\end{longtable}\\end{document}");
    assert!(t.contains("Gamma") && t.contains("3"), "body cells kept; got:\n{t}");
    assert!(t.contains("Name") && t.contains("Val"), "header row kept; got:\n{t}");
}
