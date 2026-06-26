//! siunitx quantity macros were dropped entirely — `\SI{3}{\meter}`,
//! `\qty{}{}`, `\si{}`, `\ang{}` rendered EMPTY, so physical quantities (value +
//! unit) vanished from physics/engineering papers. Render them: `\si` maps the
//! unit macros to symbols (m/s, Ω, …), `\SI`/`\qty` prepend the value, `\ang`
//! appends a degree sign. Found by direct validation on 2605.31009 (iopjournal,
//! 31 siunitx uses).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn si_renders_value_and_unit() {
    let t = typ(r"Speed \SI{3.0}{\meter\per\second}.");
    assert!(t.contains("3.0"), "value dropped; got:\n{t}");
    assert!(t.contains("m/s"), "unit dropped; got:\n{t}");
}

#[test]
fn qty_is_an_si_alias() {
    let t = typ(r"Mass \qty{5}{\kilo\gram}.");
    assert!(t.contains('5') && t.contains("kg"), "qty dropped; got:\n{t}");
}

#[test]
fn si_unit_only() {
    let t = typ(r"Resistivity \si{\ohm\meter\squared}.");
    assert!(t.contains("Ω") && t.contains("m²"), "unit-only dropped; got:\n{t}");
}

#[test]
fn ang_gets_degree_sign() {
    let t = typ(r"Angle \ang{45}.");
    assert!(t.contains("45°"), "angle dropped; got:\n{t}");
}
