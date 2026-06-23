//! ACL author block (fidelity-fix Phase 2). ACL papers use the same multi-`\textbf{Name
//! \textsuperscript{n}}` + `\quad` author row as NeurIPS, with `\textsuperscript{n} Institution`
//! legend lines. Routed through the generic parser, the real institutions were DROPPED and a
//! `\thanks{Correspondence…}` note was mis-used as the affiliation (the "footnote added to the
//! author section incorrectly" the user saw). Route ACL through the neurips author parser so
//! each name maps to its real institution.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const ACL_AUTHORS: &str = r"\documentclass[11pt]{article}\usepackage{acl}
\title{T}
\author{
    \textbf{Songhao Wu\textsuperscript{1}}
    \quad \textbf{Ang Lv\textsuperscript{1}}
    \quad \textbf{Ruobing Xie\textsuperscript{2}\thanks{Correspondence to: Ruobing Xie, Yankai Lin.}}
    \quad \textbf{Yankai Lin\textsuperscript{1}\footnotemark[1]} \\
    \textsuperscript{1} Gaoling School of Artificial Intelligence, Renmin University of China \\
    \textsuperscript{2} Large Language Model Department, Tencent \\
}
\begin{document}\maketitle\end{document}";

#[test]
fn acl_real_institutions_are_kept() {
    let t = typ(ACL_AUTHORS);
    assert!(t.contains("Gaoling School of Artificial Intelligence"),
        "affiliation 1 (Gaoling) must appear; got:\n{t}");
    assert!(t.contains("Large Language Model Department, Tencent"),
        "affiliation 2 (Tencent) must appear; got:\n{t}");
}

#[test]
fn acl_thanks_note_is_not_the_affiliation() {
    // The correspondence \thanks must NOT be rendered as an affiliation (the visible bug).
    let t = typ(ACL_AUTHORS);
    assert!(!t.contains("#super[1] Correspondence"),
        "the \\thanks correspondence note must not be used as affiliation #1; got:\n{t}");
}

#[test]
fn acl_all_four_author_names_present() {
    let t = typ(ACL_AUTHORS);
    for name in ["Songhao Wu", "Ang Lv", "Ruobing Xie", "Yankai Lin"] {
        assert!(t.contains(name), "author {name:?} must appear; got:\n{t}");
    }
    // And the \thanks / \footnotemark commands must not leak as literal LaTeX or stray `[1]`.
    assert!(!t.contains("\\thanks"), "\\thanks must not leak into output; got:\n{t}");
    assert!(!t.contains("\\footnotemark"), "\\footnotemark must not leak; got:\n{t}");
    assert!(!t.contains("Lin\\[1\\]") && !t.contains("Lin[1]"),
        "the \\footnotemark[1] bracket must not leak after the name; got:\n{t}");
}
