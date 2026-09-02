//! LaTeX's `align` numbers EVERY line; ByeTex emitted one Typst block, so it got
//! one number — and every equation number after it was wrong for the rest of the
//! document.
//!
//! Found by the dogfood loop (2605.31499, 2026-08-14): a 3-line `align` shifted
//! all subsequent numbers by 2, so a reference to "(21)" landed on equation 19.
//! Corpus-wide: 32 papers, 271 multi-line blocks, **472 lost equation numbers**
//! (one paper loses 187, making its numbering meaningless). Silent — it compiles
//! cleanly and no warning fires.
//!
//! The fix uses `@preview/equate` in `number-mode: "line"`, imported only when a
//! numbered multi-line align-family environment is actually present (same
//! conditional-import pattern as `@preview/subpar` for multi-caption floats).
//!
//! `equation`/`multline` are numbered ONCE by LaTeX even when their body spans
//! lines (via `split`/`aligned`), so their body is wrapped in `#box[$…$]` — the
//! box hides the line breaks from equate, keeping one number AND keeping the
//! block's `<label>` attachable. That matters: 158 of the 195 such corpus blocks
//! carry a `\label`, and equate's documented escape (`<equate:revoke>`) cannot
//! coexist with a label on the same element.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    )
    .typst
}

// Equation numbering in ByeTex is demand-driven — a reference is what turns it
// on — so every fixture that expects numbers includes one. Without it the
// document prints no numbers at all and (correctly) needs no equate.
const ALIGN: &str = r"\begin{align}
a &= 1, \\
b &= 2, \\
c &= 3. \label{eq:c}
\end{align}
See \eqref{eq:c}.";

#[test]
fn a_multiline_align_pulls_in_equate_line_mode() {
    let out = typ(ALIGN);
    assert!(
        out.contains("@preview/equate"),
        "a numbered multi-line align needs the equate import; got:\n{out}"
    );
    assert!(
        out.contains("number-mode: \"line\""),
        "equate must run in per-line numbering mode; got:\n{out}"
    );
}

#[test]
fn a_single_line_equation_does_not_pull_in_equate() {
    // Don't add a network-fetched dependency to documents that gain nothing.
    let out = typ(r"\begin{equation} a = 1 \end{equation}");
    assert!(
        !out.contains("@preview/equate"),
        "a single-line equation needs no equate import; got:\n{out}"
    );
}

#[test]
fn an_unnumbered_align_star_does_not_pull_in_equate() {
    // `align*` prints no numbers at all, so per-line numbering is irrelevant.
    let out = typ(
        r"\begin{align*}
a &= 1, \\
b &= 2.
\end{align*}",
    );
    assert!(
        !out.contains("@preview/equate"),
        "align* is unnumbered; no equate needed. got:\n{out}"
    );
}

#[test]
fn a_multiline_equation_body_is_boxed_to_stay_single_numbered() {
    // `equation` + `split` spans lines but LaTeX gives it ONE number. Under a
    // global line-mode rule it would wrongly get one per line, so the body is
    // boxed to hide the breaks from equate.
    let out = typ(
        r"\begin{align}
x &= 1 \\ y &= 2 \label{eq:y}
\end{align}
\begin{equation}
\begin{split}
p &= 1, \\
q &= 2.
\end{split}
\end{equation}
See \eqref{eq:y}.",
    );
    assert!(
        out.contains("#box[$"),
        "a multi-line equation body must be boxed; got:\n{out}"
    );
}

#[test]
fn the_align_body_itself_is_not_boxed() {
    // Boxing the align would defeat the whole fix.
    let out = typ(ALIGN);
    assert!(
        !out.contains("#box[$"),
        "the align body must stay visible to equate; got:\n{out}"
    );
}

#[test]
fn gather_and_eqnarray_are_also_per_line() {
    for env in ["gather", "eqnarray"] {
        let out = typ(&format!(
            "\\begin{{{env}}}\na &= 1, \\\\\nb &= 2. \\label{{eq:b}}\n\\end{{{env}}}\nSee \\eqref{{eq:b}}."
        ));
        assert!(
            out.contains("@preview/equate"),
            "{env} numbers each line too; got:\n{out}"
        );
    }
}

#[test]
fn nonumber_lines_are_revoked_so_they_stay_unnumbered() {
    // Review finding (HIGH). `\nonumber`/`\notag` drop to nothing normally —
    // Typst doesn't number an equation unless asked — but under equate's line
    // mode EVERY line is numbered, so the suppression must be expressed or a
    // `\nonumber` line takes a number LaTeX never gave it and everything after
    // it shifts. Verified against tectonic: this document is c=(1), d=(2).
    // 21 of the 27 corpus align papers use `\nonumber` (232×), so without this
    // the per-line fix would be a net REGRESSION.
    let out = typ(
        r"\begin{align}
a &= 1 \nonumber \\
b &= 2 \nonumber \\
c &= 3 \label{eq:c}
\end{align}
See \eqref{eq:c}.",
    );
    assert_eq!(
        out.matches("#<equate:revoke>").count(),
        2,
        "each \\nonumber line needs its own revoke marker; got:\n{out}"
    );
}

#[test]
fn nonumber_outside_a_per_line_env_still_drops_to_nothing() {
    // The revoke marker is only meaningful under equate's line mode; a plain
    // `equation` must keep emitting nothing for `\nonumber`.
    let out = typ(r"\begin{equation} a = 1 \nonumber \end{equation}");
    assert!(
        !out.contains("equate:revoke"),
        "no revoke marker outside a per-line env; got:\n{out}"
    );
}

// ── `alignat`'s mandatory column-count argument ──────────────────────────────
//
// `\begin{alignat}{2}` takes a REQUIRED argument: the number of column pairs.
// Typst's `$ … $` aligns on `&` and needs no such count, so the argument must be
// consumed — it was instead copied into the math body and rendered as a literal
// `{2}` at the head of the equation. 15 equations on corpus 2605.22728.
//
// A dogfood agent found this by eye and read it as a stray `equate`
// sub-numbering directive; it is not — `align` (no argument) is unaffected.

#[test]
fn alignat_column_count_is_not_rendered() {
    let t = typ("\\begin{alignat}{2}\na &= b \\\\\nc &= d\n\\end{alignat}");
    assert!(
        !t.contains("{2}"),
        "alignat's column count must not reach the body; got:\n{t}"
    );
    assert!(t.contains("a &= b"), "the body still renders; got:\n{t}");
    assert!(t.contains("c &= d"), "every line still renders; got:\n{t}");
}

#[test]
fn starred_alignat_too() {
    let t = typ("\\begin{alignat*}{3}\nx &= y\n\\end{alignat*}");
    assert!(!t.contains("{3}"), "alignat* also takes the count; got:\n{t}");
    assert!(t.contains("x &= y"), "the body still renders; got:\n{t}");
}

#[test]
fn align_without_an_argument_is_unaffected() {
    // The control: `align` has no such argument, so a change that stripped a
    // leading group unconditionally would show up here.
    let t = typ("\\begin{align}\np &= q\n\\end{align}");
    assert!(t.contains("p &= q"), "align body intact; got:\n{t}");
}

#[test]
fn a_brace_group_that_is_real_math_survives() {
    // The other control: only the COUNT is dropped. A brace group that is part
    // of the mathematics must not be.
    let t = typ("\\begin{align}\n{a+b} &= c\n\\end{align}");
    assert!(
        t.contains("a+b"),
        "a real brace group in the body is not an argument; got:\n{t}"
    );
}

// ── Regressions found in review ──────────────────────────────────────────────

#[test]
fn a_nested_alignat_drops_its_count_too() {
    // `emit_math_environment` returns early when already inside math and builds
    // its own body list; the first fix only covered the outer path.
    let t = typ("\\begin{equation}\\begin{alignat}{2}a &= b\\end{alignat}\\end{equation}");
    assert!(!t.contains("{2}"), "nested alignat leaks its count; got:\n{t}");
    assert!(t.contains("a &= b"), "nested body renders; got:\n{t}");
}

#[test]
fn alignedat_is_covered() {
    // `alignedat` is the ALIGNED variant, so it is normally written inside an
    // equation — it only ever reaches the nested path, and so leaked
    // unconditionally. Covering `alignat` alone was half the family.
    let t = typ("\\begin{equation}\\begin{alignedat}{2}a &= b \\\\ c &= d\\end{alignedat}\\end{equation}");
    assert!(!t.contains("{2}"), "alignedat leaks its count; got:\n{t}");
    assert!(t.contains("a &= b"), "body renders; got:\n{t}");
}

#[test]
fn a_comment_before_the_count_does_not_reopen_the_leak() {
    // People annotate the environment line; the count is then not `body[0]`.
    let t = typ("\\begin{alignat}%c\n{2}\na &= b\n\\end{alignat}");
    assert!(!t.contains("{2}"), "a comment must not hide the count; got:\n{t}");
    assert!(t.contains("a &= b"), "body renders; got:\n{t}");
}

#[test]
fn a_missing_count_does_not_eat_real_mathematics() {
    // Dropping the first brace group unconditionally silently deleted `a+b`.
    // Losing mathematics is worse than a stray `{2}`, so the group must read as
    // a small integer before it is treated as the count.
    let t = typ("\\begin{alignat}\n{a+b} &= c\n\\end{alignat}");
    assert!(
        t.contains("a+b"),
        "a non-numeric group is content, not the column count; got:\n{t}"
    );
}
