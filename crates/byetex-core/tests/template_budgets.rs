//! Per-template warning budget regression test.
//!
//! For every template under `tests/inhouse/<name>/`, runs `byetex_core::convert`
//! on the entry `.tex` and asserts the warning count stays at or below the
//! recorded budget. The budgets are intentionally tight — a regression
//! (e.g. a refactor that re-warns on previously-handled commands) trips this
//! test before the change can land.
//!
//! Tightening a budget: see T5 in the plan; once `<20` is achievable across
//! the board, drop each value here to the new ceiling.

use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/byetex-core has at least two parents")
        .to_path_buf()
}

fn check_template(rel: &str, budget: usize) {
    let path = workspace_root().join(rel);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let out = convert(
        &source,
        &ConvertOptions {
            source_name: Some(rel.to_string()),
            ..Default::default()
        },
    );
    let count = out.warnings.len();
    eprintln!("{rel}: {count} warnings (budget {budget})");
    assert!(
        count <= budget,
        "{rel} produced {count} warnings (budget {budget}). \
         Either the budget needs widening with justification, or recent emitter \
         changes regressed coverage.",
        rel = rel,
        count = count,
        budget = budget,
    );
}

// Baselines captured after T1 land — current as of the v0.2 plan kickoff.
// These will tighten as T2, T3, T4 patch the highest-recurrence gaps.

// Budgets snapshot:
//   v0.2 T1 baseline                                       IEEE 43 ACM 22 NeurIPS 37 thesis 19
//   v0.2 T2 (title block + typography)                     IEEE 43 ACM 16 NeurIPS 34 thesis 15
//   v0.2 T3 (math splitting + tables + theorems + bib map) IEEE 37 ACM 13 NeurIPS 21 thesis 12
//   v0.2 T4 (inline cleanup + usepackage allowlist)        IEEE 20 ACM  1 NeurIPS  9 thesis  0
//   post-merge corpus pass (math accents, escapes, footnote, multirow,
//   href, url, label, font sizes, appendix, more no-op packages, more
//   transparent envs)                                      IEEE 17 ACM  0 NeurIPS  1 thesis  0
//   class-aware template emission (charged-ieee, clean-acmart,
//   lucky-icml; \IEEEkeywords captured; abstract field captured for
//   classes that accept it)                                IEEE 16 ACM  0 NeurIPS  1 thesis  0
//   silent-drop audit: converted previously-silent drops of ACM author-info
//   fields (\authornote, \email, …) and IEEEtran content commands into
//   UnsupportedCommand warnings. ACM +3 = \authornote + 2×\email (the
//   \institution/\city/\country inside \affiliation are consumed by the
//   \affiliation silent-drop before they reach the dispatcher).
//                                                          IEEE 16 ACM  3 NeurIPS  1 thesis  0
//   silent-drop-to-DropOnly audit: \acmConference and 2×\affiliation
//   now emit DropOnly warnings in ACM template (+3); \tableofcontents
//   and \listoffigures now emit DropOnly warnings in thesis (+2).
//                                                          IEEE 16 ACM  6 NeurIPS  1 thesis  2
//   PR1: font-size family (\small/\large/\Large/…) converted from
//   UnsupportedCommand to silent drop; text-mode symbols (\texttimes,
//   \textuparrow, \textdownarrow, \checkmark, \AA, \l, \newline,
//   \tabularnewline) now emit Unicode directly.
//                                                          IEEE 13 ACM  6 NeurIPS  1 thesis  2
//   PR2: \nolinkurl/\hyperlink/\hypertarget as inline wraps; \num/
//   \texorpdfstring/\ensuremath via KATEX_BUILTIN passthroughs.
//   No template budget change (none of these appear in the 4 templates).
//                                                          IEEE 13 ACM  6 NeurIPS  1 thesis  2
//   PR3: preamble silencing allowlist — \typeout, \theoremstyle,
//   \crefname/\Crefname, \hypersetup, \enlargethispage, \looseness,
//   \endcsname, \expandafter, \makeatletter/\makeatother, \addlinespace,
//   \AddToHook, \FloatBarrier, \colorlet, \ifthenelse/\fi/\else.
//   No template budget change (none appear in the 4 templates).
//                                                          IEEE 13 ACM  6 NeurIPS  1 thesis  2

#[test]
fn ieee_template_within_budget() {
    check_template("tests/inhouse/ieee/conference_101719.tex", 13);
}

#[test]
fn acm_template_within_budget() {
    check_template("tests/inhouse/acm/sample-sigconf.tex", 6);
}

#[test]
fn neurips_template_within_budget() {
    check_template("tests/inhouse/neurips/neurips_paper.tex", 1);
}

#[test]
fn thesis_template_within_budget() {
    check_template("tests/inhouse/thesis/thesis_skeleton.tex", 2);
}

#[test]
fn physics_package_within_budget() {
    check_template("tests/inhouse/physics/paper.tex", 0);
}

#[test]
fn bm_package_within_budget() {
    check_template("tests/inhouse/bm/paper.tex", 0);
}

#[test]
fn stmaryrd_package_within_budget() {
    check_template("tests/inhouse/stmaryrd/paper.tex", 0);
}
