//! Tests for the project-level planner (byetex_core::project::plan_project).
//! Materializer tests are in byetex-cli; these tests stay IO-light.

use std::path::PathBuf;

use byetex_core::project::plan_project;

fn fixture(rel: &str) -> PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    root.join("tests/fixtures").join(rel)
}

#[test]
fn plan_project_finds_image_and_bib_assets() {
    let main = fixture("mini-project/main.tex");
    let plan = plan_project(&main, false, false).expect("plan_project failed");

    // intro.tex is inlined — NOT an asset copy.
    // fig/a.pdf and refs.bib ARE asset copies.
    assert_eq!(
        plan.assets.len(),
        2,
        "expected 2 assets (image + bib), got {}: {:?}",
        plan.assets.len(),
        plan.assets
    );

    let has_image = plan
        .assets
        .iter()
        .any(|a| a.rel_dest.to_string_lossy().contains("a.pdf"));
    let has_bib = plan
        .assets
        .iter()
        .any(|a| a.rel_dest.to_string_lossy().ends_with("refs.bib"));
    assert!(has_image, "missing fig/a.pdf asset: {:?}", plan.assets);
    assert!(has_bib, "missing refs.bib asset: {:?}", plan.assets);
}

#[test]
fn plan_project_no_toml_suppresses_manifest() {
    let main = fixture("mini-project/main.tex");
    let plan = plan_project(&main, true, false).expect("plan_project failed");
    assert!(
        plan.manifest.is_none(),
        "manifest should be None when no_toml=true"
    );
}

#[test]
fn plan_project_typst_body_is_non_empty() {
    let main = fixture("mini-project/main.tex");
    let plan = plan_project(&main, true, false).expect("plan_project failed");
    assert!(
        !plan.main_typst.is_empty(),
        "main_typst should not be empty"
    );
    // The intro text should have been inlined.
    assert!(
        plan.main_typst.contains("introduction"),
        "intro.tex content not inlined: {}",
        plan.main_typst
    );
}
