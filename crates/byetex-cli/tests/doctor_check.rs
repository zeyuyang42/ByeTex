//! End-to-end CLI tests for `byetex doctor` — the Stage-0 input-validation
//! oracle. `doctor` shells out to `tectonic` to confirm the *input* LaTeX
//! itself compiles, distinguishing "input is broken" from "ByeTex bug".
//!
//! These tests force the tectonic binary via `BYETEX_TECTONIC_BIN` so they
//! are hermetic: pointing it at a non-existent binary deterministically
//! exercises the "tectonic unavailable" path regardless of whether the dev
//! machine actually has tectonic installed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-doctor-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

/// A binary name that cannot exist on PATH, used to force the
/// "tectonic unavailable" branch deterministically.
const MISSING_BIN: &str = "byetex-tectonic-does-not-exist-xyz";

#[test]
fn doctor_skips_gracefully_when_tectonic_unavailable() {
    let dir = tmpdir("skip-when-absent");
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    let input = dir.join("main.tex");

    let out = Command::new(binary_path())
        .arg("doctor")
        .arg(&input)
        .env("BYETEX_TECTONIC_BIN", MISSING_BIN)
        .output()
        .expect("running byetex doctor");

    // Graceful skip: never a hard failure just because tectonic is missing.
    assert!(
        out.status.success(),
        "expected exit 0 when tectonic is unavailable; got {:?}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    // A doctor.json sidecar is written alongside the input.
    let sidecar = dir.join("main.doctor.json");
    assert!(
        sidecar.is_file(),
        "expected doctor.json sidecar at {}",
        sidecar.display()
    );

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&sidecar).unwrap()).expect("valid doctor.json");

    assert_eq!(
        json.get("verdict").and_then(|v| v.as_str()),
        Some("tectonic_unavailable"),
        "verdict should be tectonic_unavailable; got {json}"
    );
    // Honest unknown — never claim the input is broken when we couldn't check.
    assert!(
        json.get("input_compiles").map(|v| v.is_null()).unwrap_or(false),
        "input_compiles should be null when tectonic is unavailable; got {json}"
    );
}

/// Write an executable fake CLI tool (`tectonic` or `typst`) that answers
/// `--version` with success and, for any other (compile) invocation, emits
/// `compile_stderr` and exits with `compile_code`. Lets the doctor's
/// shell-out logic be tested without a real install. Unix-only (shell shim).
#[cfg(unix)]
fn write_fake_tool(dir: &Path, name: &str, compile_code: i32, compile_stderr: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    let script = format!(
        "#!/bin/sh\nfor a in \"$@\"; do\n  if [ \"$a\" = \"--version\" ]; then\n    echo 'fake 0.0.0'\n    exit 0\n  fi\ndone\nprintf '%s' {stderr} 1>&2\nexit {code}\n",
        stderr = shell_single_quote(compile_stderr),
        code = compile_code,
    );
    fs::write(&path, script).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(unix)]
#[test]
fn doctor_reports_ok_when_input_compiles() {
    let dir = tmpdir("input-ok");
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    let input = dir.join("main.tex");
    let fake = write_fake_tool(&dir, "fake-tectonic.sh", 0, "");

    let out = Command::new(binary_path())
        .arg("doctor")
        .arg(&input)
        .env("BYETEX_TECTONIC_BIN", &fake)
        .output()
        .expect("running byetex doctor");

    assert!(
        out.status.success(),
        "expected exit 0 when input compiles; got {:?}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("main.doctor.json")).unwrap())
            .expect("valid doctor.json");

    assert_eq!(
        json.get("verdict").and_then(|v| v.as_str()),
        Some("ok"),
        "verdict should be ok; got {json}"
    );
    assert_eq!(
        json.get("input_compiles").and_then(|v| v.as_bool()),
        Some(true),
        "input_compiles should be true; got {json}"
    );
}

#[cfg(unix)]
#[test]
fn doctor_reports_input_broken_when_compile_fails() {
    let dir = tmpdir("input-broken");
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\n\\undefinedmacro\n\\end{document}\n",
    );
    let input = dir.join("main.tex");
    let fake = write_fake_tool(
        &dir,
        "fake-tectonic.sh",
        1,
        "error: Undefined control sequence \\undefinedmacro",
    );

    let out = Command::new(binary_path())
        .arg("doctor")
        .arg(&input)
        .env("BYETEX_TECTONIC_BIN", &fake)
        .output()
        .expect("running byetex doctor");

    // Without --strict, a broken input is reported but not fatal.
    assert!(
        out.status.success(),
        "expected exit 0 for broken input without --strict; got {:?}",
        out.status
    );

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("main.doctor.json")).unwrap())
            .expect("valid doctor.json");

    assert_eq!(
        json.get("verdict").and_then(|v| v.as_str()),
        Some("input_broken"),
        "verdict should be input_broken; got {json}"
    );
    assert_eq!(
        json.get("input_compiles").and_then(|v| v.as_bool()),
        Some(false),
        "input_compiles should be false; got {json}"
    );
    let excerpt = json
        .get("tectonic_log_excerpt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        excerpt.contains("Undefined control sequence"),
        "log excerpt should capture the tectonic error; got {excerpt:?}"
    );
}

#[cfg(unix)]
#[test]
fn doctor_strict_exits_2_on_broken_input() {
    let dir = tmpdir("strict-broken");
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\n\\undefinedmacro\n\\end{document}\n",
    );
    let input = dir.join("main.tex");
    let fake = write_fake_tool(&dir, "fake-tectonic.sh", 1, "error: Undefined control sequence");

    let out = Command::new(binary_path())
        .arg("doctor")
        .arg(&input)
        .arg("--strict")
        .env("BYETEX_TECTONIC_BIN", &fake)
        .output()
        .expect("running byetex doctor");

    assert_eq!(
        out.status.code(),
        Some(2),
        "expected exit code 2 for broken input under --strict; got {:?}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    // The sidecar is still written even when --strict makes it fatal.
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("main.doctor.json")).unwrap())
            .expect("valid doctor.json");
    assert_eq!(
        json.get("verdict").and_then(|v| v.as_str()),
        Some("input_broken"),
        "verdict should still be input_broken under --strict; got {json}"
    );
}

#[cfg(unix)]
#[test]
fn doctor_full_reports_byetex_bug_and_exits_3_when_typst_fails() {
    // Input compiles fine under Tectonic, but `typst compile` of ByeTex's
    // generated output fails → the bug is ours, not the input's.
    let dir = tmpdir("full-byetex-bug");
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    let input = dir.join("main.tex");
    let fake_tectonic = write_fake_tool(&dir, "fake-tectonic.sh", 0, ""); // input OK
    let fake_typst = write_fake_tool(&dir, "fake-typst.sh", 1, "error: typst boom"); // our output fails

    let out = Command::new(binary_path())
        .arg("doctor")
        .arg(&input)
        .arg("--full")
        .env("BYETEX_TECTONIC_BIN", &fake_tectonic)
        .env("BYETEX_TYPST_BIN", &fake_typst)
        .output()
        .expect("running byetex doctor --full");

    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit code 3 (byetex bug) under --full; got {:?}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("main.doctor.json")).unwrap())
            .expect("valid doctor.json");
    assert_eq!(
        json.get("verdict").and_then(|v| v.as_str()),
        Some("byetex_bug"),
        "verdict should be byetex_bug; got {json}"
    );
    assert_eq!(
        json.get("input_compiles").and_then(|v| v.as_bool()),
        Some(true),
        "input_compiles should be true; got {json}"
    );
    assert_eq!(
        json.get("byetex_typst_compiles").and_then(|v| v.as_bool()),
        Some(false),
        "byetex_typst_compiles should be false; got {json}"
    );
}

#[cfg(unix)]
#[test]
fn doctor_full_reports_ok_when_both_compile() {
    let dir = tmpdir("full-all-ok");
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    let input = dir.join("main.tex");
    let fake_tectonic = write_fake_tool(&dir, "fake-tectonic.sh", 0, "");
    let fake_typst = write_fake_tool(&dir, "fake-typst.sh", 0, "");

    let out = Command::new(binary_path())
        .arg("doctor")
        .arg(&input)
        .arg("--full")
        .env("BYETEX_TECTONIC_BIN", &fake_tectonic)
        .env("BYETEX_TYPST_BIN", &fake_typst)
        .output()
        .expect("running byetex doctor --full");

    assert!(
        out.status.success(),
        "expected exit 0 when both compile; got {:?}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("main.doctor.json")).unwrap())
            .expect("valid doctor.json");
    assert_eq!(
        json.get("verdict").and_then(|v| v.as_str()),
        Some("ok"),
        "verdict should be ok; got {json}"
    );
    assert_eq!(
        json.get("byetex_typst_compiles").and_then(|v| v.as_bool()),
        Some(true),
        "byetex_typst_compiles should be true; got {json}"
    );
}
