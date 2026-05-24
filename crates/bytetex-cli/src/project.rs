//! Filesystem materializer for a [`ProjectPlan`].
//!
//! Takes the in-memory plan produced by [`bytetex_core::project::plan_project`]
//! and writes it to a directory on disk: the converted Typst body, all asset
//! copies, and (optionally) a `typst.toml` manifest.

use std::path::Path;

use bytetex_core::project::ProjectPlan;

/// Write the project plan to `out_dir`.
///
/// - Creates `out_dir` and any missing parent directories.
/// - Refuses to overwrite a non-empty `out_dir` unless `force` is `true`.
///   When `force` is `true` and `out_dir` already exists, its contents are
///   removed before writing so stale files from a previous run don't
///   contaminate the result.
/// - Refuses to copy any asset whose resolved source path is outside `base_dir`
///   (path-traversal guard). Such assets are skipped with a warning printed to
///   stderr. If `base_dir` itself cannot be canonicalised, the guard returns
///   an error rather than silently dropping every asset.
/// - Writes `typst.toml` iff `plan.manifest.is_some()`.
pub fn materialize_project(
    plan: &ProjectPlan,
    out_dir: &Path,
    base_dir: &Path,
    force: bool,
) -> std::io::Result<()> {
    // Guard: refuse non-empty output dir unless --force. With --force, also
    // clean the existing directory so removed assets don't survive a re-run.
    if out_dir.exists() {
        let metadata = std::fs::metadata(out_dir)?;
        if !metadata.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "output path `{}` exists and is not a directory",
                    out_dir.display()
                ),
            ));
        }
        let is_empty = std::fs::read_dir(out_dir)?.next().is_none();
        if !is_empty {
            if !force {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!(
                        "output directory `{}` is not empty; pass --force to overwrite",
                        out_dir.display()
                    ),
                ));
            }
            clean_directory_contents(out_dir)?;
        }
    }
    std::fs::create_dir_all(out_dir)?;

    // Write main.typ.
    let main_typ = out_dir.join("main.typ");
    std::fs::write(&main_typ, &plan.main_typst)?;

    // Canonicalise base_dir up front. If it can't be canonicalised the
    // path-traversal guard would degenerate into rejecting every asset
    // silently; surface the error instead.
    let canonical_base = base_dir.canonicalize().map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!(
                "cannot canonicalise base directory `{}`: {}",
                base_dir.display(),
                e
            ),
        )
    })?;

    // Copy assets.
    for asset in &plan.assets {
        // Path-traversal guard: skip any asset whose source escapes base_dir.
        // `asset.source` is already canonicalised by `asset_ref_to_copy`, but
        // re-canonicalise to defend against TOCTOU races where the file was
        // replaced between planning and materialisation.
        let canonical_src = match asset.source.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "bytetex: skipping asset `{}` — source path could not be canonicalised at materialise time",
                    asset.source.display()
                );
                continue;
            }
        };
        if !canonical_src.starts_with(&canonical_base) {
            eprintln!(
                "bytetex: skipping asset `{}` — source path escapes base directory (path traversal guard)",
                asset.source.display()
            );
            continue;
        }

        let dest = out_dir.join(&asset.rel_dest);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&asset.source, &dest)?;
    }

    // Write typst.toml if present.
    if let Some(ref manifest) = plan.manifest {
        std::fs::write(out_dir.join("typst.toml"), manifest)?;
    }

    Ok(())
}

/// Remove every entry inside `dir` without removing `dir` itself.
fn clean_directory_contents(dir: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() && !file_type.is_symlink() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}
