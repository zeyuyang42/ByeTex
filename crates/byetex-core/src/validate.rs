//! Stage-0 input validation (the `doctor` oracle): compile the *input* LaTeX
//! with tectonic to tell "the source itself is broken" apart from "ByeTex
//! produced output that won't compile".
//!
//! Mirrors [`crate::diagnose`]: the verdict logic ([`compute_verdict`]) and the
//! log extraction ([`tectonic_log_excerpt`]) are pure and unit-testable without
//! either binary, while the orchestrator ([`run_doctor`]) owns the
//! `tectonic`/`typst` shell-outs so the CLI `doctor` command and the MCP
//! `validate` tool share one code path and return the same [`DoctorReport`].

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{convert, ConvertOptions};

/// Attribution verdict. Serialises to the snake_case strings used in the
/// `<stem>.doctor.json` sidecar and the MCP `validate` result.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Input compiles and (if checked) ByeTex's output compiles too.
    Ok,
    /// The input LaTeX itself fails to compile — not ByeTex's fault.
    InputBroken,
    /// Input compiles but ByeTex's generated Typst fails to compile — our bug.
    ByetexBug,
    /// Tectonic isn't on PATH, so the input couldn't be checked.
    TectonicUnavailable,
}

/// The `<stem>.doctor.json` sidecar shape (field order is the public contract)
/// and the MCP `validate` tool result.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct DoctorReport {
    /// Whether the input compiled under tectonic. `None` when tectonic was
    /// unavailable (honest unknown — never claim broken when we couldn't check).
    pub input_compiles: Option<bool>,
    /// First few error-ish lines of tectonic's stderr (`None` when the input
    /// compiled or tectonic was unavailable).
    pub tectonic_log_excerpt: Option<String>,
    /// Whether ByeTex's own output compiled under typst. `None` unless the
    /// `--full` check ran (which requires the input to compile first).
    pub byetex_typst_compiles: Option<bool>,
    /// The attribution verdict.
    pub verdict: Verdict,
}

/// Pure verdict from the two compile signals.
///
/// `input_compiles == None` means tectonic couldn't be run; `output_compiles
/// == None` means the ByeTex-output check wasn't performed (no `--full`, or the
/// input was already broken so the check was skipped).
pub fn compute_verdict(input_compiles: Option<bool>, output_compiles: Option<bool>) -> Verdict {
    match input_compiles {
        None => Verdict::TectonicUnavailable,
        Some(false) => Verdict::InputBroken,
        Some(true) => match output_compiles {
            Some(false) => Verdict::ByetexBug,
            _ => Verdict::Ok,
        },
    }
}

/// Pure extraction of the most relevant tectonic stderr lines: the first ~10
/// error-ish lines, else the first 10 lines verbatim. `None` when the input
/// compiled (there's nothing to explain).
pub fn tectonic_log_excerpt(stderr: &str, input_compiles: bool) -> Option<String> {
    if input_compiles {
        return None;
    }
    // Keep only the first few error/warning lines — enough to see why, not the
    // whole engine transcript. Fall back to the first 10 raw lines if nothing
    // matched the error markers.
    let lines: Vec<&str> = stderr
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("error:") || t.starts_with("! ") || t.contains("error")
        })
        .take(10)
        .collect();
    let joined = if lines.is_empty() {
        stderr.lines().take(10).collect::<Vec<_>>().join("\n")
    } else {
        lines.join("\n")
    };
    Some(joined)
}

/// Whether `bin` responds to `--version` — the availability probe.
fn tool_available(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Base directory for `input`: its parent, or `.` for a bare filename.
fn parent_or_dot(input: &Path) -> PathBuf {
    input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Convert `input` and `typst compile` the result in a scratch dir anchored in
/// the input's own directory (removed on drop). Returns whether the generated
/// Typst compiles — the signal that separates `input_broken` from `byetex_bug`.
fn byetex_output_compiles(input: &Path, typst_bin: &str) -> Result<bool> {
    let source =
        std::fs::read_to_string(input).with_context(|| format!("read {}", input.display()))?;
    let result = convert(
        &source,
        &ConvertOptions {
            source_name: Some(input.display().to_string()),
            base_dir: input.parent().map(|p| p.to_path_buf()),
        },
    );

    let parent = parent_or_dot(input);
    let scratch = tempfile::Builder::new()
        .prefix(".byetex-doctor-typ-")
        .tempdir_in(&parent)
        .with_context(|| format!("creating scratch dir in {}", parent.display()))?;
    let typ_path = scratch.path().join("main.typ");
    let pdf_path = scratch.path().join("main.pdf");
    std::fs::write(&typ_path, &result.typst)
        .with_context(|| format!("writing {}", typ_path.display()))?;

    let out = std::process::Command::new(typst_bin)
        .arg("compile")
        .arg(&typ_path)
        .arg(&pdf_path)
        .output()
        .with_context(|| format!("spawning `{}`", typst_bin))?;
    Ok(out.status.success())
}

/// Run the Stage-0 oracle: probe tectonic, compile the input, optionally check
/// ByeTex's own output (`full`), and return a [`DoctorReport`]. Owns the
/// `tectonic`/`typst` shell-outs; callers add presentation on top (the CLI
/// writes the sidecar + sets exit codes; the MCP `validate` tool returns the
/// report JSON directly). Binary names are injected so callers can honour
/// `BYETEX_TECTONIC_BIN` / `BYETEX_TYPST_BIN` and tests can force either path.
pub fn run_doctor(
    input: &Path,
    full: bool,
    tectonic_bin: &str,
    typst_bin: &str,
) -> Result<DoctorReport> {
    if !tool_available(tectonic_bin) {
        return Ok(DoctorReport {
            input_compiles: None,
            tectonic_log_excerpt: None,
            byetex_typst_compiles: None,
            verdict: Verdict::TectonicUnavailable,
        });
    }

    // Anchor tectonic's scratch output inside the input's own directory so
    // nothing lands in the system temp dir; the TempDir is removed on drop.
    let scratch_parent = parent_or_dot(input);
    let scratch = tempfile::Builder::new()
        .prefix(".byetex-doctor-")
        .tempdir_in(&scratch_parent)
        .with_context(|| format!("creating scratch dir in {}", scratch_parent.display()))?;
    let out = std::process::Command::new(tectonic_bin)
        .arg("--outdir")
        .arg(scratch.path())
        .arg(input)
        .output()
        .with_context(|| format!("spawning `{}`", tectonic_bin))?;
    drop(scratch);

    let input_compiles = out.status.success();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let tectonic_log_excerpt = tectonic_log_excerpt(&stderr, input_compiles);

    // Only worth checking our own output when the input itself is valid.
    let byetex_typst_compiles = if full && input_compiles {
        Some(byetex_output_compiles(input, typst_bin)?)
    } else {
        None
    };

    let verdict = compute_verdict(Some(input_compiles), byetex_typst_compiles);

    Ok(DoctorReport {
        input_compiles: Some(input_compiles),
        tectonic_log_excerpt,
        byetex_typst_compiles,
        verdict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_tectonic_unavailable_when_input_unknown() {
        assert_eq!(compute_verdict(None, None), Verdict::TectonicUnavailable);
        // Output signal is irrelevant when we never ran tectonic.
        assert_eq!(
            compute_verdict(None, Some(true)),
            Verdict::TectonicUnavailable
        );
    }

    #[test]
    fn verdict_input_broken_when_input_fails() {
        assert_eq!(compute_verdict(Some(false), None), Verdict::InputBroken);
        // Even if we somehow had an output signal, broken input dominates.
        assert_eq!(
            compute_verdict(Some(false), Some(true)),
            Verdict::InputBroken
        );
    }

    #[test]
    fn verdict_byetex_bug_when_input_ok_but_output_fails() {
        assert_eq!(compute_verdict(Some(true), Some(false)), Verdict::ByetexBug);
    }

    #[test]
    fn verdict_ok_when_input_ok_and_output_ok_or_unchecked() {
        assert_eq!(compute_verdict(Some(true), Some(true)), Verdict::Ok);
        // No --full check performed → still OK (we only know the input compiles).
        assert_eq!(compute_verdict(Some(true), None), Verdict::Ok);
    }

    #[test]
    fn excerpt_is_none_when_input_compiles() {
        assert_eq!(tectonic_log_excerpt("anything here", true), None);
    }

    #[test]
    fn excerpt_prefers_error_lines() {
        let stderr =
            "note: setup\nerror: Undefined control sequence \\foo\nblah\n! some tex error\n";
        let got = tectonic_log_excerpt(stderr, false).unwrap();
        assert!(got.contains("Undefined control sequence"), "got: {got:?}");
        assert!(got.contains("! some tex error"), "got: {got:?}");
        // A pure 'note' line with no error marker is filtered out.
        assert!(!got.contains("note: setup"), "got: {got:?}");
    }

    #[test]
    fn excerpt_falls_back_to_first_lines_when_no_error_marker() {
        let stderr = "line one\nline two\nline three\n";
        let got = tectonic_log_excerpt(stderr, false).unwrap();
        assert!(got.contains("line one"), "got: {got:?}");
    }

    #[test]
    fn report_serialises_to_sidecar_shape() {
        let report = DoctorReport {
            input_compiles: Some(true),
            tectonic_log_excerpt: None,
            byetex_typst_compiles: Some(false),
            verdict: Verdict::ByetexBug,
        };
        let v: serde_json::Value = serde_json::to_value(&report).unwrap();
        assert_eq!(v["verdict"], "byetex_bug");
        assert_eq!(v["input_compiles"], true);
        assert_eq!(v["byetex_typst_compiles"], false);
        assert!(v["tectonic_log_excerpt"].is_null());
    }

    #[test]
    fn unavailable_verdict_serialises_with_null_signals() {
        let report = DoctorReport {
            input_compiles: None,
            tectonic_log_excerpt: None,
            byetex_typst_compiles: None,
            verdict: Verdict::TectonicUnavailable,
        };
        let v: serde_json::Value = serde_json::to_value(&report).unwrap();
        assert_eq!(v["verdict"], "tectonic_unavailable");
        assert!(v["input_compiles"].is_null());
    }
}
