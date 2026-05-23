//! M3 golden tests: math.

use std::path::PathBuf;

use bytetex_core::{convert, ConvertOptions};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/bytetex-core has at least two parents")
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
        out.typst.contains("$x^dagger$"),
        "expected `$x^dagger$`, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("$y^dagger.double$"),
        "expected `$y^dagger.double$`, got:\n{}",
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
