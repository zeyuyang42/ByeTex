//! ByeTex MCP server — stdio JSON-RPC service exposing the LaTeX → Typst
//! converter and its bundled skill catalogue to AI agents.
//!
//! Five tools are exposed:
//!
//! - `convert(tex: String, strict: bool?) -> { typst, warnings }`
//! - `convert_file(path: String, strict: bool?) -> { typst_path, warnings_path, warnings }`
//! - `convert_fragment(tex: String, context_hint: String) -> { typst, warnings }`
//!   (`context_hint` is one of `inline | block | math | math_display`; reserved
//!   for future use — currently behaves identically to `convert`.)
//! - `list_skills() -> [{name, description}]`
//! - `read_skill(name: String) -> { name, description, body }`

use std::sync::Arc;

use bytetex_core::{convert, project::plan_project, ConvertOptions};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConvertParams {
    /// Raw LaTeX source.
    pub tex: String,
    /// If true, any warning becomes an error (currently reserved for future use).
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConvertFileParams {
    /// Path to a `.tex` file. The server reads it, writes `<stem>.typ` and
    /// `<stem>.warnings.json` next to it.
    pub path: String,
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConvertFragmentParams {
    pub tex: String,
    /// `inline | block | math | math_display`. Currently informational only.
    #[serde(default)]
    pub context_hint: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadSkillParams {
    /// Name as listed by `list_skills` (matches the skill's frontmatter `name`).
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConvertProjectParams {
    /// Path to the main `.tex` entry file. Assets and `\input` files are
    /// resolved relative to this file's parent directory.
    pub main_tex: String,
    /// Directory where the Typst project will be written. Created if absent.
    pub out_dir: String,
    /// Skip writing `typst.toml` even for known document classes.
    #[serde(default)]
    pub no_toml: bool,
    /// Overwrite a non-empty `out_dir` if it already exists.
    #[serde(default)]
    pub force: bool,
}

#[derive(Clone)]
pub struct ByeTexServer {
    tool_router: ToolRouter<Self>,
    #[allow(dead_code)]
    inner: Arc<()>,
}

impl Default for ByeTexServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl ByeTexServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            inner: Arc::new(()),
        }
    }

    #[tool(description = "Convert a LaTeX source string to Typst. Returns the \
                          Typst source plus a list of warnings.")]
    async fn convert(
        &self,
        Parameters(p): Parameters<ConvertParams>,
    ) -> Result<CallToolResult, McpError> {
        let _ = p.strict;
        let out = convert(&p.tex, &ConvertOptions::default());
        let json = serde_json::json!({
            "typst": out.typst,
            "warnings": out.warnings,
        });
        Ok(CallToolResult::success(vec![Content::text(
            json.to_string(),
        )]))
    }

    #[tool(description = "Convert a .tex file on disk. Writes <stem>.typ and \
                          <stem>.warnings.json. Returns the new paths and the warnings.")]
    async fn convert_file(
        &self,
        Parameters(p): Parameters<ConvertFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let _ = p.strict;
        let path = std::path::PathBuf::from(&p.path);
        let source = std::fs::read_to_string(&path).map_err(|e| {
            McpError::internal_error(format!("read {}: {}", path.display(), e), None)
        })?;
        let opts = ConvertOptions {
            source_name: Some(p.path.clone()),
            base_dir: path.parent().map(|p| p.to_path_buf()),
        };
        let result = convert(&source, &opts);
        let typst_path = path.with_extension("typ");
        let warnings_path = typst_path.with_extension("warnings.json");
        std::fs::write(&typst_path, &result.typst).map_err(|e| {
            McpError::internal_error(format!("write {}: {}", typst_path.display(), e), None)
        })?;
        let warnings_json = serde_json::to_string_pretty(&result.warnings)
            .map_err(|e| McpError::internal_error(format!("serialize warnings: {e}"), None))?;
        std::fs::write(&warnings_path, warnings_json).map_err(|e| {
            McpError::internal_error(format!("write {}: {}", warnings_path.display(), e), None)
        })?;
        let json = serde_json::json!({
            "typst_path": typst_path.display().to_string(),
            "warnings_path": warnings_path.display().to_string(),
            "warnings": result.warnings,
        });
        Ok(CallToolResult::success(vec![Content::text(
            json.to_string(),
        )]))
    }

    #[tool(
        description = "Convert a LaTeX fragment with an optional context hint \
                          (inline | block | math | math_display). Behaves like \
                          `convert` today; the hint is reserved for future use."
    )]
    async fn convert_fragment(
        &self,
        Parameters(p): Parameters<ConvertFragmentParams>,
    ) -> Result<CallToolResult, McpError> {
        let _ = p.context_hint;
        let out = convert(&p.tex, &ConvertOptions::default());
        let json = serde_json::json!({
            "typst": out.typst,
            "warnings": out.warnings,
        });
        Ok(CallToolResult::success(vec![Content::text(
            json.to_string(),
        )]))
    }

    #[tool(description = "List all bundled skills (name + one-line description).")]
    async fn list_skills(&self) -> Result<CallToolResult, McpError> {
        let summary: Vec<_> = bytetex_core::skills::list_skills()
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "description": s.description,
                })
            })
            .collect();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::Value::Array(summary).to_string(),
        )]))
    }

    #[tool(description = "Convert a LaTeX project to a self-contained Typst project directory. \
                          Reads the main .tex file, copies all referenced assets (images, .bib \
                          files), and writes main.typ + optionally typst.toml to out_dir. \
                          Returns the list of written files and any conversion warnings.")]
    async fn convert_project(
        &self,
        Parameters(p): Parameters<ConvertProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        let main_tex = std::path::PathBuf::from(&p.main_tex);
        let out_dir = std::path::PathBuf::from(&p.out_dir);
        // `Path::parent()` of a bare filename returns Some("") — not None —
        // so the `unwrap_or` here doesn't fire. Without normalisation the
        // empty PathBuf disables the path-traversal guard in
        // materialize_project_mcp (canonicalize fails → guard becomes
        // starts_with("") which matches every path). Coerce "" → ".".
        let base_dir = main_tex
            .parent()
            .map(|d| {
                if d.as_os_str().is_empty() {
                    std::path::PathBuf::from(".")
                } else {
                    d.to_path_buf()
                }
            })
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let plan = plan_project(&main_tex, p.no_toml).map_err(|e| {
            McpError::internal_error(format!("plan_project: {}", e), None)
        })?;

        // Materialise the project (path-traversal guard included).
        let force = p.force;
        materialize_project_mcp(&plan, &out_dir, &base_dir, force).map_err(|e| {
            McpError::internal_error(format!("materialize: {}", e), None)
        })?;

        let mut written: Vec<String> = vec![out_dir.join("main.typ").display().to_string()];
        for asset in &plan.assets {
            written.push(out_dir.join(&asset.rel_dest).display().to_string());
        }
        if plan.manifest.is_some() {
            written.push(out_dir.join("typst.toml").display().to_string());
        }

        let json = serde_json::json!({
            "written_files": written,
            "warnings": plan.warnings,
        });
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    #[tool(description = "Return the full markdown body of a single skill by name.")]
    async fn read_skill(
        &self,
        Parameters(p): Parameters<ReadSkillParams>,
    ) -> Result<CallToolResult, McpError> {
        match bytetex_core::skills::read_skill(&p.name) {
            Some(s) => {
                let json = serde_json::json!({
                    "name": s.name,
                    "description": s.description,
                    "body": s.body,
                });
                Ok(CallToolResult::success(vec![Content::text(
                    json.to_string(),
                )]))
            }
            None => Err(McpError::invalid_params(
                format!("skill '{}' not found", p.name),
                None,
            )),
        }
    }
}

#[tool_handler]
impl ServerHandler for ByeTexServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "bytetex".to_string(),
                title: Some("ByeTex MCP server".to_string()),
                description: Some("LaTeX → Typst conversion with structured warnings.".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                website_url: None,
                icons: None,
            },
            instructions: Some(
                "ByeTex converts LaTeX to Typst and reports unconvertible \
                 constructs as structured warnings. Use `convert` for in-memory \
                 source, `convert_file` for paths, and `list_skills` + \
                 `read_skill` to discover how to act on warnings."
                    .to_string(),
            ),
        }
    }
}

/// Inline materializer used by the MCP `convert_project` tool.
/// Mirrors the logic in `bytetex-cli/src/project.rs` to avoid a circular dep.
fn materialize_project_mcp(
    plan: &bytetex_core::project::ProjectPlan,
    out_dir: &std::path::Path,
    base_dir: &std::path::Path,
    force: bool,
) -> std::io::Result<()> {
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
                        "output directory `{}` is not empty; pass force=true to overwrite",
                        out_dir.display()
                    ),
                ));
            }
            // Wipe stale files so a re-run with `force=true` doesn't leave
            // assets from the previous plan in the project.
            for entry in std::fs::read_dir(out_dir)? {
                let entry = entry?;
                let path = entry.path();
                let ft = entry.file_type()?;
                if ft.is_dir() && !ft.is_symlink() {
                    std::fs::remove_dir_all(&path)?;
                } else {
                    std::fs::remove_file(&path)?;
                }
            }
        }
    }
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("main.typ"), &plan.main_typst)?;
    // If base_dir cannot be canonicalised, surface the error rather than
    // letting the path-traversal guard silently drop every asset.
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
    for asset in &plan.assets {
        let canonical_src = match asset.source.canonicalize() {
            Ok(p) => p,
            Err(_) => continue, // unreadable at materialise time
        };
        if !canonical_src.starts_with(&canonical_base) {
            continue;
        }
        let dest = out_dir.join(&asset.rel_dest);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&asset.source, &dest)?;
    }
    if let Some(ref manifest) = plan.manifest {
        std::fs::write(out_dir.join("typst.toml"), manifest)?;
    }
    Ok(())
}

/// Run the server over stdio (the transport every major MCP client speaks).
/// Returns once the client disconnects or the process is signalled.
pub async fn run_stdio() -> anyhow::Result<()> {
    use rmcp::{transport::stdio, ServiceExt};
    let service = ByeTexServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
