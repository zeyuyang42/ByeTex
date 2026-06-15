# Contributing to ByeTex

Thanks for your interest! ByeTex is a Rust workspace — `byetex-core` (the pure
converter), `byetex` (the CLI), and `byetex-mcp` (the MCP server) — that turns
LaTeX into Typst for AI agents.

## Setup

- Rust 1.84+ (`rust-toolchain.toml` pins stable). Build with `cargo build --workspace`.
- `typst` and `tectonic` on PATH enable the compile / visual tests; they skip
  gracefully when absent.

## Workflow

- **Branch per change** — a git worktree per feature/fix is recommended.
- **TDD** — write a failing test first, watch it fail, then implement.
- **`cargo test --workspace` must pass** before opening a PR.
- **Gates before merge:**
  - `./scripts/acceptance.sh` — compile-rate gate; must not regress a
    known-passing corpus paper.
  - `./scripts/fidelity_gate.sh` — for fidelity-affecting changes; flags render
    regressions vs `scripts/fidelity_baseline.json`.
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
  keep CI green.

## Corpus

The arXiv corpus is gitignored except `corpus/manifest.json`. Fetch the pinned
set with `python scripts/corpus_harvest.py --pinned`. See [CLAUDE.md](CLAUDE.md)
for the corpus sweep and acceptance details.

## Adding a skill

Skills live in `skills/<name>/SKILL.md` (Claude plugin format) and are embedded
into the binary at build time. Add a directory with a `SKILL.md` (YAML
frontmatter `name` + `description`), and point a warning's `suggested_skill` at
it where relevant. `byetex skills list` should then show it.

## License

By contributing you agree your work is dual-licensed under MIT OR Apache-2.0.
