use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod project;

#[derive(Parser, Debug)]
#[command(name = "bytetex", version, about = "LaTeX -> Typst converter")]
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

    /// Convert a paper, attempt to compile it with `typst`, and write a
    /// per-paper `<stem>.agent_brief.md` that bundles everything an LLM
    /// agent needs to patch the residual issues. The brief is portable:
    /// paste it into any chat that can see the source `.tex` and the
    /// generated `.typ`.
    AgentBrief {
        /// Path to the input .tex file.
        input: PathBuf,
        /// Skip the `typst compile` step (useful when typst isn't on PATH).
        #[arg(long)]
        no_compile: bool,
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
        /// Skill name as listed by `bytetex skills list`.
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Convert { input, output, project, project_out, no_toml, force } => {
            if project {
                run_convert_project(input, project_out, no_toml, force)
            } else {
                run_convert(input, output)
            }
        }
        Command::Skills { action } => run_skills(action),
        #[cfg(feature = "mcp")]
        Command::Serve => run_serve(),
        Command::Corpus { action } => run_corpus(action),
        Command::AgentBrief { input, no_compile } => run_agent_brief(input, no_compile),
    }
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
        let tree = bytetex_core::parser::parse(&source);
        if tree.root_node().has_error() {
            parse_error += 1;
            *by_category.entry("parse_error".into()).or_default() += 1;
            continue;
        }
        let result = bytetex_core::convert(
            &source,
            &bytetex_core::ConvertOptions {
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

fn category_kind_name(c: &bytetex_core::Category) -> String {
    use bytetex_core::Category::*;
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
    rt.block_on(bytetex_mcp::run_stdio())
}

fn run_skills(action: SkillsAction) -> Result<()> {
    match action {
        SkillsAction::List => {
            for s in bytetex_core::skills::list_skills() {
                println!("{}\n    {}", s.name, s.description);
            }
            Ok(())
        }
        SkillsAction::Read { name } => match bytetex_core::skills::read_skill(&name) {
            Some(s) => {
                print!("{}", s.body);
                Ok(())
            }
            None => {
                anyhow::bail!(
                    "skill '{name}' not found. Run `bytetex skills list` to see available skills."
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
) -> Result<()> {
    let out_dir = project_out.unwrap_or_else(|| {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("project");
        input
            .parent()
            .map(|p| p.join(format!("{}.typst-project", stem)))
            .unwrap_or_else(|| PathBuf::from(format!("{}.typst-project", stem)))
    });

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

    let plan = bytetex_core::project::plan_project(&input, no_toml)
        .with_context(|| format!("planning project from {}", input.display()))?;

    let n_warnings = plan.warnings.len();
    let n_assets = plan.assets.len();
    let has_manifest = plan.manifest.is_some();

    project::materialize_project(&plan, &out_dir, &base_dir, force)
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
    Ok(())
}

fn run_convert(input: PathBuf, output: Option<PathBuf>) -> Result<()> {
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

    let opts = bytetex_core::ConvertOptions {
        source_name: Some(input.display().to_string()),
        base_dir,
    };
    let result = bytetex_core::convert(&source, &opts);

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
    Ok(())
}

/// `bytetex agent-brief <paper.tex> [--no-compile]`
///
/// Convert + try to compile + emit a per-paper Markdown brief an LLM
/// agent can read to fix the residual issues. The brief lives at
/// `<paper-stem>.agent_brief.md` alongside the `.typ` / `.warnings.json`.
fn run_agent_brief(input: PathBuf, no_compile: bool) -> Result<()> {
    // Reuse the standard conversion path.
    run_convert(input.clone(), None)?;
    let typst_path = input.with_extension("typ");
    let warnings_path = typst_path.with_extension("warnings.json");
    let brief_path = input.with_extension("agent_brief.md");

    // Read the artifacts so we can paste them inline.
    let tex =
        std::fs::read_to_string(&input).with_context(|| format!("reading {}", input.display()))?;
    let typ = std::fs::read_to_string(&typst_path).unwrap_or_default();
    let warnings_text =
        std::fs::read_to_string(&warnings_path).unwrap_or_else(|_| "[]".to_string());

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
            .arg(&typst_path)
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
        None => "(skipped — --no-compile)".to_string(),
        Some(true) => "✅ typst compile succeeded".to_string(),
        Some(false) => "❌ typst compile failed".to_string(),
    };

    let manual_path = input.with_file_name(format!(
        "{}_manual.typ",
        typst_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("paper")
    ));

    let brief = format!(
        r#"# ByeTex agent brief: `{stem}`

Source: `{src}`
Typst output: `{typ}`
Warnings sidecar: `{warn}`
**Suggested patched output: `{manual}`**

## Detection
- Typst template binding: **{template}**
- Bytetex warnings: ```
{warnings_summary}
```

## Compile status
{compile_status}

### First errors (15 max — full log below)
```
{first_errors}
```

## What to do

You are an LLM with file access. Your job: read the source `.tex` and
the generated `.typ`, then **write a patched copy to `{manual}`** that
compiles cleanly with `typst compile`. Don't rewrite the whole
document — preserve what works and apply the smallest possible
local edits to fix each compile error.

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
        stem = typst_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("paper"),
        src = input.display(),
        typ = typst_path.display(),
        warn = warnings_path.display(),
        manual = manual_path.display(),
        template = detected_template,
        warnings_summary = warnings_summary,
        compile_status = compile_status,
        first_errors = if first_errors.is_empty() {
            "(no errors)".to_string()
        } else {
            first_errors
        },
        tex = if tex.len() > 30_000 {
            format!(
                "{}\n... [truncated, see {}]",
                truncate_at_char_boundary(&tex, 30_000),
                input.display()
            )
        } else {
            tex
        },
        typ_inline = if typ.len() > 30_000 {
            format!(
                "{}\n... [truncated, see {}]",
                truncate_at_char_boundary(&typ, 30_000),
                typst_path.display()
            )
        } else {
            typ
        },
        compile_log = if compile_log.is_empty() {
            "(empty)".to_string()
        } else if compile_log.len() > 10_000 {
            format!("{}\n... [truncated]", truncate_at_char_boundary(&compile_log, 10_000))
        } else {
            compile_log
        },
        warnings_text = if warnings_text.len() > 20_000 {
            format!(
                "{}\n... [truncated, see {}]",
                truncate_at_char_boundary(&warnings_text, 20_000),
                warnings_path.display()
            )
        } else {
            warnings_text
        },
    );

    std::fs::write(&brief_path, brief)
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
