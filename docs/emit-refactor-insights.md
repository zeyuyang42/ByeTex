# `emit.rs` Refactor Insights

Generated 2026-05-25 from a static analysis pass over `crates/byetex-core/src/emit.rs`
(8,230 lines as of this writing). Kept here as a stable reference so future PRs have
shared language for where to cut.

---

## Context

`emit.rs` is the LaTeX → Typst emitter — 70% of `byetex-core`'s total LOC (~3× the
next-largest file, `class_map.rs` at 1,069 lines). It has accumulated 38 explicit
`Bug #` annotations across 40+ patches. This document maps the current structure and
proposes a staged refactor so incremental improvements can land without destabilizing
the heavy regression-test suite.

**Key facts that make refactoring tractable:**

- The cross-module public surface is tiny — only **7 `use` sites in 4 files** import
  anything from `emit::`. Internal reorganization is almost entirely API-break-free.
- The test suite is dense — `crates/byetex-core/tests/` has 33 test files, most tagged
  by bug number. Each refactor phase can run the full suite as a gate.

---

## Current structure (as of this doc)

The file is organized into two large blocks with no structural markers (now fixed by
the Phase 0 banner additions in this PR):

| Block | Lines | Description |
|---|---|---|
| Pre-`impl` free fns | L47–L180 | Macro harvesting helpers |
| `impl<'a> Emitter<'a>` | L282–L5503 | ~110 methods, ~5,200 lines |
| Post-`impl` free fns | L5506–L8230 | ~50 helpers, ~2,700 lines in 8+ themes |

**`Emitter` struct carries 21 fields** (L182–L273), including several that are
effectively method-local state (see Smell S6 below).

**Phase 0 section markers** have been added in this PR, dividing the file into these
named regions:

*Inside `impl Emitter`:*
1. Construction & lifecycle
2. Node dispatch
3. Generic commands & macro expansion
4. Environment dispatch & lists
5. Theorem / proof / bibliography environments
6. Theorem & tcolorbox macro harvesting
7. Math primitives & letter-boundary helpers
8. Math environment containers
9. Math commands & operators
10. Math layout & structures
11. Cross-references & bibliography
12. Figures, graphics & tabular
13. Sectioning

*Module-level free functions:*
14. Node classification helpers
15. Node span & text utilities
16. Math / text font helpers
17. Label, citation & graphics extraction
18. Tabular, math rows & math sanitization
19. Math symbol table
20. Braceless-arg & macro machinery
21. Command dispatch helpers
22. Document class, path & package resolution
23. Asset & bibliography filesystem probing
24. Math word recognition & post-processing
25. Label extraction & normalization

---

## The smells (evidence-based)

### S1 — No internal navigation (now partially fixed)

Before Phase 0 there were zero `// ──` section markers. The single `impl Emitter`
block spanned ~5,200 lines with adjacency as the only organizing principle. The
Phase 0 banner additions in this PR address discoverability; they do not yet move
code into separate files.

### S2 — Mega-functions

| Method | Approx. lines | Why it grew |
|---|---|---|
| `emit_generic_command` | ~912 | One match arm per LaTeX command, organic additions |
| `lookup_math_symbol` | ~556 | Giant `match` over symbol names (one case per symbol) |
| `emit_node` | ~468 | Tree-sitter node-kind dispatch — two match blocks + 11 guards |
| `emit_math_command` | ~298 | Math command dispatch |
| `expand_user_macro` | ~219 | Macro expansion with recursion guard |

`emit_generic_command` alone (~912 lines) is larger than the entire `bib.rs` module
(492 lines). Each new LaTeX command handled during bug-fix work added a match arm
in place.

### S3 — Scar-tissue accumulation

**38 `Bug #` annotations** in the source, 0 `TODO`/`FIXME`/`HACK`. Fixes were applied
directly (good discipline), but the resulting bug-tag clusters now mark natural
boundaries that should become module splits. Densest clusters:
- L3284, L3305, L3342 — Bug #44 label-flushing logic
- L4488, L4503, L4547, L4566 — matrix/cases Bug #20/#26/#31 interaction

### S4 — Duplicated escape / sanitize logic (no shared policy)

Five separate escape entry points exist with no common interface:

| Function | Kind | Location | What it escapes |
|---|---|---|---|
| `escape_math_semicolons` | method | L3110 | `;` inside math `self.out` in-place |
| `sanitize_label_key` | free fn | L5818 | label keys — strips illegal Typst chars |
| `escape_paren_semicolons` | free fn | L6095 | `;` inside paren-delimited sub-expressions |
| `escape_unbalanced_math_brackets` | free fn | L6150 | unmatched `[`/`]` in math |
| `escape_text_cell` | free fn | L7359 | `_`/`*`/`#` in tabular cell content |

Worse: **letter-boundary checks** (`is_ascii_alpha*`) are duplicated inline at 14+
sites (L731, L2851, L3062, L3076, L3081, L3184, L4425, L5577, L5821, L6103, L6815,
L7109, L7658, L7971, L8106). A canonical guard exists (`ensure_math_letter_boundary`
at ~L3075) but most call sites re-implement the check rather than routing through it.
Bugs #11, #25, #26, and #33 each added a new ad-hoc check to a different site.

**This is the root cause of many recurring fusion bugs.** Centralizing the policy
behind a single helper is the highest-yield semantic refactor.

### S5 — Tree-sitter node-kind dispatch is fragmented

- Two big `match node.kind()` blocks in `emit_node` (around L621 and L817).
- 11 standalone `node.kind() == "..."` guards inside `emit_node` (around L725, L780,
  L785, L791, L937, L961, L975, L1012, L1019, L1022, L1045).
- A third dispatcher in `prepass_collect` (around L367).
- The same string literals (`"generic_command"`, `"curly_group"`, etc.) are re-typed
  at every site — no const table or enum centralizes the node-kind vocabulary.

### S6 — Implicit state on `Emitter` (21 fields, several effectively method-local)

Suspect fields (candidates for stack-frame extraction):

| Field | Why it's suspect |
|---|---|
| `pending_math_labels: Vec<String>` | Saved/restored via `mem::take` at 3 sites; classic accidentally-global pattern |
| `pending_bib_style: Option<String>` | Single-method lifecycle |
| `pending_bibitem_key: Option<String>` | Single-method lifecycle |
| `in_math: bool` | Also a param to `render_in_sub_emitter`; two sources of truth |
| `skip_until: usize` | Position-advance flag that could be a loop return value |
| `macro_depth: u32` | Recursion guard — could live in `expand_user_macro`'s call chain |

### S7 — Leaky sub-emitter abstraction

`render_in_sub_emitter` (around L548) is the only recursion primitive — used at
9 call sites. Of `Emitter`'s 21 fields, only 3 are threaded into the sub-emitter
(`in_math`, `macro_depth`, the `src` slice). `macros`, `theorem_kinds`,
`bibliography_keys`, `visited_includes`, and `pending_*` fields are **not** propagated.

The label-stashing dance (around L3270, L3290) that does `mem::take` before calling
`render_in_sub_emitter` and reinstates the labels after is direct compensation for
this leakage. It's the strongest signal that the abstraction needs an explicit
context object.

### S8 — Raw `push_str` with no builder (96 call sites)

Every emit site calls `self.out.push_str(...)` directly (96 occurrences). Escape
policy lives at the call site, not in a builder. This makes it easy to introduce a
new emit site that forgets to escape, and makes refactoring escape behavior
labour-intensive (must find all 96 sites).

---

## Proposed refactor strategy

Staged — each phase is an independent PR with the full test suite green.

### Phase 0 — Section markers (this PR) ✓

Add `// ─── Section: Name` banner comments at the 25 natural sub-system boundaries.
Comment-only delta, zero semantic change. All tests stay green by construction.

**Completed in this PR.**

### Phase 1 — Module split

Rename `src/emit.rs` → `src/emit/mod.rs` and extract into sibling files. Rust allows
`impl Emitter` blocks to be spread across files within the same module, so this is
mostly a *move* operation with no method-body changes.

Proposed layout:

```
crates/byetex-core/src/emit/
├── mod.rs              # Emitter struct + lifecycle + render_in_sub_emitter
├── node.rs             # range_of, command_name_of, first_curly_group, …
├── escape.rs           # all escape / sanitize helpers (S4 target)
├── commands.rs         # emit_generic_command + per-command helpers
├── macros.rs           # expand_user_macro, harvest_*, extract_newcommand*
├── environments.rs     # emit_generic_environment, lists, theorem, proof
├── math/
│   ├── mod.rs          # emit_inline_math, emit_display_math, push_math_symbol
│   ├── commands.rs     # emit_math_command + lookup_math_symbol
│   └── structures.rs   # matrices, cases, fractions, subscripts, arrows
├── assets.rs           # figures, includegraphics, asset probing
├── tabular.rs          # emit_tabular, parse_column_spec, escape_text_cell
├── bibliography.rs     # cite, bibliography, bibitem, bib/bbl probing
└── sections.rs         # emit_section, section_level, is_section_kind
```

**Files to update after the split** (the entire cross-module surface):
- `crates/byetex-core/src/lib.rs` — 4 use sites
- `crates/byetex-core/src/package_macros.rs` — 1 use site
- `crates/byetex-core/src/project.rs` — 2 use sites

**Target per-file size:** ≤ 1,500 lines (most will be 200–600 lines).

### Phase 2 — Consolidate the scar tissue

After Phase 1, attack the semantic smells inside their isolated modules:

**`escape.rs` — centralize all escape logic (targets S4)**
- Collapse the 14 inline letter-boundary checks behind `ensure_math_letter_boundary`.
- Unify the 5 escape entry points: one policy module, one place to audit correctness.
- This is the highest-ROI semantic refactor — directly prevents recurrence of
  fusion bugs (#11, #25, #26, #33).

**`node.rs` — node-kind const table (targets S5)**
- Define `const NODE_KIND_GENERIC_COMMAND: &str = "generic_command"` etc. (or a
  `NodeKind` enum with a `from_str` impl).
- Replace the 11 string-literal guards in `emit_node` with a single consolidated
  dispatcher.

**`mod.rs` — extract pending state (targets S6 / S7)**
- Move `pending_math_labels` + `pending_bib_*` into a `MathEmitCtx` / `BibCtx` struct
  owned by the call stack, not the `Emitter`.
- `render_in_sub_emitter` accepts/returns that context explicitly, eliminating the
  `mem::take` workarounds.

**`commands.rs` — chip away at `emit_generic_command` (targets S2)**
- Extract "one-arg wrapper" commands into a data table
  `(&str, typst_wrapper_open, typst_wrapper_close)` and loop over it.
- This alone cuts hundreds of match arms from the function body.

### Phase 3 — `lookup_math_symbol` as data (optional, large win)

The 556-line `lookup_math_symbol` is a `match name { ... }` over symbol names.
Converting it to a `phf::Map<&'static str, &'static str>` (compile-time perfect hash)
turns it into pure data — easier to diff, extend, and auto-generate from
`vendor/katex/`. Trade-off: adds a build-time dep (`phf`). Best tackled after Phase
1–2 are merged so the symbol table lives in an isolated `math/commands.rs`.

---

## Sequencing recommendation

1. **Land Phase 0** (this PR) — no risk, immediate navigation win.
2. **Land Phase 1** (module split) — after `fix-bug-43-cite-validation` merges and
   `main` is stable. One PR per top-level module file is safer than one giant PR.
3. **Land Phase 2 (escape consolidation first)** — `escape.rs` is the highest-leverage
   semantic change and is isolated enough to land before the other Phase 2 items.
4. **Remaining Phase 2 items** — `node.rs` const table, pending-state extraction,
   `emit_generic_command` breakup — each as its own PR.
5. **Phase 3** — only after Phases 1–2 are complete.

**Risk management at every step:** run `cargo test --workspace` before and after each
PR. If any visual-regression tests fail, stop and investigate before merging.
