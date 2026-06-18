use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

// `materialize_project` lives in byetex-core now; no local module needed.

#[derive(Parser, Debug)]
#[command(
    name = "byetex",
    version,
    about = "LaTeX -> Typst converter with an agent repair loop",
    long_about = "ByeTex deterministically converts an academic subset of LaTeX to Typst, \
                  and for everything outside that subset emits a warnings.json sidecar plus \
                  a catalogue of repair skills.\n\n\
                  Typical agent flow:\n  \
                  byetex convert paper.tex      # → paper.typ + warnings + agent_brief\n  \
                  byetex diagnose paper.tex     # compile + map each typst error to its LaTeX fragment + skill\n  \
                  byetex skills read <name>     # read the repair guide\n  \
                  typst compile paper.typ       # the success criterion\n\n\
                  Start with `byetex skills read byetex-getting-started`."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Convert a .tex file to .typ, writing <stem>.warnings.json alongside it.
    /// With --project, emits a self-contained Typst project directory instead.
    Convert {
        /// Path to the input .tex file, or `-` to read LaTeX from stdin. Omit
        /// when using `-c/--code`. Stdin and `-c` print Typst to stdout and
        /// write no files (a quick, reproducible one-off check).
        input: Option<PathBuf>,

        /// Convert a LaTeX string directly and print the Typst to stdout
        /// (no files written).
        #[arg(short = 'c', long = "code", conflicts_with_all = ["input", "project", "output"])]
        code: Option<String>,

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

        /// Also run `typst compile` on the output and fold the real compile
        /// log into the brief. Equivalent to `byetex agent-brief <input>`.
        #[arg(long)]
        compile: bool,
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

    /// Convert a paper AND run `typst compile` so the brief includes the real
    /// compile log. A convenience shorthand for `byetex convert <input> --compile`
    /// (`convert` already emits the brief by default; `--compile` adds the live
    /// typst run). The brief is portable: paste it into any chat that can see the
    /// source `.tex` and the generated `.typ`.
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

    /// Convert, compile with typst, and write `<stem>.diagnostics.json`
    /// mapping each typst error back to its LaTeX source fragment + repair
    /// skill. Exits 0 even when the paper has compile errors (the errors are
    /// recorded in the JSON). When typst is absent the .typ is still written
    /// and an empty `[]` diagnostics file is produced.
    ///
    /// Set `BYETEX_TYPST_BIN` to point at a non-default `typst` binary.
    ///
    /// With `--project` (or a directory input) it materialises a self-contained
    /// Typst project (assets, `.bib`, `main.typ`) and diagnoses that.
    Diagnose {
        /// Path to the input `.tex` file, a project directory, or an already-edited
        /// `.typ` file. A `.typ` input is diagnosed IN PLACE — compiled and its typst
        /// errors mapped without re-converting from source — so manual edits survive
        /// (use this to re-scan between fixes). `src_fragment`/`skill_name` are null
        /// for a `.typ` input since there is no LaTeX source map.
        input: PathBuf,
        /// Convert as a project: materialise a self-contained Typst project
        /// (copy assets, preprocess `.bib`, resolve `\input`) before compiling.
        /// Implied when `input` is a directory.
        #[arg(long)]
        project: bool,
        /// Output location. Flat mode: the `.typ` path (default `<stem>.typ`).
        /// Project mode: the output project directory (default
        /// `<entry-stem>.typst-project/`).
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Stage-0 input-validation oracle: compile the INPUT LaTeX with
    /// `tectonic` to confirm the document itself is valid before/around
    /// conversion. This distinguishes "the input is broken" from "ByeTex
    /// has a bug". Writes a `<stem>.doctor.json` sidecar with the verdict.
    ///
    /// If `tectonic` is not on PATH the command skips cleanly (exit 0,
    /// verdict `tectonic_unavailable`) — it never claims the input is
    /// broken when it could not actually check. Set `BYETEX_TECTONIC_BIN`
    /// to point at a non-default `tectonic` binary.
    Doctor {
        /// Path to the input `.tex` file.
        input: PathBuf,

        /// Treat a failed input compile as a hard error (exit code 2).
        /// Without this, a broken input is reported in the sidecar but the
        /// command still exits 0.
        #[arg(long)]
        strict: bool,

        /// Also run the conversion and `typst compile` the output, recording
        /// whether the generated Typst compiles. Lets the verdict separate
        /// `input_broken` from `byetex_bug`.
        #[arg(long)]
        full: bool,
    },

    /// Compile a generated `.typ` to PDF with `typst`, printing STRUCTURED
    /// errors (message + line:col) instead of raw typst stderr. `input` may be
    /// a `.typ` or a `.tex` (converted flat first). Set `BYETEX_TYPST_BIN` to
    /// point at a non-default `typst` binary.
    Compile {
        /// Path to a `.typ` file (or a `.tex`, converted flat first).
        input: PathBuf,
        /// Output PDF path. Defaults to `<input>.pdf`.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Render a generated `.typ` to per-page PNG images at a chosen DPI, for
    /// visual inspection / fidelity grading. `input` may be a `.typ` or a
    /// `.tex` (converted flat first). Prints the page image paths to stdout.
    Render {
        /// Path to a `.typ` file (or a `.tex`, converted flat first).
        input: PathBuf,
        /// Pixels-per-inch for the PNG export. Defaults to 144.
        #[arg(long, default_value_t = 144)]
        dpi: u32,
        /// Output directory for the page PNGs. Defaults to `<input-stem>.pages/`.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Explain a conversion node-by-node: print a JSON array mapping each LaTeX
    /// source fragment to the Typst it produced ("why did this LaTeX emit this
    /// Typst?"). Input is a `.tex` file, `-` for stdin, or `-c <code>`.
    Explain {
        /// Path to a `.tex` file, or `-` to read stdin. Omit when using `-c`.
        input: Option<PathBuf>,
        /// Explain a LaTeX string directly instead of a file.
        #[arg(short = 'c', long = "code", conflicts_with = "input")]
        code: Option<String>,
    },

    /// Build a visual-fidelity grading packet for a paper: render the converted
    /// Typst to per-page PNGs and, when a truth PDF is available, rasterise the
    /// original LaTeX render alongside, then write `grading_packet.json` for the
    /// `byetex-visual-grading` skill. Truth comes from `--truth`, a cached `.pdf`
    /// in the paper dir, or a `tectonic` compile of the source (skipped if none).
    Review {
        /// Path to a `.tex` entry file or a paper directory.
        input: PathBuf,
        /// Reference (truth) PDF to compare against. Defaults to a cached `.pdf`
        /// beside the source, else a `tectonic` compile of it.
        #[arg(long)]
        truth: Option<PathBuf>,
        /// Pixels-per-inch for rasterisation. Defaults to 120.
        #[arg(long, default_value_t = 120)]
        dpi: u32,
        /// Output directory for the packet + page images. Defaults to
        /// `<input-stem>.review/`.
        #[arg(long)]
        out: Option<PathBuf>,
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
            code,
            output,
            project,
            project_out,
            no_toml,
            force,
            no_brief,
            compile,
        } => {
            // Snippet modes (`-c` or `-`) convert in-memory and print Typst to
            // stdout, writing no files.
            if let Some(src) = code {
                run_convert_snippet(&src)
            } else if input.as_deref().map(is_stdin_dash).unwrap_or(false) {
                run_convert_snippet(&read_stdin()?)
            } else if let Some(input) = input {
                let brief_opts = BriefOpts {
                    skip: no_brief,
                    // `convert` is the fast path (no implicit typst spawn) unless
                    // `--compile` is given, in which case it behaves like `agent-brief`.
                    no_compile: !compile,
                };
                let mode = if project {
                    ConvertMode::Project {
                        project_out,
                        no_toml,
                        force,
                    }
                } else {
                    ConvertMode::Flat { output }
                };
                run_convert_dispatch(input, mode, brief_opts)
            } else {
                anyhow::bail!(
                    "provide an input file, `-c <code>`, or `-` to read LaTeX from stdin"
                );
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
            let mode = if project {
                ConvertMode::Project {
                    project_out,
                    no_toml,
                    force,
                }
            } else {
                ConvertMode::Flat { output: None }
            };
            run_convert_dispatch(input, mode, brief_opts)
        }
        Command::Diagnose {
            input,
            project,
            out,
        } => run_diagnose(input, project, out),
        Command::Doctor {
            input,
            strict,
            full,
        } => run_doctor(input, strict, full),
        Command::Compile { input, out } => run_compile(input, out),
        Command::Render { input, dpi, out } => run_render(input, dpi, out),
        Command::Explain { input, code } => run_explain(input, code),
        Command::Review {
            input,
            truth,
            dpi,
            out,
        } => run_review(input, truth, dpi, out),
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

/// Name of the tectonic binary to invoke. Overridable via
/// `BYETEX_TECTONIC_BIN` so users can point at a non-default install and
/// tests can force the "unavailable" path deterministically.
fn tectonic_bin() -> String {
    std::env::var("BYETEX_TECTONIC_BIN").unwrap_or_else(|_| "tectonic".to_string())
}

/// Name of the typst binary to invoke. Overridable via `BYETEX_TYPST_BIN`
/// (same rationale as [`tectonic_bin`]). Used by `doctor --full`.
fn typst_bin() -> String {
    std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".to_string())
}

/// Convert `input`, compile with typst, and write `<stem>.diagnostics.json`
/// that maps each typst error back to its originating LaTeX source fragment
/// and the repair skill (if any warning covers that span).
///
/// Exits 0 in all cases — errors are recorded in the JSON file, not in the
/// exit code. When typst is absent the `.typ` is still written and an empty
/// `[]` diagnostics file is produced.
fn run_diagnose(input: PathBuf, project: bool, out: Option<PathBuf>) -> Result<()> {
    let typst = typst_bin();
    // A directory input, or an explicit --project, goes through the project
    // planner+materialiser (copies assets, preprocesses .bib, resolves \input)
    // so `typst compile` of the produced `main.typ` sees a self-contained tree.
    // A single .tex without --project uses the fast flat path. The orchestration
    // lives in `byetex_core::diagnose` so the MCP `diagnose` tool shares it.
    // A `.typ` input is an already-converted (often agent-edited) file: diagnose it
    // IN PLACE — compile + map errors without re-converting, so edits survive.
    let input_is_typ = input.extension().and_then(|e| e.to_str()) == Some("typ");
    let (typ_path, diags) = if input_is_typ {
        let diags = byetex_core::diagnose::diagnose_typ(&input, &typst)?;
        (input.clone(), diags)
    } else if project || input.is_dir() {
        byetex_core::diagnose::diagnose_project(&input, out.as_deref(), &typst)?
    } else {
        byetex_core::diagnose::diagnose_flat(&input, out.as_deref(), &typst)?
    };
    let diag_path = typ_path.with_extension("diagnostics.json");
    std::fs::write(&diag_path, serde_json::to_string_pretty(&diags)?)
        .with_context(|| format!("write {}", diag_path.display()))?;
    eprintln!(
        "byetex diagnose: {} typst error(s) → {}",
        diags.len(),
        diag_path.display()
    );
    Ok(())
}

/// Doctor sidecar path for an input: `<stem>.doctor.json` alongside it,
/// mirroring how warnings land at `<stem>.warnings.json`.
fn doctor_sidecar_path(input: &Path) -> PathBuf {
    input.with_extension("doctor.json")
}

/// Stage-0 oracle: compile the input LaTeX with tectonic and write a
/// `<stem>.doctor.json` verdict. Skips cleanly when tectonic is absent.
///
/// The orchestration (tectonic/typst shell-outs + verdict) lives in
/// [`byetex_core::validate`] so the MCP `validate` tool shares one code path;
/// the CLI adds the sidecar write, the human-readable messages, and the
/// attribution-driven exit codes on top.
fn run_doctor(input: PathBuf, strict: bool, full: bool) -> Result<()> {
    use byetex_core::validate::Verdict;

    let report = byetex_core::validate::run_doctor(&input, full, &tectonic_bin(), &typst_bin())?;
    let sidecar = doctor_sidecar_path(&input);
    std::fs::write(&sidecar, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("writing {}", sidecar.display()))?;

    match report.verdict {
        Verdict::TectonicUnavailable => eprintln!(
            "byetex doctor: `{}` not found on PATH — skipping input validation. \
             Install Tectonic to enable the Stage-0 oracle. Wrote {}.",
            tectonic_bin(),
            sidecar.display()
        ),
        Verdict::Ok => eprintln!(
            "byetex doctor: input compiles ✅ — wrote {}.",
            sidecar.display()
        ),
        Verdict::ByetexBug => eprintln!(
            "byetex doctor: input compiles but ByeTex's output FAILED `typst compile` ❌ \
             — this is a ByeTex bug. Wrote {}.",
            sidecar.display()
        ),
        Verdict::InputBroken => eprintln!(
            "byetex doctor: input FAILED to compile ❌ — the source itself is broken. Wrote {}.",
            sidecar.display()
        ),
    }

    // Attribution-driven exit codes so callers (corpus sweep, CI) can act:
    //   2 = broken input under --strict (not ByeTex's fault)
    //   3 = valid input but ByeTex produced output that won't compile
    if report.verdict == Verdict::ByetexBug {
        std::process::exit(3);
    }
    if report.input_compiles == Some(false) && strict {
        std::process::exit(2);
    }

    Ok(())
}

/// Compile a generated `.typ` (or flat-convert a `.tex` first) and print the
/// structured typst errors. Exits 0 even on compile errors — they're printed
/// for the agent to read, not signalled via exit code. The PDF is written
/// alongside the `.typ` (or to `--out`).
fn run_compile(input: PathBuf, out: Option<PathBuf>) -> Result<()> {
    let typst = typst_bin();
    let typ = byetex_core::compile::ensure_typ(&input)?;
    let res = byetex_core::compile::compile_typ(&typ, out.as_deref(), &typst)?;
    if res.ok {
        eprintln!(
            "byetex compile: {} → {} ✅",
            typ.display(),
            res.pdf_path.as_deref().unwrap_or("<pdf>")
        );
    } else {
        eprintln!(
            "byetex compile: {} FAILED with {} error(s):",
            typ.display(),
            res.errors.len()
        );
        for e in &res.errors {
            eprintln!("  {}:{}  {}", e.line, e.col, e.message);
        }
    }
    Ok(())
}

/// Render a generated `.typ` (or flat-convert a `.tex` first) to per-page PNGs
/// and print their paths to stdout (one per line) so they can be piped.
fn run_render(input: PathBuf, dpi: u32, out: Option<PathBuf>) -> Result<()> {
    let typst = typst_bin();
    let typ = byetex_core::compile::ensure_typ(&input)?;
    let out_dir = out.unwrap_or_else(|| {
        let stem = typ.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        typ.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("{stem}.pages"))
    });
    let res = byetex_core::compile::render_typ(&typ, &out_dir, dpi, &typst)?;
    if res.ok {
        eprintln!(
            "byetex render: {} → {} page image(s) in {} ✅",
            typ.display(),
            res.image_paths.len(),
            out_dir.display()
        );
    } else {
        eprintln!(
            "byetex render: {} produced {} error(s):",
            typ.display(),
            res.errors.len()
        );
        for e in &res.errors {
            eprintln!("  {}:{}  {}", e.line, e.col, e.message);
        }
    }
    for p in &res.image_paths {
        println!("{p}");
    }
    Ok(())
}

/// True for the `-` sentinel meaning "read from stdin".
fn is_stdin_dash(p: &std::path::Path) -> bool {
    p.as_os_str() == "-"
}

/// Read all of stdin as a UTF-8 string.
fn read_stdin() -> Result<String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("reading LaTeX from stdin")?;
    Ok(buf)
}

/// Convert a LaTeX string in-memory and print the Typst to stdout. Writes no
/// files; a warning count (if any) goes to stderr. Powers `convert -c <code>`
/// and `convert -`.
fn run_convert_snippet(source: &str) -> Result<()> {
    let out = byetex_core::convert(source, &byetex_core::ConvertOptions::default());
    print!("{}", out.typst);
    if !out.typst.ends_with('\n') {
        println!();
    }
    if !out.warnings.is_empty() {
        eprintln!("byetex: {} warning(s)", out.warnings.len());
    }
    Ok(())
}

/// Resolve snippet source from a `--code` string, stdin (`-`), or a file path.
fn resolve_snippet_source(input: Option<PathBuf>, code: Option<String>) -> Result<String> {
    if let Some(src) = code {
        return Ok(src);
    }
    match input {
        Some(p) if is_stdin_dash(&p) => read_stdin(),
        Some(p) => std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display())),
        None => anyhow::bail!("provide a `.tex` file, `-c <code>`, or `-` to read stdin"),
    }
}

/// Explain a conversion node-by-node: print a JSON array mapping each LaTeX
/// source fragment to the Typst it produced ("why did this LaTeX emit this
/// Typst?").
fn run_explain(input: Option<PathBuf>, code: Option<String>) -> Result<()> {
    let source = resolve_snippet_source(input, code)?;
    let ex = byetex_core::snippet::explain(&source, &byetex_core::ConvertOptions::default());
    println!("{}", serde_json::to_string_pretty(&ex)?);
    Ok(())
}

/// Build a visual-fidelity grading packet: materialise the project, render the
/// Typst to per-page PNGs, rasterise a truth PDF (provided / cached / tectonic)
/// alongside, and write `grading_packet.json` for the `byetex-visual-grading`
/// skill. Truth is best-effort — without it the packet is typst-only. Prints
/// the packet path to stdout.
fn run_review(
    input: PathBuf,
    truth: Option<PathBuf>,
    dpi: u32,
    out: Option<PathBuf>,
) -> Result<()> {
    let input_is_dir = input.is_dir();
    let stem = if input_is_dir {
        input.file_name().and_then(|s| s.to_str())
    } else {
        input.file_stem().and_then(|s| s.to_str())
    }
    .unwrap_or("paper")
    .to_string();

    let out_dir = out.unwrap_or_else(|| {
        let parent = if input_is_dir {
            input.clone()
        } else {
            input
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        };
        parent.join(format!("{stem}.review"))
    });
    std::fs::create_dir_all(&out_dir).with_context(|| format!("create {}", out_dir.display()))?;

    // 1) Materialise the Typst project so assets sit next to main.typ.
    let plan = if input_is_dir {
        byetex_core::project::plan_project_from_dir(&input, true, false)?
    } else {
        byetex_core::project::plan_project(&input, true, false)?
    };
    let base_dir = if input_is_dir {
        input.clone()
    } else {
        base_dir_from_file(&plan.entry_tex)
    };
    let proj_dir = out_dir.join("typst-project");
    byetex_core::project::materialize_project(&plan, &proj_dir, &base_dir, true)?;
    let main_typ = proj_dir.join("main.typ");

    // 2) Render the Typst side to per-page PNGs.
    let typst = typst_bin();
    let render =
        byetex_core::compile::render_typ(&main_typ, &out_dir.join("typst-pages"), dpi, &typst)?;
    if !render.ok {
        eprintln!(
            "byetex review: typst render reported {} error(s); packet may be partial.",
            render.errors.len()
        );
    }

    // 3) Resolve + rasterise a truth PDF (best-effort).
    let (truth_pdf, truth_source) = resolve_truth_pdf(
        &input,
        input_is_dir,
        &plan.entry_tex,
        truth.as_deref(),
        &out_dir,
    )?;
    let truth_images = match &truth_pdf {
        Some(pdf) => rasterize_pdf(pdf, &out_dir.join("truth-pages"), dpi)?,
        None => Vec::new(),
    };

    // 4) detected_class from the entry source (best-effort).
    let detected_class = std::fs::read_to_string(&plan.entry_tex)
        .ok()
        .and_then(|s| detect_document_class(&s));

    // 5) Assemble + write the packet.
    let packet = serde_json::json!({
        "id": stem,
        "detected_class": detected_class,
        "truth_source": truth_source,
        "front_matter": {
            "typst": render.image_paths.first(),
            "truth": truth_images.first(),
        },
        "pages": build_page_pairs(&render.image_paths, &truth_images),
        "warnings": {
            "total": plan.warnings.len(),
            "by_kind": warning_kind_counts(&plan.warnings),
        },
        "rubric": "docs/fidelity-rubric.md",
    });
    let packet_path = out_dir.join("grading_packet.json");
    std::fs::write(&packet_path, serde_json::to_string_pretty(&packet)?)
        .with_context(|| format!("write {}", packet_path.display()))?;

    eprintln!(
        "byetex review: {} typst page(s), {} truth page(s) [{}] → {}",
        render.image_paths.len(),
        truth_images.len(),
        truth_source,
        packet_path.display()
    );
    println!("{}", packet_path.display());
    Ok(())
}

/// Resolve a truth (reference) PDF: an explicit `--truth`, else a cached `.pdf`
/// next to the source, else a `tectonic` compile of the entry file. Returns the
/// PDF path and a provenance tag (`provided` | `cached` | `tectonic` | `none`).
fn resolve_truth_pdf(
    input: &Path,
    input_is_dir: bool,
    entry_tex: &Path,
    truth_arg: Option<&Path>,
    out_dir: &Path,
) -> Result<(Option<PathBuf>, &'static str)> {
    if let Some(t) = truth_arg {
        return Ok((Some(t.to_path_buf()), "provided"));
    }
    // A cached reference PDF beside the source.
    let src_dir = if input_is_dir {
        input.to_path_buf()
    } else {
        base_dir_from_file(entry_tex)
    };
    if let Ok(rd) = std::fs::read_dir(&src_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("pdf") {
                return Ok((Some(p), "cached"));
            }
        }
    }
    // Compile the original LaTeX with tectonic (best-effort; absent → no truth).
    let tectonic = std::env::var("BYETEX_TECTONIC_BIN").unwrap_or_else(|_| "tectonic".to_string());
    let status = std::process::Command::new(&tectonic)
        .arg("--outdir")
        .arg(out_dir)
        .arg(entry_tex)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if let Ok(s) = status {
        if s.success() {
            let pdf_stem = entry_tex
                .file_stem()
                .and_then(|x| x.to_str())
                .unwrap_or("main");
            let produced = out_dir.join(format!("{pdf_stem}.pdf"));
            if produced.exists() {
                return Ok((Some(produced), "tectonic"));
            }
        }
    }
    Ok((None, "none"))
}

/// Rasterise `pdf` to per-page PNGs (`truth-N.png`) in `out_dir` via `pdftoppm`
/// at `dpi`. Returns the page image paths in order, or an empty list (with a
/// note) if pdftoppm is unavailable / fails. Override the binary with
/// `BYETEX_PDFTOPPM_BIN`.
fn rasterize_pdf(pdf: &Path, out_dir: &Path, dpi: u32) -> Result<Vec<String>> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let bin = std::env::var("BYETEX_PDFTOPPM_BIN").unwrap_or_else(|_| "pdftoppm".to_string());
    let status = std::process::Command::new(&bin)
        .arg("-png")
        .arg("-r")
        .arg(dpi.to_string())
        .arg(pdf)
        .arg(out_dir.join("truth"))
        .status();
    match status {
        Ok(s) if s.success() => Ok(collect_numbered_pngs(out_dir, "truth")),
        _ => {
            eprintln!("byetex review: `{bin}` unavailable or failed; truth pages skipped.");
            Ok(Vec::new())
        }
    }
}

/// Collect `<prefix>-<N>.png` files in `dir`, sorted numerically by `N`
/// (pdftoppm may zero-pad, so parse the trailing number rather than sort
/// lexically).
fn collect_numbered_pngs(dir: &Path, prefix: &str) -> Vec<String> {
    let mut pages: Vec<(u32, PathBuf)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("png") {
                continue;
            }
            if let Some(n) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix(prefix))
                .and_then(|s| s.trim_start_matches('-').parse::<u32>().ok())
            {
                pages.push((n, path));
            }
        }
    }
    pages.sort_by_key(|(n, _)| *n);
    pages
        .into_iter()
        .map(|(_, p)| p.display().to_string())
        .collect()
}

/// Pair typst and truth page images by index into `{ page, typst?, truth? }`
/// rows (page is 1-based). The longer side governs the row count.
fn build_page_pairs(typst: &[String], truth: &[String]) -> Vec<serde_json::Value> {
    let n = typst.len().max(truth.len());
    (0..n)
        .map(|i| {
            serde_json::json!({
                "page": i + 1,
                "typst": typst.get(i),
                "truth": truth.get(i),
            })
        })
        .collect()
}

/// Count warnings by category kind for the packet's `warnings.by_kind` summary.
fn warning_kind_counts(
    warnings: &[byetex_core::Warning],
) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for w in warnings {
        *counts.entry(category_kind_name(&w.category)).or_insert(0) += 1;
    }
    counts
}

/// Extract the class name from the first `\documentclass[opts]{name}` in `src`.
fn detect_document_class(src: &str) -> Option<String> {
    let idx = src.find("\\documentclass")?;
    let mut rest = src[idx + "\\documentclass".len()..].trim_start();
    if let Some(stripped) = rest.strip_prefix('[') {
        let close = stripped.find(']')?;
        rest = stripped[close + 1..].trim_start();
    }
    let rest = rest.strip_prefix('{')?;
    let close = rest.find('}')?;
    Some(rest[..close].trim().to_string())
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
        byetex_core::project::plan_project_from_dir(&input, no_toml, false)
            .with_context(|| format!("planning project from {}", input.display()))?
    } else {
        byetex_core::project::plan_project(&input, no_toml, false)
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
        ConvertMode::Flat { output } => run_flat(
            input,
            plan,
            &default_stem,
            parent_for_outputs,
            output,
            input_is_dir,
            brief,
        ),
        ConvertMode::Project {
            project_out,
            no_toml: _,
            force,
        } => run_project(
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
    let warnings_json =
        serde_json::to_string_pretty(&plan.warnings).with_context(|| "serialising warnings")?;
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
        // Use forward slashes unconditionally — the brief is a Markdown file
        // consumed by humans and LLMs, not the OS path resolver. Native
        // backslashes on Windows make the brief non-portable.
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
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
            std::fs::create_dir_all(parent)
                .with_context(|| format!("ensuring brief directory {}", parent.display()))?;
        }
    }

    let brief_dir = brief_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Read the .typ for template-detection only (first 8 lines are enough).
    let typ = std::fs::read_to_string(typst_path).unwrap_or_default();
    let warnings_text = std::fs::read_to_string(warnings_path).unwrap_or_else(|_| "[]".to_string());

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
                // First non-null `suggested_skill` seen per category, so the
                // histogram tells the agent which skill to read for each kind.
                let mut skill_of: std::collections::BTreeMap<String, String> =
                    std::collections::BTreeMap::new();
                for w in &arr {
                    let kind = w
                        .get("category")
                        .and_then(|c| c.get("kind"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    *counts.entry(kind.clone()).or_default() += 1;
                    if let Some(s) = w.get("suggested_skill").and_then(|s| s.as_str()) {
                        skill_of.entry(kind).or_insert_with(|| s.to_string());
                    }
                }
                let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                let histogram = if sorted.is_empty() {
                    "(none)".to_string()
                } else {
                    sorted
                        .iter()
                        .map(|(k, c)| match skill_of.get(k) {
                            Some(s) => format!("  - {k}: {c} → {s}"),
                            None => format!("  - {k}: {c}"),
                        })
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
            "(not run — pass `byetex convert --compile` to capture the typst log)".to_string()
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

    // If a diagnostics sidecar already sits next to the `.typ` (e.g. the paper
    // was produced via `byetex diagnose`), link it so the agent can use the
    // error→fragment→skill mapping directly.
    let diag_path = typst_path.with_extension("diagnostics.json");
    let diag_note = if diag_path.exists() {
        format!(
            "\nA mapped error→fragment→skill list is already in `{}`.",
            relativize(&diag_path, &brief_dir)
        )
    } else {
        String::new()
    };

    let brief = format!(
        r#"# ByeTex agent brief: `{stem}` ({mode_label} mode)

> **Start here:** run `byetex skills read byetex-getting-started` (or read `AGENTS.md` in the repo) for the repair workflow.

**Task** — Read `{src_rel}` (source) and `{typ_rel}` (current Typst output).
Write a patched copy to **`{manual_rel}`** that compiles cleanly with
`typst compile {typ_rel}`. Apply the smallest possible local edits per
compile error; preserve what already works.

## How to repair
Run `byetex diagnose {src_rel}` to map each typst error to its LaTeX fragment +
repair skill (`{stem}.diagnostics.json`). For each: read the named skill with
`byetex skills read <skill_name>`, edit `{typ_rel}`, then re-run
`typst compile {typ_rel}`. To re-scan AFTER edits, run `byetex diagnose {typ_rel}`
(pass the `.typ`) — it diagnoses the edited file IN PLACE and does not overwrite it
(`src_fragment`/`skill_name` are null, since there's no source map for an edited
file). Do NOT re-run `byetex diagnose {src_rel}` (the source) between edits — that
re-converts and overwrites your work. Full procedure:
`byetex skills read byetex-repair-loop`.{diag_note}

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
        diag_note = diag_note,
    );

    std::fs::write(brief_path, brief)
        .with_context(|| format!("writing {}", brief_path.display()))?;
    eprintln!("wrote {}", brief_path.display());
    Ok(())
}
