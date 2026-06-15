//! Compile/render orchestration: run `typst compile` on a generated `.typ` and
//! return a structured result — a PDF compile with parsed errors
//! ([`compile_typ`]) or a per-page PNG render ([`render_typ`]). Shared by the
//! CLI `compile`/`render` commands and the MCP `compile`/`render` tools so an
//! agent never has to shell out to `typst` and scrape its stderr by hand.
//!
//! [`ensure_typ`] is the convenience front door: a `.tex` input is converted
//! (flat) to a sibling `.typ` first; a `.typ` input is used as-is.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::typst_diag::{parse_typst_errors, TypstError};
use crate::{convert, ConvertOptions};

/// Result of a `typst compile` to PDF.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CompileResult {
    /// Whether `typst compile` exited successfully.
    pub ok: bool,
    /// Structured errors parsed from typst's stderr (those carrying a
    /// `file:line:col` location). Empty on success.
    pub errors: Vec<TypstError>,
    /// Path of the produced PDF (`None` only when typst couldn't be spawned).
    pub pdf_path: Option<String>,
}

/// Result of a `typst compile` to per-page PNGs.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct RenderResult {
    /// Whether `typst compile` exited successfully.
    pub ok: bool,
    /// Structured errors parsed from typst's stderr. Empty on success.
    pub errors: Vec<TypstError>,
    /// One path per rendered page, in page order (`page-1.png`, `page-2.png`, …).
    pub image_paths: Vec<String>,
}

/// Compile `typ_path` to a PDF (`out_pdf`, or `<typ>.pdf` by default) and parse
/// typst's stderr into structured errors. `ok` reflects the process exit.
pub fn compile_typ(
    typ_path: &Path,
    out_pdf: Option<&Path>,
    typst_bin: &str,
) -> Result<CompileResult> {
    let pdf = out_pdf
        .map(Path::to_path_buf)
        .unwrap_or_else(|| typ_path.with_extension("pdf"));
    let out = std::process::Command::new(typst_bin)
        .arg("compile")
        .arg(typ_path)
        .arg(&pdf)
        .output()
        .with_context(|| format!("spawning `{}`", typst_bin))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    Ok(CompileResult {
        ok: out.status.success(),
        errors: parse_typst_errors(&stderr),
        pdf_path: Some(pdf.display().to_string()),
    })
}

/// Render `typ_path` to per-page PNGs in `out_dir` at `dpi` ppi, using typst's
/// native PNG export (the `{p}` page placeholder), so no external rasteriser is
/// needed. Returns the page image paths in numeric order.
pub fn render_typ(
    typ_path: &Path,
    out_dir: &Path,
    dpi: u32,
    typst_bin: &str,
) -> Result<RenderResult> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    // typst replaces `{p}` with the 1-based page number, producing page-1.png,
    // page-2.png, … inside out_dir.
    let template = out_dir.join("page-{p}.png");
    let out = std::process::Command::new(typst_bin)
        .arg("compile")
        .arg("--ppi")
        .arg(dpi.to_string())
        .arg(typ_path)
        .arg(&template)
        .output()
        .with_context(|| format!("spawning `{}`", typst_bin))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    Ok(RenderResult {
        ok: out.status.success(),
        errors: parse_typst_errors(&stderr),
        image_paths: collect_page_pngs(out_dir),
    })
}

/// Collect `page-<N>.png` files in `dir`, sorted by `N`. Pure over the
/// filesystem so the page ordering is deterministic regardless of `read_dir`
/// order (and numeric, so page-10 sorts after page-2).
fn collect_page_pngs(dir: &Path) -> Vec<String> {
    let mut pages: Vec<(u32, PathBuf)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("png") {
                continue;
            }
            if let Some(n) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("page-"))
                .and_then(|n| n.parse::<u32>().ok())
            {
                pages.push((n, path));
            }
        }
    }
    pages.sort_by_key(|(n, _)| *n);
    pages
        .into_iter()
        .map(|(_, p)| p.display().to_string())
        .collect()
}

/// Front door for the compile/render commands: if `input` is a `.tex`, convert
/// it (flat) to a sibling `.typ` and return that path; if it is already a
/// `.typ`, return it unchanged. Flat conversion does not copy assets — for a
/// multi-file paper, materialise a project first (`convert --project`) and point
/// at its `main.typ`.
pub fn ensure_typ(input: &Path) -> Result<PathBuf> {
    if input.extension().and_then(|s| s.to_str()) == Some("typ") {
        return Ok(input.to_path_buf());
    }
    let source =
        std::fs::read_to_string(input).with_context(|| format!("read {}", input.display()))?;
    let result = convert(
        &source,
        &ConvertOptions {
            source_name: Some(input.display().to_string()),
            base_dir: input.parent().map(|p| p.to_path_buf()),
        },
    );
    let typ = input.with_extension("typ");
    std::fs::write(&typ, &result.typst).with_context(|| format!("write {}", typ.display()))?;
    Ok(typ)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_page_pngs_sorts_numerically_and_ignores_others() {
        let dir = tempfile::tempdir().unwrap();
        for n in ["1", "2", "10"] {
            std::fs::write(dir.path().join(format!("page-{n}.png")), b"x").unwrap();
        }
        // Non-matching files are ignored.
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        std::fs::write(dir.path().join("page-1.pdf"), b"x").unwrap();

        let pages = collect_page_pngs(dir.path());
        assert_eq!(pages.len(), 3, "got {pages:?}");
        assert!(pages[0].ends_with("page-1.png"), "{pages:?}");
        assert!(pages[1].ends_with("page-2.png"), "{pages:?}");
        // Numeric order: page-10 sorts after page-2, not lexically before it.
        assert!(pages[2].ends_with("page-10.png"), "{pages:?}");
    }
}
