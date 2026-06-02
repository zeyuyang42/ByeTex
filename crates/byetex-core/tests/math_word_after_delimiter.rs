//! Regression: a bare multi-letter math word that immediately follows a
//! non-alphabetic delimiter (e.g. `|arrival` after a `}` in
//! `$P(B_{\tau_i}|arrival\;process)$`) is parsed by tree-sitter as a single
//! `word` node whose alphabetic prefix is empty. The word-split arm only
//! handled an alphabetic prefix or a *digit* prefix, so a leading symbol like
//! `|` fell through to a verbatim copy and Typst then read `arrival` as an
//! unknown variable (corpus 2605.31072).

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn word_after_pipe_is_split() {
    // LaTeX: `$|arrival$` — the whole `|arrival` is one word node.
    let out = convert(r"$|arrival$");
    assert!(
        out.typst.contains("a r r i v a l"),
        "`arrival` after `|` must be letter-split, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("|arrival"),
        "`|arrival` must not be emitted verbatim, got: {}",
        out.typst
    );
}

#[test]
fn corpus_31072_conditional_prob_splits() {
    // The exact corpus shape: `|arrival` follows the `}` of a subscript group.
    let out = convert(r"$P(B_{\tau_i}|arrival\;process)$");
    assert!(
        out.typst.contains("a r r i v a l"),
        "`arrival` must be letter-split, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("p r o c e s s"),
        "`process` must be letter-split, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("|arrival"),
        "`|arrival` must not be emitted verbatim, got: {}",
        out.typst
    );
}

#[test]
fn pipe_delimiter_preserved() {
    // The `|` delimiter itself must be preserved (a valid Typst math bar).
    let out = convert(r"$|arrival$");
    assert!(
        out.typst.contains('|'),
        "the `|` delimiter must be preserved, got: {}",
        out.typst
    );
}

#[test]
fn plain_word_still_splits() {
    // Regression guard: a bare word with no prefix still splits.
    let out = convert(r"$arrival$");
    assert!(
        out.typst.contains("a r r i v a l"),
        "plain `arrival` must still split, got: {}",
        out.typst
    );
}

#[test]
fn digit_prefix_still_splits() {
    // Regression guard: the existing digit-prefix path is unaffected (it is now
    // a subset of the general non-alpha-prefix path).
    let out = convert(r"$2JX$");
    assert!(
        out.typst.contains("J X"),
        "`2JX` must still split to `J X`, got: {}",
        out.typst
    );
}
