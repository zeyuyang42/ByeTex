/// Snapshot tests for Phase 1a KaTeX symbol coverage expansion.
/// Each test verifies that specific LaTeX commands produce the expected
/// Typst math symbol expressions and generate zero conversion warnings.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn katex_phase1a_ams_relations() {
    let src = r"$\subseteq \supseteq \subsetneq \supsetneq \nsubseteq \nsupseteq$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("subset.eq"), "got: {}", out.typst);
    assert!(out.typst.contains("supset.eq"), "got: {}", out.typst);
    assert!(out.typst.contains("subset.neq"), "got: {}", out.typst);
    assert!(out.typst.contains("supset.neq"), "got: {}", out.typst);
    assert!(out.typst.contains("subset.eq.not"), "got: {}", out.typst);
    assert!(out.typst.contains("supset.eq.not"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_sq_subset_supset() {
    let src = r"$\sqsubseteq \sqsupseteq \Subset \Supset$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("subset.eq.sq"), "got: {}", out.typst);
    assert!(out.typst.contains("supset.eq.sq"), "got: {}", out.typst);
    assert!(out.typst.contains("subset.double"), "got: {}", out.typst);
    assert!(out.typst.contains("supset.double"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_ordering() {
    let src = r"$\prec \succ \preceq \succeq \ll \gg \lll \ggg$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("prec"), "got: {}", out.typst);
    assert!(out.typst.contains("succ"), "got: {}", out.typst);
    assert!(out.typst.contains("prec.eq"), "got: {}", out.typst);
    assert!(out.typst.contains("succ.eq"), "got: {}", out.typst);
    assert!(out.typst.contains("lt.double"), "got: {}", out.typst);
    assert!(out.typst.contains("gt.double"), "got: {}", out.typst);
    assert!(out.typst.contains("lt.triple"), "got: {}", out.typst);
    assert!(out.typst.contains("gt.triple"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_ordering_negations() {
    let src = r"$\nleq \ngeq \nless \ngtr \nsim$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("lt.eq.not"), "got: {}", out.typst);
    assert!(out.typst.contains("gt.eq.not"), "got: {}", out.typst);
    assert!(out.typst.contains("lt.not"), "got: {}", out.typst);
    assert!(out.typst.contains("gt.not"), "got: {}", out.typst);
    assert!(out.typst.contains("tilde.not"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_turnstile_logic() {
    let src = r"$\vdash \dashv \Vdash \models \mid \nmid \nparallel$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("tack.r"), "got: {}", out.typst);
    assert!(out.typst.contains("tack.l"), "got: {}", out.typst);
    assert!(out.typst.contains("tack.r.double"), "got: {}", out.typst);
    assert!(out.typst.contains("models"), "got: {}", out.typst);
    assert!(out.typst.contains("divides"), "got: {}", out.typst);
    assert!(out.typst.contains("divides.not"), "got: {}", out.typst);
    assert!(out.typst.contains("parallel.not"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_long_arrows() {
    let src = r"$\longrightarrow \longleftarrow \Longrightarrow \Longleftarrow \Longleftrightarrow$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("arrow.r.long"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.l.long"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.r.double.long"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.l.double.long"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.l.r.double.long"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_mapsto_longmapsto() {
    let src = r"$\mapsto \longmapsto$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("arrow.r.bar"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.r.bar.long"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_harpoons_diagonals() {
    let src = r"$\rightharpoonup \leftharpoonup \rightharpoondown \leftharpoondown \rightleftharpoons \nearrow \searrow \nwarrow \swarrow$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("harpoon.rt"), "got: {}", out.typst);
    assert!(out.typst.contains("harpoon.lt"), "got: {}", out.typst);
    assert!(out.typst.contains("harpoon.rb"), "got: {}", out.typst);
    assert!(out.typst.contains("harpoon.lb"), "got: {}", out.typst);
    assert!(out.typst.contains("harpoons.rtlb"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.tr"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.br"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.tl"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.bl"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_triple_arrows() {
    let src = r"$\Lsh \Rsh \Lleftarrow \Rrightarrow$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("arrow.l.hook"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.r.hook"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.l.triple"), "got: {}", out.typst);
    assert!(out.typst.contains("arrow.r.triple"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_big_operators() {
    let src = r"$\bigcup A_i \bigcap B_j \bigvee \bigwedge \bigoplus \bigotimes \bigodot \coprod$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("union.big"), "got: {}", out.typst);
    assert!(out.typst.contains("inter.big"), "got: {}", out.typst);
    assert!(out.typst.contains("or.big"), "got: {}", out.typst);
    assert!(out.typst.contains("and.big"), "got: {}", out.typst);
    assert!(out.typst.contains("plus.o.big"), "got: {}", out.typst);
    assert!(out.typst.contains("times.o.big"), "got: {}", out.typst);
    assert!(out.typst.contains("dot.o.big"), "got: {}", out.typst);
    assert!(out.typst.contains("product.co"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_binary_operators() {
    let src = r"$\rtimes \ltimes \circledast \circledcirc \wr \uplus \sqcup \sqcap$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("times.r"), "got: {}", out.typst);
    assert!(out.typst.contains("times.l"), "got: {}", out.typst);
    assert!(out.typst.contains("ast.op.o"), "got: {}", out.typst);
    assert!(out.typst.contains("compose.o"), "got: {}", out.typst);
    assert!(out.typst.contains("wreath"), "got: {}", out.typst);
    assert!(out.typst.contains("union.plus"), "got: {}", out.typst);
    assert!(out.typst.contains("union.sq"), "got: {}", out.typst);
    assert!(out.typst.contains("inter.sq"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_misc() {
    let src = r"$\therefore \because \complement \aleph \beth \gimel \daleth \ell \hbar \wr$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("therefore"), "got: {}", out.typst);
    assert!(out.typst.contains("because"), "got: {}", out.typst);
    assert!(out.typst.contains("complement"), "got: {}", out.typst);
    assert!(out.typst.contains("aleph"), "got: {}", out.typst);
    assert!(out.typst.contains("beth"), "got: {}", out.typst);
    assert!(out.typst.contains("gimel"), "got: {}", out.typst);
    assert!(out.typst.contains("daleth"), "got: {}", out.typst);
    assert!(out.typst.contains("ell"), "got: {}", out.typst);
    assert!(out.typst.contains("planck"), "got: {}", out.typst);
    assert!(out.typst.contains("wreath"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_hslash_alias() {
    let src = r"$\hslash$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("planck"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_greek_misc() {
    let src = r"$\varkappa \digamma \backprime$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("kappa.alt"), "got: {}", out.typst);
    assert!(out.typst.contains("digamma"), "got: {}", out.typst);
    assert!(out.typst.contains("prime.rev"), "got: {}", out.typst);
}

#[test]
fn katex_phase1a_additional_relations() {
    let src = r"$\approxeq \backsim \backsimeq \Cap \Cup \backepsilon$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("approx.eq"), "got: {}", out.typst);
    assert!(out.typst.contains("tilde.rev"), "got: {}", out.typst);
    assert!(out.typst.contains("tilde.rev.eq"), "got: {}", out.typst);
    assert!(out.typst.contains("inter.double"), "got: {}", out.typst);
    assert!(out.typst.contains("union.double"), "got: {}", out.typst);
    assert!(out.typst.contains("in.rev"), "got: {}", out.typst);
}
