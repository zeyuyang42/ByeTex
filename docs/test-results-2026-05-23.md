# Test results — 2026-05-23

Executed `docs/test-plan.md` against commit `d48ad6e` (tag `v0.2.0`) on
macOS arm64, Rust 1.95, typst 0.14.2. Scenarios that require an external
Claude Code session (E3–E5) were deferred and not exercised here.

Final tally: **6 PASS, 2 PARTIAL, 2 FAIL** out of 10 in-scope scenarios.
Zero converter panics in any scenario. Three substantive findings worth
filing (see "Findings" at the bottom).

---

## Scenario A — Get the binary

**PASS.** Local binary at `target/release/bytetex` (~7 MB) reports
`bytetex 0.0.1`. A2/A3 install paths not exercised; the binary was built
locally from `d48ad6e` immediately before this run.

## Scenario B — Convert a known-good template

**PASS.**

```
$ bytetex convert templates/IEEE/conference_101719.tex
wrote templates/IEEE/conference_101719.typ (17 warnings)
```

| metric    | expected         | got     |
|-----------|------------------|---------|
| warnings  | <25              | 17      |
| typst     | exit 0           | exit 0  |
| PDF size  | ~100 KB          | 95 KB   |

The PDF opens with title block, numbered sections, equation refs, and
citations rendering correctly.

## Scenario C — Inspect warnings and look up skills

**PASS.** Warning histogram dominated by `unsupported_command` (16) plus
1 `unsupported_environment` — matches documented expectation. Sample
warning has the documented shape (`range`, `category`, `severity`,
`message`, `snippet`, `suggested_skill`). `bytetex skills list` returns
all 6 expected skill names; `bytetex skills read bytetex-using-warnings-json`
emits the documented frontmatter + body.

## Scenario D1 — Corpus presence

**PASS.** `templates/manifest.json` has **45 entries** (15 latextemplates +
30 arXiv), well above the small-batch 5 floor. Both source breakdowns
present.

## Scenario D2 — Spot-check 3 templates

**FAIL.** The documented criterion is "at least 2 of 3 picks produce a
viewable PDF without manual intervention." All 3 picks converted cleanly
(0 panics, .typ written, reasonable warning counts) but **none** produced
a PDF: each `typst compile` failed because the source documents reference
undefined custom macros / unclosed delimiters that ByeTex's
warn-and-passthrough strategy leaves in the output.

| pick                              | warnings | compile             |
|-----------------------------------|---------:|---------------------|
| `latextemplates:tufte-essay`      |       50 | error: expected expression |
| `arxiv:2605.22821` (cs.LG)        |    1,150 | error: unclosed delimiter  |
| `arxiv:2605.22728` (math.NA)      |      496 | error: unclosed delimiter  |

This is the headline finding of the run. See **Finding #1** below.

## Scenario D3 — Batch eval across the corpus

**PARTIAL.** Hard floor (0 panics) holds across all 45 documents.
Compile-pass rate is 13/45 (29%), well below the ~60% the test plan
loosely expects. The warning distribution is heavily skewed:

```
total docs:           45
converted (no panic): 45
compiled to PDF:      13
panics:                0
total warnings:       13,763
avg warnings/doc:        305

warning category histogram:
  ambiguous_math         12,122  (88%)
  unsupported_command     1,347
  needs_manual_review       166
  unsupported_environment   128

top unsupported names (long tail of user-defined macros, expected per
the v1 non-goals; \num is siunitx; \mathrm/\it/\bf are font commands)
```

The `ambiguous_math` dominance is the second headline finding: real arXiv
papers use many math commands not in the supported subset, and the
current behavior (emit `"name"` placeholder and warn) often leaves the
math unclosed and breaks `typst compile`. See **Finding #2**.

## Scenarios E1+E2 — MCP handshake

**PASS** (with documentation defect — see Finding #4).

All five tools are present in `tools/list`: `convert`, `convert_file`,
`convert_fragment`, `list_skills`, `read_skill`. Initialize succeeds with
protocol version 2025-03-26.

The test plan's one-liner (`printf … | bytetex serve | grep …`) needs a
small change to actually return the tools list — see Finding #4.

## Scenarios E3–E5 — Agent loop via Claude Code

**DEFERRED.** Cannot drive these from the same session that hosts the
ByeTex MCP server. To finish, you'd run in a separate Claude Code
session:

```bash
claude mcp add bytetex /Users/zeyuyang42/Workspace/tools/ByeTex/target/release/bytetex serve
```

Then in that session, ask Claude to follow the prompts in
`docs/test-plan.md#e4-drive-a-real-conversion-through-the-agent` and
`#e5-have-the-agent-apply-fixes-end-to-end`.

## Scenario F1 — Math-heavy NeurIPS template

**PASS (better than documented).**

```
$ bytetex convert templates/NeurIPS/neurips_paper.tex
wrote templates/NeurIPS/neurips_paper.typ (1 warning)
$ typst compile … → 64 KB PDF
```

Test plan expected ~9 warnings; we got 1 (the document's `\And` author
separator). Worth updating the test plan's expected count.

## Scenario F2 — `\verb` containing fake refs

**PASS.** `\verb|\ref{eq:foo}|` converts to `` `\ref{eq:foo}` `` (a Typst
raw block), the embedded `\ref` does NOT become a live `@eq:foo`
reference, and `typst compile` succeeds.

## Scenario F3 — Empty input

**PASS.** Empty input produces a 1-byte `.typ` (trailing newline) and
`[]` warnings. No panic.

## Scenario F4 — Malformed LaTeX

**PARTIAL.** No panic (the floor holds), but the converted output emits
**zero** warnings even though tree-sitter clearly marked the unclosed
brace as an `ERROR` node:

```
$ /tmp/ts_latex_probe broken.tex
…
  ERROR [8..73] "{Missing brace\nThe body continues…"
```

The test plan expects "at least one `parse_error` warning with
`suggested_skill: bytetex-parse-error`". The converter never emits
`Category::ParseError` because `tree.root_node().has_error()` is only
inspected by `corpus_parse_smoke.rs`, never by `bytetex_core::convert`.

See **Finding #3**.

## Scenario G — Release artifact smoke

**FAIL.**

- No `v0.2.0` release exists on GitHub Releases (`gh release view v0.2.0`
  returns "release not found").
- The `release.yml` run for the v0.2.0 tag is `failure`: 4 of 5 cross-
  compile targets built (`x86_64-linux-musl`, `x86_64-darwin`,
  `aarch64-darwin`, `x86_64-windows-msvc`), but `aarch64-unknown-linux-musl`
  failed, so the downstream `publish GitHub Release` job was skipped.

Worth investigating — likely a missing `gcc-aarch64-linux-gnu` step or a
target-installation glitch in `release.yml`.

---

## Findings

### Finding #1 (high) — converter handles arXiv-class macro density poorly

3 of 3 D2 picks (and 32 of 45 D3 documents) fail to compile after
conversion. Root causes, in order of frequency:

1. **User-defined `\newcommand` macros are silently passed through** as
   raw text (e.g. `\bpetok` appears literally in the `.typ`). Typst then
   errors with "unknown variable: bpetok" or "unclosed delimiter".
2. **Math commands not in the symbol table** produce `#text(red)["name"]`
   placeholders inside math; the surrounding `$ … $` then sometimes
   fails to balance because the placeholder string contains characters
   Typst's math lexer rejects.

The v1 plan explicitly punted on custom-macro expansion. Without it,
real-world arXiv papers will keep failing here. Options:

- (a) Naive textual macro expansion: at the start of conversion, scan for
  `\newcommand{\foo}{body}` / `\DeclareMathOperator{\foo}{...}` and
  substitute every `\foo` invocation in the source with `body` before
  parsing. Doesn't cover argument-taking macros but covers the common
  shorthands seen in the corpus.
- (b) Emit each unknown command as a quoted Typst text token
  (`text("\foo")` instead of a `#text(red)[...]` block) — at least the
  output would compile, just with the literal command name visible.

Either approach would lift the corpus compile rate from 29% to probably
60–70%.

### Finding #2 (high) — `ambiguous_math` is 88% of all warnings

Even after the v0.2 symbol-table expansion, math accounts for the vast
majority of unhandled tokens on real papers. The top unhandled names
inside math are user-defined and won't yield to symbol-table additions —
they need Finding #1's macro expansion to unblock.

For the non-user-defined long tail (`\bm`, `\operatorname`, etc. — already
added in this session), the gap is small. Skill-driven manual fix is the
documented escape hatch.

### Finding #3 (medium) — converter doesn't emit `Category::ParseError`

The `Category::ParseError` variant exists in the warnings schema and is
documented in `docs/warnings.schema.json` and the `bytetex-parse-error`
skill, but no code path in `bytetex_core::convert` ever produces one.
Tree-sitter marks malformed regions as `ERROR` nodes; the emitter just
copies their source bytes through as text.

**Fix sketch:** in `crates/bytetex-core/src/emit.rs::emit_node`, when
`node.kind() == "ERROR"` (or any descendant has `has_error()`), emit a
`Category::ParseError { tree_sitter_node: "<kind>" }` warning before
continuing. A small change; maybe 10 lines.

### Finding #4 (low) — test-plan MCP one-liner is racy

The documented one-liner in `docs/test-plan.md` Scenario E2:

```bash
printf '…' | bytetex serve … | grep -oE '"name":"…"'
```

closes stdin before the server reads the `tools/list` message, so the
grep returns nothing for tools other than `initialize`'s response.

**Fix sketch:** wrap the printf in a subshell with a trailing `sleep 3`
so the server has time to process the queued frames:

```bash
( printf '…'; sleep 3 ) | bytetex serve …
```

The internal `tests/mcp_smoke.rs` test doesn't hit this because it uses
a long-lived stdin stream.

### Finding #5 (high) — Windows CI fails on every push

Every CI run since v0.1 has failed on the `test (windows-latest)` job
with `m1_with_comments` snapshot mismatch. Ubuntu and macOS succeed.
Likely cause: git checks out with CRLF on Windows by default, which
changes byte ranges in snapshots that include source positions.

**Fix sketch:** either (a) add `* text eol=lf` to `.gitattributes` for the
fixture files, or (b) make snapshots line-ending agnostic. (a) is one
file and one line.

### Finding #6 (medium) — `aarch64-unknown-linux-musl` release target fails

The release.yml workflow for tag `v0.2.0` failed at the
`build aarch64-unknown-linux-musl` step, which blocked the GitHub
Release from being published. The other 4 targets built successfully.

**Fix sketch:** the cross-compile step needs `gcc-aarch64-linux-gnu`
installed; the existing workflow has the conditional `if:
contains(matrix.target, 'aarch64-unknown-linux-musl')` but it may not be
running before the build step. Inspect the run log:

```bash
gh run view 26332481083 --repo zeyuyang42/ByeTeX --log-failed | grep -i aarch64
```

---

## Reproduction & artifacts

- Eval script: `/tmp/corpus_eval.py` (Python 3, no deps beyond stdlib).
- Per-document `.typ` and `.warnings.json` are under each
  `templates/.../source/` directory (gitignored).
- For the 32 documents that didn't compile, the warning sidecars are the
  starting point for the per-doc fix path documented in
  `skills/bytetex-using-warnings-json.md`.

## Next steps

In priority order:

1. **Findings #1 / #5** — high-impact, small change for each. Address
   first.
2. **Finding #3** — small change in the emitter, large gain in agent
   debuggability (the parse-error skill becomes usable).
3. **Finding #6** — unblocks the v0.2.0 release. After fixing, retag
   `v0.2.0` (or cut `v0.2.1`).
4. **Finding #2** — large open question; revisit after #1 lands.
5. **Finding #4** — documentation tweak; one line.

The deferred E3–E5 scenarios remain available for a follow-up run from
a separate Claude Code session.
