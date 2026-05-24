/// Bundled macro expansion tables for common LaTeX packages.
///
/// When `\usepackage{physics}` (or `bm`, `stmaryrd`, `mathtools`) is encountered
/// during the prepass, these seeds are inserted into the emitter's macro table
/// with `or_insert` so user-defined macros always win.
///
/// Invariant: no seed name may collide with `lookup_math_symbol` — the built-in
/// math symbol table takes priority at emit time and seeded macros would be silently
/// shadowed. The unit test `no_seed_collides_with_builtin` enforces this.

pub(crate) struct MacroSeed {
    pub params: usize,
    pub body: &'static str,
}

/// Returns the bundled seed table for the given package name, or `None` if the
/// package is not in the bundled list.
pub(crate) fn package_macros(name: &str) -> Option<&'static [(&'static str, MacroSeed)]> {
    Some(match name {
        "physics"   => PHYSICS,
        "bm"        => BM,
        "stmaryrd"  => STMARYRD,
        "mathtools" => MATHTOOLS,
        _ => return None,
    })
}

// physics package — omitting \Re, \Im, \det, \arg, \div which collide with
// lookup_math_symbol built-ins.
static PHYSICS: &[(&str, MacroSeed)] = &[
    (r"\dv",         MacroSeed { params: 2, body: r"\frac{d#1}{d#2}" }),
    (r"\pdv",        MacroSeed { params: 2, body: r"\frac{\partial #1}{\partial #2}" }),
    (r"\dd",         MacroSeed { params: 0, body: r"\mathrm{d}" }),
    (r"\bra",        MacroSeed { params: 1, body: r"\left\langle #1 \right|" }),
    (r"\ket",        MacroSeed { params: 1, body: r"\left| #1 \right\rangle" }),
    (r"\braket",     MacroSeed { params: 2, body: r"\left\langle #1 \middle| #2 \right\rangle" }),
    (r"\expval",     MacroSeed { params: 1, body: r"\left\langle #1 \right\rangle" }),
    (r"\norm",       MacroSeed { params: 1, body: r"\left\| #1 \right\|" }),
    (r"\abs",        MacroSeed { params: 1, body: r"\left| #1 \right|" }),
    (r"\eval",       MacroSeed { params: 1, body: r"\left. #1 \right|" }),
    (r"\vb",         MacroSeed { params: 1, body: r"\boldsymbol{#1}" }),
    (r"\va",         MacroSeed { params: 1, body: r"\vec{#1}" }),
    (r"\grad",       MacroSeed { params: 0, body: r"\nabla" }),
    (r"\curl",       MacroSeed { params: 0, body: r"\nabla \times" }),
    (r"\divergence", MacroSeed { params: 0, body: r"\nabla \cdot" }),
    (r"\laplacian",  MacroSeed { params: 0, body: r"\nabla^2" }),
];

static BM: &[(&str, MacroSeed)] = &[
    (r"\bm", MacroSeed { params: 1, body: r"\boldsymbol{#1}" }),
];

static STMARYRD: &[(&str, MacroSeed)] = &[
    (r"\llbracket", MacroSeed { params: 0, body: r"[\![" }),
    (r"\rrbracket", MacroSeed { params: 0, body: r"]\!]" }),
    (r"\Mapsto",    MacroSeed { params: 0, body: r"\Rightarrow\!\!" }),
];

// \coloneqq and \eqqcolon are already in lookup_math_symbol (built-in wins),
// so only seed macros that are not covered by the built-in table.
static MATHTOOLS: &[(&str, MacroSeed)] = &[
    (r"\dblcolon", MacroSeed { params: 0, body: r"\mathrel{::}" }),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::lookup_math_symbol;

    const ALL_PACKAGES: &[&str] = &["physics", "bm", "stmaryrd", "mathtools"];

    #[test]
    fn no_seed_collides_with_builtin() {
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
    }
}
