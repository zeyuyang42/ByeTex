//! Design fidelity: align section-heading sizes + spacing to LaTeX `article`.
//! article.cls (10pt): \section=\Large (1.44em), before 3.5ex (~1.5em), after
//! 2.3ex (~1.0em); \subsection=\large (1.2em) / \subsubsection=\normalsize
//! (1.0em), before 3.25ex (~1.4em), after 1.5ex (~0.65em). byetex had
//! 1.3/1.15/1.0em with a uniform block(above:1.2em, below:0.6em).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn preamble() -> String {
    typ("\\documentclass{article}\n\\begin{document}\n\\section{S}\nBody.\n\\end{document}")
}

#[test]
fn heading_sizes_match_latex_article() {
    let t = preamble();
    assert!(t.contains("level: 1): set text(size: 1.44em"), "L1 = \\Large 1.44em; got:\n{t}");
    assert!(t.contains("level: 2): set text(size: 1.2em"), "L2 = \\large 1.2em; got:\n{t}");
    assert!(t.contains("level: 3): set text(size: 1em"), "L3 = \\normalsize 1em; got:\n{t}");
}

#[test]
fn heading_block_spacing_is_per_level() {
    let t = preamble();
    // A single transform rule with a per-level conditional (section gets more
    // space than sub/subsubsection), not the old uniform 1.2em/0.6em.
    assert!(
        t.contains("block(above:") && t.contains("it.level == 1"),
        "expected per-level heading block spacing; got:\n{t}"
    );
    assert!(
        !t.contains("block(above: 1.2em, below: 0.6em, it)"),
        "old uniform heading spacing should be gone; got:\n{t}"
    );
}
