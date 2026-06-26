//! ulem/soul text decorations were dropped, taking their CONTENT with them
//! (`\sout{struck}` rendered empty). Map them to Typst functions so the text
//! survives: `\sout`/`\xout`/`\st` → `#strike[…]`, `\uline`/`\uuline`/`\uwave`
//! → `#underline[…]`. Found by direct validation.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn sout_strikes_through_keeping_text() {
    let t = typ(r"A \sout{struck} word.");
    assert!(t.contains("#strike[struck]"), "sout dropped/wrong; got:\n{t}");
}

#[test]
fn uline_underlines_keeping_text() {
    let t = typ(r"An \uline{underlined} word.");
    assert!(t.contains("#underline[underlined]"), "uline dropped/wrong; got:\n{t}");
}
