//! Per-paper warning-budget regression test.
//!
//! For each of the 5 pinned arXiv papers, runs `plan_project` (the same code
//! path as the CLI) and asserts the warning count stays at or below the
//! recorded budget.  Budgets are intentionally tight — a regression trips this
//! test before the change can land.
//!
//! **Prerequisites**: run `python scripts/corpus_harvest.py --pinned` before
//! executing these tests.  The 5 pinned papers are:
//!   2605.22507  2605.22557  2605.22776  2605.22159  2605.22820
//!
//! Tightening a budget: lower the value once the new ceiling is achievable.

use std::path::PathBuf;

use byetex_core::project::plan_project;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/byetex-core has at least two parents")
        .to_path_buf()
}

fn check_paper(arxiv_id: &str, primary_tex: &str, budget: usize) {
    let root = workspace_root();
    let source_dir = root.join("corpus").join(arxiv_id).join("source");

    if !source_dir.is_dir() {
        panic!(
            "corpus/{arxiv_id}/source/ is missing.\n\
             Run `python scripts/corpus_harvest.py --pinned` to fetch the \
             pinned regression set.",
        );
    }

    let tex_path = source_dir.join(primary_tex);
    if !tex_path.exists() {
        panic!(
            "{} not found. The corpus may be corrupted — re-run \
             `python scripts/corpus_harvest.py --pinned`.",
            tex_path.display()
        );
    }

    let plan = plan_project(&tex_path, false)
        .unwrap_or_else(|e| panic!("plan_project({arxiv_id}/{primary_tex}): {e}"));

    let count = plan.warnings.len();
    let rel = format!("corpus/{arxiv_id}/source/{primary_tex}");
    eprintln!("{rel}: {count} warnings (budget {budget})");
    assert!(
        count <= budget,
        "{rel} produced {count} warnings (budget {budget}). \
         Either the budget needs widening with justification, or recent emitter \
         changes regressed coverage.",
    );
}

// Budgets snapshot:
//   arxiv-baseline-2026-05-27  22507:32  22557:14  22776:134  22159:478  22820:17
//   2026-05-31  22820:17→83 — tabularx/tabulary now dispatch to emit_tabular
//     (previously dropped wholesale). 22820 has 7 tabularx tables whose cells
//     were silently discarded; rendering them recovers the content (paper still
//     compiles) but surfaces pre-existing unhandled commands inside the cells:
//     60 `\path|...|` (path package, verb-like) + 13 `\linewidth`. Those are
//     tracked as a separate warning-reduction follow-up, not tabularx defects.

#[test]
fn arxiv_2605_22507_within_budget() {
    // cs.LG — multi-file \input, math-heavy (0-main.tex)
    check_paper("2605.22507", "0-main.tex", 32);
}

#[test]
fn arxiv_2605_22557_within_budget() {
    // math.NA — math-heavy (main_sinum.tex)
    check_paper("2605.22557", "main_sinum.tex", 14);
}

#[test]
fn arxiv_2605_22776_within_budget() {
    // cs.LG — single-file (main_en.tex)
    check_paper("2605.22776", "main_en.tex", 134);
}

#[test]
fn arxiv_2605_22159_within_budget() {
    // math.NA — multi-file + custom macros (GS4AGBEM.tex)
    check_paper("2605.22159", "GS4AGBEM.tex", 478);
}

#[test]
fn arxiv_2605_22820_within_budget() {
    // cs.LG — exercises PDF download path (main.tex); 7 tabularx tables now
    // rendered (see budget-snapshot note above).
    check_paper("2605.22820", "main.tex", 83);
}
