# ByeTex v0.3.0 — Agent in the loop

ByeTex converts LaTeX to Typst for AI agents. v0.3.0 is the **"agent in the loop"** release:
the MCP server grows from 7 to **11 tools**, the visual-fidelity loop becomes one command,
and ByeTex now installs as a Claude Code plugin (plus an install script, crates.io, and Homebrew).

## Highlights

### 11 MCP tools (was 7) — with full CLI parity
- **`validate`** — Stage-0 oracle: compile the *input* LaTeX with tectonic to tell a broken
  source from a ByeTex bug, before you start repairing.
- **`compile`** / **`render`** — `typst compile` to PDF / per-page PNGs with **structured**
  errors (`{ok, errors, …}`) — no more shelling out to `typst` and scraping stderr.
- **`explain`** — a per-node LaTeX→Typst map: "why did this LaTeX emit this Typst?"
- **`convert_fragment`** now honours its `context_hint` (math hints wrap the fragment so bare
  math like `\frac{1}{2}` converts as math, not an unknown text command).

### One-command visual-fidelity loop
- **`byetex review <paper>`** renders the converted Typst to per-page PNGs and rasterises the
  original LaTeX render alongside, emitting a `grading_packet.json` for the
  `byetex-visual-grading` skill.
- A **two-layer fidelity regression gate**: deterministic structural metrics
  (`scripts/fidelity_gate.sh`) + a vision-graded findings diff (`scripts/findings_diff.py`).

### Reproducible snippet primitives
- `byetex convert -c '\frac12'` (or `-` for stdin) — convert a snippet to stdout, no files.
- `byetex explain -c '…'` — the source map for any fragment.

### Installs four ways
```bash
# Claude Code plugin (skills + MCP server)
claude plugin marketplace add zeyuyang42/ByeTex && claude plugin install byetex@byetex
# Prebuilt binary
curl -fsSL https://raw.githubusercontent.com/zeyuyang42/ByeTex/main/install.sh | sh
# crates.io
cargo install byetex --features mcp
# Homebrew
brew install zeyuyang42/byetex/byetex
```

## Status
- **Compile-rate (the gate): 59/59** ByeTex-attributable arXiv papers compile (100%).
- **Visual fidelity (the driver): 0.821** corpus composite score.
- CI runs `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test --workspace` (904 tests).

## Notes
- The CLI crate is now published as **`byetex`** (so `cargo install byetex` works); the binary
  name and the `crates/byetex-cli` directory are unchanged.
- No conversion-logic changes in this release — it is entirely new agent tooling, packaging,
  and docs, so the 59/59 compile-rate is preserved.
- Full changelog: [CHANGELOG.md](CHANGELOG.md).

**Full Changelog**: https://github.com/zeyuyang42/ByeTex/compare/v0.2.0...v0.3.0
