# A beginner's tour of ByeTex

## The problem in one paragraph

LaTeX is the standard for typesetting academic papers, but the language is fussy: you write things like `\textbf{hello}` for bold, install a TeX distribution, debug arcane errors. **Typst** is a newer tool that does the same job with a simpler language — `*hello*` for bold, a single static binary to install, errors that point at the right line. People with LaTeX documents want to migrate to Typst, but doing it by hand is tedious. **ByeTex** is a translator: feed it a `.tex` file, get back a `.typ` file. It works best on academic papers today — that's where its fidelity is tuned — and anything ByeTex doesn't understand gets flagged so a human (or an AI) can finish the job.

---

## A taste of each language

**LaTeX** (the input):
```latex
\section{Introduction}
This is \emph{important} and $E = mc^2$.
\begin{itemize}
\item First point.
\item Second point.
\end{itemize}
```

**Typst** (the output):
```typst
= Introduction
This is _important_ and $E = m c^2$.
- First point.
- Second point.
```

Notice the patterns: `\section{X}` → `= X`, `\emph{X}` → `_X_`, math is `$...$` in both, lists become `-` bullets. ByeTex's job is to apply rules like these everywhere.

**Rust** (what we wrote the converter in): a compiled systems language like C++, but safer. We picked it because the output is a small, fast, standalone binary — no Python interpreter or Node runtime to install on the user's machine. The same binary runs on macOS, Linux, and Windows.

---

## How the converter actually works

A converter has three stages. Imagine reading a page of LaTeX with a highlighter:

1. **Parse** — break the text into a tree of pieces ("this is a section, its title is `Introduction`, its body has these paragraphs"). We don't write this ourselves; we use **tree-sitter-latex**, a parser library originally written for code editors. It's the same library that powers syntax highlighting in VS Code and Neovim for LaTeX files. Tree-sitter gives us a tree like:

   ```
   source_file
   ├── section
   │   ├── \section
   │   ├── curly_group "{Introduction}"
   │   └── text "This is ..."
   └── ...
   ```

2. **Walk the tree** — visit every node and decide what to emit. For a `section` node, emit `=` + the title. For a `generic_command` named `\textbf`, emit `*` + content + `*`. For something we don't recognize (`\tikzpicture`, `\marginpar`, whatever) — emit a **warning** in a sidecar JSON file, and either drop or pass through the text.

3. **Write the output** — one `.typ` file with the Typst code, plus a `.warnings.json` file listing everything that needed human attention.

The walk lives in `crates/byetex-core/src/emit.rs` — that's the file with the giant `match node.kind() { ... }` that does the translation. It's like a big lookup table: see this LaTeX shape, emit that Typst shape.

---

## What tree-sitter is (the 30-second version)

Tree-sitter takes a grammar (rules for what LaTeX looks like, written in JavaScript) and generates a parser in C. The generated C file is ~42 MB of state tables — that's why our `crates/byetex-core/vendor/tree-sitter-latex/src/parser.c` is huge. We compile it into our Rust binary at build time using a small `build.rs` script. So the user just downloads one file (`byetex`) and it has the whole LaTeX parser inside.

---

## The project structure

```
ByeTex/
├── Cargo.toml                              ← Rust workspace config
├── README.md                               ← human-facing intro
│
├── crates/                                 ← two Rust libraries
│   ├── byetex-core/                       ← the brain
│   │   ├── src/
│   │   │   ├── lib.rs                      ← public `convert()` function
│   │   │   ├── parser.rs                   ← wraps tree-sitter
│   │   │   ├── emit.rs                     ← the big LaTeX→Typst translator
│   │   │   ├── warnings.rs                 ← shape of warnings.json
│   │   │   └── skills.rs                   ← embedded help docs
│   │   ├── build.rs                        ← compiles parser.c + embeds skills
│   │   ├── vendor/tree-sitter-latex/       ← the 42 MB grammar (vendored)
│   │   └── tests/                          ← unit + integration tests
│   │
│   └── byetex-cli/                        ← the `byetex` command-line tool
│       └── src/main.rs                     ← subcommands: convert, diagnose, doctor,
│                                              compile, render, review, explain, skills, corpus
│
├── skills/                                 ← markdown how-to files for humans/AIs
│   ├── byetex-using-warnings-json/SKILL.md  ← read this first
│   ├── byetex-tikz-to-typst/SKILL.md        ← how to rewrite a TikZ diagram
│   └── ... (12 more; see skills/INDEX.md)
│
├── docs/
│   ├── for-agents.md                       ← entry doc for AI assistants
│   └── warnings.schema.json                ← JSON Schema for the sidecar
│
├── tests/fixtures/                         ← small .tex → .typ examples used in tests
│   ├── m1_passthrough/                     ← plain text
│   ├── m2_sectioning/                      ← \section, lists, formatting
│   ├── m3_math/                            ← $x = y^2$, matrices, \frac
│   └── m4_floats/                          ← tables, figures, citations
│
├── corpus/manifest.json                    ← committed manifest of known arXiv papers
│   (5 marked pinned:true are the regression set; payloads gitignored)
│
├── context/                                ← scraped LaTeX/Typst reference docs
│   │   (gitignored — generated locally, not in a fresh clone)
│   ├── latex-context.md
│   └── typst-context.md
│
└── .github/workflows/                      ← CI pipelines on GitHub
    ├── ci.yml                              ← test on every push
    └── release.yml                         ← build binaries when you tag a version
```

**The mental model**: two Rust crates (libraries) in one workspace. `core` does the conversion. `cli` is what users (and AI agents) run. The `skills/` folder has human-readable repair instructions; `tests/fixtures/` has small targeted snippets; `corpus/manifest.json` lists real arXiv papers for regression testing (payloads gitignored, fetched by `scripts/corpus_harvest.py`).

---

## How to use it (the everyday path)

### 1. Convert a paper

```bash
byetex convert paper.tex
```

This writes two files next to the input:
- `paper.typ` — your Typst document
- `paper.warnings.json` — a list of things ByeTex couldn't fully translate

Then compile:

```bash
typst compile paper.typ
```

You get `paper.pdf`. Done — assuming the conversion was clean.

### 2. When there are warnings

Look at the sidecar:

```bash
cat paper.warnings.json | jq '.[].category.kind' | sort | uniq -c
```

You might see:
```
   3 tikz
   8 unsupported_command
   1 parse_error
```

Each warning has a `suggested_skill` field pointing to a markdown file. Read it:

```bash
byetex skills read byetex-tikz-to-typst
```

That tells you how to manually rewrite TikZ diagrams in Typst's CeTZ library. Apply the fix to `paper.typ`, re-compile.

### 3. From an AI assistant

ByeTex is built to be driven by an AI coding agent (Claude Code, Cursor, etc.) straight through its CLI. The agent runs `byetex convert` to translate your paper, `byetex diagnose` to map compile errors to repair skills, and `byetex skills read <name>` to look them up — then patches the `.typ` for you. The 14 bundled skills are also available as a Claude Code plugin (`/byetex:<name>`); see [`plugin-setup.md`](plugin-setup.md). The same machinery you'd use by hand, driven by the agent.

### 4. Building from source

If you cloned the repo:

```bash
cargo build --release
# → target/release/byetex (single binary, ~7 MB)
```

Run the test suite:

```bash
cargo test --workspace
# ~1,300 tests: golden snapshots + corpus check + compile check + schema lock
```

Try a real arXiv paper:

```bash
python scripts/corpus_harvest.py --pinned   # fetch the 5 regression papers (~25 MB)
./target/release/byetex convert corpus/2605.22507/source/0-main.tex
typst compile corpus/2605.22507/source/0-main.typ corpus/2605.22507/source/0-main.pdf
open corpus/2605.22507/source/0-main.pdf
```

That's a real cs.LG paper — the LaTeX source becomes a compilable Typst doc with ~32 things flagged for manual review.

---

## The four key files to read if you want to understand the code

1. **`crates/byetex-core/src/lib.rs`** (~200 lines) — the public API: `convert(source, opts) -> ConvertOutput { typst, warnings, … }`.

2. **`crates/byetex-core/src/warnings.rs`** (~50 lines) — what a warning *is*. A `Range`, a `Category`, a message, a snippet, a suggested skill name. This shape is locked by a test so it can't drift.

3. **`crates/byetex-core/src/emit.rs`** (~5,800 lines, plus 15 `emit/` submodules) — the actual translation logic. Big but pattern-rich. Reading the top-level `emit_node` function tells you everything the converter handles: math containers, section commands, generic commands, environments, etc. Everything else in the file is a helper for one of those.

4. **`crates/byetex-cli/src/main.rs`** (~1,550 lines) — the CLI. Just argument parsing (via the `clap` library) and calls into `byetex-core`. Good first place to land if you want to add a new subcommand.

---

## What ByeTex is not

- **Not** a full LaTeX engine. We translate the *structure* rather than running TeX. Custom macros (`\newcommand`/`\def`) *are* pre-scanned and expanded, but arbitrary TeX programming (`\ifthenelse`, category-code tricks, `\makeatletter` internals) is not.
- **Not** a perfect 1-to-1 mapping. Some LaTeX idioms have no Typst equivalent: TikZ pictures become a marked placeholder, `.eps` figures become a grey box (Typst can't read EPS), and page density often differs from the original.
- **Not** a black box. Every conversion is deterministic and reproducible; the same input always produces the same output. The warnings tell you exactly what's unfinished.

That's the whole picture — a Rust binary that uses a tree-sitter grammar to parse LaTeX, walks the tree applying translation rules, emits Typst code, and writes a JSON file listing the cases that need human follow-up.
