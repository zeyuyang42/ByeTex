//! Fidelity backlog L13 — a `\ref`/`\cite` inside a TABLE CELL rendered as dead
//! literal text.
//!
//! `escape_text_cell` runs over already-EMITTED cell content and escapes `@`,
//! which is right for an `@` that came from the source (an email address in a
//! cell must not become a Typst reference) but wrong for an `@key` ByeTex itself
//! emitted for a `\ref`/`\cite`. The result was `[see Fig.~\@fig:a]` — the
//! reference printed as the literal text "@fig:a" and never resolved.
//!
//! 19 corpus papers, 258 occurrences. Found while fixing the reference
//! double-prefix (PR #469) and confirmed pre-existing on main.
//!
//! The fix marks the token at the EMIT site (where we know it is ours) rather
//! than pattern-matching `@` in the escaper, so a source `@` is still escaped.

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

#[test]
fn a_ref_in_a_table_cell_stays_a_reference() {
    let out = typ(
        "\\begin{figure}\\caption{c}\\label{fig:a}\\end{figure}\n\
         \\begin{tabular}{ll}\nx & see Fig.~\\ref{fig:a} \\\\\n\\end{tabular}\n",
    );
    assert!(
        !out.contains("\\@fig:a"),
        "the emitted ref must not be escaped into literal text; got:\n{out}"
    );
    assert!(
        out.contains("@fig:a"),
        "the reference should survive; got:\n{out}"
    );
}

#[test]
fn a_cite_in_a_table_cell_stays_a_citation() {
    let out = typ(
        "\\begin{tabular}{ll}\nx & \\cite{knuth1984} \\\\\n\\end{tabular}\n\
         \\begin{thebibliography}{9}\n\\bibitem{knuth1984} Knuth.\n\\end{thebibliography}\n",
    );
    assert!(
        !out.contains("\\@knuth1984"),
        "the emitted citation must not be escaped; got:\n{out}"
    );
}

#[test]
fn a_source_at_sign_in_a_cell_is_still_escaped() {
    // The whole reason the escaper exists: a literal `@` from the source (an
    // email, a handle) would otherwise be parsed by Typst as a reference and
    // error with "label does not exist".
    let out = typ("\\begin{tabular}{ll}\nx & ada@example.com \\\\\n\\end{tabular}\n");
    assert!(
        out.contains("\\@example.com") || out.contains("ada\\@"),
        "a source `@` must still be escaped; got:\n{out}"
    );
}

#[test]
fn no_marker_leaks_into_the_output() {
    // Whatever mechanism protects the token must not survive into the .typ.
    let out = typ(
        "\\begin{figure}\\caption{c}\\label{fig:a}\\end{figure}\n\
         \\begin{tabular}{ll}\nx & \\ref{fig:a} \\\\\n\\end{tabular}\n",
    );
    assert!(
        !out.chars().any(|c| c.is_control() && c != '\n' && c != '\t'),
        "no control characters may leak into the output; got:\n{out:?}"
    );
}

// ─── review findings ─────────────────────────────────────────────────────────

#[test]
fn a_ref_reached_through_a_macro_in_a_cell_is_protected() {
    // Review finding (medium): `in_table_cell` wasn't propagated into the
    // sub-emitter, so a ref reached via a macro emitted `[Fig.~\@fig:a]` — dead
    // literal text — while a bare `\ref` in the SAME table worked. `\figref`
    // wrapper macros are widespread, so this was a large share of the 258 sites.
    let out = typ(
        "\\newcommand{\\figref}[1]{Fig.~\\ref{#1}}\n\
         \\begin{figure}\\caption{c}\\label{fig:a}\\end{figure}\n\
         \\begin{tabular}{ll}\nx & \\figref{fig:a} \\\\\n\\end{tabular}\n",
    );
    assert!(
        !out.contains("\\@fig:a"),
        "a macro-reached ref must be protected too; got:\n{out}"
    );
}

#[test]
fn a_ref_inside_a_font_wrapper_in_a_cell_is_protected() {
    // Same root cause via the other common shape: `{\bf \ref{x}}` routes the ref
    // through a sub-emitter as well.
    let out = typ(
        "\\begin{figure}\\caption{c}\\label{fig:a}\\end{figure}\n\
         \\begin{tabular}{ll}\nx & {\\bf \\ref{fig:a}} \\\\\n\\end{tabular}\n",
    );
    assert!(
        !out.contains("\\@fig:a"),
        "a font-wrapped ref must be protected too; got:\n{out}"
    );
}

#[test]
fn a_ref_in_a_nested_tabular_survives_double_escaping() {
    // Review finding (medium): a `tabular` inside a cell is escaped TWICE.
    // Consuming the fences on the first pass left the ref bare for the second,
    // which escaped it after all. The fences are now kept until `finish()`.
    let out = typ(
        "\\begin{figure}\\caption{c}\\label{fig:a}\\end{figure}\n\
         \\begin{tabular}{ll}\n\
         outer & \\begin{tabular}{l} \\ref{fig:a} \\end{tabular} \\\\\n\
         \\end{tabular}\n",
    );
    assert!(
        !out.contains("\\@fig:a"),
        "a ref in a nested tabular must survive both escape passes; got:\n{out}"
    );
}

#[test]
fn the_multicolumn_path_leaks_no_marker() {
    // `\multicolumn` emits a `table.cell(...)` call directly, bypassing
    // `escape_text_cell` — `strip_cell_keep_markers` is the backstop, and this
    // is the path it exists for. A regression here would ship a raw control
    // character into the .typ.
    let out = typ(
        "\\begin{figure}\\caption{c}\\label{fig:a}\\end{figure}\n\
         \\begin{tabular}{ll}\n\\multicolumn{2}{c}{see \\ref{fig:a}} \\\\\n\\end{tabular}\n",
    );
    assert!(
        !out.chars().any(|c| c.is_control() && c != '\n' && c != '\t'),
        "no control characters may reach the output; got:\n{out:?}"
    );
}
