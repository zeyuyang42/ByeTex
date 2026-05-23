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

    Symbols: $infinity$, $diff$, $nabla f$, $forall x exists y$, $A union B inter C$.
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
