//! Regression for the comma-splitting allowlist (follow-up to PR #137).
//!
//! #137 restricted comma-splitting of a label-ref brace to a hardcoded set of
//! cleveref list commands, but omitted their STARRED forms (`\cref*`, `\Cref*`,
//! `\cpageref*`, ...) and the plural name-cref commands. A starred cleveref list
//! then collapsed into one literal key (`\cref*{a,b}` → dangling `@a-b`) instead
//! of two refs. The starred form behaves identically to the unstarred one, so it
//! must split too. (`\eqref` & friends still keep a comma as a literal char.)

use byetex_core::{convert, ConvertOptions};

fn refs(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    )
    .typst
}

#[test]
fn starred_cleveref_list_commands_split_on_comma() {
    // The starred list commands present in the tree-sitter-latex grammar.
    for cmd in ["\\cref*", "\\Cref*", "\\labelcref*", "\\labelcpageref*"] {
        let out = refs(&format!("See {cmd}{{a,b}}.\n"));
        assert!(
            out.contains("@a") && out.contains("@b"),
            "{cmd}{{a,b}} should emit both @a and @b; got:\n{out}"
        );
        assert!(
            !out.contains("@a-b") && !out.contains("@a,b"),
            "{cmd}{{a,b}} must not fuse the keys into one ref; got:\n{out}"
        );
    }
}

#[test]
fn plural_namecref_commands_split_on_comma() {
    for cmd in ["\\namecrefs", "\\nameCrefs", "\\lcnamecrefs"] {
        let out = refs(&format!("the {cmd}{{a,b}}.\n"));
        assert!(
            !out.contains("@a-b"),
            "{cmd}{{a,b}} (a plural list command) must not fuse keys; got:\n{out}"
        );
    }
}

#[test]
fn eqref_comma_still_literal_after_fix() {
    // Regression guard: the #137 behaviour for single-key commands is preserved.
    let out = refs("Eq. \\eqref{eqn:a,eqn:b}.\n");
    assert!(
        out.contains("(@eqn:a-eqn:b)"),
        "single-key \\eqref must keep the comma as a literal key; got:\n{out}"
    );
}
