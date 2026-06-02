// ─── Math symbol table ────────────────────────────────────────────────────────

/// Translate a LaTeX math command (with the leading backslash) into the
/// corresponding Typst math fragment. Returns `None` for unknown commands so
/// callers can decide between structural emission, warning, or pass-through.
pub(crate) fn lookup_math_symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        // Lowercase Greek
        "\\alpha" => "alpha",
        "\\beta" => "beta",
        "\\gamma" => "gamma",
        "\\delta" => "delta",
        "\\epsilon" => "epsilon",
        "\\varepsilon" => "epsilon.alt",
        "\\zeta" => "zeta",
        "\\eta" => "eta",
        "\\theta" => "theta",
        "\\vartheta" => "theta.alt",
        "\\iota" => "iota",
        "\\kappa" => "kappa",
        "\\lambda" => "lambda",
        "\\mu" => "mu",
        "\\nu" => "nu",
        "\\xi" => "xi",
        "\\pi" => "pi",
        "\\varpi" => "pi.alt",
        "\\rho" => "rho",
        "\\varrho" => "rho.alt",
        "\\sigma" => "sigma",
        "\\varsigma" => "sigma.alt",
        "\\tau" => "tau",
        "\\upsilon" => "upsilon",
        "\\phi" => "phi",
        "\\varphi" => "phi.alt",
        "\\chi" => "chi",
        "\\psi" => "psi",
        "\\omega" => "omega",
        // Uppercase Greek
        "\\Gamma" => "Gamma",
        "\\Delta" => "Delta",
        "\\Theta" => "Theta",
        "\\Lambda" => "Lambda",
        "\\Xi" => "Xi",
        "\\Pi" => "Pi",
        "\\Sigma" => "Sigma",
        "\\Upsilon" => "Upsilon",
        "\\Phi" => "Phi",
        "\\Psi" => "Psi",
        "\\Omega" => "Omega",
        // Operators
        "\\cdot" => "dot.c",
        "\\times" => "times",
        "\\div" => "div",
        "\\pm" => "plus.minus",
        "\\mp" => "minus.plus",
        "\\leq" | "\\le" => "<=",
        "\\geq" | "\\ge" => ">=",
        "\\neq" | "\\ne" => "!=",
        "\\equiv" => "equiv",
        "\\approx" => "approx",
        "\\sim" => "tilde.op",
        "\\simeq" => "tilde.eq",
        "\\cong" => "tilde.equiv",
        "\\asymp" => "≍",
        "\\propto" => "prop",
        "\\ngeq" => "gt.eq.not",
        "\\ngtr" => "gt.not",
        "\\nleq" => "lt.eq.not",
        "\\nless" => "lt.not",
        "\\coloneqq" | "\\coloneq" | "\\defeq" => "colon.eq",
        "\\eqqcolon" | "\\eqcolon" => "eq.colon",
        // `\vcentcolon` (mathtools): a vertically-centered colon used
        // in combinations like `\vcentcolon=` to form `:=`. Map to the
        // plain colon character; users who wanted the full `:=` typed
        // `\coloneqq` directly.
        "\\vcentcolon" => "colon",
        // `\lbrace`/`\rbrace` — alternate names for `\{`/`\}`,
        // frequent in arXiv math. Emit the escaped brace glyph (Typst
        // would parse a bare `{` as a group-start syntax). The plain
        // `\{`/`\}` aliases are handled elsewhere with the same shape.
        "\\lbrace" => "\\{",
        "\\rbrace" => "\\}",
        // `\llbracket` / `\rrbracket` (stmaryrd, mathbb-related):
        // Iverson-style double square brackets. Typst has dedicated
        // glyphs.
        "\\llbracket" => "bracket.l.double",
        "\\rrbracket" => "bracket.r.double",
        "\\bowtie" => "join",
        "\\to" | "\\rightarrow" => "arrow.r",
        "\\leftarrow" => "arrow.l",
        "\\leftrightarrow" => "arrow.l.r",
        "\\Rightarrow" => "arrow.r.double",
        "\\Leftarrow" => "arrow.l.double",
        "\\Leftrightarrow" => "arrow.l.r.double",
        "\\mapsto" => "arrow.r.bar",
        "\\hookrightarrow" => "arrow.r.hook",
        "\\hookleftarrow" => "arrow.l.hook",
        "\\uparrow" => "arrow.t",
        "\\downarrow" => "arrow.b",
        "\\updownarrow" => "arrow.t.b",
        "\\Uparrow" => "arrow.t.double",
        "\\Downarrow" => "arrow.b.double",
        "\\circ" => "circle.small",
        "\\bullet" => "bullet",
        "\\star" => "star.op",
        "\\ast" => "ast.op",
        // Circled / boxed operators
        "\\otimes" => "times.circle",
        "\\oplus" => "plus.circle",
        "\\ominus" => "minus.circle",
        "\\odot" => "dot.circle",
        "\\oslash" => "slash.circle",
        "\\boxtimes" => "times.square",
        "\\boxplus" => "plus.square",
        // Geometric / order
        "\\Box" | "\\square" => "square",
        "\\diamond" | "\\Diamond" | "\\diamondsuit" => "diamond",
        "\\triangle" | "\\bigtriangleup" => "triangle",
        "\\bigtriangledown" => "triangle.b",
        "\\angle" => "angle",
        "\\perp" => "perp",
        "\\parallel" => "parallel",
        "\\top" => "top",
        "\\bot" => "bot",
        // Sets and logic
        "\\in" => "in",
        "\\notin" => "in.not",
        "\\subset" => "subset",
        "\\supset" => "supset",
        "\\subseteq" => "subset.eq",
        "\\supseteq" => "supset.eq",
        "\\cup" => "union",
        "\\cap" => "inter",
        "\\setminus" => "without",
        "\\emptyset" => "nothing",
        "\\forall" => "forall",
        "\\exists" => "exists",
        "\\neg" | "\\lnot" => "not",
        "\\land" | "\\wedge" => "and",
        "\\lor" | "\\vee" => "or",
        "\\implies" => "==>",
        "\\iff" => "<==>",
        // Sums / products / integrals
        "\\sum" => "sum",
        "\\prod" => "product",
        "\\int" => "integral",
        "\\iint" => "integral.double",
        "\\iiint" => "integral.triple",
        "\\oint" => "integral.cont",
        "\\lim" => "lim",
        "\\sup" => "sup",
        "\\inf" => "inf",
        "\\max" => "max",
        "\\min" => "min",
        // Number sets (require amsfonts in LaTeX). \mathbb{R} is handled
        // elsewhere; common shorthand commands below.
        "\\R" => "RR",
        "\\Z" => "ZZ",
        "\\N" => "NN",
        "\\Q" => "QQ",
        "\\C" => "CC",
        // Special
        "\\infty" => "infinity",
        "\\partial" => "partial",
        "\\nabla" => "nabla",
        "\\hbar" | "\\hslash" => "planck",
        "\\ell" => "ell",
        "\\dots" | "\\ldots" => "dots.h",
        "\\cdots" => "dots.c",
        "\\vdots" => "dots.v",
        "\\ddots" => "dots.down",
        "\\degree" => "degree",
        "\\dagger" => "dagger",
        "\\ddagger" => "dagger.double",
        "\\prime" => "prime",
        "\\Re" => "Re",
        "\\Im" => "Im",
        // `\notag` / `\nonumber` suppress equation numbering. Typst
        // doesn't number untagged equations either, so drop silently.
        "\\notag" | "\\nonumber" => "",
        // `\colon` is the typed colon glyph in amsmath; in Typst math
        // a plain `:` renders identically.
        "\\colon" => ":",
        "\\aleph" => "aleph",
        "\\beth" => "beth",
        "\\gimel" => "gimel",
        "\\imath" => "dotless.i",
        "\\jmath" => "dotless.j",
        "\\backslash" => "backslash",
        "\\flat" => "♭",
        "\\sharp" => "♯",
        "\\natural" => "♮",
        "\\clubsuit" => "♣",
        "\\spadesuit" => "♠",
        "\\heartsuit" => "♥",
        // `\not` is handled by an explicit arm in emit_math_command that
        // emits a DropOnly warning; it must not appear here or push_math_symbol
        // would silently swallow it via the empty-string early-return.
        // Trig and log functions — Typst recognises these by name in math.
        "\\sin" => "sin",
        "\\cos" => "cos",
        "\\tan" => "tan",
        "\\cot" => "cot",
        "\\sec" => "sec",
        "\\csc" => "csc",
        "\\arcsin" => "arcsin",
        "\\arccos" => "arccos",
        "\\arctan" => "arctan",
        "\\sinh" => "sinh",
        "\\cosh" => "cosh",
        "\\tanh" => "tanh",
        "\\log" => "log",
        "\\ln" => "ln",
        "\\exp" => "exp",
        "\\coth" => "coth",
        // Standard math operators — Typst renders these upright by name.
        "\\det" => "det",
        "\\dim" => "dim",
        "\\ker" => "ker",
        "\\arg" => "arg",
        "\\deg" => "deg",
        "\\hom" => "hom",
        "\\Pr" => "Pr",
        "\\lg" => "lg",
        // Other small bits
        "\\pmod" => "mod",
        "\\bmod" => "mod",
        "\\gcd" => "gcd",
        // Norm / bar delimiters
        "\\|" | "\\Vert" => "||",
        "\\vert" => "|",
        "\\lvert" => "|",
        "\\rvert" => "|",
        "\\lVert" | "\\rVert" => "||",
        // Typst 0.13+ deprecated `angle.l` / `angle.r` in favour of
        // `chevron.l` / `chevron.r`; emitting the new names keeps the
        // compile clean of deprecation warnings.
        "\\langle" => "chevron.l",
        "\\rangle" => "chevron.r",
        "\\lceil" => "ceil.l",
        "\\rceil" => "ceil.r",
        "\\lfloor" => "floor.l",
        "\\rfloor" => "floor.r",
        // Math spacing — LaTeX positive-space commands. Typst's `thin`,
        // `med`, `thick`, `quad` are the equivalent named symbols. Without
        // these the bare LaTeX command name leaked into the output and
        // fused with the next identifier (`\thinspace` adjacent to `d`
        // would produce the unknown variable `thinspaced`).
        "\\qquad" => "quad quad",
        "\\quad" => "quad",
        "\\," | "\\thinspace" => "thin",
        "\\:" | "\\medspace" => "med",
        "\\;" | "\\thickspace" => "thick",
        // Negative spacing — Typst has no direct named equivalent. Drop;
        // the visual difference at the call sites is sub-em.
        "\\!" | "\\negthinspace" | "\\negmedspace" | "\\negthickspace" => "",
        // Delimiter-size commands (Typst auto-sizes via `lr(...)`); drop.
        "\\big" | "\\Big" | "\\bigg" | "\\Bigg" | "\\bigl" | "\\Bigl" | "\\biggl" | "\\Biggl"
        | "\\bigr" | "\\Bigr" | "\\biggr" | "\\Biggr" | "\\bigm" | "\\Bigm" | "\\biggm"
        | "\\Biggm" => "",
        // `\left` / `\right` in math — Typst's math grammar auto-pairs
        // `(`, `[`, `\{` style delimiters and provides `lr(...)` for
        // explicit stretching. Dropping the command keeps the following
        // delimiter character (which is emitted as its own node) intact.
        // Previously `\left(V-G\right)` leaked into the output as raw
        // `\left(V-G\right)`, and Typst read `\l` as the math escape for
        // `l`, leaving the unknown identifier `eft(...)`.
        "\\left" | "\\right" => "",
        // `\middle` is the same pattern for mid-fence stretching; no
        // Typst equivalent for the bare form, so drop and let the
        // following delimiter render literally.
        "\\middle" => "",
        // Operator-display modifiers — Typst always places sub/super
        // in display position for `lim`, `sum`, `int`, etc., so the
        // explicit force is a no-op. Drop the command name itself; if
        // left in, `\limits` was emitted as the literal word and the
        // subscript that followed became an unknown symbol modifier.
        "\\limits" | "\\nolimits" => "",
        // `\displaystyle` / `\textstyle` / `\scriptstyle` /
        // `\scriptscriptstyle` are handled by explicit warning arms in
        // emit_math_command; they must not appear here.
        // Math escapes for ASCII chars. Keep the leading backslash so Typst
        // treats them as math escapes — emitting the bare character would
        // trigger Typst's own special handling: `#` opens code context,
        // `$` toggles math, `&` is alignment, `_` / `^` are sub/superscript,
        // `{` / `}` are paired delimiters. Concretely, `f_\#` previously
        // emitted as `f_(#)` was parsed as `(code)` and failed with
        // "unexpected closing paren".
        "\\#" => "\\#",
        "\\$" => "\\$",
        "\\%" => "\\%",
        "\\&" => "\\&",
        "\\_" => "\\_",
        "\\{" => "\\{",
        "\\}" => "\\}",
        // === AMS subset/supset relations (Phase 1a) ===
        "\\subsetneq" | "\\varsubsetneq" => "subset.neq",
        "\\supsetneq" | "\\varsupsetneq" => "supset.neq",
        "\\nsubseteq" => "subset.eq.not",
        "\\nsupseteq" => "supset.eq.not",
        "\\sqsubseteq" => "subset.eq.sq",
        "\\sqsupseteq" => "supset.eq.sq",
        "\\Subset" => "subset.double",
        "\\Supset" => "supset.double",
        // === AMS ordering relations (Phase 1a) ===
        "\\prec" => "prec",
        "\\succ" => "succ",
        "\\preceq" => "prec.eq",
        "\\succeq" => "succ.eq",
        "\\ll" => "lt.double",
        "\\gg" => "gt.double",
        "\\lll" | "\\llless" => "lt.triple",
        "\\ggg" | "\\gggtr" => "gt.triple",
        "\\doteq" => "eq.dots",
        "\\nsim" => "tilde.not",
        "\\nequiv" => "equiv.not",
        // === AMS turnstile / logic (Phase 1a) ===
        "\\vdash" => "tack.r",
        "\\dashv" => "tack.l",
        "\\Vdash" | "\\vDash" => "tack.r.double",
        "\\models" => "models",
        "\\mid" => "divides",
        "\\nmid" => "divides.not",
        "\\nparallel" | "\\nshortparallel" => "parallel.not",
        // === Long arrows (Phase 1a) ===
        "\\longrightarrow" => "arrow.r.long",
        "\\longleftarrow" => "arrow.l.long",
        "\\longleftrightarrow" => "arrow.l.r.long",
        "\\Longrightarrow" => "arrow.r.double.long",
        "\\Longleftarrow" => "arrow.l.double.long",
        "\\Longleftrightarrow" => "arrow.l.r.double.long",
        "\\longmapsto" => "arrow.r.bar.long",
        // === Harpoons and diagonal arrows (Phase 1a) ===
        "\\rightharpoonup" => "harpoon.rt",
        "\\leftharpoonup" => "harpoon.lt",
        "\\rightharpoondown" => "harpoon.rb",
        "\\leftharpoondown" => "harpoon.lb",
        "\\rightleftharpoons" | "\\rightleftarrows" => "harpoons.rtlb",
        "\\nearrow" => "arrow.tr",
        "\\searrow" => "arrow.br",
        "\\nwarrow" => "arrow.tl",
        "\\swarrow" => "arrow.bl",
        "\\Lsh" => "arrow.l.hook",
        "\\Rsh" => "arrow.r.hook",
        "\\Lleftarrow" => "arrow.l.triple",
        "\\Rrightarrow" => "arrow.r.triple",
        // === Big operators (Phase 1a) ===
        "\\bigcup" => "union.big",
        "\\bigcap" => "inter.big",
        "\\bigvee" => "or.big",
        "\\bigwedge" => "and.big",
        "\\bigoplus" => "plus.o.big",
        "\\bigotimes" => "times.o.big",
        "\\bigodot" => "dot.o.big",
        "\\coprod" => "product.co",
        // === Binary operators (Phase 1a) ===
        "\\rtimes" => "times.r",
        "\\ltimes" => "times.l",
        "\\circledast" => "ast.op.o",
        "\\circledcirc" => "compose.o",
        "\\wr" => "wreath",
        "\\uplus" => "union.plus",
        "\\sqcup" => "union.sq",
        "\\sqcap" => "inter.sq",
        // === Misc AMS symbols (Phase 1a) ===
        "\\therefore" => "therefore",
        "\\because" => "because",
        "\\complement" => "complement",
        "\\daleth" => "daleth",
        "\\backprime" => "prime.rev",
        "\\varkappa" => "kappa.alt",
        "\\digamma" => "digamma",
        // === Additional AMS relations (Phase 1a) ===
        "\\approxeq" => "approx.eq",
        "\\backsim" => "tilde.rev",
        "\\backsimeq" => "tilde.rev.eq",
        "\\eqcirc" => "eq.o",
        "\\Cap" | "\\doublecap" => "inter.double",
        "\\Cup" | "\\doublecup" => "union.double",
        "\\backepsilon" => "in.rev",
        // === Extended ordering relations (Phase 1b) ===
        "\\geqq" => "gt.equiv",
        "\\leqq" => "lt.equiv",
        "\\geqslant" => "gt.eq.slant",
        "\\leqslant" => "lt.eq.slant",
        "\\gtrsim" => "gt.tilde",
        "\\lesssim" => "lt.tilde",
        "\\gtrapprox" => "gt.approx",
        "\\lessapprox" => "lt.approx",
        "\\gtrdot" => "gt.dot",
        "\\lessdot" => "lt.dot",
        "\\gtrless" => "gt.lt",
        "\\lessgtr" => "lt.gt",
        "\\gtreqless" => "gt.eq.lt",
        "\\lesseqgtr" => "lt.eq.gt",
        // \gtreqqless / \lesseqqgtr: no Typst named symbol, defer
        // "\\gtreqqless" => "gt.equiv.lt",  // invalid
        // "\\lesseqqgtr" => "lt.equiv.gt",  // invalid
        "\\gnapprox" => "gt.napprox",
        "\\lnapprox" => "lt.napprox",
        "\\gneq" => "gt.nequiv",
        "\\lneq" => "lt.nequiv",
        // \gneqq / \lneqq: gt.nequiv.double / lt.nequiv.double do not exist in Typst 0.14.2, defer
        // "\\gneqq" => "gt.nequiv.double",
        // "\\lneqq" => "lt.nequiv.double",
        "\\gnsim" => "gt.ntilde",
        "\\lnsim" => "lt.ntilde",
        // \eqsim: eq.tilde does not exist in Typst 0.14.2, defer
        // \eqslantgtr / \eqslantless: eq.slant.gt / eq.slant.lt do not exist, defer
        // \fallingdotseq / \risingdotseq: eq.dots.fall / eq.dots.rise do not exist, defer
        "\\precapprox" => "prec.approx",
        "\\succapprox" => "succ.approx",
        "\\preccurlyeq" | "\\curlyeqprec" => "prec.curly.eq",
        "\\succcurlyeq" | "\\curlyeqsucc" => "succ.curly.eq",
        "\\precnapprox" => "prec.napprox",
        "\\succnapprox" => "succ.napprox",
        "\\precneqq" => "prec.nequiv",
        "\\succneqq" => "succ.nequiv",
        "\\precnsim" => "prec.ntilde",
        "\\succnsim" => "succ.ntilde",
        "\\precsim" => "prec.tilde",
        "\\succsim" => "succ.tilde",
        // === Triangle symbols (Phase 1b) ===
        "\\triangleleft" | "\\vartriangleleft" => "lt.tri",
        "\\triangleright" | "\\vartriangleright" => "gt.tri",
        "\\trianglelefteq" => "lt.tri.eq",
        "\\trianglerighteq" => "gt.tri.eq",
        "\\ntriangleleft" => "lt.tri.not",
        "\\ntriangleright" => "gt.tri.not",
        "\\ntrianglelefteq" => "lt.tri.eq.not",
        "\\ntrianglerighteq" => "gt.tri.eq.not",
        "\\vartriangle" => "triangle.t",
        "\\triangledown" => "triangle.b",
        "\\blacktriangle" => "triangle.filled.t",
        "\\blacktriangledown" => "triangle.filled.b",
        "\\blacktriangleleft" => "triangle.filled.l",
        "\\blacktriangleright" => "triangle.filled.r",
        // === Extended arrows (Phase 1b) ===
        "\\rightsquigarrow" | "\\leadsto" => "arrow.r.squiggly",
        // \leftrightsquigarrow: arrow.l.r.squiggly does not exist in Typst 0.14.2, defer
        "\\twoheadrightarrow" => "arrow.r.twohead",
        "\\twoheadleftarrow" => "arrow.l.twohead",
        "\\rightarrowtail" => "arrow.r.tail",
        "\\leftarrowtail" => "arrow.l.tail",
        "\\multimap" => "multimap",
        "\\upuparrows" => "arrows.tt",
        "\\downdownarrows" => "arrows.bb",
        "\\leftrightarrows" => "arrows.lr",
        "\\leftleftarrows" => "arrows.ll",
        "\\rightrightarrows" => "arrows.rr",
        "\\Updownarrow" => "arrows.tb",
        "\\looparrowleft" => "arrow.l.loop",
        "\\looparrowright" => "arrow.r.loop",
        "\\curvearrowleft" => "arrow.l.curve",
        "\\curvearrowright" => "arrow.r.curve",
        "\\dashleftarrow" => "arrow.l.dashed",
        "\\dashrightarrow" => "arrow.r.dashed",
        // === Vertical harpoons (Phase 1b) ===
        "\\upharpoonright" | "\\restriction" => "harpoon.tr",
        "\\upharpoonleft" => "harpoon.tl",
        "\\downharpoonright" => "harpoon.br",
        "\\downharpoonleft" => "harpoon.bl",
        "\\leftrightharpoons" => "harpoons.ltrb",
        // === Negated arrows (Phase 1b) ===
        "\\nleftarrow" => "arrow.l.not",
        "\\nrightarrow" => "arrow.r.not",
        "\\nleftrightarrow" => "arrow.l.r.not",
        "\\nLeftarrow" => "arrow.l.double.not",
        "\\nRightarrow" => "arrow.r.double.not",
        "\\nLeftrightarrow" => "arrow.l.r.double.not",
        // === Negated turnstile / logic (Phase 1b) ===
        "\\nvdash" => "tack.r.not",
        "\\nvDash" => "tack.r.double.not",
        "\\nVdash" => "tack.r.not.double",
        "\\nVDash" => "tack.r.double.not.double",
        "\\ncong" => "tilde.nequiv",
        "\\nprec" => "prec.not",
        "\\nsucc" => "succ.not",
        "\\npreceq" => "prec.eq.not",
        "\\nsucceq" => "succ.eq.not",
        // === Misc AMS (Phase 1b) ===
        "\\varnothing" => "emptyset",
        "\\nexists" => "exists.not",
        "\\ni" | "\\owns" => "in.rev",
        "\\smallsetminus" => "without",
        // \intercal: "intercal" is not a Typst named symbol in 0.14.2, defer
        "\\checkmark" => "checkmark",
        "\\lozenge" => "lozenge.stroked",
        "\\blacklozenge" => "lozenge.filled",
        "\\blacksquare" => "square.filled",
        "\\bigstar" => "star.filled",
        "\\yen" => "yen",
        "\\sphericalangle" => "angle.spheric",
        "\\measuredangle" => "angle.arc",
        "\\frown" | "\\smallfrown" => "frown",
        "\\smile" | "\\smallsmile" => "smile",
        "\\varpropto" => "prop",
        "\\dotplus" => "plus.dot",
        "\\divideontimes" => "times.div",
        // \veebar: or.excl does not exist in Typst 0.14.2, defer
        "\\boxminus" => "minus.square",
        "\\boxdot" => "dot.square",
        "\\circleddash" => "minus.o",
        "\\varvdots" => "dots.v",
        "\\mathellipsis" => "dots.h",
        "\\shortmid" => "divides",
        "\\shortparallel" => "parallel",
        "\\smallint" => "integral",
        "\\gets" => "arrow.l",
        "\\lhd" | "\\unlhd" => "lt.tri.eq",
        "\\rhd" | "\\unrhd" => "gt.tri.eq",
        "\\imageof" => "image",
        "\\origof" => "original",
        "\\cdotp" | "\\centerdot" => "dot.c",
        "\\circledR" => "circle.stroked.small",
        "\\circledS" => "circle.small.filled",
        "\\leftthreetimes" => "times.three.l",
        "\\rightthreetimes" => "times.three.r",
        "\\dag" | "\\textdagger" => "dagger",
        "\\ddag" | "\\textdaggerdbl" => "dagger.double",
        // \wp: "weierp" is not a Typst named symbol in 0.14.2, defer
        // Bold variants
        // `\\boldsymbol` / `\\pmb` are handled as wraps in
        // `emit_math_command`; keeping a symbol-table entry would mask
        // the wrap dispatch by returning early.
        // Common math fonts not handled by emit_math_wrap
        _ => return None,
    })
}
