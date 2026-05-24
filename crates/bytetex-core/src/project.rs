//! Project-level conversion: LaTeX project directory → Typst project directory.
//!
//! The [`plan_project`] function converts the main `.tex` file and returns a
//! [`ProjectPlan`] that describes the Typst body and the asset files (images,
//! bibliography) that need to be copied. Keeping planning and IO separate lets
//! the planner be unit-tested without touching the filesystem.
//!
//! The materializer lives in `bytetex-cli` to keep IO out of the library crate.

use std::path::{Path, PathBuf};

use crate::{convert, AssetKind, AssetRef, ConvertOptions, Warning};

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
}

/// Errors that can occur during project planning.
#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectError::Io(e) => Some(e),
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
    let out = convert(&source, &opts);

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

/// Peek at the generated Typst source to detect a `@preview/...` import line
/// and, if found, build a minimal `typst.toml` manifest.
fn derive_manifest(typst: &str) -> Option<String> {
    let pkg_name = typst.lines().take(8).find_map(|l| {
        let prefix = "#import \"@preview/";
        l.find(prefix).map(|i| {
            let rest = &l[i + prefix.len()..];
            rest.split('"').next().unwrap_or("").to_string()
        })
    })?;
    // Strip the version suffix (e.g. "charged-ieee:0.1.4" → "charged-ieee").
    let name = pkg_name.split(':').next().unwrap_or(&pkg_name);
    Some(format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nentrypoint = \"main.typ\"\n",
        name
    ))
}
