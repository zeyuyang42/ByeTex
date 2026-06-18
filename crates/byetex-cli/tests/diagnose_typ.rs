//! `byetex diagnose <file.typ>` diagnoses an already-edited Typst file IN PLACE:
//! it compiles the `.typ` and maps the typst errors WITHOUT re-converting from
//! LaTeX source, so an agent's manual edits survive (dogfood backlog F1 — all 3
//! agents wanted to re-scan an edited `.typ` without the materialize wiping it).
//! Gated on `typst` being available.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn typst_available() -> bool {
    Command::new(std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".into()))
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

#[test]
fn diagnose_typ_maps_errors_without_overwriting_edits() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "byetex-diagtyp-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // An "agent-edited" .typ with a UNIQUE marker the converter would never emit,
    // plus a deliberate typst error (`#undefined_var` → "unknown variable").
    let typ = dir.join("main.typ");
    let marker = "// AGENT-EDIT-MARKER-9f3a";
    fs::write(
        &typ,
        format!("{marker}\n= Title\n\nBody text.\n\n#undefined_var\n"),
    )
    .unwrap();

    let status = Command::new(bin())
        .arg("diagnose")
        .arg(&typ)
        .status()
        .unwrap();
    assert!(status.success(), "diagnose <.typ> should exit 0 even with errors");

    // 1. The edited .typ must NOT have been overwritten (no re-conversion).
    let after = fs::read_to_string(&typ).unwrap();
    assert!(
        after.contains(marker),
        "the edited .typ must be preserved in place; got:\n{after}"
    );

    // 2. The diagnostics map the typst error, line-anchored, no LaTeX fragment.
    let diag = fs::read_to_string(dir.join("main.diagnostics.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&diag).unwrap();
    let arr = v.as_array().expect("diagnostics is a JSON array");
    assert!(!arr.is_empty(), "expected a mapped typst error, got: {diag}");
    let first = &arr[0];
    assert!(first.get("message").is_some(), "has message");
    assert!(first.get("line").is_some(), "line-anchored");
    assert!(first.get("typ_region").is_some(), "has the offending .typ line");
    // An edited .typ has no LaTeX source map → these are null, not absent.
    assert!(first.get("src_fragment").unwrap().is_null(), "no LaTeX fragment for an edited .typ");
    assert!(first.get("skill_name").unwrap().is_null(), "no source-map skill for an edited .typ");

    let _ = fs::remove_dir_all(&dir);
}
