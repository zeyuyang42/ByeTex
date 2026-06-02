//! Expanded-corpus compile-blocker (2605.31596): `\sqrt\frac{d\sigma^2}{dt}` —
//! a brace-less `\sqrt` whose argument is the structural command `\frac` with
//! its own brace args. byetex consumed only the `\frac` token as the radicand,
//! emitting a literal `sqrt(\frac){a}{b}` (the args spilled out) → Typst
//! `unknown variable: rac`. `\sqrt` must take the whole `\frac{..}{..}`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn sqrt_braceless_frac_takes_full_application() {
    let t = typ("$\\sqrt\\frac{a}{b}$");
    assert!(
        !t.contains("\\frac"),
        "the \\frac must be converted, not left literal; got:\n{t}"
    );
    assert!(
        t.contains("sqrt((a) / (b))"),
        "\\sqrt must take the whole \\frac as its radicand; got:\n{t}"
    );
}

#[test]
fn sqrt_braceless_single_command_unchanged() {
    // Regression guard: `\sqrt\alpha` (a symbol, no args) still works.
    let t = typ("$\\sqrt\\alpha$");
    assert!(
        t.contains("sqrt(alpha)"),
        "\\sqrt of a bare symbol must stay `sqrt(alpha)`; got:\n{t}"
    );
}

#[test]
fn sqrt_braced_frac_still_works() {
    // Regression guard: the common braced form is unaffected.
    let t = typ("$\\sqrt{\\frac{a}{b}}$");
    assert!(
        t.contains("sqrt((a) / (b))"),
        "braced \\sqrt{{\\frac}} must still work; got:\n{t}"
    );
}
