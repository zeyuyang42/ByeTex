use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

// `materialize_project` lives in byetex-core now; no local module needed.

#[derive(Parser, Debug)]
#[command(name = "byetex", version, about = "LaTeX -> Typst converter")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Convert a .tex file to .typ, writing <stem>.warnings.json alongside it.
    /// With --project, emits a self-contained Typst project directory instead.
    Convert {
        /// Path to the input .tex file.
        input: PathBuf,

        /// Write Typst output here. Defaults to <input-stem>.typ.
        /// Mutually exclusive with --project.
        #[arg(long, conflicts_with = "project")]
        output: Option<PathBuf>,

        /// Convert as a LaTeX project: copy assets and emit a Typst project
        /// directory that compiles end-to-end with `typst compile`.
        #[arg(long)]
        project: bool,

        /// Output directory for project mode. Defaults to <input-stem>.typst-project/.
        #[arg(long, value_name = "DIR", requires = "project")]
        project_out: Option<PathBuf>,

        /// Skip writing typst.toml even when a known Typst Universe package is detected.
        #[arg(long, requires = "project")]
        no_toml: bool,

        /// Overwrite non-empty --project-out directory.
        #[arg(long, requires = "project")]
        force: bool,

        /// Skip writing the per-paper `agent_brief.md` sidecar. The brief
        /// is on by default — it bundles the source, the generated `.typ`,
        /// and the warnings into a single Markdown file an LLM can patch.
        #[arg(long)]
        no_brief: bool,
    },

    /// List or read the bundled skills that document how to act on warnings.
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },

    /// Run as an MCP server over stdio. Exposes the converter and skills to
    /// MCP-aware AI agents (Claude Code, Cursor, etc.).
    #[cfg(feature = "mcp")]
    Serve,

    /// Harvest and run a corpus of LaTeX snippets from markdown docs.
    /// Used by CI to track regressions in supported coverage.
    Corpus {
        #[command(subcommand)]
        action: CorpusAction,
    },

    /// Convert a paper AND run `typst compile` so the brief includes the
    /// real compile log. Functionally equivalent to `byetex convert <input>`
    /// (which already emits the brief by default) plus invoking `typst`
    /// inline. The brief is portable: paste it into any chat that can see
    /// the source `.tex` and the generated `.typ`.
    AgentBrief {
        /// Path to the input `.tex` file or project directory.
        input: PathBuf,
        /// Skip the `typst compile` step (useful when typst isn't on PATH).
        #[arg(long)]
        no_compile: bool,
        /// Convert as a LaTeX project: copy assets and emit a self-contained
        /// Typst project directory. Brief lives inside it as `agent_brief.md`.
        #[arg(long)]
        project: bool,
        /// Output directory for project mode. Defaults to
        /// `<input-stem>.typst-project/`.
        #[arg(long, value_name = "DIR", requires = "project")]
        project_out: Option<PathBuf>,
        /// Skip writing typst.toml even when a known Typst Universe package is detected.
        #[arg(long, requires = "project")]
        no_toml: bool,
        /// Overwrite non-empty --project-out directory.
        #[arg(long, requires = "project")]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CorpusAction {
    /// Extract every fenced ```latex / ```tex code block from a markdown
    /// file into individual .tex files in `--out`.
    Harvest {
        /// Source markdown file (e.g. context/latex-context.md).
        #[arg(long)]
        source: PathBuf,
        /// Output directory for the harvested .tex files.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run the converter on every .tex file in `--dir` and emit a JSON report
    /// summarising clean / warnings / parse_error buckets.
    Run {
        /// Directory containing the harvested .tex files.
        #[arg(long)]
        dir: PathBuf,
        /// Where to write the report (defaults to `<dir>/report.json`).
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum SkillsAction {
    /// Print the name and one-line description of every bundled skill.
    List,
    /// Print the full markdown body of a single skill by name.
    Read {
        /// Skill name as listed by `byetex skills list`.
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Convert {
            input,
            output,
            project,
            project_out,
            no_toml,
            force,
            no_brief,
        } => {
            let brief_opts = BriefOpts {
                skip: no_brief,
                no_compile: true, // convert is the fast path — no implicit typst spawn
            };
            let mode = if project {
                ConvertMode::Project { project_out, no_toml, force }
            } else {
                ConvertMode::Flat { output }
            };
            run_convert_dispatch(input, mode, brief_opts)
        }
        Command::Skills { action } => run_skills(action),
        #[cfg(feature = "mcp")]
        Command::Serve => run_serve(),
        Command::Corpus { action } => run_corpus(action),
        Command::AgentBrief {
            input,
            no_compile,
            project,
            project_out,
            no_toml,
            force,
        } => {
            // `agent-brief` is `convert` with the brief always on and a
            // real `typst compile` invocation (unless --no-compile).
            let brief_opts = BriefOpts {
                skip: false,
                no_compile,
            };
            let mode = if project {
                ConvertMode::Project { project_out, no_toml, force }
            } else {
                ConvertMode::Flat { output: None }
            };
            run_convert_dispatch(input, mode, brief_opts)
        }
    }
}

/// Per-call control over brief emission. Threaded through every convert
/// flow so `byetex convert` and `byetex agent-brief` share the same code
/// path with only this struct differing.
#[derive(Debug, Clone, Copy)]
struct BriefOpts {
    /// `true` for `--no-brief`; suppresses writing the `.agent_brief.md`.
    skip: bool,
    /// `true` to skip the embedded `typst compile` invocation. `convert`
    /// passes `true` (fast); `agent-brief` passes `false` (full log).
    no_compile: bool,
}

fn run_corpus(action: CorpusAction) -> Result<()> {
    match action {
        CorpusAction::Harvest { source, out } => corpus_harvest(source, out),
        CorpusAction::Run { dir, out } => corpus_run(dir, out),
    }
}

fn corpus_harvest(source: PathBuf, out_dir: PathBuf) -> Result<()> {
    let md = std::fs::read_to_string(&source)
        .with_context(|| format!("reading {}", source.display()))?;
    std::fs::create_dir_all(&out_dir).with_context(|| format!("create {}", out_dir.display()))?;

    let mut idx: usize = 0;
    let mut lines = md.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let opener = match trimmed.strip_prefix("```") {
            Some(rest) => rest.trim(),
            None => continue,
        };
        let is_latex = opener.eq_ignore_ascii_case("latex")
            || opener.eq_ignore_ascii_case("tex")
            || opener.to_ascii_lowercase().starts_with("latex-");
        if !is_latex {
            continue;
        }
        let mut body = String::new();
        for inner in lines.by_ref() {
            if inner.trim_start().starts_with("```") {
                break;
            }
            body.push_str(inner);
            body.push('\n');
        }
        if body.is_empty() {
            continue;
        }
        idx += 1;
        let path = out_dir.join(format!("c{:04}.tex", idx));
        std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    }
    eprintln!("harvested {} blocks into {}", idx, out_dir.display());
    Ok(())
}

fn corpus_run(dir: PathBuf, out_path: Option<PathBuf>) -> Result<()> {
    let out_path = out_path.unwrap_or_else(|| dir.join("report.json"));
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("tex"))
        .collect();
    entries.sort();

    let mut clean = 0usize;
    let mut warnings_bucket = 0usize;
    let mut parse_error = 0usize;
    let mut by_category: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for path in &entries {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let tree = byetex_core::parser::parse(&source);
        if tree.root_node().has_error() {
            parse_error += 1;
            *by_category.entry("parse_error".into()).or_default() += 1;
            continue;
        }
        let result = byetex_core::convert(
            &source,
            &byetex_core::ConvertOptions {
                source_name: Some(path.display().to_string()),
                base_dir: None,
            },
        );
        if result.warnings.is_empty() {
            clean += 1;
        } else {
            warnings_bucket += 1;
            for w in &result.warnings {
                let key = category_kind_name(&w.category);
                *by_category.entry(key).or_default() += 1;
            }
        }
    }

    let total = entries.len();
    let report = serde_json::json!({
        "total": total,
        "clean": clean,
        "warnings": warnings_bucket,
        "parse_error": parse_error,
        "by_category": by_category,
    });
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&out_path, json).with_context(|| format!("write {}", out_path.display()))?;
    eprintln!(
        "corpus report → {} (clean={} warnings={} parse_error={} total={})",
        out_path.display(),
        clean,
        warnings_bucket,
        parse_error,
        total
    );
    Ok(())
}

fn category_kind_name(c: &byetex_core::Category) -> String {
    use byetex_core::Category::*;
    match c {
        UnsupportedCommand { .. } => "unsupported_command".into(),
        UnsupportedEnvironment { .. } => "unsupported_environment".into(),
        CustomMacro { .. } => "custom_macro".into(),
        Tikz => "tikz".into(),
        ParseError { .. } => "parse_error".into(),
        AmbiguousMath { .. } => "ambiguous_math".into(),
        UnknownPackage { .. } => "unknown_package".into(),
        DropOnly { .. } => "drop_only".into(),
        NeedsManualReview { .. } => "needs_manual_review".into(),
    }
}

#[cfg(feature = "mcp")]
fn run_serve() -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(byetex_mcp::run_stdio())
}

fn run_skills(action: SkillsAction) -> Result<()> {
    match action {
        SkillsAction::List => {
            for s in byetex_core::skills::list_skills() {
                println!("{}\n    {}", s.name, s.description);
            }
            Ok(())
        }
        SkillsAction::Read { name } => match byetex_core::skills::read_skill(&name) {
            Some(s) => {
                print!("{}", s.body);
                Ok(())
            }
            None => {
                anyhow::bail!(
                    "skill '{name}' not found. Run `byetex skills list` to see available skills."
                );
            }
        },
    }
}

/// Which output shape `byetex convert` should produce. Threaded
/// through the unified dispatcher so file/dir input × flat/project
/// output is one match arm each rather than three near-duplicate
/// top-level functions.
enum ConvertMode {
    /// Write `<stem>.typ` + `<stem>.warnings.json` next to the input
    /// (or to `output` when overridden). Assets aren't copied.
    Flat { output: Option<PathBuf> },
    /// Write a self-contained `<stem>.typst-project/` directory
    /// containing `main.typ`, asset copies, `warnings.json`, and an
    /// optional `typst.toml`.
    Project {
        project_out: Option<PathBuf>,
        no_toml: bool,
        force: bool,
    },
}

/// Resolve `input.parent()` to a usable base directory, mapping empty
/// or missing parent to `.` so that an entry file passed by bare name
/// still gets `\input` resolution from the working directory.
fn base_dir_from_file(input: &std::path::Path) -> PathBuf {
    input
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Unified dispatcher for `byetex convert` and `byetex agent-brief`.
/// Owns base-dir resolution, stem derivation, sidecar writing, and
/// brief emission. Replaces three near-duplicate functions
/// (`run_convert`, `run_convert_dir_flat`, `run_convert_project`)
/// from earlier revisions.
fn run_convert_dispatch(input: PathBuf, mode: ConvertMode, brief: BriefOpts) -> Result<()> {
    let input_is_dir = input.is_dir();

    // Whether to emit `typst.toml`. Flat mode never does (no project
    // dir to put it in); project mode honours the flag.
    let no_toml = matches!(&mode, ConvertMode::Flat { .. })
        || matches!(&mode, ConvertMode::Project { no_toml: true, .. });

    // Plan the conversion. Both modes go through the project planners
    // so we get `plan.entry_tex` for the brief.
    let plan = if input_is_dir {
        byetex_core::project::plan_project_from_dir(&input, no_toml)
            .with_context(|| format!("planning project from {}", input.display()))?
    } else {
        byetex_core::project::plan_project(&input, no_toml)
            .with_context(|| format!("planning project from {}", input.display()))?
    };

    // Base directory used by the materialiser's path-traversal guard.
    let base_dir = if input_is_dir {
        input.clone()
    } else {
        base_dir_from_file(&input)
    };

    // Filename stem for default output paths. Directory input uses the
    // dir name; file input uses the file stem.
    let default_stem = if input_is_dir {
        input.file_name()
    } else {
        input.file_stem()
    }
    .and_then(|s| s.to_str())
    .unwrap_or("project")
    .to_string();

    // Parent directory for default output placement (next-to-input).
    let parent_for_outputs = input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf());

    match mode {
        ConvertMode::Flat { output } => {
            run_flat(input, plan, &default_stem, parent_for_outputs, output, input_is_dir, brief)
        }
        ConvertMode::Project { project_out, no_toml: _, force } => run_project(
            plan,
            &base_dir,
            &default_stem,
            parent_for_outputs,
            project_out,
            force,
            brief,
        ),
    }
}

/// Flat-output arm of the dispatcher: writes `<stem>.typ` +
/// `<stem>.warnings.json` (+ optional agent_brief.md). Assets are
/// dropped; the eprintln warns about that when input was a directory.
fn run_flat(
    input: PathBuf,
    plan: byetex_core::project::ProjectPlan,
    default_stem: &str,
    parent_for_outputs: Option<PathBuf>,
    output: Option<PathBuf>,
    input_is_dir: bool,
    brief: BriefOpts,
) -> Result<()> {
    let typst_path = output.unwrap_or_else(|| {
        if input_is_dir {
            let name = format!("{}.typ", default_stem);
            parent_for_outputs
                .clone()
                .map(|p| p.join(&name))
                .unwrap_or_else(|| PathBuf::from(name))
        } else {
            input.with_extension("typ")
        }
    });
    let warnings_path = typst_path.with_extension("warnings.json");

    std::fs::write(&typst_path, &plan.main_typst)
        .with_context(|| format!("writing {}", typst_path.display()))?;
    let warnings_json =
        serde_json::to_string_pretty(&plan.warnings).context("serializing warnings to JSON")?;
    std::fs::write(&warnings_path, warnings_json)
        .with_context(|| format!("writing {}", warnings_path.display()))?;

    let n_warn = plan.warnings.len();
    let suffix = if input_is_dir {
        "; assets NOT copied — use --project for that"
    } else {
        ""
    };
    eprintln!(
        "wrote {} ({} warning{}{})",
        typst_path.display(),
        n_warn,
        if n_warn == 1 { "" } else { "s" },
        suffix,
    );

    if !brief.skip {
        let brief_path = typst_path.with_extension("agent_brief.md");
        let manual_path = typst_path.with_file_name(format!(
            "{}_manual.typ",
            typst_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("paper")
        ));
        write_agent_brief(BriefInputs {
            brief_path: &brief_path,
            source_tex: &plan.entry_tex,
            typst_path: &typst_path,
            warnings_path: &warnings_path,
            manual_path: &manual_path,
            mode: BriefMode::Flat,
            no_compile: brief.no_compile,
        })?;
    }

    Ok(())
}

/// Project-output arm of the dispatcher: materialise into
/// `<stem>.typst-project/` with assets, warnings.json, optional
/// typst.toml, and the agent brief.
fn run_project(
    plan: byetex_core::project::ProjectPlan,
    base_dir: &std::path::Path,
    default_stem: &str,
    parent_for_outputs: Option<PathBuf>,
    project_out: Option<PathBuf>,
    force: bool,
    brief: BriefOpts,
) -> Result<()> {
    let out_dir = project_out.unwrap_or_else(|| {
        let name = format!("{}.typst-project", default_stem);
        parent_for_outputs
            .map(|p| p.join(&name))
            .unwrap_or_else(|| PathBuf::from(name))
    });

    let n_warnings = plan.warnings.len();
    let n_assets = plan.assets.len();
    let has_manifest = plan.manifest.is_some();

    byetex_core::project::materialize_project(&plan, &out_dir, base_dir, force)
        .with_context(|| format!("writing project to {}", out_dir.display()))?;

    // Persist warnings as a sidecar so downstream tooling (agent-brief,
    // skill-driven remediation) can act on them.
    let warnings_path = out_dir.join("warnings.json");
    let warnings_json = serde_json::to_string_pretty(&plan.warnings)
        .with_context(|| "serialising warnings")?;
    std::fs::write(&warnings_path, warnings_json)
        .with_context(|| format!("writing {}", warnings_path.display()))?;

    eprintln!(
        "wrote project → {} ({} asset{}, {} warning{}, typst.toml: {})",
        out_dir.display(),
        n_assets,
        if n_assets == 1 { "" } else { "s" },
        n_warnings,
        if n_warnings == 1 { "" } else { "s" },
        if has_manifest { "yes" } else { "no" },
    );

    if !brief.skip {
        let typst_path = out_dir.join("main.typ");
        let brief_path = out_dir.join("agent_brief.md");
        let manual_path = out_dir.join("main_manual.typ");
        write_agent_brief(BriefInputs {
            brief_path: &brief_path,
            source_tex: &plan.entry_tex,
            typst_path: &typst_path,
            warnings_path: &warnings_path,
            manual_path: &manual_path,
            mode: BriefMode::Project,
            no_compile: brief.no_compile,
        })?;
    }

    Ok(())
}

/// Whether the brief describes a flat-output convert or a `--project`
/// directory. Drives the wording of the "What to do" section, the
/// validation command suggested to the LLM, and the note about whether
/// assets were copied.
#[derive(Debug, Clone, Copy)]
enum BriefMode {
    Flat,
    Project,
}

/// Everything `write_agent_brief` needs. All paths are absolute or
/// relative-to-cwd; the helper relativises them against
/// `brief_path.parent()` when rendering so the brief is portable across
/// working directories.
struct BriefInputs<'a> {
    /// Where the `.agent_brief.md` will be written.
    brief_path: &'a Path,
    /// The entry `.tex` file (the one driving conversion). In folder
    /// mode this is the detected entry, not the folder itself.
    source_tex: &'a Path,
    /// Generated Typst output: `paper.typ` in flat mode,
    /// `<out_dir>/main.typ` in project mode.
    typst_path: &'a Path,
    /// `warnings.json` sidecar location.
    warnings_path: &'a Path,
    /// Where the brief asks the LLM to write its patched output.
    manual_path: &'a Path,
    mode: BriefMode,
    /// Skip the `typst compile` invocation. `convert`-driven briefs
    /// default to `true` (fast); `agent-brief` sets it to `false`.
    no_compile: bool,
}

/// Render an absolute-or-relative `target` as a path relative to
/// `base_dir`. Walks both paths component-by-component and emits
/// `../` prefixes when needed. Falls back to the absolute `display()`
/// when the paths can't share a common prefix (different drive letters
/// on Windows, etc.).
fn relativize(target: &Path, base_dir: &Path) -> String {
    fn absolutize(p: &Path) -> PathBuf {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(p)
        }
    }
    let target_abs = absolutize(target);
    let base_abs = absolutize(base_dir);
    let t: Vec<_> = target_abs.components().collect();
    let b: Vec<_> = base_abs.components().collect();
    // Bail to absolute display when the roots differ (different drives,
    // VerbatimDisk vs Disk on Windows, etc.) so we never produce a
    // misleading relative path.
    if t.first().map(|c| c.as_os_str()) != b.first().map(|c| c.as_os_str()) {
        return target.display().to_string();
    }
    let common = t.iter().zip(b.iter()).take_while(|(a, b)| a == b).count();
    let mut rel = PathBuf::new();
    for _ in common..b.len() {
        rel.push("..");
    }
    for c in &t[common..] {
        rel.push(c.as_os_str());
    }
    if rel.as_os_str().is_empty() {
        ".".to_string()
    } else {
        rel.display().to_string()
    }
}

/// Render and write a per-paper `agent_brief.md`. Called by every
/// `byetex convert` flow (unless `--no-brief`) and by `byetex agent-brief`.
///
/// The brief embeds the source `.tex`, generated `.typ`, optional
/// Paths shown are relativised against the brief's own directory so they
/// remain meaningful regardless of the caller's cwd.
fn write_agent_brief(inputs: BriefInputs<'_>) -> Result<()> {
    let BriefInputs {
        brief_path,
        source_tex,
        typst_path,
        warnings_path,
        manual_path,
        mode,
        no_compile,
    } = inputs;

    // Ensure the brief directory exists (matters for project mode,
    // where the dir is created by materialize_project; in flat mode
    // the dir already exists because we just wrote the .typ there).
    if let Some(parent) = brief_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("ensuring brief directory {}", parent.display())
            })?;
        }
    }

    let brief_dir = brief_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Read the .typ for template-detection only (first 8 lines are enough).
    let typ = std::fs::read_to_string(typst_path).unwrap_or_default();
    let warnings_text =
        std::fs::read_to_string(warnings_path).unwrap_or_else(|_| "[]".to_string());

    // Detect template binding by peeking the first few lines of the
    // generated `.typ` (looking for our `#import "@preview/X:V":` line).
    let detected_template = typ
        .lines()
        .take(8)
        .find_map(|l| {
            let prefix = "#import \"@preview/";
            l.find(prefix).map(|i| {
                let rest = &l[i + prefix.len()..];
                rest.split('"').next().unwrap_or("").to_string()
            })
        })
        .unwrap_or_else(|| "(none — hand-rolled fallback)".to_string());

    // Try to compile (unless --no-compile). Capture exit + stderr for the brief.
    let (compile_ok, compile_log) = if no_compile {
        (None, String::new())
    } else {
        let pdf_path = typst_path.with_extension("pdf");
        let _ = std::fs::remove_file(&pdf_path);
        let out = std::process::Command::new("typst")
            .arg("compile")
            .arg(typst_path)
            .arg(&pdf_path)
            .output()
            .with_context(|| "spawning `typst compile`")?;
        let log = String::from_utf8_lossy(&out.stderr).to_string();
        (Some(out.status.success()), log)
    };

    // Warnings: total count + category histogram sorted by count desc.
    let (warnings_total, warnings_histogram): (usize, String) =
        match serde_json::from_str::<Vec<serde_json::Value>>(&warnings_text) {
            Ok(arr) => {
                let mut counts: std::collections::BTreeMap<String, usize> =
                    std::collections::BTreeMap::new();
                for w in &arr {
                    let kind = w
                        .get("category")
                        .and_then(|c| c.get("kind"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    *counts.entry(kind).or_default() += 1;
                }
                let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                let histogram = if sorted.is_empty() {
                    "(none)".to_string()
                } else {
                    sorted
                        .iter()
                        .map(|(k, c)| format!("  - {k}: {c}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                (arr.len(), histogram)
            }
            Err(_) => (0, "(could not parse warnings.json)".to_string()),
        };

    // First compile errors — enough context to start patching, not the full log.
    let first_errors: String = compile_log
        .lines()
        .filter(|l| l.starts_with("error:") || l.starts_with("warning:"))
        .take(10)
        .collect::<Vec<_>>()
        .join("\n");

    let compile_status = match compile_ok {
        None if no_compile => {
            "(not run — pass `byetex agent-brief` to capture the typst log)".to_string()
        }
        None => "(skipped)".to_string(),
        Some(true) => "✅ typst compile succeeded".to_string(),
        Some(false) => "❌ typst compile failed".to_string(),
    };

    // Render all path references relative to the brief's own dir so the
    // brief stays portable regardless of the caller's cwd.
    let src_rel = relativize(source_tex, &brief_dir);
    let typ_rel = relativize(typst_path, &brief_dir);
    let warn_rel = relativize(warnings_path, &brief_dir);
    let manual_rel = relativize(manual_path, &brief_dir);

    let stem = typst_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("paper")
        .to_string();

    let mode_label = match mode {
        BriefMode::Flat => "flat",
        BriefMode::Project => "project",
    };

    // Project mode: note that figures and .bib files are colocated.
    let colocated_note = match mode {
        BriefMode::Flat => String::new(),
        BriefMode::Project => {
            "\n\n_Figures and `.bib` files are colocated in this directory._".to_string()
        }
    };

    let brief = format!(
        r#"# ByeTex agent brief: `{stem}` ({mode_label} mode)

**Task** — Read `{src_rel}` (source) and `{typ_rel}` (current Typst output).
Write a patched copy to **`{manual_rel}`** that compiles cleanly with
`typst compile {typ_rel}`. Apply the smallest possible local edits per
compile error; preserve what already works.

## Compile status
{compile_status}

```
{first_errors}
```

## Warnings ({warnings_total})
{warnings_histogram}

Full sidecar: `{warn_rel}`

## Files
- Typst template: **{detected_template}**
- Source `.tex`: `{src_rel}`
- Generated `.typ`: `{typ_rel}`
- Write patched output here: **`{manual_rel}`**
- Warnings JSON: `{warn_rel}`{colocated_note}
"#,
        stem = stem,
        mode_label = mode_label,
        src_rel = src_rel,
        typ_rel = typ_rel,
        warn_rel = warn_rel,
        manual_rel = manual_rel,
        compile_status = compile_status,
        first_errors = if first_errors.is_empty() {
            "(no errors)".to_string()
        } else {
            first_errors
        },
        warnings_total = warnings_total,
        warnings_histogram = warnings_histogram,
        detected_template = detected_template,
        colocated_note = colocated_note,
    );

    std::fs::write(brief_path, brief)
        .with_context(|| format!("writing {}", brief_path.display()))?;
    eprintln!("wrote {}", brief_path.display());
    Ok(())
}
