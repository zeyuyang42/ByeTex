# Author-block sanitization & robust parsing — design

## Context

The vision-grading fidelity audit (`docs/fidelity-backlog.md`, PR #218) found that **author-block
LaTeX leakage is a MAJOR defect in 6 of 8 sampled papers** — the #1 fidelity issue and the most
reader-visible defect on page 1. Raw LaTeX tokens leak into the rendered author line and authors
are dropped or collapsed. Observed across the corpus:

- `2605.22507` (neurips): `% Pablo Moreno-Munoz … pablo.moreno@upf.edu \, Adrian Müller … \}` — a
  leading `%` comment, literal `\,` separators, a trailing `\}`.
- `2605.22765` (neurips): a literal `1 \textbf{ Umut Simsekli$^3$ \quad Eric Moulines$^{4,5}$ …}`
  line; authors stacked one-per-line instead of grouped.
- `2605.22820` (iclr): `\hspace{0.5cm}` → leaks `0.5cm`; `&` leaks; `\textit{}` literal; a
  `\newcolumntype` p-column spec leaks above Keywords.
- `2605.31526` (ieeetran): seven-author line collapses to the first name "Zhaole Wan".
- `2605.22776` (article): comma-separated authors collapse to the first; affiliation/email dropped.
- `2605.22159` (article): `\thanks{…}` affiliation/email runs inline into the first name; `ß`→glyph loss.

### Root causes (confirmed in `crates/byetex-core/src/class_map.rs`)

The author parser is a per-command stripper over raw bytes with three class entry points
(`parse_generic_block` / `parse_ieee_block` / `parse_neurips_block`, dispatched by `parse_authors`).
It fails because:

1. **Non-displaying macros leak.** `strip_unknown_author_cmds` only matches `\` + an ASCII *letter*,
   so control symbols `\,` `\;` `\!` `\}` and bare `&`, `~` pass through verbatim. And it *unwraps*
   unknown braced commands (`out.push_str(inner)`), so `\hspace{0.5cm}` → `0.5cm` leaks.
2. **Comments leak.** A `%` in the raw `\author{}` capture is never stripped.
3. **Separators too narrow.** `parse_generic_block` splits only on `\and`/`\And`/`\AND`. Comma- and
   `\\`-separated author lists collapse to the first author; `\textbf{A \quad B}` grouped authors
   aren't split.
4. **Generic parser lacks line structure.** Only `parse_neurips_block` handles the
   `name \\ affiliation \\ email` line shape; the generic path leaks those lines or drops them.

## Goal & scope (user-approved)

**Stop the leakage.** The rendered author block must be CLEAN (no raw `\,`/`\quad`/`\hspace`/`%`/`&`/
`\}` tokens) and COMPLETE (every author present, names correct), with affiliations/emails captured
best-effort in the **existing centered layout**. NOT in scope: per-class author-block *geometry*
(NeurIPS 3-column row, IEEE grid, page-bottom footnotes, superscript affiliation linking) — that is a
separate, larger layout effort.

**`\thanks` handling (approved):** a `\thanks{…equal…/…contribut…}` sets the equal-contribution flag
(today's behavior). A *substantive* `\thanks{…}` (the article-class affiliation idiom
`\author{Name\thanks{Dept, Univ, email}}`) is treated as affiliation content — any email is pulled
into `email`, the remainder becomes the author's `affiliation`, rendered in the existing block.

## Architecture: two-stage "sanitize → parse"

Replace the fragile per-command stripping with a shared **sanitize** front-end (a denylist that kills
the leak *class*) followed by a structure-aware **parse** stage. All three class parsers call
sanitize first.

### Stage A — `sanitize_author_block(raw: &str) -> String`  (new, class_map.rs)

A single forward char/byte tokenizer over the raw block that removes, in one pass:

- **Comments:** `%` to end-of-line, honoring an escaped `\%` (which is kept and later un-escaped by
  `latex_text_to_typst`).
- **Non-displaying control symbols:** `\,` `\;` `\!` `\:` `\>` `\ ` (backslash-space) and stray
  `\{` / `\}` that are not part of a matched group.
- **Spacing macros (with their brace/length args):** `\quad` `\qquad` `\thinspace` `\medspace`
  `\negthinspace` `\hspace{..}` `\hspace*{..}` `\vspace{..}` `\vspace*{..}` `\\[len]` (drop only the
  optional `[len]`, keep the `\\` line break as a structural separator).
- **Bare `&`** (tabular column separator inside author blocks) → space; **`~`** → space.
- **Unknown braced commands:** drop the WHOLE `\cmd{body}` (command + body), EXCEPT the
  display-unwrap set `\textbf` / `\textit` / `\emph` / `\text` whose inner text is kept. This is the
  key fix vs today's unwrap-everything behavior.

The sanitizer is UTF-8-safe (advance by codepoint, never split multibyte chars) and preserves `\\`
as a separator token and the structured commands (`\email` / `\affiliation` / `\thanks` / `\and` /
`\orcid` …) for the parse stage to consume. Idempotent.

### Stage B — structure-aware parsing

`parse_generic_block` (and the IEEE/NeurIPS variants) operate on the sanitized text and recognize the
three real structural patterns, by precedence:

1. **`\and`-separated** (`\and`/`\And`/`\AND`): split into self-contained author chunks; each chunk
   is parsed by `parse_one_author` (its own `\\` lines / `\affiliation{}` / `\thanks{}` etc.).
2. **comma-separated names + shared trailing `\\` lines** (the `A, B, C \\ Affil \\ email` shape):
   when there is NO `\and` but there ARE `\\` lines, treat the text BEFORE the first `\\` as the
   name list — split on TOP-LEVEL commas (not commas inside braces) into authors — and the lines
   AFTER the first `\\` as a shared affiliation/email applied to every author (first non-email line
   → affiliation; an email-looking line / `\email{}` → email).
3. **`\textbf{A \quad B \quad C}` grouped names:** after the display-unwrap, a name fragment
   containing `\quad`/`\qquad` boundaries is split into multiple authors.

`parse_one_author` keeps its structured-command extraction (`\email`/`\affiliation`/`\orcid`/
`\thanks`), with the `\thanks`→affiliation/email change from the goal. Names are finalized through
the existing `latex_text_to_typst` (accents, named letters) — now fed clean input.

**Edge case (accepted):** a bare comma list with NO `\and` and NO `\\` lines (`\author{A, B, C}`)
is left as a single author "A, B, C" rather than comma-split — top-level commas are ambiguous with
single-author affiliation commas ("Barcelona, Spain"), and every audit failure had `\\` lines, so
pattern 2 covers them. This is clean (no leak), just possibly under-split — acceptable under the
stop-the-leakage scope; revisit only if a real paper needs it.

### Invariants (post-condition, asserted by tests)

For every emitted `Author.name`: contains no `\`, `%`, `&`, or unmatched `{`/`}`; is non-empty. Every
source author appears as a distinct `Author`. A `\thanks`/affiliation present in source yields a
non-empty `affiliation` or `email`.

## Components touched

- `crates/byetex-core/src/class_map.rs` — new `sanitize_author_block`; `parse_generic_block` gains
  the comma+`\\`-shared-lines and `\quad`-group patterns; `parse_one_author` `\thanks`→affiliation;
  `parse_ieee_block` / `parse_neurips_block` call sanitize first. `strip_unknown_author_cmds` is
  superseded by the sanitizer (kept only if still referenced elsewhere; otherwise removed).
- `crates/byetex-core/src/emit/preamble.rs` — no behavior change expected (consumes `Author`
  records); verify the centered block renders the captured affiliation/email.
- Tests: new `crates/byetex-core/tests/author_block_sanitize.rs` (fixtures = the exact audit-leak
  strings); existing `crates/byetex-core/tests/author_parsing.rs` stays green (regression guard).

## Error handling

The sanitizer never panics on malformed input (unmatched braces → drop to end-of-fragment;
incomplete macro → drop the token). A chunk that parses to an empty name is dropped (not emitted as a
blank author). Unknown structured content that survives sanitize is plain text, never raw LaTeX.

## Testing

TDD. `author_block_sanitize.rs` cases, each asserting the invariants above:
- neurips `% Pablo Moreno-Munoz … \, Adrian Müller … \}` → 3 clean authors, no `%`/`\,`/`\}`.
- neurips `\textbf{ Umut Simsekli$^3$ \quad Eric Moulines$^{4,5}$ … }` → ≥2 authors split on `\quad`,
  superscripts stripped, no `\textbf`.
- article `A. Kirpichenko, A. Konstantinov, L. Utkin \\ Peter the Great … \\ email@x` → 3 authors,
  shared affiliation + email populated.
- iclr `Name \hspace{0.5cm}& Name2 …` → no `0.5cm`, no `&`.
- article `Benedikt Graßle\thanks{Institut für Mathematik, … ; benedikt@math.uzh.ch}` → name clean,
  affiliation+email captured (not inline in the name).
- regression: a plain `\and` block and an IEEE `\IEEEauthorblockN/A` block parse unchanged.

Then: `cargo test --workspace`, `cargo clippy --workspace`, the acceptance gate (45/45, no compile
regression), and **re-run the vision-grading loop** on the worst papers (2605.22507, 2605.22765,
2605.22159) — show before/after front-matter crops to confirm the leakage is gone.

## Out of scope / flagged

- `2605.22820` `\newcolumntype` leak: likely a *capture-boundary* bug (preamble content swept into
  `raw_authors` in `emit.rs`), not a parse bug. Investigate during implementation; fix if it is a
  one-liner in the `\author` capture, else log it as a separate backlog item.
- `2605.22159` `ß`→glyph loss: likely a Typst font issue (New Computer Modern), not the parser.
  Defer.
- Per-class author-block geometry (footnotes, columns, superscript linking): deferred to a future
  full-fidelity pass.
