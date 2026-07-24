//! Warning snippets are cut from the emitter's source, which by then has had
//! `_` inside cross-reference keys replaced by the `U+001F` sentinel
//! (`ir::neutralize_ref_key_underscores`). The restore only ran over the
//! emitted Typst, so `warnings.json` — the surface agents are pointed at for
//! repairs — carried a raw control character where the author wrote `_`,
//! making the snippet ungreppable against the real LaTeX (review finding #8).

use byetex_core::{convert, ConvertOptions};

#[test]
fn warning_snippets_never_contain_the_refkey_sentinel() {
    let out = convert(
        "\\documentclass{article}\n\\begin{document}\n\\weirdcmd{\\label{tab:some_key}}\n\\end{document}\n",
        &ConvertOptions::default(),
    );
    assert!(
        !out.warnings.is_empty(),
        "fixture should produce at least one warning"
    );
    for w in &out.warnings {
        assert!(
            !w.snippet.contains('\u{1f}'),
            "sentinel leaked into warning snippet: {:?}",
            w.snippet
        );
        assert!(
            !w.message.contains('\u{1f}'),
            "sentinel leaked into warning message: {:?}",
            w.message
        );
    }
}

#[test]
fn warning_snippet_restores_the_original_underscore() {
    let out = convert(
        "\\documentclass{article}\n\\begin{document}\n\\weirdcmd{\\label{tab:some_key}}\n\\end{document}\n",
        &ConvertOptions::default(),
    );
    let joined: String = out
        .warnings
        .iter()
        .map(|w| w.snippet.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("tab:some_key"),
        "snippet should be greppable against the LaTeX source: {joined:?}"
    );
}
