# Manual test plan — end-to-end user walkthrough

This document is what you execute by hand to validate ByeTex from a real user's
perspective. Budget: ~45 minutes if everything works on the first try (~30 min
without the corpus stress-test in Scenario D).

Each scenario has explicit **expected output** lines. If yours diverges, that's
a finding worth recording. Stop and report.

---

## Prerequisites

You need on your machine:

- `bash` or `zsh`
- A working `git` and `gh` (GitHub CLI, authenticated)
- `cargo` (Rust toolchain ≥ 1.85). Verify: `rustc --version`
- `typst` CLI (any 0.14+). Verify: `typst --version`
- `jq` for inspecting `warnings.json` (optional but recommended)
- `python3` ≥ 3.10 (only needed for Scenario D)
- About 200 MB free disk for the build, plus ~50–200 MB for the harvested corpus

If any of those is missing, install before continuing. The rest of the plan
assumes they're present.

**Corpus prerequisite (only for Scenario D):** the LaTeX template harvester
described in `~/.claude/plans/i-want-to-download-flickering-mango.md` populates
a top-level `templates/` directory with real-world templates from
latextemplates.com and recent arXiv papers. If you plan to run Scenario D,
let the harvester finish first. The rest of the scenarios work without it.

```bash
# Check whether the corpus is ready:
ls -la templates/latextemplates/ templates/arxiv/ 2>/dev/null && \
  jq '.entries | length' templates/manifest.json 2>/dev/null
```
Expected: at least one entry in `manifest.json` (the small-batch run produces
~5; the large-batch run ~45). If the directory doesn't exist, Scenario D
falls back to "bring your own paper".

---

## Scenario A — Get the binary

You have three options. Pick **one** — you don't need all three.

### A1. Build from the source tree you already have

```bash
cd ~/Workspace/tools/ByeTex          # adjust path
git fetch && git checkout v0.2.0     # ensure you're on the tagged commit
cargo build --release -p bytetex-cli --features mcp
```

**Expected**: `target/release/bytetex` exists, ~7 MB.

```bash
./target/release/bytetex --version
```

**Expected**: `bytetex 0.0.1` (Cargo workspace version — the GitHub tag is what
gates release, not the crate version field).

### A2. Download from GitHub Releases

```bash
gh release view v0.2.0 --repo zeyuyang42/ByeTeX
```

**Expected**: the v0.2.0 release page with 5 platform asset tarballs. If the
release.yml workflow is still running, check back in a few minutes:

```bash
gh run list --workflow=release.yml --repo zeyuyang42/ByeTeX --limit 3
```

Once the assets are present, pick yours (macOS arm64 in your case):

```bash
mkdir -p /tmp/bytetex-test && cd /tmp/bytetex-test
gh release download v0.2.0 --repo zeyuyang42/ByeTeX --pattern '*aarch64-apple-darwin*'
tar -xzf bytetex-*aarch64-apple-darwin*.tar.gz
ls bytetex-*aarch64-apple-darwin*/
```

**Expected**: a directory containing `bytetex` (the binary) and `skills/`.

```bash
./bytetex-*aarch64-apple-darwin*/bytetex --version
```

Set a convenience symlink so the rest of this document's `bytetex` references work:

```bash
ln -sf $(pwd)/bytetex-*aarch64-apple-darwin*/bytetex /usr/local/bin/bytetex
which bytetex
bytetex --version
```

### A3. `cargo install` from the GitHub repo

```bash
cargo install --git https://github.com/zeyuyang42/ByeTeX --tag v0.2.0 bytetex-cli --features mcp
which bytetex
bytetex --version
```

**Expected**: install completes (3–5 min), `bytetex` is on `PATH`.

---

## Scenario B — Convert a known-good template

Use the IEEE template that ships in the repo. This is the canary: if this
fails, something's wrong with the binary itself.

```bash
cd ~/Workspace/tools/ByeTex          # repo root, adjust path
bytetex convert templates/IEEE/conference_101719.tex
```

**Expected output**:
```
wrote templates/IEEE/conference_101719.typ (20 warnings)
```

The exact count may shift by a few if you're on a later commit; **anything
under 25 is fine**.

```bash
ls templates/IEEE/conference_101719.{typ,warnings.json}
```

Both files should be present and non-empty.

```bash
typst compile templates/IEEE/conference_101719.typ
ls -lh templates/IEEE/conference_101719.pdf
```

**Expected**: a PDF in the ~100 KB range. Open it:

```bash
open templates/IEEE/conference_101719.pdf
```

**Pass criteria**:
- The PDF renders without errors.
- The title appears at the top (centered, bold).
- Section headings are numbered (1, 1.1, 1.2, ...).
- Equation references like "Equation (1)" link to the right equation.
- Citations like `[1]` link to the bibliography section.
- The figure box is empty (no `fig1.png` referenced in the working dir — that's
  expected and would be picked up as an info warning in a real workflow).

**Fail signals**: typst exits non-zero, the PDF is blank, references are
broken (`?? ` placeholders), or the title block doesn't appear.

---

## Scenario C — Inspect warnings and look up skills

Same converted document, now inspect what ByeTex flagged.

```bash
jq '[.[].category.kind] | group_by(.) | map({kind: .[0], count: length}) | sort_by(-.count)' \
   templates/IEEE/conference_101719.warnings.json
```

**Expected**: a histogram dominated by `unsupported_command` (IEEE-class
specific stuff like `\IEEEauthorblockN`).

Look at a single warning to see the shape:

```bash
jq '.[0]' templates/IEEE/conference_101719.warnings.json
```

**Expected**: a JSON object with `range`, `category`, `severity`, `message`,
`snippet`, `suggested_skill`. Confirm `range` has `start_line`, `start_col`,
etc. — that's the field that lets agents jump to the right source location.

List the bundled skills:

```bash
bytetex skills list
```

**Expected**: 6 entries, each with name + one-line description:
- `bytetex-using-warnings-json`
- `bytetex-tikz-to-typst`
- `bytetex-custom-macros`
- `bytetex-unsupported-environment`
- `bytetex-parse-error`
- `bytetex-bibliography`

Read the entry-point skill:

```bash
bytetex skills read bytetex-using-warnings-json | head -30
```

**Expected**: the markdown frontmatter (`name:` / `description:`) followed by
the workflow explanation. This is what an AI agent would read first before
acting on any warning.

---

## Scenario D — Stress-test against the real-world corpus

This is the real test. The corpus harvester pulls real LaTeX (templates from
latextemplates.com + recent arXiv submissions) into `templates/`. We run ByeTex
across the whole set and look at the aggregate behavior — which templates
convert cleanly, which fail, and which warning categories dominate.

### D1. Confirm the corpus is present

```bash
cd ~/Workspace/tools/ByeTex
jq '.entries | length' templates/manifest.json
jq '.entries | group_by(.source) | map({source: .[0].source, count: length})' \
   templates/manifest.json
```

**Expected**: at least 5 entries (small-batch run). The breakdown should show
some from `latextemplates` and some from `arxiv`.

**If the harvester hasn't run yet**: either wait for it (recommended — the
small batch takes a few minutes) or skip to Scenario D-alt below for the
bring-your-own fallback.

### D2. Spot-check three templates by hand

Pick one latextemplates entry and two arXiv papers — favor a math-heavy one:

```bash
# List candidates:
jq -r '.entries[] | "\(.id)  [\(.source):\(.category)]  \(.title // "?")"' templates/manifest.json
```

For each pick, run ByeTex and inspect:

```bash
# Replace <slug> with one from the list above. Source path lives under
# templates/<source>/<category-slug>/<id>/source/ — find the main .tex file
# (usually the one with \documentclass).
SAMPLE="templates/latextemplates/essay/tufte-essay/source"
MAIN=$(grep -l '\\documentclass' $SAMPLE/*.tex 2>/dev/null | head -1)
echo "main: $MAIN"
bytetex convert "$MAIN"
jq 'length' "${MAIN%.tex}.warnings.json"
typst compile "${MAIN%.tex}.typ" 2>&1 | head -3
```

**Expected for each**:
- `bytetex convert` exits 0 (no panic — this is the floor).
- The `.typ` file exists and is well-formed Typst.
- Warning count is reasonable: ≤ 5% of `wc -l` on the source for academic
  papers; templates with heavy custom commands may be higher.
- `typst compile` either succeeds OR fails with an error that maps to a
  specific warning entry (check via `jq`).

**Pass criteria**: at least 2 of 3 picks produce a viewable PDF without
manual intervention.

### D3. Batch-eval across the whole corpus

This is the aggregate signal — how does ByeTex behave at scale?

Drop this script into the repo (or copy into a temp file):

```bash
cat > /tmp/corpus_eval.sh << 'EOF'
#!/usr/bin/env bash
set -uo pipefail
ROOT=${1:-templates}
TOTAL=0; CONVERTED=0; COMPILED=0; PANICKED=0
TOTAL_WARN=0
declare -A KIND_COUNTS
for tex in $(find "$ROOT" -path '*/source/*.tex' -type f 2>/dev/null); do
  # Heuristic: only pick files that actually look like a doc root.
  grep -q '\\documentclass' "$tex" || continue
  TOTAL=$((TOTAL+1))
  base="${tex%.tex}"
  out=$(bytetex convert "$tex" 2>&1 || echo "PANIC")
  if echo "$out" | grep -q "PANIC"; then
    PANICKED=$((PANICKED+1))
    continue
  fi
  CONVERTED=$((CONVERTED+1))
  warn_file="${base}.warnings.json"
  if [ -f "$warn_file" ]; then
    n=$(jq 'length' "$warn_file")
    TOTAL_WARN=$((TOTAL_WARN+n))
    while read -r kind; do
      KIND_COUNTS[$kind]=$((${KIND_COUNTS[$kind]:-0}+1))
    done < <(jq -r '.[].category.kind' "$warn_file")
  fi
  if typst compile "${base}.typ" >/dev/null 2>&1; then
    COMPILED=$((COMPILED+1))
  fi
done
echo "=== corpus eval ==="
echo "total .tex sources:    $TOTAL"
echo "converted (no panic):  $CONVERTED"
echo "compiled to PDF:       $COMPILED"
echo "converter panics:      $PANICKED"
echo "total warnings:        $TOTAL_WARN"
if [ $CONVERTED -gt 0 ]; then
  echo "avg warnings/doc:    $((TOTAL_WARN/CONVERTED))"
fi
echo "=== top warning categories ==="
for k in "${!KIND_COUNTS[@]}"; do
  printf "  %s: %d\n" "$k" "${KIND_COUNTS[$k]}"
done | sort -t: -k2 -rn | head -10
EOF
chmod +x /tmp/corpus_eval.sh
/tmp/corpus_eval.sh templates
```

**Expected** (small-batch, ~5 docs):
- `converter panics: 0` — this is a hard fail signal. Any non-zero count is
  a converter bug.
- `converted ≥ TOTAL - parse_error_count`. Per-doc parse errors are OK; full
  panics are not.
- `compiled to PDF` ≥ ~60% of converted. Templates often reference missing
  bibs / figures, which legitimately fail typst — that's fine, we're not
  judging Typst compilation, we're judging conversion correctness.
- The top-categories histogram should be dominated by `unsupported_command`,
  with a tail of `unsupported_environment`, `ambiguous_math`, and `parse_error`.

**Useful follow-up** — find the highest-warning doc and look at what it's
struggling with:

```bash
find templates -name '*.warnings.json' -exec sh -c 'echo "$(jq length "$1") $1"' _ {} \; \
  | sort -rn | head -5
```

Read the top-of-list warnings to surface what categories of LaTeX we don't
yet handle. These are the inputs for the next round of emitter rules.

### D-alt. Bring your own paper (corpus unavailable)

If you don't have the corpus yet, fall back to a single paper of your own:

```bash
cp ~/path/to/your/paper.tex /tmp/bytetex-test/
cd /tmp/bytetex-test
bytetex convert paper.tex
jq 'length' paper.warnings.json
jq '[.[].category.kind] | group_by(.) | map({kind: .[0], count: length}) | sort_by(-.count) | .[0:5]' \
   paper.warnings.json
typst compile paper.typ
```

Same pass criteria as D2: convert exits 0, `.typ` is well-formed, compile
either succeeds or fails with a traceable warning.

---

## Scenario E — Agent loop via MCP

This tests the path an AI assistant would use. You need a Claude Code session
(or any MCP-aware client). Two windows: one for ByeTex, one for the client.

### E1. Start the MCP server

In window 1:

```bash
bytetex serve
```

**Expected**: the process blocks (no immediate output). It's now listening on
stdin/stdout for MCP JSON-RPC messages. Don't type anything.

### E2. Quick sanity ping (without Claude)

In window 2, send a hand-crafted initialize message:

```bash
printf '%s\n%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"manual","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | bytetex serve 2>/dev/null
```

**Expected**: three JSON lines back (one ignored notification echo plus two
responses). The `tools/list` response should mention `convert`, `convert_file`,
`convert_fragment`, `list_skills`, `read_skill`.

If you can't parse the raw output easily, just confirm those tool names appear
somewhere in the response:

```bash
printf '%s\n%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"manual","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | bytetex serve 2>/dev/null | grep -oE '"name":"(convert|convert_file|convert_fragment|list_skills|read_skill)"' | sort -u
```

**Expected**: all 5 tool names.

### E3. Wire to Claude Code

Add the MCP server to Claude Code's config. The mechanism varies by Claude
Code version; the common form is editing a JSON config file or running:

```bash
claude mcp add bytetex bytetex serve
```

Verify the connection from inside Claude Code:

```
/mcp
```

**Expected**: a list of connected servers including `bytetex` with 5 tools.

### E4. Drive a real conversion through the agent

Pick a real paper from the harvested corpus (or your own if you skipped D):

```bash
TARGET=$(find templates -path '*/source/*.tex' -exec grep -l '\\documentclass' {} \; 2>/dev/null | head -1)
echo "agent target: $TARGET"
```

In Claude Code, ask (substitute the path you printed):

> Using the bytetex MCP server, convert `<TARGET path>` to Typst.
> Then read the warnings.json and tell me what's there. For each warning
> category, read the relevant skill and summarize the remediation steps in 1–2
> sentences each.

**Expected behavior**:
- Claude calls `convert_file` with the path.
- Claude reads back the resulting warnings count and groups them.
- Claude calls `read_skill` for each `suggested_skill` referenced.
- Claude produces a per-category fix summary.

**Pass criteria**: the agent loop completes without manual intervention. The
remediation summary should match what's literally in the skill files (no
hallucination).

**Fail signals**: the agent can't reach the MCP server, the tool calls return
errors, or the summary contradicts what's in `skills/bytetex-*.md`.

### E5. Have the agent apply fixes end-to-end

Ask Claude to actually apply the fixes (substitute the same `<TARGET path>`,
or use its sibling `.typ`):

> Now, edit the converted `.typ` file at the ranges listed in its
> `.warnings.json` to resolve each warning. Use the skills you just read.
> After every fix, run `typst compile <typ-path>` to verify it still builds.
> Stop when the PDF compiles cleanly.

**Expected**: a back-and-forth where the agent edits, recompiles, and
iterates. This is the "drifting wilkes" loop the project is designed around.

**Pass criteria**: the agent converges. The final `paper.pdf` compiles, and
each agent edit is justified by a warning entry it read.

---

## Scenario F — Edge cases

Quick spot-checks for known sharp edges. Each should take under a minute.

### F1. Math-heavy paper

Pull the NeurIPS template:

```bash
bytetex convert templates/NeurIPS/neurips_paper.tex
typst compile templates/NeurIPS/neurips_paper.typ
```

**Expected**: ~9 warnings, PDF compiles to ~70 KB. Open it; the gradient
descent equation and the matrix norm formula should render correctly.

### F2. Document with `\verb` containing fake refs

```bash
cat > /tmp/verb_test.tex << 'EOF'
Use \verb|\ref{eq:foo}| inside verbatim, not a real reference.
EOF
bytetex convert /tmp/verb_test.tex
cat /tmp/verb_test.typ
```

**Expected**: the output has `` `\ref{eq:foo}` `` as a Typst raw block, **not**
a live `@eq:foo` reference. `typst compile` should succeed.

### F3. Empty input

```bash
echo "" > /tmp/empty.tex
bytetex convert /tmp/empty.tex
cat /tmp/empty.typ
cat /tmp/empty.warnings.json
```

**Expected**: empty `.typ`, `[]` warnings file. No panic, exit 0.

### F4. Malformed LaTeX

```bash
cat > /tmp/broken.tex << 'EOF'
\section{Missing brace
The body continues but the brace was never closed.
EOF
bytetex convert /tmp/broken.tex
jq '[.[].category.kind] | unique' /tmp/broken.warnings.json
```

**Expected**: at least one `parse_error` warning with `suggested_skill: "bytetex-parse-error"`. No panic. The `.typ` is produced (degraded but present).

---

## Scenario G — Release artifact smoke check (optional)

If you came through Scenario A2 (downloaded release):

```bash
cd /tmp/bytetex-test
gh release view v0.2.0 --repo zeyuyang42/ByeTeX --json assets --jq '.assets[].name'
```

**Expected**: 5 tarballs covering linux-musl x2, darwin x2, windows.

Download the manifest (if cargo-dist generated one):

```bash
gh release download v0.2.0 --repo zeyuyang42/ByeTeX --pattern '*manifest*' 2>/dev/null
ls *.json 2>/dev/null
```

If a `dist-manifest.json` is present, an AI agent would use it to pick the
right binary for the target platform.

---

## What to record

For each scenario, jot down:

- **Pass/fail/partial** with the criterion that decided it.
- **Anything surprising** — output that diverged from what's documented above,
  even if the test still passed.
- **Time spent on the scenario** — useful for budgeting future iterations.
- **Files to keep**:
  - From D2/D-alt: the `.typ` and `.warnings.json` for any source that
    panicked or whose compile errors didn't map to a warning entry.
  - From D3: the `corpus_eval.sh` output (the summary block + top-categories
    histogram). This is the single highest-signal artifact for v0.3 planning.
  - From E: a transcript of the agent loop with the final compiled PDF.

If a scenario fails, please attach the `.tex` / `.typ` / `.warnings.json`
and the exact command + first error line. For corpus runs, also note the
manifest `id` so the source is reproducible (rerun the harvester with
`--resume`).

---

## Quick reference card

```bash
# Convert
bytetex convert input.tex

# Inspect warnings
jq '.' input.warnings.json
jq '[.[].category.kind] | group_by(.) | map({kind: .[0], count: length})' input.warnings.json

# Skills
bytetex skills list
bytetex skills read bytetex-using-warnings-json

# MCP
bytetex serve                                  # blocks; for clients
claude mcp add bytetex bytetex serve           # one-time setup

# Compile
typst compile input.typ

# Corpus (after the harvester has run)
jq '.entries | length' templates/manifest.json
find templates -path '*/source/*.tex' -exec grep -l '\\documentclass' {} \;
find templates -name '*.warnings.json' -exec sh -c \
  'echo "$(jq length "$1") $1"' _ {} \; | sort -rn | head
```
