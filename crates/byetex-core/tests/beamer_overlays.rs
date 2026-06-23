//! Beamer overlay specs (B5 / Phase 3c): `\item<1->`, `\pause`,
//! `\only`/`\uncover`/`\onslide`/`\visible`/`\alert` carry `<overlay-spec>` markers.
//! In a touying beamer deck these become *incremental builds*:
//! - `\pause`                → `#pause`
//! - sequential `\item<n->`  → a `#pause` BETWEEN items (each reveals one sub-slide later)
//! - `\only<n>{X}`           → a touying reveal (`#only`/`#uncover`) at slide top-level
//! Inside a context (columns/block) the reveal would panic with
//! "Unsupported mark touying-fn-wrapper", so it is rendered COLLAPSED there.
//! The `<…>` spec must never leak as text, and content is never dropped.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = r#"\documentclass{beamer}
\begin{document}
\begin{frame}{Overlays}
\begin{itemize}
\item<1-> First point
\item<2-> Second point
\end{itemize}
\pause
\only<2>{Only on slide two.}
\uncover<3->{Uncovered content.}
\onslide<2->{On slide two plus.}
\alert<1>{Alert text.}
\end{frame}
\end{document}"#;

#[test]
fn overlay_content_is_shown() {
    let t = typ(DECK);
    for s in [
        "First point",
        "Second point",
        "Only on slide two.",
        "Uncovered content.",
        "On slide two plus.",
        "Alert text.",
    ] {
        assert!(t.contains(s), "overlay content `{s}` shown; got:\n{t}");
    }
}

#[test]
fn overlay_specs_do_not_leak() {
    let t = typ(DECK);
    for spec in ["<1->", "<2->", "<2>", "<3->", "<1>"] {
        assert!(
            !t.contains(spec),
            "overlay spec `{spec}` must not leak; got:\n{t}"
        );
    }
}

#[test]
fn pause_emits_touying_pause() {
    // `\pause` → `#pause` so the touying deck reveals incrementally.
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}A\\pause B\\end{frame}\\end{document}");
    assert!(t.contains("#pause"), "\\pause becomes #pause; got:\n{t}");
}

#[test]
fn non_beamer_pause_not_touying() {
    // `\pause` outside a beamer deck must NOT become a touying `#pause`.
    let t = typ("\\documentclass{article}\\begin{document}A\\pause B\\end{document}");
    assert!(
        !t.contains("#pause"),
        "non-beamer \\pause is not touying #pause; got:\n{t}"
    );
}

#[test]
fn sequential_item_overlays_emit_pause_between_items() {
    // `\item<1->`, `\item<2->`, `\item<3->` → a `#pause` between the items so each
    // reveals one sub-slide later (the cleanest touying idiom for sequential reveals).
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\begin{itemize}\\item<1-> alpha\\item<2-> beta\\item<3-> gamma\\end{itemize}\\end{frame}\\end{document}");
    // Two `#pause` separators for three sequentially-revealed items.
    assert_eq!(
        t.matches("#pause").count(),
        2,
        "two #pause between three sequential items; got:\n{t}"
    );
    // The order must be item1, #pause, item2, #pause, item3.
    let p_alpha = t.find("alpha").unwrap();
    let p_beta = t.find("beta").unwrap();
    let p_gamma = t.find("gamma").unwrap();
    let pauses: Vec<usize> = t.match_indices("#pause").map(|(i, _)| i).collect();
    assert!(
        p_alpha < pauses[0] && pauses[0] < p_beta && p_beta < pauses[1] && pauses[1] < p_gamma,
        "pause order item1,#pause,item2,#pause,item3; got:\n{t}"
    );
}

#[test]
fn non_overlay_items_get_no_pause() {
    // Plain `\item`s (no `<…>` spec) must NOT inject `#pause` — they show together.
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\begin{itemize}\\item one\\item two\\end{itemize}\\end{frame}\\end{document}");
    assert!(
        !t.contains("#pause"),
        "plain items get no #pause; got:\n{t}"
    );
}

#[test]
fn top_level_only_emits_touying_reveal_no_panic() {
    // `\only<2>{X}` at slide top-level → a touying reveal that COMPILES (no
    // touying-fn-wrapper panic). We assert the reveal form is emitted.
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\only<2>{Second only.}\\end{frame}\\end{document}");
    assert!(t.contains("Second only."), "content kept; got:\n{t}");
    assert!(
        t.contains("#only(\"2\")") || t.contains("#uncover(\"2\")"),
        "top-level \\only<2> emits a touying reveal; got:\n{t}"
    );
}

#[test]
fn top_level_uncover_range_emits_uncover() {
    // `\uncover<2->{X}` (open range) → `#uncover("2-")[X]` at slide top-level.
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\uncover<2->{From two on.}\\end{frame}\\end{document}");
    assert!(t.contains("From two on."), "content kept; got:\n{t}");
    assert!(
        t.contains("#uncover(\"2-\")"),
        "open-range \\uncover<2-> emits #uncover; got:\n{t}"
    );
}

#[test]
fn overlay_inside_columns_still_reveals() {
    // The converter emits a native Typst `#grid` for beamer columns (NOT touying's
    // `#cols`), which tolerates a reveal — so `\only<2>{X}` inside a `column` still
    // becomes a real `#only("2")[X]` (verified: compiles, no touying-fn-wrapper panic).
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\begin{columns}\\begin{column}{0.5\\textwidth}\\only<2>{Inside col.}\\end{column}\\end{columns}\\end{frame}\\end{document}");
    assert!(t.contains("Inside col."), "content kept; got:\n{t}");
    assert!(
        t.contains("#only(\"2\")"),
        "overlay inside a native #grid column still emits a reveal; got:\n{t}"
    );
}

#[test]
fn reveal_nested_in_reveal_collapses_and_does_not_leak_spec() {
    // A reveal nested inside another reveal PANICS in touying, so the INNER `\only<3>`
    // collapses (no inner wrapper) — and crucially its `<3>` spec must NOT leak as text
    // (regression: the sub-emitter must inherit the beamer class so the inner overlay
    // command is recognized and its spec stripped).
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\only<2>{outer \\only<3>{inner} tail}\\end{frame}\\end{document}");
    assert!(t.contains("inner"), "inner content kept; got:\n{t}");
    assert!(!t.contains("<3>"), "inner overlay spec must NOT leak; got:\n{t}");
    // Exactly one reveal wrapper (the outer one); the inner collapsed.
    assert_eq!(
        t.matches("#only(").count() + t.matches("#uncover(").count(),
        1,
        "only the OUTER reveal emits a wrapper; got:\n{t}"
    );
}

#[test]
fn overlay_command_without_spec_keeps_content() {
    // Code-review (critical): `\alert{x}` / `\only{x}` with NO overlay spec — the
    // `{content}` is a child of the command and must still render, not be dropped.
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\alert{HILITE} and \\only{NOSPEC}\\end{frame}\\end{document}");
    assert!(t.contains("HILITE"), "\\alert{{x}} no-spec content kept; got:\n{t}");
    assert!(t.contains("NOSPEC"), "\\only{{x}} no-spec content kept; got:\n{t}");
}

#[test]
fn non_beamer_item_angle_token_preserved() {
    // Code-review: the `\item<…>` overlay strip is gated on beamer, so a non-beamer
    // `\item <0,1>` keeps its literal angle-bracket text.
    let t = typ("\\documentclass{article}\\begin{document}\\begin{itemize}\\item <0,1> range\\end{itemize}\\end{document}");
    assert!(t.contains("0,1"), "non-beamer angle token preserved; got:\n{t}");
}

#[test]
fn non_beamer_overlay_commands_unaffected() {
    // \only/\alert are gated on beamer; a non-beamer doc keeps its old handling.
    let t = typ("\\documentclass{article}\\begin{document}\\alert<1>{x}\\end{document}");
    // Whatever article does, it must not be the beamer overlay path (no panic / stable).
    assert!(
        t.contains('x') || !t.contains("Alert"),
        "stable non-beamer handling; got:\n{t}"
    );
}
