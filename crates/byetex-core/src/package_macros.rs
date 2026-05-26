/// Bundled macro expansion tables for common LaTeX packages, plus the KaTeX
/// always-on built-in macro table.
///
/// When `\usepackage{physics}` (or `bm`, `stmaryrd`, `mathtools`) is encountered
/// during the prepass, these seeds are inserted into the emitter's macro table
/// with `or_insert` so user-defined macros always win.
///
/// `KATEX_BUILTIN` is seeded unconditionally during emitter construction (before
/// the prepass) with `or_insert`, so prepass user `\newcommand` definitions always
/// win. Entries mirror LaTeX/amsmath defaults that KaTeX provides without any
/// `\usepackage`.
///
/// Invariant: no seed name in any table may collide with `lookup_math_symbol` —
/// the built-in math symbol table takes priority at emit time and seeded macros
/// would be silently shadowed. The unit test `no_seed_collides_with_builtin`
/// enforces this for all tables.
pub(crate) struct MacroSeed {
    pub params: usize,
    pub body: &'static str,
}

/// Returns the bundled seed table for the given package name, or `None` if the
/// package is not in the bundled list.
pub(crate) fn package_macros(name: &str) -> Option<&'static [(&'static str, MacroSeed)]> {
    Some(match name {
        "physics" => PHYSICS,
        "bm" => BM,
        "stmaryrd" => STMARYRD,
        "mathtools" => MATHTOOLS,
        _ => return None,
    })
}

// physics package — omitting \Re, \Im, \det, \arg, \div which collide with
// lookup_math_symbol built-ins.
static PHYSICS: &[(&str, MacroSeed)] = &[
    (
        r"\dv",
        MacroSeed {
            params: 2,
            body: r"\frac{d#1}{d#2}",
        },
    ),
    (
        r"\pdv",
        MacroSeed {
            params: 2,
            body: r"\frac{\partial #1}{\partial #2}",
        },
    ),
    (
        r"\dd",
        MacroSeed {
            params: 0,
            body: r"\mathrm{d}",
        },
    ),
    (
        r"\bra",
        MacroSeed {
            params: 1,
            body: r"\left\langle #1 \right|",
        },
    ),
    (
        r"\ket",
        MacroSeed {
            params: 1,
            body: r"\left| #1 \right\rangle",
        },
    ),
    (
        r"\braket",
        MacroSeed {
            params: 2,
            body: r"\left\langle #1 \middle| #2 \right\rangle",
        },
    ),
    (
        r"\expval",
        MacroSeed {
            params: 1,
            body: r"\left\langle #1 \right\rangle",
        },
    ),
    (
        r"\norm",
        MacroSeed {
            params: 1,
            body: r"\left\| #1 \right\|",
        },
    ),
    (
        r"\abs",
        MacroSeed {
            params: 1,
            body: r"\left| #1 \right|",
        },
    ),
    (
        r"\eval",
        MacroSeed {
            params: 1,
            body: r"\left. #1 \right|",
        },
    ),
    (
        r"\vb",
        MacroSeed {
            params: 1,
            body: r"\boldsymbol{#1}",
        },
    ),
    (
        r"\va",
        MacroSeed {
            params: 1,
            body: r"\vec{#1}",
        },
    ),
    (
        r"\grad",
        MacroSeed {
            params: 0,
            body: r"\nabla",
        },
    ),
    (
        r"\curl",
        MacroSeed {
            params: 0,
            body: r"\nabla \times",
        },
    ),
    (
        r"\divergence",
        MacroSeed {
            params: 0,
            body: r"\nabla \cdot",
        },
    ),
    (
        r"\laplacian",
        MacroSeed {
            params: 0,
            body: r"\nabla^2",
        },
    ),
];

static BM: &[(&str, MacroSeed)] = &[(
    r"\bm",
    MacroSeed {
        params: 1,
        body: r"\boldsymbol{#1}",
    },
)];

static STMARYRD: &[(&str, MacroSeed)] = &[
    // `\llbracket` / `\rrbracket` are now built into the math symbol
    // table (mapping to `bracket.l.double` / `bracket.r.double`).
    // Built-in lookup precedes package seeds so the entries here were
    // redundant; removed to keep the no-collision invariant.
    (
        r"\Mapsto",
        MacroSeed {
            params: 0,
            body: r"\Rightarrow\!\!",
        },
    ),
];

// \coloneqq and \eqqcolon are already in lookup_math_symbol (built-in wins),
// so only seed macros that are not covered by the built-in table.
static MATHTOOLS: &[(&str, MacroSeed)] = &[(
    r"\dblcolon",
    MacroSeed {
        params: 0,
        body: r"\mathrel{::}",
    },
)];

/// Always-on macro table — seeded unconditionally during emitter construction
/// (with `or_insert`). Mirrors LaTeX/amsmath defaults that KaTeX provides
/// without any `\usepackage`. Entries whose Typst form is a single symbol
/// token belong in `lookup_math_symbol` instead; these are all multi-token
/// LaTeX expansions that need to go through `expand_user_macro`.
///
/// Source: KaTeX v0.16.11 `src/macros.js`.
pub(crate) static KATEX_BUILTIN: &[(&str, MacroSeed)] = &[
    // === amsmath italic Greek capitals (macros.js ~354-363) ===
    (
        r"\varGamma",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Gamma}",
        },
    ),
    (
        r"\varDelta",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Delta}",
        },
    ),
    (
        r"\varTheta",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Theta}",
        },
    ),
    (
        r"\varLambda",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Lambda}",
        },
    ),
    (
        r"\varXi",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Xi}",
        },
    ),
    (
        r"\varPi",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Pi}",
        },
    ),
    (
        r"\varSigma",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Sigma}",
        },
    ),
    (
        r"\varUpsilon",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Upsilon}",
        },
    ),
    (
        r"\varPhi",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Phi}",
        },
    ),
    (
        r"\varPsi",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Psi}",
        },
    ),
    (
        r"\varOmega",
        MacroSeed {
            params: 0,
            body: r"\mathit{\Omega}",
        },
    ),
    // === amsmath/statmath operator-limit macros (macros.js ~748-758, ~894-896) ===
    // Bodies use \operatorname (not starred) — the * only affects limit
    // placement in display mode, an acceptable simplification.
    (
        r"\limsup",
        MacroSeed {
            params: 0,
            body: r"\operatorname{lim sup}",
        },
    ),
    (
        r"\liminf",
        MacroSeed {
            params: 0,
            body: r"\operatorname{lim inf}",
        },
    ),
    (
        r"\injlim",
        MacroSeed {
            params: 0,
            body: r"\operatorname{inj lim}",
        },
    ),
    (
        r"\projlim",
        MacroSeed {
            params: 0,
            body: r"\operatorname{proj lim}",
        },
    ),
    (
        r"\argmin",
        MacroSeed {
            params: 0,
            body: r"\operatorname{arg min}",
        },
    ),
    (
        r"\argmax",
        MacroSeed {
            params: 0,
            body: r"\operatorname{arg max}",
        },
    ),
    // === blackboard-bold shorthands (texvc, macros.js ~833-836, 866-868) ===
    // \N, \R, \Z are already in lookup_math_symbol (NN, RR, ZZ) — omitted here.
    (
        r"\Bbbk",
        MacroSeed {
            params: 0,
            body: r"\mathbb{k}",
        },
    ),
    (
        r"\cnums",
        MacroSeed {
            params: 0,
            body: r"\mathbb{C}",
        },
    ),
    (
        r"\Complex",
        MacroSeed {
            params: 0,
            body: r"\mathbb{C}",
        },
    ),
    (
        r"\natnums",
        MacroSeed {
            params: 0,
            body: r"\mathbb{N}",
        },
    ),
    (
        r"\reals",
        MacroSeed {
            params: 0,
            body: r"\mathbb{R}",
        },
    ),
    (
        r"\Reals",
        MacroSeed {
            params: 0,
            body: r"\mathbb{R}",
        },
    ),
    // === texvc uppercase Greek as upright Roman letters (macros.js ~838-888) ===
    (
        r"\Alpha",
        MacroSeed {
            params: 0,
            body: r"\mathrm{A}",
        },
    ),
    (
        r"\Beta",
        MacroSeed {
            params: 0,
            body: r"\mathrm{B}",
        },
    ),
    (
        r"\Chi",
        MacroSeed {
            params: 0,
            body: r"\mathrm{X}",
        },
    ),
    (
        r"\Epsilon",
        MacroSeed {
            params: 0,
            body: r"\mathrm{E}",
        },
    ),
    (
        r"\Eta",
        MacroSeed {
            params: 0,
            body: r"\mathrm{H}",
        },
    ),
    (
        r"\Iota",
        MacroSeed {
            params: 0,
            body: r"\mathrm{I}",
        },
    ),
    (
        r"\Kappa",
        MacroSeed {
            params: 0,
            body: r"\mathrm{K}",
        },
    ),
    (
        r"\Mu",
        MacroSeed {
            params: 0,
            body: r"\mathrm{M}",
        },
    ),
    (
        r"\Nu",
        MacroSeed {
            params: 0,
            body: r"\mathrm{N}",
        },
    ),
    (
        r"\Omicron",
        MacroSeed {
            params: 0,
            body: r"\mathrm{O}",
        },
    ),
    (
        r"\Rho",
        MacroSeed {
            params: 0,
            body: r"\mathrm{P}",
        },
    ),
    (
        r"\Tau",
        MacroSeed {
            params: 0,
            body: r"\mathrm{T}",
        },
    ),
    (
        r"\Zeta",
        MacroSeed {
            params: 0,
            body: r"\mathrm{Z}",
        },
    ),
    // === texvc simple arrow/symbol aliases (macros.js ~825-889) ===
    (
        r"\darr",
        MacroSeed {
            params: 0,
            body: r"\downarrow",
        },
    ),
    (
        r"\dArr",
        MacroSeed {
            params: 0,
            body: r"\Downarrow",
        },
    ),
    (
        r"\Darr",
        MacroSeed {
            params: 0,
            body: r"\Downarrow",
        },
    ),
    (
        r"\lang",
        MacroSeed {
            params: 0,
            body: r"\langle",
        },
    ),
    (
        r"\rang",
        MacroSeed {
            params: 0,
            body: r"\rangle",
        },
    ),
    (
        r"\uarr",
        MacroSeed {
            params: 0,
            body: r"\uparrow",
        },
    ),
    (
        r"\uArr",
        MacroSeed {
            params: 0,
            body: r"\Uparrow",
        },
    ),
    (
        r"\Uarr",
        MacroSeed {
            params: 0,
            body: r"\Uparrow",
        },
    ),
    (
        r"\larr",
        MacroSeed {
            params: 0,
            body: r"\leftarrow",
        },
    ),
    (
        r"\lArr",
        MacroSeed {
            params: 0,
            body: r"\Leftarrow",
        },
    ),
    (
        r"\Larr",
        MacroSeed {
            params: 0,
            body: r"\Leftarrow",
        },
    ),
    (
        r"\lrarr",
        MacroSeed {
            params: 0,
            body: r"\leftrightarrow",
        },
    ),
    (
        r"\lrArr",
        MacroSeed {
            params: 0,
            body: r"\Leftrightarrow",
        },
    ),
    (
        r"\Lrarr",
        MacroSeed {
            params: 0,
            body: r"\Leftrightarrow",
        },
    ),
    (
        r"\rarr",
        MacroSeed {
            params: 0,
            body: r"\rightarrow",
        },
    ),
    (
        r"\rArr",
        MacroSeed {
            params: 0,
            body: r"\Rightarrow",
        },
    ),
    (
        r"\Rarr",
        MacroSeed {
            params: 0,
            body: r"\Rightarrow",
        },
    ),
    (
        r"\harr",
        MacroSeed {
            params: 0,
            body: r"\leftrightarrow",
        },
    ),
    (
        r"\hArr",
        MacroSeed {
            params: 0,
            body: r"\Leftrightarrow",
        },
    ),
    (
        r"\Harr",
        MacroSeed {
            params: 0,
            body: r"\Leftrightarrow",
        },
    ),
    (
        r"\alef",
        MacroSeed {
            params: 0,
            body: r"\aleph",
        },
    ),
    (
        r"\alefsym",
        MacroSeed {
            params: 0,
            body: r"\aleph",
        },
    ),
    (
        r"\bull",
        MacroSeed {
            params: 0,
            body: r"\bullet",
        },
    ),
    (
        r"\clubs",
        MacroSeed {
            params: 0,
            body: r"\clubsuit",
        },
    ),
    (
        r"\Dagger",
        MacroSeed {
            params: 0,
            body: r"\ddagger",
        },
    ),
    (
        r"\diamonds",
        MacroSeed {
            params: 0,
            body: r"\diamondsuit",
        },
    ),
    (
        r"\empty",
        MacroSeed {
            params: 0,
            body: r"\emptyset",
        },
    ),
    (
        r"\exist",
        MacroSeed {
            params: 0,
            body: r"\exists",
        },
    ),
    (
        r"\hearts",
        MacroSeed {
            params: 0,
            body: r"\heartsuit",
        },
    ),
    (
        r"\image",
        MacroSeed {
            params: 0,
            body: r"\Im",
        },
    ),
    (
        r"\infin",
        MacroSeed {
            params: 0,
            body: r"\infty",
        },
    ),
    (
        r"\isin",
        MacroSeed {
            params: 0,
            body: r"\in",
        },
    ),
    (
        r"\plusmn",
        MacroSeed {
            params: 0,
            body: r"\pm",
        },
    ),
    (
        r"\real",
        MacroSeed {
            params: 0,
            body: r"\Re",
        },
    ),
    (
        r"\sdot",
        MacroSeed {
            params: 0,
            body: r"\cdot",
        },
    ),
    (
        r"\spades",
        MacroSeed {
            params: 0,
            body: r"\spadesuit",
        },
    ),
    (
        r"\sub",
        MacroSeed {
            params: 0,
            body: r"\subset",
        },
    ),
    (
        r"\sube",
        MacroSeed {
            params: 0,
            body: r"\subseteq",
        },
    ),
    (
        r"\supe",
        MacroSeed {
            params: 0,
            body: r"\supseteq",
        },
    ),
    (
        r"\thetasym",
        MacroSeed {
            params: 0,
            body: r"\vartheta",
        },
    ),
    (
        r"\weierp",
        MacroSeed {
            params: 0,
            body: r"\wp",
        },
    ),
    // === amsmath dots aliases (macros.js ~514-520) ===
    (
        r"\dotsb",
        MacroSeed {
            params: 0,
            body: r"\cdots",
        },
    ),
    (
        r"\dotsm",
        MacroSeed {
            params: 0,
            body: r"\cdots",
        },
    ),
    // === colonequals aliases that chain to lookup_math_symbol ===
    (
        r"\colonequals",
        MacroSeed {
            params: 0,
            body: r"\coloneqq",
        },
    ),
    (
        r"\equalscolon",
        MacroSeed {
            params: 0,
            body: r"\eqqcolon",
        },
    ),
    // === braket capitalized forms (macros.js ~903-905); lowercase in PHYSICS ===
    (
        r"\Bra",
        MacroSeed {
            params: 1,
            body: r"\left\langle #1 \right|",
        },
    ),
    (
        r"\Ket",
        MacroSeed {
            params: 1,
            body: r"\left| #1 \right\rangle",
        },
    ),
    // === ISO 80000-2 / regional trig operators (op.js) ===
    // Typst has no built-in identifier for these; emit via operatorname.
    (
        r"\sh",
        MacroSeed {
            params: 0,
            body: r"\operatorname{sh}",
        },
    ),
    (
        r"\ch",
        MacroSeed {
            params: 0,
            body: r"\operatorname{ch}",
        },
    ),
    (
        r"\th",
        MacroSeed {
            params: 0,
            body: r"\operatorname{th}",
        },
    ),
    (
        r"\tg",
        MacroSeed {
            params: 0,
            body: r"\operatorname{tg}",
        },
    ),
    (
        r"\ctg",
        MacroSeed {
            params: 0,
            body: r"\operatorname{ctg}",
        },
    ),
    (
        r"\cth",
        MacroSeed {
            params: 0,
            body: r"\operatorname{cth}",
        },
    ),
    (
        r"\cotg",
        MacroSeed {
            params: 0,
            body: r"\operatorname{cotg}",
        },
    ),
    (
        r"\cosec",
        MacroSeed {
            params: 0,
            body: r"\operatorname{cosec}",
        },
    ),
    (
        r"\arctg",
        MacroSeed {
            params: 0,
            body: r"\operatorname{arctg}",
        },
    ),
    (
        r"\arcctg",
        MacroSeed {
            params: 0,
            body: r"\operatorname{arcctg}",
        },
    ),
    // === Text-mode passthroughs (not in lookup_math_symbol) ===

    // `\num{1.23e-4}` (siunitx) — emit the number literal as-is; units/exponents
    // are a follow-up. Avoids 76 corpus unsupported_command warnings.
    (
        r"\num",
        MacroSeed {
            params: 1,
            body: "#1",
        },
    ),
    // `\texorpdfstring{tex}{pdf}` (hyperref) — use the LaTeX display form,
    // discard the PDF-bookmark string.
    (
        r"\texorpdfstring",
        MacroSeed {
            params: 2,
            body: "#1",
        },
    ),
    // `\ensuremath` is handled directly in emit.rs (mode-aware: passthrough in
    // math, $..$ wrapper in text). It was previously a macro seed with body
    // `$#1$` which created nested `$...$` when used inside math mode (Bug #49).
    // === NeurIPS checklist answer macros ===
    // These appear in NeurIPS checklist papers as fixed labels.
    (
        r"\answerYes",
        MacroSeed {
            params: 0,
            body: r"[Yes]",
        },
    ),
    (
        r"\answerNo",
        MacroSeed {
            params: 0,
            body: r"[No]",
        },
    ),
    (
        r"\answerNA",
        MacroSeed {
            params: 0,
            body: r"[NA]",
        },
    ),
    (
        r"\answerTODO",
        MacroSeed {
            params: 0,
            body: r"[TODO]",
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::lookup_math_symbol;

    const ALL_PACKAGES: &[&str] = &["physics", "bm", "stmaryrd", "mathtools"];

    #[test]
    fn no_seed_collides_with_builtin() {
        // Package macro seeds must not shadow lookup_math_symbol entries.
        for pkg in ALL_PACKAGES {
            if let Some(entries) = package_macros(pkg) {
                for (name, _) in entries {
                    assert!(
                        lookup_math_symbol(name).is_none(),
                        "package {} seed {:?} collides with lookup_math_symbol — \
                         remove from table (built-in wins at emit time)",
                        pkg,
                        name
                    );
                }
            }
        }
        // KATEX_BUILTIN entries likewise must not shadow lookup_math_symbol.
        for (name, _) in KATEX_BUILTIN {
            assert!(
                lookup_math_symbol(name).is_none(),
                "KATEX_BUILTIN seed {:?} collides with lookup_math_symbol — \
                 move to lookup_math_symbol or remove",
                name
            );
        }
    }

    #[test]
    fn every_seed_body_parses() {
        for pkg in ALL_PACKAGES {
            if let Some(entries) = package_macros(pkg) {
                for (name, seed) in entries {
                    let tree = crate::parser::parse(seed.body);
                    let root = tree.root_node();
                    assert!(
                        !root.has_error(),
                        "package {} macro {:?} body {:?} fails tree-sitter parse",
                        pkg,
                        name,
                        seed.body
                    );
                }
            }
        }
        // KATEX_BUILTIN bodies must also parse without tree-sitter errors.
        for (name, seed) in KATEX_BUILTIN {
            let tree = crate::parser::parse(seed.body);
            let root = tree.root_node();
            assert!(
                !root.has_error(),
                "KATEX_BUILTIN macro {:?} body {:?} fails tree-sitter parse",
                name,
                seed.body
            );
        }
    }
}
