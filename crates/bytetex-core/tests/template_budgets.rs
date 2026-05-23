//! Per-template warning budget regression test.
//!
//! For every template under `templates/<name>/`, runs `bytetex_core::convert`
//! on the entry `.tex` and asserts the warning count stays at or below the
//! recorded budget. The budgets are intentionally tight — a regression
//! (e.g. a refactor that re-warns on previously-handled commands) trips this
//! test before the change can land.
//!
//! Tightening a budget: see T5 in the plan; once `<20` is achievable across
//! the board, drop each value here to the new ceiling.

use std::path::PathBuf;

use bytetex_core::{convert, ConvertOptions};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/bytetex-core has at least two parents")
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
// All four templates compile to PDF, all at or below the <20 plan target.
// IEEE's residual 20 are IEEE-class-specific commands (IEEEauthorblockN/A,
// IEEEpubid, etc.) — covered by a future IEEE-specific skill rather than
// emitter rules.

#[test]
fn ieee_template_within_budget() {
    check_template("templates/IEEE/conference_101719.tex", 20);
}

#[test]
fn acm_template_within_budget() {
    check_template("templates/ACM/sample-sigconf.tex", 1);
}

#[test]
fn neurips_template_within_budget() {
    check_template("templates/NeurIPS/neurips_paper.tex", 9);
}

#[test]
fn thesis_template_within_budget() {
    check_template("templates/thesis/thesis_skeleton.tex", 0);
}
