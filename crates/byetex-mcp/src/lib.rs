//! ByeTex MCP server — stdio JSON-RPC service exposing the LaTeX → Typst
//! converter and its bundled skill catalogue to AI agents.
//!
//! Eleven tools are exposed:
//!
//! - `convert(tex: String, strict: bool?) -> { typst, warnings }`
//! - `convert_file(path: String, strict: bool?) -> { typst_path, warnings_path, warnings }`
//! - `convert_fragment(tex: String, context_hint: String) -> { typst, warnings }`
//!   (`context_hint` ∈ `inline | block | math | math_display`; math hints wrap the
//!   fragment so e.g. `\frac{1}{2}` converts as math, not an unknown text command.)
//! - `convert_project(main_tex: String, out_dir: String, …) -> { written_files, warnings }`
//! - `diagnose(path: String, project: bool?) -> [{ message, line, col, src_fragment, typ_region, skill_name }]`
//!   (converts + `typst compile`s and maps each error back to its LaTeX fragment + repair skill.)
//! - `validate(path: String, full: bool?) -> { input_compiles, tectonic_log_excerpt, byetex_typst_compiles, verdict }`
//!   (Stage-0 oracle: compiles the *input* with tectonic to tell a broken source from a ByeTex bug.)
//! - `compile(path: String, out_pdf: String?) -> { ok, errors:[{message,line,col}], pdf_path }`
//!   (runs `typst compile` on a `.typ`/`.tex` and returns parsed errors — no raw shell-out needed.)
//! - `render(path: String, dpi: u32?, out_dir: String?) -> { ok, errors, image_paths }`
//!   (renders the output to per-page PNGs for visual inspection / fidelity grading.)
//! - `explain(tex: String) -> [{ src_fragment, typst_output, src_start, src_end }]`
//!   (per-node LaTeX→Typst mapping — "why did this LaTeX emit this Typst?".)
//! - `list_skills() -> [{name, description}]`
//! - `read_skill(name: String) -> { name, description, body }`

use std::sync::Arc;

use byetex_core::{convert, project::plan_project, ConvertOptions};
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
    /// `inline | block | math | math_display`. Math hints wrap the fragment in
    /// `$…$` / `\[…\]` before converting; unknown/empty defaults to `inline`.
    #[serde(default)]
    pub context_hint: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExplainParams {
    /// Raw LaTeX source to explain node-by-node.
    pub tex: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadSkillParams {
    /// Name as listed by `list_skills` (matches the skill's frontmatter `name`).
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiagnoseParams {
    /// Path to a `.tex` file, or a project directory / entry `.tex`.
    pub path: String,
    /// Materialise a self-contained Typst project before compiling (implied when
    /// `path` is a directory).
    #[serde(default)]
    pub project: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidateParams {
    /// Path to a `.tex` file whose *input* validity should be checked.
    pub path: String,
    /// Also check that ByeTex's own output compiles, separating `input_broken`
    /// from `byetex_bug`. Defaults to true.
    #[serde(default = "default_true")]
    pub full: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CompileParams {
    /// Path to a `.typ` file (or a `.tex`, which is converted flat first).
    pub path: String,
    /// Output PDF path. Defaults to `<input>.pdf`.
    #[serde(default)]
    pub out_pdf: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RenderParams {
    /// Path to a `.typ` file (or a `.tex`, which is converted flat first).
    pub path: String,
    /// Pixels-per-inch for the PNG export. Defaults to 144.
    #[serde(default = "default_dpi")]
    pub dpi: u32,
    /// Output directory for the page PNGs. Defaults to `<input-stem>.pages/`.
    #[serde(default)]
    pub out_dir: Option<String>,
}

fn default_dpi() -> u32 {
    144
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
        description = "Convert a LaTeX fragment with a context hint \
                          (inline | block | math | math_display). Math hints wrap the \
                          fragment so bare math like `\\frac{1}{2}` converts as Typst math \
                          rather than an unknown text command. Returns { typst, warnings }."
    )]
    async fn convert_fragment(
        &self,
        Parameters(p): Parameters<ConvertFragmentParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = byetex_core::snippet::FragmentContext::parse(&p.context_hint);
        let out = byetex_core::snippet::convert_fragment(&p.tex, ctx, &ConvertOptions::default());
        let json = serde_json::json!({
            "typst": out.typst,
            "warnings": out.warnings,
        });
        Ok(CallToolResult::success(vec![Content::text(
            json.to_string(),
        )]))
    }

    #[tool(
        description = "Explain a conversion node-by-node: returns an array of \
                          { src_fragment, typst_output, src_start, src_end } mapping each \
                          LaTeX source fragment to the Typst it produced. Answers \
                          'why did this LaTeX emit this Typst?' for reproducible debugging."
    )]
    async fn explain(
        &self,
        Parameters(p): Parameters<ExplainParams>,
    ) -> Result<CallToolResult, McpError> {
        let ex = byetex_core::snippet::explain(&p.tex, &ConvertOptions::default());
        let json = serde_json::to_string(&ex)
            .map_err(|e| McpError::internal_error(format!("serialize explanations: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List all bundled skills (name + one-line description).")]
    async fn list_skills(&self) -> Result<CallToolResult, McpError> {
        let summary: Vec<_> = byetex_core::skills::list_skills()
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

    #[tool(
        description = "Convert a LaTeX project to a self-contained Typst project directory. \
                          Reads the main .tex file, copies all referenced assets (images, .bib \
                          files), and writes main.typ + optionally typst.toml to out_dir. \
                          Returns the list of written files and any conversion warnings."
    )]
    async fn convert_project(
        &self,
        Parameters(p): Parameters<ConvertProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        let main_tex = std::path::PathBuf::from(&p.main_tex);
        let out_dir = std::path::PathBuf::from(&p.out_dir);
        // `Path::parent()` of a bare filename returns Some("") — not None —
        // so the `unwrap_or` here doesn't fire. Without normalisation the
        // empty PathBuf disables the path-traversal guard in
        // materialize_project (canonicalize fails → guard becomes
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

        let plan = plan_project(&main_tex, p.no_toml, false)
            .map_err(|e| McpError::internal_error(format!("plan_project: {}", e), None))?;

        // Materialise the project (path-traversal guard included).
        let force = p.force;
        byetex_core::project::materialize_project(&plan, &out_dir, &base_dir, force)
            .map_err(|e| McpError::internal_error(format!("materialize: {}", e), None))?;

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
        Ok(CallToolResult::success(vec![Content::text(
            json.to_string(),
        )]))
    }

    #[tool(
        description = "Convert a .tex file (or project) and `typst compile` it, returning an \
                          array of {message, line, col, src_fragment, typ_region, skill_name} \
                          diagnostics that map each typst error back to its LaTeX source fragment \
                          and repair skill. Writes <stem>.typ (or a materialised project dir) as a \
                          side effect. Set `project: true` (or pass a directory) for multi-file papers."
    )]
    async fn diagnose(
        &self,
        Parameters(p): Parameters<DiagnoseParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = std::path::PathBuf::from(&p.path);
        let typst = std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".to_string());
        let (_out, diags) = if p.project || path.is_dir() {
            byetex_core::diagnose::diagnose_project(&path, None, &typst)
        } else {
            byetex_core::diagnose::diagnose_flat(&path, None, &typst)
        }
        .map_err(|e| {
            McpError::internal_error(format!("diagnose {}: {}", path.display(), e), None)
        })?;
        let json = serde_json::to_string(&diags)
            .map_err(|e| McpError::internal_error(format!("serialize diagnostics: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Stage-0 input oracle: compile the *input* LaTeX with tectonic and report \
                          whether the source itself is valid. Returns {input_compiles, \
                          tectonic_log_excerpt, byetex_typst_compiles, verdict}, where verdict is one \
                          of ok | input_broken | byetex_bug | tectonic_unavailable. Use this BEFORE \
                          repairing a conversion to tell a broken source apart from a ByeTex bug. \
                          Requires `tectonic` on PATH (verdict is `tectonic_unavailable` otherwise)."
    )]
    async fn validate(
        &self,
        Parameters(p): Parameters<ValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = std::path::PathBuf::from(&p.path);
        let tectonic =
            std::env::var("BYETEX_TECTONIC_BIN").unwrap_or_else(|_| "tectonic".to_string());
        let typst = std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".to_string());
        let report =
            byetex_core::validate::run_doctor(&path, p.full, &tectonic, &typst).map_err(|e| {
                McpError::internal_error(format!("validate {}: {}", path.display(), e), None)
            })?;
        let json = serde_json::to_string(&report)
            .map_err(|e| McpError::internal_error(format!("serialize report: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Compile a generated Typst file to PDF and return STRUCTURED typst errors. \
                          `path` is a `.typ` (or a `.tex`, converted flat first). Returns \
                          {ok, errors:[{message,line,col}], pdf_path}. Prefer this over shelling \
                          out to `typst` so compile errors come back parsed and mapped to lines."
    )]
    async fn compile(
        &self,
        Parameters(p): Parameters<CompileParams>,
    ) -> Result<CallToolResult, McpError> {
        let typst = std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".to_string());
        let input = std::path::PathBuf::from(&p.path);
        let typ = byetex_core::compile::ensure_typ(&input).map_err(|e| {
            McpError::internal_error(format!("ensure_typ {}: {}", input.display(), e), None)
        })?;
        let out_pdf = p.out_pdf.as_ref().map(std::path::PathBuf::from);
        let res = byetex_core::compile::compile_typ(&typ, out_pdf.as_deref(), &typst)
            .map_err(|e| {
                McpError::internal_error(format!("compile {}: {}", typ.display(), e), None)
            })?;
        let json = serde_json::to_string(&res)
            .map_err(|e| McpError::internal_error(format!("serialize compile result: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Render a generated Typst file to per-page PNG images at a given DPI and \
                          return their paths. `path` is a `.typ` (or a `.tex`, converted flat \
                          first). Returns {ok, errors, image_paths}. Use this to visually inspect \
                          the conversion — e.g. front-matter and per-page fidelity grading."
    )]
    async fn render(
        &self,
        Parameters(p): Parameters<RenderParams>,
    ) -> Result<CallToolResult, McpError> {
        let typst = std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".to_string());
        let input = std::path::PathBuf::from(&p.path);
        let typ = byetex_core::compile::ensure_typ(&input).map_err(|e| {
            McpError::internal_error(format!("ensure_typ {}: {}", input.display(), e), None)
        })?;
        let out_dir = p.out_dir.map(std::path::PathBuf::from).unwrap_or_else(|| {
            let stem = typ.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            typ.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(format!("{stem}.pages"))
        });
        let res = byetex_core::compile::render_typ(&typ, &out_dir, p.dpi, &typst).map_err(|e| {
            McpError::internal_error(format!("render {}: {}", typ.display(), e), None)
        })?;
        let json = serde_json::to_string(&res)
            .map_err(|e| McpError::internal_error(format!("serialize render result: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Return the full markdown body of a single skill by name.")]
    async fn read_skill(
        &self,
        Parameters(p): Parameters<ReadSkillParams>,
    ) -> Result<CallToolResult, McpError> {
        match byetex_core::skills::read_skill(&p.name) {
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
                name: "byetex".to_string(),
                title: Some("ByeTex MCP server".to_string()),
                description: Some("LaTeX → Typst conversion with structured warnings.".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                website_url: None,
                icons: None,
            },
            instructions: Some(
                "ByeTex converts LaTeX to Typst and reports unconvertible \
                 constructs as structured warnings. Use `convert` for in-memory \
                 source, `convert_file`/`convert_project` for paths, `diagnose` to \
                 compile the output and map each typst error back to its LaTeX \
                 fragment + repair skill, `validate` to check whether the *input* \
                 LaTeX itself compiles (a broken source vs. a ByeTex bug), and \
                 `list_skills` + `read_skill` to discover how to act on warnings."
                    .to_string(),
            ),
        }
    }
}

// `materialize_project` previously lived inline here as a near-duplicate of
// the CLI's version. Both implementations have moved into
// `byetex_core::project::materialize_project` to eliminate the drift (the
// MCP copy used to silently skip unreadable assets; the CLI copy warned).
// The MCP call site below now delegates to the shared core function.

/// Run the server over stdio (the transport every major MCP client speaks).
/// Returns once the client disconnects or the process is signalled.
pub async fn run_stdio() -> anyhow::Result<()> {
    use rmcp::{transport::stdio, ServiceExt};
    let service = ByeTexServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
