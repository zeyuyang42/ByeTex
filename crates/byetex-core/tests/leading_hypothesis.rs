//! Why global VERTICAL SPACING constants are not the remaining layout lever.
//!
//! Two separate hypotheses were measured against the corpus and both rejected.
//! Recorded so they are not re-run: in each case the reasoning is sound, the
//! arithmetic checks out, and the change is still wrong.

use byetex_core::{convert, ConvertOptions};

#[test]
fn the_default_leading_is_deliberate() {
    // The preamble emits `leading: 0.65em`. Measured baseline-to-baseline, that
    // puts our lines at ~1.33x the font size, where LaTeX's default
    // `\baselineskip` is 1.2x — truth measures 14.5pt for a 12pt font, ours
    // 16.0pt. Typst's leading is the GAP between lines, so 1.2x would need
    // ~0.52em (0.65 - (1.333 - 1.2)).
    //
    // That reasoning is what makes this tempting, and the corpus says no. Page
    // counts with 0.52em against the LaTeX truth:
    //
    //     paper        truth  0.65em  0.52em   verdict
    //     2605.22159      24      25      24   better
    //     2605.22485      26      25      24   WORSE
    //     2605.30718      49      50      47   WORSE
    //     2605.31072      79      68      64   WORSE
    //     2605.22800      58      53      51   WORSE
    //     2605.22795      46      47      45   same
    //
    // 4 worse, 1 better, 1 same. The reason is visible in the base column: our
    // page counts are already mostly BELOW the truth's (68 vs 79, 53 vs 58), so
    // the output is too DENSE, and tightening the leading makes it denser.
    //
    // Note also that the harness's `layout_leading_ratio` reads 0.73-0.95 on
    // these papers — i.e. our leading measures SMALLER than truth's by its
    // definition, the opposite of the raw baseline measurement above. The two
    // measure different things, and the population is "mixed" (21 below vs 41
    // above), not systematic. `lines_per_page_ratio` IS systematic (median
    // 0.740, 58 of 65 below) but leading is not its cause.
    let out = convert(
        "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}",
        &ConvertOptions::default(),
    )
    .typst;
    assert!(
        out.contains("leading: 0.65em"),
        "the 0.65em default is load-bearing; see the comment before changing it:\n{out}"
    );
}


#[test]
fn display_skips_are_not_added_either() {
    // LaTeX puts `\abovedisplayskip`/`\belowdisplayskip` (~11pt at 11pt body)
    // around display math; Typst gives block equations the ordinary paragraph
    // spacing. The corpus shows the effect clearly — on 2605.30843, which has no
    // figures at all, truth's inter-line gaps run p75 22.4 / p90 30.1 against our
    // 16.0 / 22.9, and we pack 349 tokens per page to truth's 284.
    //
    // Adding `#show math.equation.where(block: true): set block(above: 11pt,
    // below: 11pt)` gives page counts:
    //
    //     paper        truth  base  +disp   verdict
    //     2605.30843      58    43     45   better
    //     2605.30609      40    29     31   better
    //     2605.22315       8     8      8   same
    //     2605.22765      47    47     50   WORSE
    //     2605.22728      22    27     29   WORSE
    //     2605.22779      12    15     15   same
    //
    // 2 better, 2 worse, 2 same. It helps papers that are already SHORT of truth
    // and hurts those already over it — and the corpus median page_ratio is
    // exactly 1.000 (24 papers below 0.95, 20 above 1.05, 21 within 5%), so a
    // global constant just moves the error around.
    //
    // The conclusion for both experiments: the remaining page-geometry deltas are
    // per-paper — float placement, figure sizing, content specifics — not a
    // global spacing constant that is set wrongly. A future attempt should target
    // a paper-level cause, not a preamble number.
    let _ = ();
}
