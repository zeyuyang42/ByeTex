//! `diagnose_flat`/`diagnose_project` must write a `<stem>.warnings.json` sidecar next
//! to the `.typ` — otherwise an agent repairing a sandbox (e.g. the dogfood harness, which
//! runs `byetex diagnose --project`) is blind to silently-dropped constructs (round-4 R2).
//! Not gated on typst: the sidecar write doesn't depend on a compile.

use std::fs;
use std::path::PathBuf;

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-diagws-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn diagnose_flat_writes_warnings_sidecar() {
    let dir = tmpdir("flat");
    let tex = dir.join("paper.tex");
    // `\tableofcontents` (non-beamer) emits a warning ("Typst equivalents not yet emitted").
    fs::write(
        &tex,
        "\\documentclass{article}\n\\begin{document}\n\\tableofcontents\nBody.\n\\end{document}\n",
    )
    .unwrap();

    let (typ_path, _diags) =
        byetex_core::diagnose::diagnose_flat(&tex, None, "typst").expect("diagnose_flat");

    let sidecar = typ_path.with_extension("warnings.json");
    assert!(sidecar.exists(), "a warnings.json sidecar must be written next to the .typ");
    let txt = fs::read_to_string(&sidecar).unwrap();
    let arr: serde_json::Value = serde_json::from_str(&txt).expect("valid JSON array");
    assert!(arr.is_array(), "sidecar is a JSON array; got {txt}");
    assert!(
        txt.contains("tableofcontents"),
        "the \\tableofcontents warning is recorded; got {txt}"
    );
    let _ = fs::remove_dir_all(&dir);
}
