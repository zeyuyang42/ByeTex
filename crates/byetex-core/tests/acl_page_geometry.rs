//! ACL Anthology papers use `\documentclass[11pt]{article}` + `\usepackage{acl}`,
//! where `acl.sty` hard-codes a4 paper, 2.5cm margins and a 10pt body
//! (`\PassOptionsToPackage{a4paper,margin=2.5cm}{geometry}` + `\xpt`) — overriding
//! the `11pt` class option. The emitter used to keep the article defaults
//! (us-letter, 11pt), inflating the page count ~50% (corpus 2605.31586, dogfood
//! backlog F2). The ACL venue style now drives the page geometry.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const ACL: &str =
    "\\documentclass[11pt]{article}\\usepackage{acl}\\begin{document}Hello.\\end{document}";

#[test]
fn acl_forces_a4_paper() {
    let t = typ(ACL);
    assert!(
        t.contains("paper: \"a4\""),
        "ACL must set a4 paper; got:\n{t}"
    );
    assert!(
        !t.contains("us-letter"),
        "ACL must not keep us-letter; got:\n{t}"
    );
}

#[test]
fn acl_forces_10pt_body_over_11pt_option() {
    let t = typ(ACL);
    assert!(
        t.contains("size: 10pt"),
        "ACL must override 11pt to 10pt body; got:\n{t}"
    );
    assert!(
        !t.contains("size: 11pt"),
        "ACL must not emit 11pt; got:\n{t}"
    );
}

#[test]
fn acl_forces_2p5cm_margin() {
    let t = typ(ACL);
    assert!(
        t.contains("margin: 2.5cm"),
        "ACL must set 2.5cm margins; got:\n{t}"
    );
}

#[test]
fn non_acl_article_keeps_defaults() {
    // A plain article must be untouched by the ACL venue override.
    let t = typ(r"\documentclass[11pt]{article}\begin{document}Hi.\end{document}");
    assert!(t.contains("us-letter"), "plain article keeps us-letter; got:\n{t}");
    assert!(t.contains("size: 11pt"), "plain article keeps 11pt; got:\n{t}");
}

#[test]
fn acl_respects_explicit_user_geometry_margin() {
    // If the document explicitly loads geometry, that margin wins over the 2.5cm
    // venue default (paper + font are still ACL-forced).
    let t = typ(
        "\\documentclass[11pt]{article}\\usepackage{acl}\\usepackage[margin=1in]{geometry}\\begin{document}Hi.\\end{document}",
    );
    assert!(t.contains("margin: 1in"), "explicit geometry margin wins; got:\n{t}");
    assert!(t.contains("paper: \"a4\""), "paper still ACL-forced; got:\n{t}");
}
