//! Bug (corpus 2605.31567): a bare `cov(R_i, R_j)` in math emitted `cov(...)`,
//! but Typst has NO built-in `cov` operator (unlike sin/cos/log/…), so it
//! parsed as `unknown variable: cov`. byetex listed cov/var/argmax/argmin as
//! "math function names" (so they weren't letter-split) but emitted them bare.
//! Typst is missing exactly these four — emit them as `op("…")` (upright, like
//! \operatorname), which renders correctly.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn typst_missing_operators_use_op_call() {
    for (name, _) in [("cov", ()), ("var", ()), ("argmax", ()), ("argmin", ())] {
        let t = typ(&format!("$x = {name}(y)$"));
        assert!(
            t.contains(&format!("op(\"{name}\")")),
            "`{name}` must be emitted as op(\"{name}\"); got:\n{t}"
        );
        // Not split into letters, not left as a bare identifier.
        assert!(
            !t.contains(&format!(
                " {} ",
                name.chars()
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            )),
            "`{name}` must not be letter-split; got:\n{t}"
        );
    }
}

#[test]
fn typst_builtin_functions_stay_bare() {
    // Regression guard: sin/cos/log/max/det ARE Typst built-ins — keep bare.
    let t = typ("$sin(x) + cos(x) + log(x) + max(a, b) + det(M)$");
    for f in ["sin", "cos", "log", "max", "det"] {
        assert!(t.contains(f), "{f} must stay; got:\n{t}");
    }
    assert!(
        !t.contains("op(\"sin\")"),
        "builtin sin must not be wrapped in op(); got:\n{t}"
    );
}

#[test]
fn plain_word_still_splits() {
    // Regression guard: a non-function multi-letter word still splits.
    let t = typ("$abc$");
    assert!(
        t.contains("a b c"),
        "plain word must still split; got:\n{t}"
    );
}
