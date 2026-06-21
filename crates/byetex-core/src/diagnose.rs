//! Diagnose orchestration: convert → `typst compile` → map each typst error
//! back to its originating LaTeX source fragment + repair skill.
//!
//! The pure mapping ([`map_typst_errors`]) is unit-testable without the typst
//! binary. The orchestrators ([`diagnose_flat`], [`diagnose_project`]) compose
//! the converter, the project materialiser, and a `typst compile` shell-out so
//! both the CLI `diagnose` command and the MCP `diagnose` tool share one path
//! and return the same [`Diagnostic`] shape.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::project::{materialize_project, plan_project, plan_project_from_dir};
use crate::{
    convert_capturing_source_map, parse_typst_errors, resolve_error_at_col, ConvertOptions,
    NodeOutput, Warning,
};

/// One mapped typst compile error. Field order is the public sidecar shape
/// (`<stem>.diagnostics.json`) and the MCP `diagnose` tool result.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// The raw typst error message.
    pub message: String,
    /// 1-based line in the generated `.typ`.
    pub line: usize,
    /// 0-based column in the generated `.typ`.
    pub col: usize,
    /// The LaTeX source text that produced the failing region (`None` when the
    /// error can't be anchored — e.g. preamble or already-edited output).
    pub src_fragment: Option<String>,
    /// The offending `.typ` line text.
    pub typ_region: String,
    /// Repair skill suggested by a warning covering the same span (`None` when
    /// no warning overlaps).
    pub skill_name: Option<String>,
}

/// Map a `typst compile` stderr to [`Diagnostic`]s over a content-anchored
/// source map + the conversion warnings. Pure — no process spawning or IO.
pub fn map_typst_errors(
    stderr: &str,
    typst: &str,
    source: &str,
    source_map: &[NodeOutput],
    warnings: &[Warning],
) -> Vec<Diagnostic> {
    let typ_lines: Vec<&str> = typst.lines().collect();
    parse_typst_errors(stderr)
        .into_iter()
        .map(|e| {
            let line_text = typ_lines
                .get(e.line.saturating_sub(1))
                .copied()
                .unwrap_or("");
            let span = resolve_error_at_col(source_map, line_text, e.col);
            let src_fragment = span.map(|(a, b)| source[a..b].to_string());
            let skill_name = span.and_then(|(a, b)| {
                warnings
                    .iter()
                    .find(|w| (w.range.byte_start as usize) < b && (w.range.byte_end as usize) > a)
                    .and_then(|w| w.suggested_skill.clone())
            });
            Diagnostic {
                message: e.message,
                line: e.line,
                col: e.col,
                src_fragment,
                typ_region: line_text.to_string(),
                skill_name,
            }
        })
        .collect()
}

/// Spawn `<typst_bin> compile <typ_path> <typ_path>.pdf`, return its stderr.
/// The PDF is removed afterwards. Returns an empty string when typst can't be
/// spawned (so a missing typst yields zero diagnostics rather than an error).
pub fn compile_typ_stderr(typ_path: &Path, typst_bin: &str) -> String {
    let pdf = typ_path.with_extension("pdf");
    match std::process::Command::new(typst_bin)
        .arg("compile")
        .arg(typ_path)
        .arg(&pdf)
        .output()
    {
        Ok(o) => {
            let _ = std::fs::remove_file(&pdf);
            String::from_utf8_lossy(&o.stderr).into_owned()
        }
        Err(_) => String::new(),
    }
}

/// Diagnose an EXISTING `.typ` file in place: compile it and map the typst errors
/// WITHOUT re-converting from LaTeX source, so an agent's manual edits survive
/// (re-running the source-based diagnose would overwrite them). Because there is no
/// LaTeX source map for an edited `.typ`, each [`Diagnostic`] carries the typst
/// message + the offending `.typ` line only — `src_fragment` and `skill_name` are
/// `None`. The `.typ` is left untouched.
pub fn diagnose_typ(typ_path: &Path, typst_bin: &str) -> Result<Vec<Diagnostic>> {
    let typst = std::fs::read_to_string(typ_path)
        .with_context(|| format!("read {}", typ_path.display()))?;
    let stderr = compile_typ_stderr(typ_path, typst_bin);
    let typ_lines: Vec<&str> = typst.lines().collect();
    let mut diags: Vec<Diagnostic> = parse_typst_errors(&stderr)
        .into_iter()
        .map(|e| Diagnostic {
            typ_region: typ_lines
                .get(e.line.saturating_sub(1))
                .copied()
                .unwrap_or("")
                .to_string(),
            message: e.message,
            line: e.line,
            col: e.col,
            src_fragment: None,
            skill_name: None,
        })
        .collect();
    // Compile errors don't cover leaked LaTeX (it compiles but renders literally) —
    // append a leak scan so the in-place diagnose surfaces fidelity issues too.
    diags.extend(scan_typ_leaks(&typst));
    Ok(diags)
}

/// Scan an already-converted `.typ` for **leaked LaTeX** — un-converted commands
/// (`\textbf`, `\cite`, `\STATE`, …) and `\[..\]` markers that compile fine but render
/// as literal text. These are invisible to `typst compile` (no error) yet are exactly
/// the fidelity bugs agents hit; surfacing them is the dogfood loop's most-requested
/// `diagnose <.typ>` capability.
///
/// Pure and IO-free. Skips fenced ```` ``` ```` code blocks and `//` comment lines,
/// ignores single-char escapes (`\#` `\$` `\_` `\&` `\%`) and the Typst linebreak `\`,
/// and de-dups repeated commands per line. Each hit is a [`Diagnostic`] with no
/// `src_fragment` (the source map is gone for an edited `.typ`).
pub fn scan_typ_leaks(typst: &str) -> Vec<Diagnostic> {
    let leak = |line: usize, col: usize, line_text: &str, message: String| Diagnostic {
        message,
        line,
        col,
        src_fragment: None,
        typ_region: line_text.to_string(),
        skill_name: Some("byetex-using-warnings-json".to_string()),
    };
    let mut out = Vec::new();
    let mut in_fence = false;
    for (idx, line) in typst.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.starts_with("//") {
            continue;
        }
        let bytes = line.as_bytes();
        let mut seen: Vec<String> = Vec::new();
        let mut j = 0;
        while j < bytes.len() {
            if bytes[j] == b'\\' {
                // Escaped backslash `\\` — inside a `#raw("…")` code string the emitter
                // DOUBLES backslashes, so a LaTeX listing reads `\\textbf` etc. Those are
                // literal, correctly-rendered code, not leaks: skip both backslashes so
                // the following letters aren't mistaken for a leaked command. A real leak
                // is a SINGLE backslash (`\textbf`) in ordinary markup/math.
                if bytes.get(j + 1) == Some(&b'\\') {
                    j += 2;
                    continue;
                }
                // `\cmd` — backslash followed by 2+ ASCII letters.
                let mut k = j + 1;
                while k < bytes.len() && bytes[k].is_ascii_alphabetic() {
                    k += 1;
                }
                if k - (j + 1) >= 2 {
                    let name = line[j..k].to_string();
                    if !seen.contains(&name) {
                        let msg = format!(
                            "possible leaked LaTeX command `{name}` — renders literally in Typst; \
                             convert or remove it"
                        );
                        seen.push(name);
                        out.push(leak(idx + 1, j, line, msg));
                    }
                    j = k;
                    continue;
                }
                // `\[..\]` — escaped-bracket marker (footnote/optional-arg leak).
                if bytes.get(j + 1) == Some(&b'[') {
                    let key = "\\[".to_string();
                    if !seen.contains(&key) {
                        seen.push(key);
                        out.push(leak(
                            idx + 1,
                            j,
                            line,
                            "possible leaked LaTeX `\\[..\\]` marker — renders as literal brackets"
                                .to_string(),
                        ));
                    }
                    j += 2;
                    continue;
                }
            }
            j += 1;
        }
    }
    out
}

/// Write the conversion warnings as a `<stem>.warnings.json` sidecar next to `typ_path`
/// (always — an empty `[]` distinguishes "0 warnings" from "no file / unknown"). This is
/// what lets an agent repairing a diagnosed project (e.g. the dogfood harness, which runs
/// `byetex diagnose --project`) see silently-dropped constructs. Best-effort: a write
/// failure is ignored so it never breaks the diagnose run.
fn write_warnings_sidecar(typ_path: &Path, warnings: &[Warning]) {
    let sidecar = typ_path.with_extension("warnings.json");
    if let Ok(json) = serde_json::to_string_pretty(warnings) {
        let _ = std::fs::write(&sidecar, json);
    }
}

/// Base directory for a `.tex` file: its parent, or `.` for a bare filename.
fn base_dir_of(input: &Path) -> PathBuf {
    input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Flat single-file diagnose: convert `input` capturing the source map, write
/// the `.typ` (to `out` or `<stem>.typ`), compile it, and map the errors.
/// Returns the written `.typ` path and the diagnostics.
pub fn diagnose_flat(
    input: &Path,
    out: Option<&Path>,
    typst_bin: &str,
) -> Result<(PathBuf, Vec<Diagnostic>)> {
    let source =
        std::fs::read_to_string(input).with_context(|| format!("read {}", input.display()))?;
    let converted = convert_capturing_source_map(
        &source,
        &ConvertOptions {
            source_name: Some(input.display().to_string()),
            base_dir: Some(base_dir_of(input)),
        },
    );
    let typ_path = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension("typ"));
    std::fs::write(&typ_path, &converted.typst)
        .with_context(|| format!("write {}", typ_path.display()))?;
    write_warnings_sidecar(&typ_path, &converted.warnings);
    let stderr = compile_typ_stderr(&typ_path, typst_bin);
    let diags = map_typst_errors(
        &stderr,
        &converted.typst,
        &source,
        &converted.source_map,
        &converted.warnings,
    );
    Ok((typ_path, diags))
}

/// Project diagnose: materialise a self-contained Typst project (assets, `.bib`,
/// `main.typ`) from `input` (a `.tex` entry file or a project directory) into
/// `out_dir`, then compile + map `main.typ`. Returns the `main.typ` path and the
/// diagnostics.
pub fn diagnose_project(
    input: &Path,
    out_dir: Option<&Path>,
    typst_bin: &str,
) -> Result<(PathBuf, Vec<Diagnostic>)> {
    let input_is_dir = input.is_dir();
    // no_toml=true (a diagnostics run doesn't need typst.toml); capture the map.
    let plan = if input_is_dir {
        plan_project_from_dir(input, true, true)
            .with_context(|| format!("planning project from {}", input.display()))?
    } else {
        plan_project(input, true, true)
            .with_context(|| format!("planning project from {}", input.display()))?
    };
    let base_dir = if input_is_dir {
        input.to_path_buf()
    } else {
        base_dir_of(&plan.entry_tex)
    };
    let out_dir = out_dir.map(Path::to_path_buf).unwrap_or_else(|| {
        let stem = plan
            .entry_tex
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("project");
        plan.entry_tex
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{stem}.typst-project"))
    });
    materialize_project(&plan, &out_dir, &base_dir, true)
        .with_context(|| format!("materialising project to {}", out_dir.display()))?;

    let main_typ = out_dir.join("main.typ");
    write_warnings_sidecar(&main_typ, &plan.warnings);
    let source = std::fs::read_to_string(&plan.entry_tex)
        .with_context(|| format!("read {}", plan.entry_tex.display()))?;
    let stderr = compile_typ_stderr(&main_typ, typst_bin);
    let diags = map_typst_errors(
        &stderr,
        &plan.main_typst,
        &source,
        &plan.source_map,
        &plan.warnings,
    );
    Ok((main_typ, diags))
}
