//! Project-level conversion: LaTeX project directory → Typst project directory.
//!
//! The [`plan_project`] function converts the main `.tex` file and returns a
//! [`ProjectPlan`] that describes the Typst body and the asset files (images,
//! bibliography) that need to be copied. Keeping planning and IO separate lets
//! the planner be unit-tested without touching the filesystem.
//!
//! The materializer lives in `byetex-cli` to keep IO out of the library crate.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::emit::MacroDef;
use crate::{convert_with_macros, AssetKind, AssetRef, ConvertOptions, Warning};

/// A single file that must be copied from the source project into the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetCopy {
    /// Absolute (or base-dir-relative) path of the source file.
    pub source: PathBuf,
    /// Relative destination path within the output project directory.
    /// Preserves the sub-directory layout the Typst source already references.
    pub rel_dest: PathBuf,
}

/// The result of planning a project conversion. All fields are in-memory;
/// no files are read or written by [`plan_project`] beyond loading the source.
#[derive(Debug)]
pub struct ProjectPlan {
    /// The converted Typst body (contents of `main.typ`).
    pub main_typst: String,
    /// Assets to copy into the output directory.
    pub assets: Vec<AssetCopy>,
    /// Warnings produced during conversion.
    pub warnings: Vec<Warning>,
    /// Optional `typst.toml` content for the output project.
    /// `None` when the document class does not map to a known Typst Universe
    /// package or when the caller opts out via `no_toml`.
    pub manifest: Option<String>,
    /// The `.tex` file that drove this conversion. For [`plan_project`] this
    /// is the path the caller passed in; for [`plan_project_from_dir`] it is
    /// the entry file `detect_entry_file` selected. Carried so downstream
    /// callers (e.g. the CLI's agent-brief writer) can reference the
    /// original source without re-running detection.
    pub entry_tex: PathBuf,
    /// Content-anchored provenance map for `main_typst` (`.typ` text → source
    /// span in `entry_tex`). Empty unless the planner was asked to capture it
    /// (`record_source_map`). Used by `byetex diagnose --project`.
    pub source_map: Vec<crate::source_map::NodeOutput>,
}

/// Errors that can occur during project planning.
#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
    /// No `.tex` file in the project tree carries a `\documentclass`
    /// declaration. The caller should re-check the input directory.
    NoEntryFile {
        searched: PathBuf,
    },
    /// More than one `.tex` file declares `\documentclass`. The caller
    /// has to disambiguate by passing the path to the desired entry
    /// directly instead of the directory.
    AmbiguousEntryFile {
        candidates: Vec<PathBuf>,
    },
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Io(e) => write!(f, "I/O error: {}", e),
            ProjectError::NoEntryFile { searched } => write!(
                f,
                "no `.tex` file with a `\\documentclass` declaration was found under `{}`",
                searched.display()
            ),
            ProjectError::AmbiguousEntryFile { candidates } => {
                writeln!(
                    f,
                    "multiple `.tex` files declare `\\documentclass`; pass one of these paths directly:"
                )?;
                for c in candidates {
                    writeln!(f, "  - {}", c.display())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ProjectError {
    fn from(e: std::io::Error) -> Self {
        ProjectError::Io(e)
    }
}

/// Plan the conversion of a LaTeX project rooted at `main_tex`.
///
/// Reads the source file, converts it with `base_dir = main_tex.parent()`,
/// and translates each [`AssetRef`] into an [`AssetCopy`] with the relative
/// destination matching the path already written into the Typst source.
///
/// Set `no_toml = true` to suppress `typst.toml` generation even when the
/// document class maps to a known Typst Universe package.
pub fn plan_project(
    main_tex: &Path,
    no_toml: bool,
    record_source_map: bool,
) -> Result<ProjectPlan, ProjectError> {
    let source = std::fs::read_to_string(main_tex)?;
    let base_dir = main_tex
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("."));

    let opts = ConvertOptions {
        source_name: Some(main_tex.display().to_string()),
        base_dir: Some(base_dir.clone()),
    };
    // Pre-scan sibling files for `\ref` targets so cross-file multi-label
    // sections attach the referenced alias (the `\ref` and the labelled
    // `\section` often live in different `\input`'d files).
    let refs = harvest_project_referenced_labels(&base_dir).unwrap_or_default();
    let out = convert_with_macros(&source, &opts, HashMap::new(), refs, record_source_map);

    let assets = out
        .asset_refs
        .iter()
        .map(|r| asset_ref_to_copy(r, &base_dir))
        .collect();

    let manifest = if no_toml {
        None
    } else {
        derive_manifest(&out.typst)
    };

    Ok(ProjectPlan {
        main_typst: out.typst,
        assets,
        warnings: out.warnings,
        manifest,
        entry_tex: main_tex.to_path_buf(),
        source_map: out.source_map,
    })
}

/// Convert an [`AssetRef`] into an [`AssetCopy`].
///
/// The relative destination mirrors the `typst_path` that the emitter wrote
/// into the Typst source. This preserves the sub-directory layout so that
/// `image("fig/foo.pdf")` keeps working after the project is materialised.
///
/// `source_path` in [`AssetRef`] is the path returned by the probe helpers,
/// which is already `base_dir.join(asset_stem[.ext])` — i.e., it already
/// contains the base-dir prefix. We use it as-is (absolute when possible,
/// otherwise as the probe returned it).
fn asset_ref_to_copy(r: &AssetRef, _base_dir: &Path) -> AssetCopy {
    let rel_dest = match r.kind {
        AssetKind::Image | AssetKind::Bibliography => PathBuf::from(&r.typst_path),
    };
    // Canonicalise if possible so downstream path-traversal checks are reliable.
    let source = r
        .source_path
        .canonicalize()
        .unwrap_or_else(|_| r.source_path.clone());
    AssetCopy { source, rel_dest }
}

/// Whether the generated Typst source needs a `typst.toml` manifest.
///
/// ByeTex now self-generates a fully self-contained preamble (no
/// `#import "@preview/..."`), so the output never depends on a Typst Universe
/// package and never needs a manifest. Kept as a function (rather than inlining
/// `None`) so the `ProjectPlan.manifest` field and the `no_toml` switch stay
/// meaningful if a future change reintroduces a package dependency.
fn derive_manifest(_typst: &str) -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Folder-input mode
// ---------------------------------------------------------------------------
//
// Real-world LaTeX projects (arXiv tarballs, paper repos) hand you a folder,
// not a single .tex. The functions below let callers point ByeTex at that
// folder directly:
//
// - `detect_entry_file` finds the single `.tex` that carries
//   `\documentclass` (the entry point).
// - `harvest_project_macros` pre-scans every `.tex`/`.sty`/`.cls` in the
//   tree for `\newcommand`/`\def` so a macro defined in a sibling file
//   never reached via `\input` is still available at every call site.
// - `plan_project_from_dir` glues both together and runs the standard
//   `plan_project` pipeline on the detected entry.

/// Walk `dir` recursively and collect every file whose extension matches
/// one of `wanted`. Skips hidden directories (any path component
/// starting with `.`), doesn't follow symlinks (avoids escaping the
/// project tree), and ignores `target/`/`node_modules/` build outputs.
fn walk_project_files(dir: &Path, wanted: &[&str]) -> Result<Vec<PathBuf>, ProjectError> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let read = match std::fs::read_dir(&current) {
            Ok(r) => r,
            // A dir that vanished mid-walk is uninteresting, not fatal.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(ProjectError::Io(e)),
        };
        for entry in read {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Skip dotfiles and well-known build dirs.
            if name_str.starts_with('.') {
                continue;
            }
            if file_type.is_dir() {
                if matches!(name_str.as_ref(), "target" | "node_modules") {
                    continue;
                }
                if file_type.is_symlink() {
                    continue;
                }
                stack.push(entry.path());
            } else if file_type.is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if wanted.iter().any(|w| w.eq_ignore_ascii_case(ext)) {
                        out.push(path);
                    }
                }
            }
        }
    }
    out.sort(); // deterministic order across runs / OSes
    Ok(out)
}

/// True if `source` has a `\documentclass` declaration on a line that
/// isn't commented out. Tolerates leading whitespace; doesn't try to
/// reason about `\verb|...|` blocks (would be vanishingly rare).
fn source_declares_documentclass(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('%') {
            continue;
        }
        if trimmed.contains("\\documentclass") {
            return true;
        }
    }
    false
}

/// Find the single `.tex` file under `dir` that declares `\documentclass`.
///
/// Returns:
/// - `Ok(path)` when exactly one candidate is found.
/// - `Err(ProjectError::NoEntryFile)` when zero candidates exist.
/// - `Err(ProjectError::AmbiguousEntryFile)` when more than one does.
///
/// The walk is recursive but skips hidden directories and `target/`
/// build outputs. Use this when the caller wants to convert "a project
/// tree" without manually identifying the entry file.
pub fn detect_entry_file(dir: &Path) -> Result<PathBuf, ProjectError> {
    let tex_files = walk_project_files(dir, &["tex"])?;
    let mut candidates = Vec::new();
    for path in tex_files {
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue, // unreadable files don't disqualify the search
        };
        if source_declares_documentclass(&source) {
            candidates.push(path);
        }
    }
    match candidates.len() {
        0 => Err(ProjectError::NoEntryFile {
            searched: dir.to_path_buf(),
        }),
        1 => Ok(candidates.remove(0)),
        _ => Err(ProjectError::AmbiguousEntryFile { candidates }),
    }
}

/// Pre-scan every `.tex` / `.sty` / `.cls` under `dir` and merge their
/// `\newcommand` / `\def` declarations into one table. Last-write-wins
/// across files; the entry file's own definitions are NOT included here
/// (they're picked up during the main conversion walk and would over-
/// write any duplicates with their own values, which is the desired
/// "definition closest to use" semantics).
///
/// Unreadable files are skipped silently — a missing or
/// permission-denied file shouldn't sabotage the whole pre-scan.
pub(crate) fn harvest_project_macros(
    dir: &Path,
) -> Result<HashMap<String, MacroDef>, ProjectError> {
    let files = walk_project_files(dir, &["tex", "sty", "cls"])?;
    let mut merged: HashMap<String, MacroDef> = HashMap::new();
    for path in files {
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let local = crate::emit::harvest_macros_from_source(&source);
        for (k, v) in local {
            merged.insert(k, v);
        }
    }
    Ok(merged)
}

/// Pre-scan every `.tex` in the tree for labels referenced by
/// `\ref`/`\cref`/`\eqref`/... so a reference in one file informs which alias
/// a multi-`\label` section in another file should attach. Unreadable files
/// are skipped silently.
pub(crate) fn harvest_project_referenced_labels(
    dir: &Path,
) -> Result<HashSet<String>, ProjectError> {
    let files = walk_project_files(dir, &["tex"])?;
    let mut refs: HashSet<String> = HashSet::new();
    for path in files {
        if let Ok(source) = std::fs::read_to_string(&path) {
            refs.extend(crate::emit::harvest_referenced_labels_from_source(&source));
        }
    }
    Ok(refs)
}

/// Plan a conversion when the caller has a project directory rather
/// than a specific main `.tex` file.
///
/// 1. [`detect_entry_file`] picks the single `\documentclass`-bearing
///    `.tex` (errors clearly when 0 or >1 candidates exist).
/// 2. [`harvest_project_macros`] pre-scans every `.tex`/`.sty`/`.cls`
///    in the tree for `\newcommand`/`\def`. Without this step, a macro
///    defined in (say) a sibling file that the entry file never
///    `\input`s would be unknown at its call site.
/// 3. The entry file is converted with `base_dir = dir`, with the
///    harvested macros pre-seeded into the emitter.
/// 4. The returned [`ProjectPlan`] is identical in shape to
///    [`plan_project`]'s, so the same materialiser can write it out.
pub fn plan_project_from_dir(
    dir: &Path,
    no_toml: bool,
    record_source_map: bool,
) -> Result<ProjectPlan, ProjectError> {
    let entry = detect_entry_file(dir)?;
    let preseeded = harvest_project_macros(dir)?;
    let refs = harvest_project_referenced_labels(dir).unwrap_or_default();

    let source = std::fs::read_to_string(&entry)?;
    let opts = ConvertOptions {
        source_name: Some(entry.display().to_string()),
        base_dir: Some(dir.to_path_buf()),
    };
    let out = convert_with_macros(&source, &opts, preseeded, refs, record_source_map);

    let assets = out
        .asset_refs
        .iter()
        .map(|r| asset_ref_to_copy(r, dir))
        .collect();
    let manifest = if no_toml {
        None
    } else {
        derive_manifest(&out.typst)
    };
    Ok(ProjectPlan {
        main_typst: out.typst,
        assets,
        warnings: out.warnings,
        manifest,
        entry_tex: entry,
        source_map: out.source_map,
    })
}

// ---------------------------------------------------------------------------
// Materializer
// ---------------------------------------------------------------------------
//
// Writes a [`ProjectPlan`] to disk. Both the CLI and the MCP server invoke
// this function; previously each carried a near-duplicate copy. The MCP
// version silently dropped unreadable assets while the CLI warned — a real
// drift. The unified implementation here always warns (CLI behaviour),
// which is what an agent caller actually wants so it can flag broken
// includes.

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
) -> Result<(), ProjectError> {
    // Guard: refuse non-empty output dir unless --force. With --force, also
    // clean the existing directory so removed assets don't survive a re-run.
    if out_dir.exists() {
        let metadata = std::fs::metadata(out_dir)?;
        if !metadata.is_dir() {
            return Err(ProjectError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "output path `{}` exists and is not a directory",
                    out_dir.display()
                ),
            )));
        }
        let is_empty = std::fs::read_dir(out_dir)?.next().is_none();
        if !is_empty {
            if !force {
                return Err(ProjectError::Io(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!(
                        "output directory `{}` is not empty; pass force=true to overwrite",
                        out_dir.display()
                    ),
                )));
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
        ProjectError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "cannot canonicalise base directory `{}`: {}",
                base_dir.display(),
                e
            ),
        ))
    })?;

    // Copy assets. A single `seen_keys` set is shared across every `.bib`
    // file so a key defined in more than one of them (e.g. a master
    // `allbib.bib` re-listing entries from `ngbib.bib`) is emitted only once —
    // otherwise Typst's `#bibliography((a, b, c))` aborts with "duplicate
    // bibliography keys". Files are processed in `plan.assets` order, which is
    // the `\bibliography{...}` order, so the first file wins (matching BibTeX).
    let mut bib_seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    for asset in &plan.assets {
        // Path-traversal guard: skip any asset whose source escapes base_dir.
        // `asset.source` is already canonicalised by `asset_ref_to_copy`, but
        // re-canonicalise to defend against TOCTOU races where the file was
        // replaced between planning and materialisation.
        let canonical_src = match asset.source.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Always warn — silently skipping (the old MCP behaviour)
                // hides broken includes from callers that need to react.
                eprintln!(
                    "byetex: skipping asset `{}` — source path could not be canonicalised at materialise time",
                    asset.source.display()
                );
                continue;
            }
        };
        if !canonical_src.starts_with(&canonical_base) {
            eprintln!(
                "byetex: skipping asset `{}` — source path escapes base directory (path traversal guard)",
                asset.source.display()
            );
            continue;
        }

        let dest = out_dir.join(&asset.rel_dest);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // `.bib` files get preprocessed (resolve `@string` macros,
        // quote unresolved bare identifiers, normalise key whitespace)
        // so Typst's strict Hayagriva parser accepts them. See the
        // `bib` module for the rewrites. Non-`.bib` assets (images,
        // etc.) are byte-copied unchanged.
        let is_bib = asset
            .rel_dest
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("bib"));
        if is_bib {
            let raw = std::fs::read_to_string(&asset.source)?;
            let processed = crate::bib::preprocess_bib_with_seen(&raw, &mut bib_seen_keys);
            std::fs::write(&dest, processed)?;
        } else {
            std::fs::copy(&asset.source, &dest)?;
        }
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
