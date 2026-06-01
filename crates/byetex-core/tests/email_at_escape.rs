//! Expanded-corpus compile-blocker (2605.31564): an email address in the author
//! block — `\{qxw5305, jacob.devasier\}@mavs.uta.edu` — emitted a BARE `@mavs`,
//! which Typst parses as a reference `@mavs` → `label <mavs.uta.edu> does not
//! exist` → compile failure. byetex emits every real `@ref` preceded by
//! whitespace, so a `@` glued to a non-space char (here `}`) is mid-word and
//! must be escaped to `\@`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn email_at_after_brace_is_escaped() {
    let t = typ("Contact \\{qxw5305, jacob.devasier\\}@mavs.uta.edu, cli@uta.edu today.");
    // No UNESCAPED `@` (a live reference) may survive — every email `@` must be
    // `\@`. Check there's no `@` whose preceding char isn't a backslash.
    let bare_at = t.match_indices('@').any(|(i, _)| {
        i == 0 || t.as_bytes()[i - 1] != b'\\'
    });
    assert!(
        !bare_at,
        "every email @ must be escaped to \\@ (no live reference); got:\n{t}"
    );
    // And it's escaped, not dropped.
    assert!(t.contains("\\@mavs"), "the @ must be escaped to \\@; got:\n{t}");
}

#[test]
fn real_reference_after_space_is_preserved() {
    // Regression guard: a genuine \cref/\ref (byetex emits ` @key`) stays live.
    let t = typ(
        "See \\cref{eq:x}.\n\\begin{equation}E=mc^2\\label{eq:x}\\end{equation}",
    );
    assert!(
        t.contains("@eq:x"),
        "a real reference preceded by whitespace must stay `@eq:x`; got:\n{t}"
    );
    assert!(!t.contains("\\@eq:x"), "real ref must NOT be escaped; got:\n{t}");
}
