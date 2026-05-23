# A beginner's tour of ByeTex

## The problem in one paragraph

LaTeX is the standard for typesetting academic papers, but the language is fussy: you write things like `\textbf{hello}` for bold, install a TeX distribution, debug arcane errors. **Typst** is a newer tool that does the same job with a simpler language — `*hello*` for bold, a single static binary to install, errors that point at the right line. People with LaTeX papers want to migrate to Typst, but doing it by hand is tedious. **ByeTex** is a translator: feed it a `.tex` file, get back a `.typ` file. Anything ByeTex doesn't understand gets flagged so a human (or an AI) can finish the job.

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

The walk lives in `crates/bytetex-core/src/emit.rs` — that's the file with the giant `match node.kind() { ... }` that does the translation. It's like a big lookup table: see this LaTeX shape, emit that Typst shape.

---

## What tree-sitter is (the 30-second version)

Tree-sitter takes a grammar (rules for what LaTeX looks like, written in JavaScript) and generates a parser in C. The generated C file is ~42 MB of state tables — that's why our `crates/bytetex-core/vendor/tree-sitter-latex/src/parser.c` is huge. We compile it into our Rust binary at build time using a small `build.rs` script. So the user just downloads one file (`bytetex`) and it has the whole LaTeX parser inside.

---

## The project structure

```
ByeTex/
├── Cargo.toml                              ← Rust workspace config
├── README.md                               ← human-facing intro
│
├── crates/                                 ← three Rust libraries
│   ├── bytetex-core/                       ← the brain
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
│   ├── bytetex-cli/                        ← the `bytetex` command-line tool
│   │   └── src/main.rs                     ← subcommands: convert, skills, serve, corpus
│   │
│   └── bytetex-mcp/                        ← lets AI agents call ByeTex over a protocol
│       └── src/lib.rs                      ← exposes 5 "tools" over stdio JSON-RPC
│
├── skills/                                 ← markdown how-to files for humans/AIs
│   ├── bytetex-using-warnings-json.md      ← read this first
│   ├── bytetex-tikz-to-typst.md            ← how to rewrite a TikZ diagram
│   └── ... (4 more)
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
├── tests/inhouse/                          ← committed regression templates
│   ├── ieee/conference_101719.tex          ← IEEE conference paper
│   ├── acm/sample-sigconf.tex              ← ACM SIGCONF paper
│   ├── neurips/neurips_paper.tex           ← NeurIPS-style paper
│   └── thesis/thesis_skeleton.tex          ← thesis skeleton
│
├── context/                                ← scraped LaTeX/Typst docs (495 examples)
│   ├── latex-context.md
│   └── typst-context.md
│
└── .github/workflows/                      ← CI pipelines on GitHub
    ├── ci.yml                              ← test on every push
    └── release.yml                         ← build binaries when you tag a version
```

**The mental model**: three Rust crates (libraries) in one workspace. `core` does the conversion. `cli` is what users run. `mcp` is for AI tools. The `skills/` folder has human-readable repair instructions; `tests/inhouse/` has committed full-paper regression templates; `tests/fixtures/` has small targeted snippets. The gitignored `corpus/` holds the broader downloaded test corpus.

---

## How to use it (the everyday path)

### 1. Convert a paper

```bash
bytetex convert paper.tex
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
bytetex skills read bytetex-tikz-to-typst
```

That tells you how to manually rewrite TikZ diagrams in Typst's CeTZ library. Apply the fix to `paper.typ`, re-compile.

### 3. From an AI assistant (the MCP path)

Start ByeTex as a server:

```bash
bytetex serve
```

Now Claude Code, Cursor, etc. can call five "tools" — `convert`, `convert_file`, `convert_fragment`, `list_skills`, `read_skill` — and the AI uses these to convert your paper, read warnings, look up skills, and patch the `.typ` for you. The same machinery you'd use by hand, but exposed as a protocol.

### 4. Building from source

If you cloned the repo:

```bash
cargo build --release
# → target/release/bytetex (single binary, ~7 MB)
```

Run the test suite:

```bash
cargo test --workspace
# 29 golden tests + corpus check + compile check + MCP smoke + schema lock
```

Try a real template:

```bash
python scripts/setup_corpus.py
./target/release/bytetex convert corpus/inhouse/ieee/conference_101719.tex
typst compile corpus/inhouse/ieee/conference_101719.typ
open corpus/inhouse/ieee/conference_101719.pdf
```

That's the IEEE conference paper template — 288 lines of LaTeX become a compilable Typst doc with 16 things flagged for manual review.

---

## The four key files to read if you want to understand the code

1. **`crates/bytetex-core/src/lib.rs`** (~30 lines) — the public API. Just shows `convert(source) -> (typst, warnings)`. The whole thing.

2. **`crates/bytetex-core/src/warnings.rs`** (~50 lines) — what a warning *is*. A `Range`, a `Category`, a message, a snippet, a suggested skill name. This shape is locked by a test so it can't drift.

3. **`crates/bytetex-core/src/emit.rs`** (~1000 lines) — the actual translation logic. Big but pattern-rich. Reading the top-level `emit_node` function tells you everything the converter handles: math containers, section commands, generic commands, environments, etc. Everything else in the file is a helper for one of those.

4. **`crates/bytetex-cli/src/main.rs`** (~250 lines) — the CLI. Just argument parsing (via the `clap` library) and calls into `bytetex-core`. Good first place to land if you want to add a new subcommand.

---

## What ByeTex is not

- **Not** a full LaTeX engine. We translate the *structure*, not run TeX macros. `\def\foo{...}` and custom commands fall outside our scope and become warnings.
- **Not** a perfect 1-to-1 mapping. Some LaTeX idioms have no Typst equivalent (page-level layout primitives, certain `\verb` corner cases, exotic packages like TikZ).
- **Not** a black box. Every conversion is deterministic and reproducible; the same input always produces the same output. The warnings tell you exactly what's unfinished.

That's the whole picture — a Rust binary that uses a tree-sitter grammar to parse LaTeX, walks the tree applying translation rules, emits Typst code, and writes a JSON file listing the cases that need human follow-up.
