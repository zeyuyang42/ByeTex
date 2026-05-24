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
            if project {
                run_convert_project(input, project_out, no_toml, force, brief_opts)
            } else {
                run_convert(input, output, brief_opts)
            }
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
            if project {
                run_convert_project(input, project_out, no_toml, force, brief_opts)
            } else {
                run_convert(input, None, brief_opts)
            }
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
        DropOnly => "drop_only".into(),
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

fn run_convert_project(
    input: PathBuf,
    project_out: Option<PathBuf>,
    no_toml: bool,
    force: bool,
    brief: BriefOpts,
) -> Result<()> {
    // Resolve the input shape. When the user passes a directory, ByeTex
    // detects the entry `.tex` file (`\documentclass`-bearing) and
    // pre-scans every `.tex`/`.sty`/`.cls` in the tree for macros.
    // When they pass a file, behaviour is unchanged.
    let input_is_dir = input.is_dir();
    let (plan, base_dir, default_out_stem) = if input_is_dir {
        let plan = byetex_core::project::plan_project_from_dir(&input, no_toml)
            .with_context(|| format!("planning project from {}", input.display()))?;
        let stem = input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project")
            .to_string();
        (plan, input.clone(), stem)
    } else {
        let plan = byetex_core::project::plan_project(&input, no_toml)
            .with_context(|| format!("planning project from {}", input.display()))?;
        let base_dir = input
            .parent()
            .map(|p| {
                if p.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    p.to_path_buf()
                }
            })
            .unwrap_or_else(|| PathBuf::from("."));
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("project")
            .to_string();
        (plan, base_dir, stem)
    };

    let out_dir = project_out.unwrap_or_else(|| {
        // For dir input the output sits next to the input directory; for
        // file input it sits next to the file. In both cases use the
        // stem to name the output: `<stem>.typst-project/`.
        let parent = if input_is_dir {
            input.parent().map(|p| p.to_path_buf())
        } else {
            input.parent().map(|p| p.to_path_buf())
        };
        let name = format!("{}.typst-project", default_out_stem);
        parent
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.join(&name))
            .unwrap_or_else(|| PathBuf::from(name))
    });

    let n_warnings = plan.warnings.len();
    let n_assets = plan.assets.len();
    let has_manifest = plan.manifest.is_some();

    byetex_core::project::materialize_project(&plan, &out_dir, &base_dir, force)
        .with_context(|| format!("writing project to {}", out_dir.display()))?;

    // Persist warnings as a sidecar so downstream tooling (agent-brief,
    // skill-driven remediation) can act on them. Without this the project
    // path was a regression versus `run_convert`, which always writes
    // `<stem>.warnings.json` next to the .typ.
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
        // Project briefs live INSIDE the typst-project dir, alongside
        // main.typ. No stem prefix needed — it's a dedicated dir.
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

fn run_convert(input: PathBuf, output: Option<PathBuf>, brief: BriefOpts) -> Result<()> {
    // When the user hands us a directory, route through the same
    // entry-detect + macro pre-scan pipeline as project mode but emit
    // a flat `<dir>.typ` next to the dir instead of a project tree.
    if input.is_dir() {
        return run_convert_dir_flat(input, output, brief);
    }

    let source =
        std::fs::read_to_string(&input).with_context(|| format!("reading {}", input.display()))?;

    // Resolve `\input{...}` / `\include{...}` relative to the input file's
    // parent directory. Falls back to "." when the path has no parent so
    // that an entry file passed by bare name still gets includes resolved
    // from the working directory.
    let base_dir = input
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .or_else(|| Some(PathBuf::from(".")));

    let opts = byetex_core::ConvertOptions {
        source_name: Some(input.display().to_string()),
        base_dir,
    };
    let result = byetex_core::convert(&source, &opts);

    let typst_path = output.unwrap_or_else(|| input.with_extension("typ"));
    let warnings_path = typst_path.with_extension("warnings.json");

    std::fs::write(&typst_path, &result.typst)
        .with_context(|| format!("writing {}", typst_path.display()))?;

    let warnings_json =
        serde_json::to_string_pretty(&result.warnings).context("serializing warnings to JSON")?;
    std::fs::write(&warnings_path, warnings_json)
        .with_context(|| format!("writing {}", warnings_path.display()))?;

    eprintln!(
        "wrote {} ({} warning{})",
        typst_path.display(),
        result.warnings.len(),
        if result.warnings.len() == 1 { "" } else { "s" }
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
            source_tex: &input,
            typst_path: &typst_path,
            warnings_path: &warnings_path,
            manual_path: &manual_path,
            mode: BriefMode::Flat,
            no_compile: brief.no_compile,
        })?;
    }

    Ok(())
}

/// Non-project conversion when the user hands us a directory. Detects
/// the entry `.tex` and writes `<dir>.typ` + `<dir>.warnings.json` next
/// to the dir. Asset files are NOT copied — use `--project` for that.
fn run_convert_dir_flat(input: PathBuf, output: Option<PathBuf>, brief: BriefOpts) -> Result<()> {
    let plan = byetex_core::project::plan_project_from_dir(&input, true)
        .with_context(|| format!("planning project from {}", input.display()))?;

    let stem = input
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let parent = input.parent().filter(|p| !p.as_os_str().is_empty());
    let typst_path = output.unwrap_or_else(|| {
        let name = format!("{}.typ", stem);
        parent
            .map(|p| p.join(&name))
            .unwrap_or_else(|| PathBuf::from(name))
    });
    let warnings_path = typst_path.with_extension("warnings.json");

    std::fs::write(&typst_path, &plan.main_typst)
        .with_context(|| format!("writing {}", typst_path.display()))?;
    let warnings_json =
        serde_json::to_string_pretty(&plan.warnings).context("serializing warnings to JSON")?;
    std::fs::write(&warnings_path, warnings_json)
        .with_context(|| format!("writing {}", warnings_path.display()))?;

    eprintln!(
        "wrote {} ({} warning{}; assets NOT copied — use --project for that)",
        typst_path.display(),
        plan.warnings.len(),
        if plan.warnings.len() == 1 { "" } else { "s" }
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
/// `typst compile` log, and the structured `warnings.json` inline so an
/// LLM can act on it without filesystem access. Paths shown at the top
/// are relativised against the brief's own directory so they remain
/// meaningful regardless of the caller's cwd.
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

    // Read the artifacts so we can paste them inline.
    let tex = std::fs::read_to_string(source_tex)
        .with_context(|| format!("reading {}", source_tex.display()))?;
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

    // Warnings: summarise category counts for the brief header.
    let warnings_summary: String =
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
                let mut s = format!("Total: {}", arr.len());
                for (k, c) in &counts {
                    s.push_str(&format!("\n  - {}: {}", k, c));
                }
                s
            }
            Err(_) => "(could not parse warnings.json)".to_string(),
        };

    // First-N compile errors (full output is in the appendix).
    let first_errors: String = compile_log
        .lines()
        .filter(|l| l.starts_with("error:") || l.starts_with("warning:"))
        .take(15)
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

    // Render all the path references relative to the brief's own dir
    // so the brief stays portable regardless of caller cwd.
    let src_rel = relativize(source_tex, &brief_dir);
    let typ_rel = relativize(typst_path, &brief_dir);
    let warn_rel = relativize(warnings_path, &brief_dir);
    let manual_rel = relativize(manual_path, &brief_dir);

    let stem = typst_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("paper")
        .to_string();

    // Mode-specific guidance: what command validates the patched output,
    // what file the LLM should write to, what's already in the dir.
    let (mode_label, what_to_do, layout_note) = match mode {
        BriefMode::Flat => (
            "flat",
            format!(
                "You are an LLM. Read the source `.tex` and the generated `.typ` (both embedded below — \
filesystem access is optional) and **write a patched copy to `{manual_rel}`** that compiles \
cleanly with `typst compile {typ_rel}`. Don't rewrite the whole document — preserve what works \
and apply the smallest possible local edits to fix each compile error."
            ),
            "_Note: this is flat-output mode — referenced figures and `.bib` files were NOT copied. \
Use `byetex convert <input> --project` if you want a self-contained Typst project directory._"
                .to_string(),
        ),
        BriefMode::Project => (
            "project",
            format!(
                "You are an LLM. This brief lives inside a self-contained Typst project directory. \
Read the source `.tex` and the generated `main.typ` (both embedded below — filesystem access is \
optional) and **write a patched copy to `{manual_rel}`** that compiles cleanly with \
`typst compile main.typ` (run from this directory). Preserve what works; apply the smallest \
possible local edits per compile error."
            ),
            "_Project layout: `main.typ` (the body), `warnings.json` (sidecar), `agent_brief.md` \
(this file), plus any figures / `.bib` files referenced by the paper, copied here by ByeTex. \
The original `.tex` lives at the `Source` path above (outside this directory)._"
                .to_string(),
        ),
    };

    let brief = format!(
        r#"# ByeTex agent brief: `{stem}` ({mode_label} mode)

_All relative paths below are resolved against the directory containing this brief:_
`{brief_dir_display}`

- **Source** `.tex`: `{src_rel}`
- **Typst output**: `{typ_rel}`
- **Warnings sidecar**: `{warn_rel}`
- **Suggested patched output**: **`{manual_rel}`**

{layout_note}

## Detection
- Typst template binding: **{template}**
- ByeTex warnings: ```
{warnings_summary}
```

## Compile status
{compile_status}

### First errors (15 max — full log below)
```
{first_errors}
```

## What to do

{what_to_do}

Useful patterns observed in this corpus (from
`docs/visual-regression-2026-05-23.md`):

- **`unclosed delimiter` in math** is usually a nested
  `\left\{{ \begin{{array}} ... \right\}}` that didn't translate cleanly.
  Rewrite as Typst `cases(...)` or a `lr({{ ... }})` block.
- **`unknown variable: <2-letter>`** (e.g. `dh`, `zK`, `pt`) is letter-
  fusion: insert a space between the two letters.
- **`unknown variable: <word>`** without a backslash often means a custom
  macro wasn't expanded; if a `\newcommand` exists for it in the source,
  inline the expansion. If it's `\<UPPER>`, wrap as `"<UPPER>"`.
- **`unclosed label` / `<label with spaces>`** — the label key contains
  spaces (`<thm:foo bar>`); replace spaces with `-`.
- **`unexpected slash` in math like `$/X$`** — replace with literal `/`.
- **`file not found` for `.bib`** — drop the `#bibliography(...)` line
  entirely or repoint to an existing `.bib` file in the source dir.
- **`character ` # ` is not valid in code`** — `##` in expl3 / `\#`
  pushforward subscript — escape with `\#` in math context.

## Source `.tex`
```
{tex}
```

## Generated `.typ`
```typst
{typ_inline}
```

## Full `typst compile` log
```
{compile_log}
```

## Full warnings.json
```json
{warnings_text}
```
"#,
        stem = stem,
        mode_label = mode_label,
        brief_dir_display = brief_dir.display(),
        src_rel = src_rel,
        typ_rel = typ_rel,
        warn_rel = warn_rel,
        manual_rel = manual_rel,
        layout_note = layout_note,
        template = detected_template,
        warnings_summary = warnings_summary,
        compile_status = compile_status,
        first_errors = if first_errors.is_empty() {
            "(no errors)".to_string()
        } else {
            first_errors
        },
        what_to_do = what_to_do,
        tex = if tex.len() > 30_000 {
            format!(
                "{}\n... [truncated, see `{}`]",
                truncate_at_char_boundary(&tex, 30_000),
                src_rel
            )
        } else {
            tex
        },
        typ_inline = if typ.len() > 30_000 {
            format!(
                "{}\n... [truncated, see `{}`]",
                truncate_at_char_boundary(&typ, 30_000),
                typ_rel
            )
        } else {
            typ
        },
        compile_log = if compile_log.is_empty() {
            "(empty)".to_string()
        } else if compile_log.len() > 10_000 {
            format!(
                "{}\n... [truncated]",
                truncate_at_char_boundary(&compile_log, 10_000)
            )
        } else {
            compile_log
        },
        warnings_text = if warnings_text.len() > 20_000 {
            format!(
                "{}\n... [truncated, see `{}`]",
                truncate_at_char_boundary(&warnings_text, 20_000),
                warn_rel
            )
        } else {
            warnings_text
        },
    );

    std::fs::write(brief_path, brief)
        .with_context(|| format!("writing {}", brief_path.display()))?;
    eprintln!("wrote {}", brief_path.display());
    Ok(())
}

/// Return `&s[..n]` rounded down to the nearest char boundary so the slice
/// never panics on multi-byte UTF-8. When `s.len() <= n`, returns `s`.
fn truncate_at_char_boundary(s: &str, n: usize) -> &str {
    if s.len() <= n {
        return s;
    }
    let mut end = n;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_at_char_boundary_handles_multibyte() {
        // 'é' is 2 bytes (0xC3 0xA9). At byte-index 1 we'd be mid-codepoint.
        let s = "aé"; // 3 bytes total: a (1) + é (2)
        assert_eq!(truncate_at_char_boundary(s, 2), "a");
        assert_eq!(truncate_at_char_boundary(s, 3), "aé");
    }

    #[test]
    fn truncate_at_char_boundary_short_input_passthrough() {
        let s = "hello";
        assert_eq!(truncate_at_char_boundary(s, 100), "hello");
    }
}
