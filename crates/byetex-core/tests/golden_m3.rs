//! M3 golden tests: math.

use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn run_str(src: &str) -> String {
    let out = convert(src, &ConvertOptions::default());
    let warnings_json = serde_json::to_string_pretty(&out.warnings).expect("warnings serialize");
    format!(
        "==== TYPST ====\n{}==== WARNINGS ====\n{}\n",
        out.typst, warnings_json
    )
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/byetex-core has at least two parents")
        .join("tests/fixtures")
}

fn run(rel: &str) -> String {
    let path = fixtures_root().join(rel);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let opts = ConvertOptions {
        source_name: Some(rel.to_string()),
        ..Default::default()
    };
    let out = convert(&source, &opts);
    let warnings_json = serde_json::to_string_pretty(&out.warnings).expect("warnings serialize");
    format!(
        "==== TYPST ====\n{}==== WARNINGS ====\n{}\n",
        out.typst, warnings_json
    )
}

#[test]
fn m3_inline_basic() {
    insta::assert_snapshot!(run("m3_math/inline_basic.tex"), @r"
    ==== TYPST ====
    A paragraph with simple inline math $x = y + 1$ and $z^2 - 4$.

    Now subscripts $a_i$ and superscripts $b^(n+1)$, with mixed $x_1^2$.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m3_display_basic() {
    insta::assert_snapshot!(run("m3_math/display_basic.tex"), @r"
    ==== TYPST ====
    Before the display.

    $ a + b = c $

    After the display, then a dollar-display:

    $ x dot.c y = z $

    End.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m3_frac_sqrt() {
    insta::assert_snapshot!(run("m3_math/frac_sqrt.tex"), @r"
    ==== TYPST ====
    Fractions: $(a) / (b)$ and $(1) / (2) + (1) / (3)$.

    Square root: $sqrt(x)$ and $sqrt(x^2 + y^2)$.

    Combined: $sqrt((a) / (b))$.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m3_greek_ops() {
    insta::assert_snapshot!(run("m3_math/greek_ops.tex"), @r"
    ==== TYPST ====
    Greek letters: $alpha + beta = gamma$ and capitals $Sigma Delta Omega$.

    Operators: $a dot.c b$, $x times y$, $a plus.minus b$, $u <= v >= w$, $p != q$, $f arrow.r g$.

    Symbols: $infinity$, $partial$, $nabla f$, $forall x exists y$, $A union B inter C$.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m3_sum_int() {
    insta::assert_snapshot!(run("m3_math/sum_int.tex"), @r"
    ==== TYPST ====
    Summation: $sum_(i=1)^(n) i = (n(n+1)) / (2)$.

    Integral: $integral_0^1 x^2 thin d x$.

    Product: $product_(k=1)^(n) k$.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m3_equation_env() {
    insta::assert_snapshot!(run("m3_math/equation_env.tex"), @r"
    ==== TYPST ====
    $ E = m c^2 $ <eq:einstein>

    $ a^2 + b^2 = c^2 $
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m3_align_env() {
    insta::assert_snapshot!(run("m3_math/align_env.tex"), @r"
    ==== TYPST ====
    $ a &= b + c \
    &= d - e $
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m3_half_open_interval_escapes_unbalanced_bracket() {
    // Regression: LaTeX half-open intervals `(0, s_*]` previously emitted
    // a bare `]` inside `$...$`, which Typst rejects with "unclosed
    // delimiter" because `[` / `]` are paired in math mode. The unmatched
    // `]` is now escaped as `\]`. Balanced ranges like `[a, b]` stay
    // as-is and still render correctly.
    let src = "$s \\in (0, s_*]$\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("\\]"),
        "expected escaped `\\]` in output, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("s_*]"),
        "raw unescaped `]` should not remain:\n{}",
        out.typst
    );
}

#[test]
fn m3_balanced_brackets_not_escaped() {
    // Balanced `[a, b]` must NOT be escaped — they pair correctly in Typst
    // math and over-escaping would be a regression in rendering.
    let src = "$[a, b]$\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("\\["),
        "balanced `[` should not be escaped, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\]"),
        "balanced `]` should not be escaped, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_dagger_ddagger_in_math_table() {
    // Regression: `\dagger` previously emitted `agger` (the `\d` accent
    // command was matched first, leaving the `agger` tail dangling and
    // tripping Typst's "unknown variable" check).
    let out = convert(
        "$x^\\dagger$ and $y^\\ddagger$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("$x^(dagger)$") || out.typst.contains("$x^dagger$"),
        "expected `$x^dagger$` or `$x^(dagger)$`, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("$y^(dagger.double)$") || out.typst.contains("$y^dagger.double$"),
        "expected `$y^dagger.double$` or `$y^(dagger.double)$`, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_letter_then_math_command_keeps_separator() {
    // Regression: `t\in[0,T]` previously emitted `tin[0,T]` — the `t` and
    // `\in` (-> `in`) collapsed into the unknown identifier `tin`. A space
    // must separate them so Typst tokenizes `t` and `in` independently.
    let out = convert(
        "$t\\in[0,T]$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("tin"),
        "letter+math-command must not fuse, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("t in"),
        "expected separator between `t` and `in`, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_paren_math_delimiters_treated_as_inline_math() {
    // `\( x = 1 \)` is the LaTeX inline-math form equivalent to `$ x = 1 $`.
    // Previously the literal `\(` / `\)` tokens leaked into the Typst body,
    // producing `$\(x = 1\)$` which Typst then rejected. The math child
    // filter now drops both delimiter kinds.
    let out = convert(
        "Before \\( x = 1 \\) after.\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("\\("),
        "raw `\\(` should not appear in output, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\)"),
        "raw `\\)` should not appear in output, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("$x = 1$") || out.typst.contains("$ x = 1 $"),
        "expected inline-math body, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_math_escape_for_hash_dollar_etc() {
    // Bug #10 regression: in math, `\#` previously mapped to bare `#`, which
    // Typst treats as the start of a code-context expression. The subscript
    // `f_\#` thus emitted as `f_(#)` and failed with "unexpected closing
    // paren". The mapping now keeps the backslash so Typst takes it as a
    // math escape for the literal character.
    let out = convert(
        "$f_{\\#}$ and $g_{\\&}$ and $h_{\\_}$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("\\#") && out.typst.contains("\\&") && out.typst.contains("\\_"),
        "expected escaped \\#, \\&, \\_ in math output, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("_(#)") && !out.typst.contains("_(&)") && !out.typst.contains("_(_)"),
        "bare subscript with special char should never appear, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_mathbb_does_not_fuse_with_preceding_letter() {
    // Bug #11 regression: `\in\mathbb{R}` previously emitted `inbb(R)`
    // because `emit_math_wrap` wrote `bb(` straight after the `in` from
    // `\in`. The letter-boundary check now also fires for function-call
    // wrappers (`bb(`, `bold(`, `sqrt(`, `binom(`, `op(`).
    let out = convert(
        "$p\\in\\mathbb{R}^n$\n$q(p,x)\\in\\mathbb{R}_+$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("inbb"),
        "`in` + `bb(...)` should not fuse into `inbb`, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("in bb(R)"),
        "expected `in bb(R)` with separator, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_partial_uses_modern_typst_name() {
    // Bug #13: Typst 0.13+ deprecates `diff` in favour of `partial`. The
    // emitter now uses the new name to keep the compile clean.
    let out = convert(
        "$\\partial f$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("partial"),
        "expected `partial`, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("diff"),
        "should not use deprecated `diff`, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_left_right_strip_for_balanced_parens() {
    // Bug B.1: `\left(...\right)` in math previously leaked as raw
    // `\left(...\right)` in the Typst output. Typst then read `\l` as the
    // math escape for the letter `l`, leaving the dangling identifier
    // `eft(...)` (and `ight)` at the close). Stripping the commands lets
    // Typst auto-pair the `(` and `)`.
    let out = convert(
        "$\\left( V - G \\right) = 0$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("eft") && !out.typst.contains("ight"),
        "raw `\\left`/`\\right` should be stripped, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("( V - G ) = 0") || out.typst.contains("(V - G) = 0"),
        "expected balanced parens, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_thin_space_doesnt_fuse() {
    // Bug B.2: `\thinspace` was emitted by the unknown-command fallback
    // as `thinspace` and fused with the next identifier
    // (`\thinspace d` → `thinspaced` → "unknown variable thind"). Mapping
    // it to `thin` plus the existing letter-boundary check keeps the two
    // tokens separate.
    let out = convert(
        "$a\\thinspace d$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("thind") && !out.typst.contains("thinspaced"),
        "spacing command must not fuse with next identifier, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("thin"),
        "expected `thin` in output, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_braceless_math_wrap_consumes_single_token() {
    // Phase 1: `\hat x`, `\mathcal A`, `\bar y` and friends previously
    // emitted only the bare letter (dropping the wrap command) because
    // `emit_math_wrap` required a curly_group argument. The brace-less
    // single-token form is now handled by consuming the next source
    // token directly.
    let out = convert(
        "$\\hat x + \\mathcal A - \\bar y + \\tilde\\alpha$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("hat(x)"),
        "expected hat(x), got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("cal(A)"),
        "expected cal(A), got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("overline(y)"),
        "expected overline(y), got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("tilde(alpha)"),
        "expected tilde(alpha) (with \\alpha resolved via symbol table), got:\n{}",
        out.typst
    );
}

#[test]
fn m3_braceless_wrap_preserves_trailing_content() {
    // Regression: the brace-less arg fix originally swallowed everything
    // after the consumed token because tree-sitter packs adjacent math
    // chars (`x + y`) into a single `text` node with children. Recursive
    // partial-skip preserves the tail.
    let out = convert(
        "$\\hat x + y - z$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("hat(x) + y - z"),
        "expected `hat(x) + y - z`, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_newcommand_expands_zero_arg() {
    // Phase 3: a `\newcommand{\R}{\mathbb{R}}` definition followed by
    // `\R` in math expands to the body's typst rendering.
    let out = convert(
        "\\newcommand{\\R}{\\mathbb{R}}\n$x \\in \\R$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("$x in RR$") || out.typst.contains("$x in bb(R)$"),
        "expected RR / bb(R) substitution, got:\n{}",
        out.typst
    );
    assert!(out.warnings.is_empty(), "got: {:?}", out.warnings);
}

#[test]
fn m3_newcommand_expands_with_args() {
    // Phase 3: `\newcommand{\norm}[1]{\|#1\|}` + `\norm{v}` →
    // typst body with `v` substituted into `#1`.
    let out = convert(
        "\\newcommand{\\norm}[1]{\\|#1\\|}\n$\\norm{v}$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("||v||"),
        "expected `||v||`, got:\n{}",
        out.typst
    );
    assert!(out.warnings.is_empty(), "got: {:?}", out.warnings);
}

#[test]
fn m3_array_in_align_emits_cases() {
    // Phase 1: when `\begin{array}` is nested inside a math env
    // (`align*`, gather, equation), it should render as Typst
    // `cases(...)` — emitting a `#table(...)` here would break the
    // surrounding `$...$` since `#table` is text-mode only.
    let src = r#"\begin{align*}
y &\lesssim \left\{\begin{array}{ll}
a & \text{if } x < 1, \\
b & \text{if } x > 1
\end{array}\right\}
\end{align*}"#;
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("cases("),
        "expected `cases(...)` for nested array in math, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("#table"),
        "should NOT emit `#table(...)` inside math, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_pmatrix() {
    insta::assert_snapshot!(run("m3_math/pmatrix.tex"), @r"
    ==== TYPST ====
    A 2x2 matrix: $mat(a, b; c, d)$.

    A 3x3 matrix:

    $ mat(1, 2, 3; 4, 5, 6; 7, 8, 9) $
    ==== WARNINGS ====
    []
    ");
}

// ============== Phase B: TDD red tests for Bugs #16, #20 ==============

#[test]
fn m3_label_with_underscores_in_math_env() {
    // Bug #16 (fixed): tree-sitter parsed `_` inside `\label{eq:foo_bar}` as a
    // subscript operator in math context, truncating the key to `eq:foo` and
    // leaking `_b a r}` as a stray subscript expression. The label emitter
    // now reads the curly_group_text byte-for-byte before any math-mode
    // processing, so the full `<eq:foo_bar>` anchor is preserved.
    let out = convert(
        "\\begin{equation}\\label{eq:foo_bar}x=1\\end{equation}\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("<eq:foo_bar>"),
        "expected full label `<eq:foo_bar>`, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("<eq:foo>"),
        "truncated label `<eq:foo>` must not appear, got:\n{}",
        out.typst
    );
}

#[test]
#[ignore = "Bug #20 — pending fix: \\\\[length] optional arg in math align"]
fn m3_align_row_break_strips_optional_length() {
    // Bug #20: `\\[1mm]` inside an `align` environment emits `\[1mm\]` in
    // Typst, which the parser reads as a math matrix delimiter — producing an
    // unclosed delimiter error. The optional length argument must be consumed
    // and dropped; only the bare row-break `\` should remain.
    let out = convert(
        "\\begin{align}a &= b \\\\[1mm] c &= d\\end{align}\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("1mm"),
        "optional length `1mm` should be stripped, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\["),
        "math-open bracket `\\[` must not appear in row-break, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains(" \\\n") || out.typst.contains("\\\\\n"),
        "expected Typst row-break `\\` in output, got:\n{}",
        out.typst
    );
}

// ============== Phase C: edge-case coverage for Bugs #14, #15 ==============

#[test]
#[ignore = "Bug #14b — pending fix: unmatched \\left not stripped (only matched left_right pairs are handled)"]
fn m3_unmatched_left_paren_does_not_break() {
    // Bug #14 residual: matched `\left(...\right)` pairs are stripped via the
    // tree-sitter `left_right` node handler (line 1849 emit.rs). An unmatched
    // `\left(` with no closing `\right` is parsed as a generic command and
    // hits the generic fallback rather than the symbol-table entry at line 3479,
    // so it still leaks verbatim into output.
    let out = convert(
        "$f\\left(x$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("eft"),
        "raw `\\left` remnant `eft` must not appear, got:\n{}",
        out.typst
    );
    assert!(
        out.warnings.is_empty(),
        "unexpected warnings, got:\n{:?}",
        out.warnings
    );
}

#[test]
fn m3_big_sizing_commands_stripped_like_left_right() {
    // Bug #14 extension: the `\bigl`/`\bigr`/`\Bigl`/`\Bigr`/`\big`/`\Big`
    // sizing family should be dropped just like `\left`/`\right`, leaving only
    // the bare delimiter. If this test fails, the `\big*` family is the next
    // fix layer — leave it red until that fix lands.
    let out = convert(
        "$\\bigl(x\\bigr)\\Bigl[y\\Bigr]$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("bigl")
            && !out.typst.contains("bigr")
            && !out.typst.contains("Bigl")
            && !out.typst.contains("Bigr"),
        "sizing prefixes must be stripped, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("(x)") || out.typst.contains("( x )"),
        "expected bare parens after stripping, got:\n{}",
        out.typst
    );
}

#[test]
fn m3_other_spacing_macros_dont_fuse() {
    // Bug #15 extension: `\medspace` and `\thickspace` should be mapped and
    // have a letter-boundary guard, just like `\thinspace`. If they fuse with
    // the preceding letter (e.g., `a` + `med` → `amed`) this test catches it.
    let out = convert(
        "$a\\medspace b\\thickspace c\\negthinspace d$\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("amed"),
        "`a` + medspace must not fuse into `amed`, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("bthick"),
        "`b` + thickspace must not fuse into `bthick`, got:\n{}",
        out.typst
    );
}

// ============== Phase D: under-tested emitter — eqref vs ref ==============

#[test]
fn m3_eqref_wraps_in_parens() {
    // `\eqref{eq:foo}` should produce `(@eq:foo)` — parenthesized per LaTeX
    // convention — while `\ref{sec:bar}` emits a bare `@sec:bar`.
    let out = convert(
        "Eq.~\\eqref{eq:foo}.\nSec.~\\ref{sec:bar}.\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("(@eq:foo)") || out.typst.contains("(#ref(<eq:foo>))"),
        "expected parenthesized eqref, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("@sec:bar"),
        "expected bare ref `@sec:bar`, got:\n{}",
        out.typst
    );
}

// ============== Bug A: \tag now emits DropOnly warning ==============

#[test]
fn m3_tag_silently_dropped() {
    let out = convert(
        r"\begin{equation}
x = y \tag{Dual LP}
\end{equation}",
        &ConvertOptions::default(),
    );
    // Typst output is unchanged — \tag carries no renderable math content.
    assert!(out.typst.contains("x = y"), "typst: {}", out.typst);
    // A DropOnly warning is now emitted so the user knows their label was lost.
    assert_eq!(out.warnings.len(), 1, "expected one warning, got: {:?}", out.warnings);
    assert!(
        matches!(&out.warnings[0].category, byetex_core::Category::DropOnly { name } if name == "\\tag"),
        "expected DropOnly {{name: \\tag}}, got: {:?}",
        out.warnings[0].category
    );
}

// ============== Bug E: dotted symbol before ( ==============

#[test]
fn m3_dotted_symbol_no_function_call() {
    let out = convert(
        r"$f: \mathbb{R} \to (0, \infty)$",
        &ConvertOptions::default(),
    );
    assert!(
        !out.typst.contains("arrow.r("),
        "expected space between arrow.r and (, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("arrow.r"),
        "expected arrow.r in output, got:\n{}",
        out.typst
    );
}

// ── Phase: two-pass macro harvest ────────────────────────────────────────────

#[test]
fn m3_newcommand_use_before_define() {
    // Macro used before its definition — prepass must collect before emit
    let src = r"$\R \in \R$
\newcommand{\R}{\mathbb{R}}
$\R$";
    let out = byetex_core::convert(src, &Default::default());
    assert!(out.warnings.is_empty(), "unexpected warnings: {:?}", out.warnings);
}

#[test]
fn m3_renewcommand_overrides_newcommand() {
    let src = r"\newcommand{\foo}{A}\renewcommand{\foo}{B}x\textbf{\foo}y";
    let out = byetex_core::convert(src, &Default::default());
    assert!(out.typst.contains('B'), "expected B in output, got: {}", out.typst);
    assert!(!out.typst.contains('A'), "unexpected A in output: {}", out.typst);
}

#[test]
fn m3_providecommand_respects_existing() {
    let src = r"\newcommand{\foo}{A}\providecommand{\foo}{B}x\textbf{\foo}y";
    let out = byetex_core::convert(src, &Default::default());
    assert!(out.typst.contains('A'), "expected A in output, got: {}", out.typst);
    assert!(!out.typst.contains('B'), "unexpected B in output: {}", out.typst);
}

#[test]
fn m3_declaremathoperator_basic() {
    let src = "\\DeclareMathOperator{\\sinc}{sinc}\n$\\sinc(x)$";
    let out = byetex_core::convert(src, &Default::default());
    assert!(out.warnings.is_empty(), "unexpected warnings: {:?}", out.warnings);
    // operatorname{sinc} should appear in the output
    assert!(out.typst.contains("sinc"), "expected sinc in output, got: {}", out.typst);
}
