# Changelog

Notable changes to ByeTex. Format loosely follows
[Keep a Changelog](https://keepachangelog.com); versions follow semver.

## [0.4.2] — unreleased

Autonomous-dev cycle: a self-improving loop that raises converter fidelity and
hardens the agent surface, dogfooded by a fresh model each tick. Highlights below.

### Added
- **`byetex diagnose <file.typ>`** — diagnose an already-edited Typst file IN PLACE
  (compile + map errors, no re-convert), so an agent's fixes survive; wired into the
  CLI, the MCP `diagnose` tool, and the agent_brief / repair-loop guidance.
- **ACL venue page geometry** — auto-detect `acl.sty` and apply a4 paper, 2.5cm
  margins and a 10pt body (the values the venue forces over the class options).
- **`tcolorbox` recipe** — a reusable `#block`-based translation in the
  `byetex-unsupported-environment` skill, plus `needs_manual_review` skill routing.
- **overset family** — `\overset`/`\underset`/`\stackrel`/`\accentset` → Typst
  `attach(base, t|b: script)` (previously dropped both args).

### Fixed
- array `>{…}` column-decorator alignment now propagates to the Typst column.
- Numerous body-leak bugs: `\footnotemark[N]`, heading underscore `\label`, TeX
  register/penalty `=NNNN` tails, `\ExplSyntaxOn…\ExplSyntaxOff` expl3 regions,
  `\setminted`/counter-command arguments.
- `figure*` / `table*` now span both columns in two-column layouts.
- `byetex-math` skill: `op(..., limits: #true)` (bare `true` doesn't compile) +
  `chevron.l/.r` over deprecated `angle.l/.r`; `byetex-getting-started` gained a
  fidelity-phase section.
- `algorithm` floats now render their `algorithmic` pseudocode instead of an empty
  placeholder.
- `\abstract{…}` **command** form (class-redefined, e.g. bytedance) is now captured
  like the `abstract` environment instead of being dropped.
- Author-block marker leak: `\footnotemark[N]` (and other `\cmd[opt]` markers) no
  longer leak their `[N]` as literal text next to author names.

## [0.3.0] — 2026-06-15

Agent-in-the-loop hardening + distribution. The MCP server grows from 7 to
**11 tools**, the visual-fidelity loop becomes one command, and ByeTex ships as a
Claude Code plugin.

### Added
- **Agent tools (7 → 11), MCP + CLI parity:** `validate` (Stage-0 input oracle),
  `compile` (typst → PDF with structured errors), `render` (→ per-page PNGs),
  `explain` (per-node LaTeX → Typst map).
- `byetex convert -c <code>` / `-` (stdin) — convert a snippet to stdout, no files.
- `convert_fragment` now honours its `context_hint` (math hints wrap the fragment
  so bare math converts as math, not an unknown text command).
- `byetex review <paper>` — one-command visual grading packet (truth↔typst page
  images) for the `byetex-visual-grading` skill.
- Two-layer fidelity regression gate: `scripts/fidelity_gate.sh` (deterministic
  structural metrics) + `scripts/findings_diff.py` (vision-graded findings).
- **Claude Code plugin:** `.claude-plugin/plugin.json` + `marketplace.json`,
  `.mcp.json`, and a SessionStart hook; skills restructured to
  `skills/<name>/SKILL.md`.
- Distribution: `install.sh`, a Homebrew formula (`packaging/byetex.rb`), and
  crates.io metadata.

### Changed
- CLI package renamed `byetex-cli` → `byetex` so `cargo install byetex` works
  (binary name and the `crates/byetex-cli` directory are unchanged).
- `byetex_core` gains shared `validate`, `compile`, and `snippet` modules,
  mirroring `diagnose` (pure logic + an orchestrator both surfaces share).

## [0.2.0] · [0.1.0]

See the git history (`git log v0.1.0..v0.2.0`). Highlights: the hand-rolled
LaTeX → Typst core, project mode, the `diagnose` repair loop + skills, per-class
fidelity, and the corpus reaching 100 % compile.
