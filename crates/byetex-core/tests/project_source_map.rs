//! plan_project captures the content-anchored source map only when asked.

use std::fs;

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("byetex-projsm-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn plan_project_captures_source_map_when_requested() {
    let d = tmp("cap");
    let main = d.join("main.tex");
    fs::write(
        &main,
        "\\documentclass{article}\\begin{document}Hello world.\\end{document}",
    )
    .unwrap();

    let off = byetex_core::project::plan_project(&main, true, false).unwrap();
    assert!(off.source_map.is_empty(), "no capture by default");

    let on = byetex_core::project::plan_project(&main, true, true).unwrap();
    assert!(!on.source_map.is_empty(), "capture when requested");
    // Same Typst either way (capture is gated, output unchanged).
    assert_eq!(off.main_typst, on.main_typst);

    let _ = fs::remove_dir_all(&d);
}
