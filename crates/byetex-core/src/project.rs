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
    // Pre-scan for `\ref` targets so cross-file multi-label sections attach the
    // referenced alias (the `\ref` and the labelled `\section` often live in
    // different `\input`'d files).
    //
    // Scoped to the entry file's `\input` CLOSURE, not every `.tex` under
    // `base_dir`. Flat `byetex convert paper.tex` routes through here too, so a
    // whole-directory scan meant converting a paper in a directory holding other
    // LaTeX projects injected THEIR label keys as hidden anchors — and made the
    // output depend on unrelated neighbouring files, breaking the documented
    // "same input, same output" guarantee. Pointing at a DIRECTORY is an
    // explicit statement that the whole tree is one project, so
    // `plan_project_from_dir` still scans it all.
    let refs = harvest_referenced_labels_in_input_closure(main_tex, &base_dir);
    // Same closure, for macros: a paper's notation usually lives in an `\input`ed
    // `def.tex`, and an empty map here left those call sites unexpanded.
    let preseeded = harvest_macros_in_input_closure(main_tex, &base_dir);
    // `\chapter`-usage detection is keyed on `base_dir` inside `convert_with_macros`.
    let out = convert_with_macros(&source, &opts, preseeded, refs, record_source_map);

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
/// starting with `.`) and ignores `target/`/`node_modules/` build outputs.
/// Symlinked directories ARE followed (see the body for why and for the
/// cycle/size bounds).
fn walk_project_files(dir: &Path, wanted: &[&str]) -> Result<Vec<PathBuf>, ProjectError> {
    // Symlinked directories are FOLLOWED, not skipped. The old code tested
    // `file_type.is_dir()` first, and `symlink_metadata` reports a
    // symlink-to-directory as neither a dir NOR a file — so the entry fell
    // through both arms, the `is_symlink()` guard below it was unreachable dead
    // code, and the whole subtree became invisible to macro harvesting,
    // referenced-label harvesting, `\chapter` detection and entry detection
    // alike (review finding #10). A `common/` or `chapters/` symlink pointing
    // at a shared tree is a normal multi-paper layout, and `\input` resolution
    // already reads straight through such links, so the scan must agree with it.
    //
    // This walk is READ-ONLY and only ever opens `.tex`/`.sty`/`.cls`. Copying
    // files out of the tree stays blocked by `materialize_project`'s own
    // path-traversal guard, which is the check that actually protects the
    // output. Two bounds keep a hostile or careless link from running away:
    // `visited` (canonical paths, so cycles terminate) and `MAX_DIRS`.
    const MAX_DIRS: usize = 10_000;
    let mut out = Vec::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if !visited.insert(
            current
                .canonicalize()
                .unwrap_or_else(|_| current.to_path_buf()),
        ) {
            continue;
        }
        if visited.len() > MAX_DIRS {
            break;
        }
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
            let path = entry.path();
            // Resolve the target kind through symlinks (`metadata` follows;
            // `entry.file_type()` does not). A dangling link resolves to
            // nothing and is skipped.
            let resolved = if file_type.is_symlink() {
                match std::fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                }
            } else {
                match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                }
            };
            if resolved.is_dir() {
                if matches!(name_str.as_ref(), "target" | "node_modules") {
                    continue;
                }
                stack.push(path.canonicalize().unwrap_or(path));
            } else if resolved.is_file() {
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
    if candidates.is_empty() {
        return Err(ProjectError::NoEntryFile {
            searched: dir.to_path_buf(),
        });
    }
    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }
    // More than one `\documentclass` is the NORMAL shape of a real LaTeX
    // repository: a root `main.tex` beside an `examples/`, `templates/` or
    // `doc-src/` directory that ships its own compilable sample. Failing
    // outright made those trees unconvertible (5 of the 60 corpus projects —
    // review finding #11). Prefer the shallowest candidate; that is the entry
    // file in every such layout, and a nested sample can never outrank it.
    let depth = |p: &Path| p.strip_prefix(dir).unwrap_or(p).components().count();
    let min_depth = candidates.iter().map(|p| depth(p)).min().unwrap_or(0);
    let mut shallowest: Vec<PathBuf> = candidates
        .iter()
        .filter(|p| depth(p) == min_depth)
        .cloned()
        .collect();
    if shallowest.len() == 1 {
        return Ok(shallowest.remove(0));
    }
    // Same-depth tie: `main.tex` is the near-universal convention.
    let mut named_main: Vec<PathBuf> = shallowest
        .iter()
        .filter(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("main"))
        })
        .cloned()
        .collect();
    if named_main.len() == 1 {
        return Ok(named_main.remove(0));
    }
    // Genuinely ambiguous — report every candidate so the caller can choose.
    Err(ProjectError::AmbiguousEntryFile { candidates })
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

/// Harvest macro definitions from the entry file's `\input` CLOSURE.
///
/// The mirror of [`harvest_referenced_labels_in_input_closure`], and closure-
/// scoped for the same reason: `plan_project` must not scan the whole directory,
/// because flat `byetex convert paper.tex` routes through it and would then pull
/// in a neighbouring paper's macros.
///
/// Without this, `plan_project` passed an EMPTY macro map, so a definition living
/// in an `\input`ed `def.tex` reached the emitter only if the `\input` expansion
/// happened to harvest it. `\newcommand` did; `\newcommandx` did not — and a
/// paper whose notation lives in `def.tex` had every call site collapse to a bare
/// identifier, losing all arguments with no warning and no compile error (818
/// call sites on corpus 2605.22765: `\pdata`, `\fw`, `\denoiser`, ...).
fn harvest_macros_in_input_closure(entry: &Path, base_dir: &Path) -> HashMap<String, MacroDef> {
    let mut macros: HashMap<String, MacroDef> = HashMap::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut queue: Vec<PathBuf> = vec![entry.to_path_buf()];
    while let Some(path) = queue.pop() {
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(key) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        // On a cross-file name collision the traversal order decides, which is
        // stack order, not document order. `harvest_project_macros` has the same
        // property (directory-walk order). Collisions are vanishingly rare, and
        // the emitter's own in-document definitions override either way.
        for (k, v) in crate::emit::harvest_macros_from_source(&source) {
            macros.insert(k, v);
        }
        for raw in crate::emit::extract_include_paths_from_source(&source) {
            let from_here = path.parent().unwrap_or(base_dir);
            if let Some(p) = crate::emit::resolve_include_path(from_here, &raw)
                .or_else(|| crate::emit::resolve_include_path(base_dir, &raw))
            {
                queue.push(p);
            }
        }
    }
    macros
}

/// Collect referenced labels from `entry` and every file reachable from it via
/// `\input`/`\include`, transitively. Cycle-safe; unreadable or unresolvable
/// includes are skipped silently.
///
/// This is the single-file counterpart to [`harvest_project_referenced_labels`]:
/// same purpose, but the reachable set is what the document actually pulls in
/// rather than whatever else happens to share a directory with it.
fn harvest_referenced_labels_in_input_closure(entry: &Path, base_dir: &Path) -> HashSet<String> {
    let mut refs: HashSet<String> = HashSet::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut queue: Vec<PathBuf> = vec![entry.to_path_buf()];
    while let Some(path) = queue.pop() {
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(key) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        refs.extend(crate::emit::harvest_referenced_labels_from_source(&source));
        for raw in crate::emit::extract_include_paths_from_source(&source) {
            let from_here = path.parent().unwrap_or(base_dir);
            if let Some(p) = crate::emit::resolve_include_path(from_here, &raw)
                .or_else(|| crate::emit::resolve_include_path(base_dir, &raw))
            {
                queue.push(p);
            }
        }
    }
    refs
}

/// True if ANY `.tex` in the project uses `\chapter` — the include-aware signal for
/// chapter-bearing layout (health-check P1). A thesis often declares `\documentclass`
/// in an entry file whose chapters live in `\input`'d sub-files; the entry file's
/// single-file prepass never sees them, so the class would otherwise be misjudged as an
/// article (flat headings, dropped ToC / front-matter page numbering).
pub(crate) fn harvest_project_uses_chapter(dir: &Path) -> bool {
    let Ok(files) = walk_project_files(dir, &["tex"]) else {
        return false;
    };
    files.iter().any(|path| {
        std::fs::read_to_string(path)
            .map(|s| crate::emit::harvest_uses_chapter_from_source(&s))
            .unwrap_or(false)
    })
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
    // `\chapter`-usage detection is keyed on `base_dir` inside `convert_with_macros`.
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
    // The same (source, destination) pair can be registered twice — e.g. a
    // document that calls `\bibliography{refs}` more than once. Copying it twice
    // is redundant for images, but for a `.bib` it is destructive: the second
    // `preprocess_bib_with_seen` finds every key already in `bib_seen_keys` and
    // writes an EMPTY file over the good first copy, breaking every citation
    // (review finding #3). Dedupe on the pair, so a genuine name collision
    // between two DIFFERENT sources keeps its existing behaviour.
    let mut copied: std::collections::HashSet<(PathBuf, PathBuf)> =
        std::collections::HashSet::new();
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

        if !copied.insert((canonical_src.clone(), asset.rel_dest.clone())) {
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
