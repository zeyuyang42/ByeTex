use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "bytetex", version, about = "LaTeX -> Typst converter")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Convert a .tex file to .typ, writing <stem>.warnings.json alongside it.
    Convert {
        /// Path to the input .tex file.
        input: PathBuf,

        /// Write Typst output here. Defaults to <input-stem>.typ.
        #[arg(long)]
        output: Option<PathBuf>,
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
        Command::Convert { input, output } => run_convert(input, output),
        Command::Skills { action } => run_skills(action),
        #[cfg(feature = "mcp")]
        Command::Serve => run_serve(),
        Command::Corpus { action } => run_corpus(action),
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
