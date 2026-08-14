//! Fidelity backlog L6 / §9 — "Fig. \ref{x}" must not render "Fig. Figure 3".
//!
//! LaTeX's plain `\ref` renders the COUNTER ONLY ("3"); `\eqref` renders the
//! counter in parens ("(3)"). Typst's `@key` shorthand auto-prepends the
//! referenced element's supplement ("Figure 3" / "Equation 1"), so the extremely
//! common `Fig.~\ref{x}` / `Section~\ref{x}` idiom rendered a DOUBLE prefix.
//!
//! The fix suppresses the supplement for exactly those two commands using the
//! `@key[]` SHORTHAND (an empty supplement content block) rather than the
//! `#ref(<key>, supplement: none)` function form — the earlier attempt at this
//! fix (reverted) used the function form and broke on `\ref{x}(ii)` call-syntax
//! and on the table-cell escaper mangling `<`/`_`. `@key[]` keeps the robust
//! shorthand shape, so both hazards are structurally impossible.
//!
//! Commands that legitimately DO print a prefix in LaTeX (`\cref`, `\Cref`,
//! `\autoref`, …) keep the bare `@key` so Typst supplies the supplement.

use byetex_core::{convert, ConvertOptions};

fn out(src: &str) -> String {
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
fn plain_ref_suppresses_the_typst_supplement() {
    let typst = out("See Fig.~\\ref{fig:a} for details.\n");
    assert!(
        typst.contains("@fig:a[]"),
        "plain \\ref must emit the supplement-less `@key[]` form; got:\n{typst}"
    );
}

#[test]
fn eqref_suppresses_the_supplement_inside_its_parens() {
    let typst = out("As in \\eqref{eq:one}.\n");
    assert!(
        typst.contains("(@eq:one[])"),
        "\\eqref must render `(counter)`, not `(Equation N)`; got:\n{typst}"
    );
}

#[test]
fn cleveref_and_autoref_keep_their_supplement() {
    for cmd in ["\\cref", "\\Cref", "\\autoref"] {
        let typst = out(&format!("See {cmd}{{sec:intro}}.\n"));
        assert!(
            typst.contains("@sec:intro") && !typst.contains("@sec:intro[]"),
            "{cmd} prints a prefix in LaTeX, so Typst must supply the supplement; got:\n{typst}"
        );
    }
}

#[test]
fn supplement_less_ref_needs_no_adjacency_space() {
    // `@key[]` self-terminates, so the label can't absorb a following `-`/`.`/
    // alnum the way a bare `@key` can — the glue guard must not fire and insert
    // a stray space into the rendered text.
    // (`--` itself becomes an en dash via the usual typography pass.)
    let typst = out("Figs.~\\ref{fig:a}--\\ref{fig:b}.\n");
    assert!(
        typst.contains("@fig:a[]\u{2013}@fig:b[]"),
        "no space should be injected after the self-terminating form; got:\n{typst}"
    );
}

#[test]
fn ref_in_math_uses_the_function_form_without_supplement() {
    // Inside math a bare `@key` parses as an identifier, so the fn form is
    // required there (Bug #24); it must also drop the supplement.
    let typst = out("$x = \\ref{eq:one}$\n");
    assert!(
        typst.contains("#ref(<eq:one>, supplement: none)"),
        "math-mode \\ref must use the fn form with supplement: none; got:\n{typst}"
    );
}

#[test]
fn ref_followed_by_a_paren_is_not_a_call() {
    // The reverted fn-form attempt emitted `#ref(<x>)(ii)`, which Typst reads as
    // CALLING the ref's result (`unknown variable: ii`, 2605.22800). The
    // shorthand can't be called, so `(ii)` stays literal text.
    let typst = out("See \\ref{def:shape}(ii).\n");
    assert!(
        typst.contains("@def:shape[](ii)"),
        "a following paren must stay literal text; got:\n{typst}"
    );
    assert!(
        !typst.contains("#ref("),
        "no function form outside math; got:\n{typst}"
    );
}

#[test]
fn ref_inside_a_table_cell_survives_the_cell_escaper() {
    // The other blocker on the reverted attempt: the table-cell content escaper
    // escaped the fn form's `<`/`_` (`#ref(\<sec\_x>…)` → "character `\` is not
    // valid in code", 2605.31072). The shorthand carries neither, so the key and
    // the empty supplement pass through untouched.
    //
    // NOTE the escaper does still prepend `\` to the `@` sigil, so a ref in a
    // cell renders as literal text — a SEPARATE, pre-existing bug (present on
    // main for the bare `@key` form too, logged as fidelity-backlog L13). This
    // test deliberately asserts only the part this change owns.
    let typst = out("\\begin{tabular}{ll}\na & see Fig.~\\ref{fig:a} \\\\\n\\end{tabular}\n");
    assert!(
        typst.contains("@fig:a[]"),
        "the cell escaper must leave the key and empty supplement intact; got:\n{typst}"
    );
    assert!(
        !typst.contains("#ref(") && !typst.contains("fig\\:a"),
        "no fn form and no mangled key inside a cell; got:\n{typst}"
    );
}

#[test]
fn math_fn_form_keeps_its_adjacency_guard() {
    // Review finding #1. The `@key[]` markup form self-terminates, but the math
    // `#ref(...)` fn form does NOT: Typst continues a CODE expression across a
    // `.`, so `#ref(<a>, supplement: none).x` parses as a field access and errors
    // with `ref does not have field "x"`. The separating space must survive.
    let typst = out("Consider $\\ref{sec:a}.x$ here.\n");
    assert!(
        typst.contains("#ref(<sec:a>, supplement: none) .x"),
        "the fn form must keep the separating space before `.x`; got:\n{typst}"
    );
}

#[test]
fn labelcref_also_drops_the_supplement() {
    // cleveref's `\labelcref` prints the bare counter in LaTeX (that is its whole
    // purpose — the no-prefix sibling of `\cref`), so it has the same
    // double-prefix defect as plain `\ref`. It is a comma-LIST command, so each
    // key gets its own empty supplement.
    let typst = out("See \\labelcref{fig:a,fig:b}.\n");
    assert!(
        typst.contains("@fig:a[]") && typst.contains("@fig:b[]"),
        "\\labelcref must drop the supplement on every key; got:\n{typst}"
    );
}

#[test]
fn pageref_is_untouched() {
    // `\pageref` already warns and renders as a normal reference; suppressing
    // its supplement is a separate question — guard the current behaviour.
    let typst = out("On page~\\pageref{sec:intro}.\n");
    assert!(
        typst.contains("@sec:intro") && !typst.contains("@sec:intro[]"),
        "\\pageref behaviour must be unchanged; got:\n{typst}"
    );
}
