//! End-to-end CLI smoke test for folder input.
//!
//! Builds the `byetex` binary via cargo and runs it against a temp dir
//! that simulates an arXiv-style layout (entry .tex + a sibling .sty
//! whose macro is never `\input`ed). The expected outcome is a
//! self-contained typst-project directory with main.typ that contains
//! the expanded macro.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary_path() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests so the
    // test always picks up the freshly-built binary in target/.
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

fn tmpdir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("byetex-cli-folder-{}-{}", name, std::process::id()));
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

#[test]
fn cli_convert_project_accepts_a_directory_and_pre_scans_macros() {
    let project = tmpdir("convert-project-dir");
    write(
        &project,
        "macros.sty",
        "\\newcommand{\\brand}{ByeTex}\n",
    );
    write(
        &project,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nHello \\brand!\n\\end{document}\n",
    );

    let out_dir = project.with_extension("typst-project");
    let _ = fs::remove_dir_all(&out_dir);

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .arg("--project")
        .arg("--project-out")
        .arg(&out_dir)
        .arg("--no-toml")
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    let main_typ = out_dir.join("main.typ");
    assert!(main_typ.is_file(), "main.typ was not written");
    let body = fs::read_to_string(&main_typ).unwrap();
    assert!(
        body.contains("ByeTex"),
        "expected pre-scanned \\brand macro to expand; main.typ:\n{}",
        body
    );

    let warnings = out_dir.join("warnings.json");
    assert!(warnings.is_file(), "warnings.json was not written");
}

#[test]
fn braceless_user_macros_dont_warn() {
    // Real arXiv papers (corpus/online/arxiv/paper) heavily use
    // brace-less calls like `$\mat X$` for 1-arg `\newcommand`s. The
    // pre-scan harvests the definition from a sibling `.tex`; the
    // expander must accept the brace-less call form.
    let project = tmpdir("braceless-macros");
    write(
        &project,
        "macros.tex",
        "\\newcommand{\\mat}[1]{\\mathbf{#1}}\n\
         \\newcommand{\\rvec}[1]{\\mathbf{#1}}\n",
    );
    write(
        &project,
        "main.tex",
        "\\documentclass{article}\n\
         \\input{macros.tex}\n\
         \\begin{document}\n\
         The matrix $\\mat X$ and the vector $\\rvec y$ are scary.\n\
         \\end{document}\n",
    );

    let out_dir = project.with_extension("typst-project");
    let _ = fs::remove_dir_all(&out_dir);

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .arg("--project")
        .arg("--project-out")
        .arg(&out_dir)
        .arg("--no-toml")
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    let warnings_text = fs::read_to_string(out_dir.join("warnings.json")).unwrap();
    let warnings: serde_json::Value = serde_json::from_str(&warnings_text).unwrap();
    let custom_macro_count = warnings
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| {
            w.get("category")
                .and_then(|c| c.get("kind"))
                .and_then(|s| s.as_str())
                == Some("custom_macro")
        })
        .count();
    assert_eq!(
        custom_macro_count, 0,
        "expected zero custom_macro warnings; got {}: {}",
        custom_macro_count, warnings_text
    );

    let body = fs::read_to_string(out_dir.join("main.typ")).unwrap();
    assert!(
        body.contains("bold(X)") || body.contains("bold( X )"),
        "expected `\\mat X` to expand to bold(X); main.typ:\n{}",
        body
    );
}

#[test]
fn brief_emitted_for_file_flat() {
    let dir = tmpdir("brief-file-flat");
    let entry = dir.join("paper.tex");
    fs::write(
        &entry,
        "\\documentclass{article}\n\\begin{document}\nhi\n\\end{document}\n",
    )
    .unwrap();

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&entry)
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    let brief = entry.with_extension("agent_brief.md");
    assert!(brief.is_file(), "brief missing at {}", brief.display());
    let body = fs::read_to_string(&brief).unwrap();
    // Relative paths only — same dir, no `../`.
    assert!(body.contains("`paper.typ`"), "brief body:\n{}", body);
    assert!(body.contains("`paper.tex`"), "brief body:\n{}", body);
    assert!(body.contains("`paper_manual.typ`"), "brief body:\n{}", body);
    // Flat-mode label appears in the title.
    assert!(body.contains("(flat mode)"), "brief body:\n{}", body);
}

#[test]
fn brief_emitted_for_dir_flat() {
    let project = tmpdir("brief-dir-flat");
    write(
        &project,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nhi\n\\end{document}\n",
    );

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    let dir_name = project.file_name().unwrap().to_str().unwrap();
    let parent = project.parent().unwrap();
    let brief = parent.join(format!("{}.agent_brief.md", dir_name));
    assert!(brief.is_file(), "brief missing at {}", brief.display());
    let body = fs::read_to_string(&brief).unwrap();
    // Source `.tex` lives inside the project dir → brief references it
    // via a relative path that goes INTO the dir.
    assert!(
        body.contains(&format!("`{}/main.tex`", dir_name)),
        "brief should reference the detected entry by relative path; body:\n{}",
        body
    );
    assert!(body.contains("(flat mode)"), "brief body:\n{}", body);
}

#[test]
fn brief_emitted_for_project() {
    let project = tmpdir("brief-project");
    write(
        &project,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nhi\n\\end{document}\n",
    );

    let out_dir = project.with_extension("typst-project");
    let _ = fs::remove_dir_all(&out_dir);

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .arg("--project")
        .arg("--project-out")
        .arg(&out_dir)
        .arg("--no-toml")
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    let brief = out_dir.join("agent_brief.md");
    assert!(brief.is_file(), "brief missing at {}", brief.display());
    let body = fs::read_to_string(&brief).unwrap();
    // Inside the project dir → typst output is `main.typ` (no path prefix).
    assert!(body.contains("`main.typ`"), "brief body:\n{}", body);
    assert!(body.contains("`main_manual.typ`"), "brief body:\n{}", body);
    assert!(body.contains("(project mode)"), "brief body:\n{}", body);
}

#[test]
fn no_brief_opt_out_suppresses_emission() {
    let dir = tmpdir("brief-opt-out");
    let entry = dir.join("paper.tex");
    fs::write(
        &entry,
        "\\documentclass{article}\n\\begin{document}\nhi\n\\end{document}\n",
    )
    .unwrap();

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&entry)
        .arg("--no-brief")
        .status()
        .expect("running byetex");
    assert!(status.success());

    // Other artifacts still appear; only the brief is suppressed.
    assert!(entry.with_extension("typ").is_file());
    let brief = entry.with_extension("agent_brief.md");
    assert!(
        !brief.exists(),
        "--no-brief should suppress {}",
        brief.display()
    );
}

#[test]
fn brief_paths_are_relative_to_brief_dir() {
    let dir = tmpdir("brief-relative-paths");
    let entry = dir.join("paper.tex");
    fs::write(
        &entry,
        "\\documentclass{article}\n\\begin{document}\nhi\n\\end{document}\n",
    )
    .unwrap();

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&entry)
        .status()
        .expect("running byetex");
    assert!(status.success());

    let brief = entry.with_extension("agent_brief.md");
    let body = fs::read_to_string(&brief).unwrap();

    // All four artifact paths should appear as plain filenames (no `../`
    // or absolute-path prefix) because the brief lives in the same directory.
    for path_ref in &[
        "`paper.tex`",
        "`paper.typ`",
        "`paper.warnings.json`",
        "`paper_manual.typ`",
    ] {
        assert!(
            body.contains(path_ref),
            "expected relative path {} in brief:\n{}",
            path_ref,
            body
        );
    }

    // No line that carries a path reference (Task line or Files bullets)
    // should contain an absolute path. Unix-only: the guard uses '/'.
    #[cfg(unix)]
    for line in body.lines() {
        if line.starts_with('-') || line.starts_with("**Task") {
            assert!(
                !line.contains("`/"),
                "absolute path in brief line: {}\nfull brief:\n{}",
                line,
                body
            );
        }
    }
}

#[test]
fn brief_is_compact() {
    // The brief must not inline file bodies — a simple "hi" document should
    // produce a brief well under 4 KB. If this trips, something is inlining
    // large blobs again.
    let dir = tmpdir("brief-compact");
    let entry = dir.join("paper.tex");
    fs::write(
        &entry,
        "\\documentclass{article}\n\\begin{document}\nhi\n\\end{document}\n",
    )
    .unwrap();

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&entry)
        .status()
        .expect("running byetex");
    assert!(status.success());

    let brief = entry.with_extension("agent_brief.md");
    let size = fs::metadata(&brief)
        .unwrap_or_else(|e| panic!("stat {}: {}", brief.display(), e))
        .len();
    assert!(
        size < 4_096,
        "brief should be < 4 KB (got {} bytes); check for inlined file bodies",
        size
    );
}

#[test]
fn cli_convert_dir_without_project_writes_flat_typ() {
    let project = tmpdir("convert-flat-dir");
    write(
        &project,
        "paper.tex",
        "\\documentclass{article}\n\\begin{document}\ncontent\n\\end{document}\n",
    );

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    // Default flat output: `<dirname>.typ` next to the dir.
    let dir_name = project.file_name().unwrap().to_str().unwrap();
    let parent = project.parent().unwrap();
    let flat = parent.join(format!("{}.typ", dir_name));
    assert!(flat.is_file(), "expected flat output at {}", flat.display());
    let warnings = parent.join(format!("{}.warnings.json", dir_name));
    assert!(warnings.is_file(), "expected warnings sidecar");
}
