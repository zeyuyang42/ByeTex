# Author-block Sanitization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop author-block LaTeX leakage (the #1 fidelity-backlog item, major in 6/8 audited papers) — every author present, names clean of raw `\,`/`\quad`/`\hspace`/`%`/`&`/`\}` tokens, affiliations/emails captured best-effort in the existing centered layout.

**Architecture:** A two-stage "sanitize → parse". A new `sanitize_author_block` denylist tokenizer strips comments + non-displaying spacing/format macros + unknown braced commands (keeping `\textbf`/`\textit`/`\emph`/`\text` inner text and the `\quad`/`\qquad` separators + structured commands). Then `parse_generic_block` recognizes three author patterns (`\and`; comma-names + shared `\\` lines; `\textbf{a \quad b}` groups), and `parse_one_author` routes a substantive `\thanks{}` into affiliation/email.

**Tech Stack:** Rust (byetex-core); `cargo test`; existing helpers `matched_close_brace`, `latex_text_to_typst`, `Content::Typst`, `document::Affiliation::from_raw`, `Author::default`.

All paths are in the worktree `/Users/zeyuyang42/Workspace/tools/ByeTex/.claude/worktrees/style-profile-title/`, branch `fix-author-block`. Spec: `docs/superpowers/specs/2026-06-12-author-block-sanitization-design.md`.

---

## File structure

- `crates/byetex-core/src/class_map.rs` — all parser changes + new helpers + a `#[cfg(test)] mod author_sanitize_tests` for unit-testing the private functions.
- `crates/byetex-core/tests/author_block_sanitize.rs` — NEW end-to-end tests via `convert()` asserting the rendered author block is clean for each audit-leak fixture.
- `crates/byetex-core/tests/author_parsing.rs` — existing; must stay green (regression guard for the `\and`/IEEE/neurips paths).

---

## Task 1: `sanitize_author_block` — the denylist sanitizer

**Files:**
- Modify: `crates/byetex-core/src/class_map.rs` (add `strip_latex_comments`, `sanitize_author_block`, two const slices, and an inline test module)

- [ ] **Step 1: Write failing unit tests** — add at the END of `crates/byetex-core/src/class_map.rs`:

```rust
#[cfg(test)]
mod author_sanitize_tests {
    use super::*;

    #[test]
    fn strips_comments_keeps_escaped_percent() {
        assert_eq!(sanitize_author_block("% lead comment\nAlice"), "Alice");
        assert_eq!(sanitize_author_block(r"50\% done"), r"50\% done");
    }

    #[test]
    fn drops_control_symbols_and_spacing() {
        // \, \; \! and a stray \} vanish; words keep single spaces.
        assert_eq!(sanitize_author_block(r"Alice \, Bob\}"), "Alice Bob");
        // \hspace{..} drops command AND body; & and ~ become spaces.
        assert_eq!(sanitize_author_block(r"A\hspace{0.5cm}& B~C"), "A B C");
    }

    #[test]
    fn unwraps_font_styles_drops_unknown_braced() {
        assert_eq!(sanitize_author_block(r"\textbf{Alice}"), "Alice");
        assert_eq!(sanitize_author_block(r"\emph{Bob} \unknown{drop me}"), "Bob");
    }

    #[test]
    fn preserves_structure_and_separators() {
        // \\ kept (with [len] dropped); \and, \email{}, \quad preserved verbatim.
        assert_eq!(sanitize_author_block(r"A\\[2pt]B"), r"A\\B");
        assert_eq!(
            sanitize_author_block(r"Alice \and Bob \email{b@x} \quad C"),
            r"Alice \and Bob \email{b@x} \quad C"
        );
    }

    #[test]
    fn utf8_safe_and_idempotent() {
        let once = sanitize_author_block(r"M\"uller \, Gra\ss e");
        // accents are NOT resolved here (that is latex_text_to_typst's job) —
        // only spacing is removed; multibyte input is never split.
        assert_eq!(once, sanitize_author_block(&once));
        assert!(once.contains("ller"));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p byetex-core --lib author_sanitize_tests`
Expected: FAIL — `cannot find function sanitize_author_block`.

- [ ] **Step 3: Implement** — add these items to `crates/byetex-core/src/class_map.rs` (near the other author helpers, before `matched_close_brace`):

```rust
/// Commands the sanitizer PRESERVES verbatim (the parsers consume them later,
/// or they're author separators). Their `{...}` body flows through and is
/// itself sanitized as text.
const AUTHOR_KEEP_CMDS: &[&str] = &[
    "and", "And", "AND", "quad", "qquad",
    "email", "affiliation", "affil", "institute", "institution", "address",
    "orcid", "orcidID", "thanks", "texttt",
    "IEEEauthorblockN", "IEEEauthorblockA",
    "corref", "fnref", "authorrefmark", "inst", "textsuperscript",
];
/// Font-style commands whose inner text is KEPT (the command stripped).
const AUTHOR_UNWRAP_CMDS: &[&str] = &["textbf", "textit", "emph", "text"];

/// Strip `%`…end-of-line LaTeX comments, honoring an escaped `\%`.
fn strip_latex_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Escaped char (incl. \%) — keep both, UTF-8 safe.
            out.push('\\');
            let ch = s[i + 1..].chars().next().unwrap();
            out.push(ch);
            i += 1 + ch.len_utf8();
            continue;
        }
        if bytes[i] == b'%' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Sanitize a raw `\author{...}` block into clean LaTeX text for the structure
/// parsers: drop comments, non-displaying spacing/format macros, and unknown
/// braced commands (unwrapping only the font-style set), while preserving `\\`
/// separators, `\quad`/`\qquad` author separators, and the structured commands
/// the parsers consume. UTF-8 safe; idempotent.
fn sanitize_author_block(raw: &str) -> String {
    let s = strip_latex_comments(raw);
    let out = sanitize_macros(&s);
    // Collapse whitespace runs (tabs/newlines/multi-space) to single spaces.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_macros(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // `\\` line break — keep, then drop an optional `[len]`.
            if i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                out.push_str("\\\\");
                i += 2;
                if i < bytes.len() && bytes[i] == b'[' {
                    if let Some(rb) = s[i..].find(']') {
                        i += rb + 1;
                    }
                }
                continue;
            }
            // `\` + non-letter control symbol.
            if i + 1 < bytes.len() && !bytes[i + 1].is_ascii_alphabetic() {
                let ch = s[i + 1..].chars().next().unwrap();
                match ch {
                    ',' | ';' | '!' | ':' | '>' | ' ' => out.push(' '), // thin/neg spaces
                    '{' | '}' => {}                                      // stray escaped brace — drop
                    _ => {
                        out.push('\\');
                        out.push(ch);
                    } // \& \% \_ … keep for latex_text_to_typst
                }
                i += 1 + ch.len_utf8();
                continue;
            }
            // `\command` — read the name.
            let name_start = i + 1;
            let mut j = name_start;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            let name = &s[name_start..j];
            // optional `*`
            let mut k = j;
            if k < bytes.len() && bytes[k] == b'*' {
                k += 1;
            }
            if AUTHOR_KEEP_CMDS.contains(&name) {
                // Re-emit the command verbatim; its `{...}` body (if any) flows
                // through the loop and is sanitized as normal text.
                out.push_str(&s[i..k]);
                i = k;
                continue;
            }
            // Skip an optional `[..]` arg then an optional `{..}` body.
            let mut a = k;
            while a < bytes.len() && bytes[a] == b' ' {
                a += 1;
            }
            if a < bytes.len() && bytes[a] == b'[' {
                if let Some(rb) = s[a..].find(']') {
                    a += rb + 1;
                }
            }
            let mut b = a;
            while b < bytes.len() && bytes[b] == b' ' {
                b += 1;
            }
            if b < bytes.len() && bytes[b] == b'{' {
                if let Some(close) = matched_close_brace(s, b) {
                    if AUTHOR_UNWRAP_CMDS.contains(&name) {
                        out.push_str(&sanitize_macros(&s[b + 1..close]));
                    }
                    // else: drop the command AND its body entirely.
                    i = close + 1;
                    continue;
                }
            }
            // Bare unknown command (no body) — drop the token.
            i = k;
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        match ch {
            '&' | '~' => out.push(' '),
            _ => out.push(ch),
        }
        i += ch.len_utf8();
    }
    out
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p byetex-core --lib author_sanitize_tests`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/class_map.rs
git commit -m "feat(authors): add sanitize_author_block denylist tokenizer"
```

---

## Task 2: Wire sanitize into the parser dispatch

**Files:**
- Modify: `crates/byetex-core/src/class_map.rs` — `parse_authors` (≈ line 351)

- [ ] **Step 1: Write failing end-to-end test** — create `crates/byetex-core/tests/author_block_sanitize.rs`:

```rust
//! End-to-end: the rendered author block must be CLEAN — no raw LaTeX tokens —
//! and COMPLETE. Drives the full convert() path (the audit-leak fixtures).

use byetex_core::{convert, ConvertOptions};

fn render(class_and_author: &str) -> String {
    let src = format!(
        r"{class_and_author}\title{{T}}\begin{{document}}Body.\end{{document}}"
    );
    convert(&src, &ConvertOptions::default()).typst
}

/// The line(s) of the generated title block that carry author content: between
/// the title text and the abstract/keywords. We assert over the whole output
/// for simplicity since titles/sections here are trivial.
fn assert_clean(typst: &str) {
    for tok in ["\\,", "\\quad", "\\hspace", "\\thanks", "\\textbf", "\\\\", " & ", "\\}"] {
        assert!(
            !typst.contains(tok),
            "author block leaked `{tok}`:\n{typst}"
        );
    }
    // A leading comment percent must never appear at the start of an author line.
    assert!(!typst.contains("[% "), "leaked comment:\n{typst}");
}

#[test]
fn neurips_comma_thinspace_block_is_clean() {
    // Mirrors 2605.22507: leading %, \, separators, trailing \}.
    let typst = render(
        "\\documentclass{article}\\usepackage{neurips_2026}\
         \\author{% lead\nPablo Moreno \\affiliation{UPF} \\email{p@upf.edu} \\and Adrian Müller \\affiliation{ETH}}",
    );
    assert_clean(&typst);
    assert!(typst.contains("Pablo Moreno"), "author 1 missing:\n{typst}");
    assert!(typst.contains("Adrian Müller"), "author 2 missing:\n{typst}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p byetex-core --test author_block_sanitize neurips_comma_thinspace_block_is_clean`
Expected: FAIL — leaked `\,` (and/or `\}`) still present (sanitize not yet wired in).

- [ ] **Step 3: Implement** — replace the body of `parse_authors` (≈ lines 351-363) with:

```rust
pub(crate) fn parse_authors(raw: &[String], class: &DocClass) -> Vec<Author> {
    let mut out = Vec::new();
    for s in raw {
        let s = sanitize_author_block(s);
        let s = s.as_str();
        match class {
            DocClass::IeeeTran { .. } => out.extend(parse_ieee_block(s)),
            DocClass::Neurips | DocClass::Icml | DocClass::Iclr => {
                out.extend(parse_neurips_block(s))
            }
            _ => out.extend(parse_generic_block(s)),
        }
    }
    out
}
```

- [ ] **Step 4: Run to verify it passes + no regression**

Run: `cargo test -p byetex-core --test author_block_sanitize --test author_parsing`
Expected: the new test PASSES; all existing `author_parsing.rs` tests still PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/class_map.rs crates/byetex-core/tests/author_block_sanitize.rs
git commit -m "feat(authors): sanitize each \\author block before parsing"
```

---

## Task 3: Broaden separators — comma+shared-`\\`-lines and `\quad` groups

**Files:**
- Modify: `crates/byetex-core/src/class_map.rs` — `parse_generic_block` (≈ line 369); add `split_top_level_commas`, `parse_shared_lines`, and a residual-`\quad` strip in `parse_one_author`.

- [ ] **Step 1: Write failing end-to-end tests** — append to `crates/byetex-core/tests/author_block_sanitize.rs`:

```rust
#[test]
fn comma_names_with_shared_lines_split_all_authors() {
    // Mirrors 2605.22776: comma-separated authors, shared \\ affiliation + email.
    let typst = render(
        "\\documentclass{article}\
         \\author{A. Kirpichenko, A. Konstantinov, L. Utkin \\\\ Peter the Great University \\\\ utkin@x.edu}",
    );
    assert_clean(&typst);
    for who in ["Kirpichenko", "Konstantinov", "Utkin"] {
        assert!(typst.contains(who), "missing author {who}:\n{typst}");
    }
    assert!(typst.contains("Peter the Great University"), "shared affiliation missing:\n{typst}");
}

#[test]
fn textbf_quad_group_splits_authors() {
    // Mirrors 2605.22765: \textbf{ A \quad B \quad C } grouped author row.
    let typst = render(
        "\\documentclass{article}\\usepackage{neurips_2026}\
         \\author{\\textbf{ Umut Simsekli \\quad Eric Moulines \\quad Anna Korba }}",
    );
    assert_clean(&typst);
    for who in ["Umut Simsekli", "Eric Moulines", "Anna Korba"] {
        assert!(typst.contains(who), "missing author {who}:\n{typst}");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p byetex-core --test author_block_sanitize comma_names_with_shared_lines_split_all_authors textbf_quad_group_splits_authors`
Expected: FAIL — only the first author is present (no comma/`\quad` splitting yet).

- [ ] **Step 3: Implement** — replace `parse_generic_block` (≈ lines 369-381) with the version below, and add the two helpers after it:

```rust
fn parse_generic_block(s: &str) -> Vec<Author> {
    let normalised = s.replace("\\AND", "\\and").replace("\\And", "\\and");

    // Pattern 1: `\and`-separated self-contained authors.
    if normalised.contains("\\and") {
        return normalised
            .split("\\and")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(parse_one_author)
            .collect();
    }

    // Pattern 2: comma-separated names followed by shared `\\` lines.
    if let Some((head, tail)) = normalised.split_once("\\\\") {
        let (shared_affil, shared_email) = parse_shared_lines(tail);
        let names = split_top_level_commas(head.trim());
        let attach = |mut a: Author| -> Author {
            if a.affiliation.is_none() {
                a.affiliation = shared_affil.clone();
            }
            if a.email.is_none() {
                a.email = shared_email.clone();
            }
            a
        };
        if names.len() > 1 {
            return names
                .iter()
                .map(|n| attach(parse_one_author(n.trim())))
                .collect();
        }
        return vec![attach(parse_one_author(head.trim()))];
    }

    // Pattern 3: `\quad`/`\qquad`-separated grouped names (post-sanitize the
    // `\textbf{...}` is unwrapped, leaving `A \quad B`).
    if normalised.contains("\\quad") || normalised.contains("\\qquad") {
        return normalised
            .replace("\\qquad", "\\quad")
            .split("\\quad")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(parse_one_author)
            .collect();
    }

    // Single author.
    vec![parse_one_author(normalised.trim())]
}

/// Split on top-level commas — commas inside `{...}` are NOT separators.
fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(s[start..i].to_string());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(s[start..].to_string());
    parts.into_iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
}

/// From the `\\`-separated lines that follow the name list, derive a shared
/// affiliation (first non-email line) and email (first line containing `@` or
/// an `\email{}`), applied to every author in the block.
fn parse_shared_lines(tail: &str) -> (Option<crate::document::Affiliation>, Option<String>) {
    let mut affil = None;
    let mut email = None;
    for line in tail.split("\\\\").map(str::trim).filter(|l| !l.is_empty()) {
        // `\email{x}` or a bare `x@y` token.
        if let Some(e) = extract_email_token(line) {
            if email.is_none() {
                email = Some(e);
            }
            continue;
        }
        if affil.is_none() {
            affil = Some(crate::document::Affiliation::from_raw(Content::Typst(
                latex_text_to_typst(line),
            )));
        }
    }
    (affil, email)
}

/// Pull an email from a line: `\email{x@y}` body, or the first `@`-containing
/// whitespace token. Returns `None` if the line has no email.
fn extract_email_token(line: &str) -> Option<String> {
    if let Some(i) = line.find("\\email") {
        let after = i + "\\email".len();
        if line[after..].trim_start().starts_with('{') {
            let bpos = after + line[after..].find('{').unwrap();
            if let Some(end) = matched_close_brace(line, bpos) {
                return Some(line[bpos + 1..end].trim().to_string());
            }
        }
    }
    line.split_whitespace().find(|t| t.contains('@')).map(|t| {
        t.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-')
            .to_string()
    })
}
```

Then, in `parse_one_author`, normalize any residual `\quad`/`\qquad` in the name to a space. Change the `let cleaned_name = strip_unknown_author_cmds(name.trim());` line (≈ line 473) to:

```rust
    let despaced = name.replace("\\qquad", " ").replace("\\quad", " ");
    let cleaned_name = strip_unknown_author_cmds(despaced.trim());
```

- [ ] **Step 4: Run to verify it passes + no regression**

Run: `cargo test -p byetex-core --test author_block_sanitize --test author_parsing`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/class_map.rs crates/byetex-core/tests/author_block_sanitize.rs
git commit -m "feat(authors): split comma+shared-line and \\quad-grouped author blocks"
```

---

## Task 4: `\thanks` → affiliation/email

**Files:**
- Modify: `crates/byetex-core/src/class_map.rs` — `parse_one_author` `thanks` arm (≈ lines 456-458)

- [ ] **Step 1: Write failing end-to-end test** — append to `crates/byetex-core/tests/author_block_sanitize.rs`:

```rust
#[test]
fn substantive_thanks_becomes_affiliation_and_email() {
    // Mirrors 2605.22159: article \thanks holds the affiliation + email.
    let typst = render(
        "\\documentclass{article}\
         \\author{Benedikt Grassle\\thanks{Institut fur Mathematik, Universitat Zurich; benedikt@math.uzh.ch}}",
    );
    assert_clean(&typst);
    assert!(typst.contains("Benedikt Grassle"), "name missing/mangled:\n{typst}");
    // The affiliation text from \thanks must appear (not inline-glued to the name).
    assert!(typst.contains("Institut fur Mathematik"), "thanks affiliation missing:\n{typst}");
    assert!(!typst.contains("GrassleInstitut"), "affiliation glued into name:\n{typst}");
}

#[test]
fn equal_contribution_thanks_still_flags_not_affiliation() {
    let typst = render(
        "\\documentclass{article}\\usepackage{neurips_2026}\
         \\author{Alice\\thanks{Equal contribution} \\and Bob}",
    );
    assert_clean(&typst);
    assert!(typst.contains("Alice") && typst.contains("Bob"));
    // "Equal contribution" is a flag, not an affiliation — must not render as text.
    assert!(!typst.contains("Equal contribution"), "equal-contrib text leaked as affiliation:\n{typst}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p byetex-core --test author_block_sanitize substantive_thanks_becomes_affiliation_and_email`
Expected: FAIL — the `\thanks` affiliation text is dropped (not attached).

- [ ] **Step 3: Implement** — in `parse_one_author`, replace the `thanks` match arm (≈ lines 456-458):

```rust
                        "thanks" if body.to_ascii_lowercase().contains("equal")
                            || body.to_ascii_lowercase().contains("contribut") =>
                        {
                            equal = true;
                        }
                        "thanks" => {
                            // Substantive \thanks (article affiliation idiom): pull
                            // an email out, the rest becomes the affiliation.
                            if email.is_none() {
                                email = extract_email_token(&body);
                            }
                            if affiliation_raw.is_none() {
                                let aff = strip_email_token(&body);
                                if !aff.trim().is_empty() {
                                    affiliation_raw = Some(aff.trim().to_string());
                                }
                            }
                        }
```

Add the helper after `extract_email_token`:

```rust
/// Remove an `\email{...}` command or a bare `x@y` token from a line, leaving
/// the rest (used to separate a \thanks affiliation from its email).
fn strip_email_token(line: &str) -> String {
    let mut out = line.to_string();
    if let Some(i) = out.find("\\email") {
        let after = i + "\\email".len();
        if let Some(rel) = out[after..].find('{') {
            let bpos = after + rel;
            if let Some(end) = matched_close_brace(&out, bpos) {
                out.replace_range(i..=end, "");
            }
        }
    }
    out.split_whitespace().filter(|t| !t.contains('@')).collect::<Vec<_>>().join(" ")
}
```

Note: `email`, `affiliation_raw` are the existing `let mut` bindings in `parse_one_author`; the `thanks` body is the existing `body` local. This arm sits inside the same `match *cmd { … }` so all are in scope.

- [ ] **Step 4: Run to verify it passes + no regression**

Run: `cargo test -p byetex-core --test author_block_sanitize --test author_parsing`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/class_map.rs crates/byetex-core/tests/author_block_sanitize.rs
git commit -m "feat(authors): route substantive \\thanks into affiliation + email"
```

---

## Task 5: Investigate the `\newcolumntype` capture-boundary (out-of-scope flag)

**Files:**
- Read: `crates/byetex-core/src/emit.rs` (the `\author` raw capture ≈ lines 1245-1256 and 2645-2660)

- [ ] **Step 1: Reproduce** — add a probe test to `crates/byetex-core/tests/author_block_sanitize.rs`:

```rust
#[test]
fn newcolumntype_near_author_does_not_leak() {
    // 2605.22820 leaked a \newcolumntype p-column spec into the author block.
    let typst = render(
        "\\documentclass{article}\
         \\newcolumntype{C}[1]{>{\\centering}p{#1}}\
         \\author{Jane Doe \\affiliation{IAMM}}",
    );
    assert_clean(&typst);
    assert!(!typst.contains("p{#1}") && !typst.contains("newcolumntype"), "preamble leaked into author block:\n{typst}");
    assert!(typst.contains("Jane Doe"));
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p byetex-core --test author_block_sanitize newcolumntype_near_author_does_not_leak`
- If it PASSES: the leak was a `\author`-content artifact already fixed by sanitize — record that in the commit message and SKIP to Task 6.
- If it FAILS: the `\newcolumntype` is being swept into `raw_authors`. Inspect the `\author` capture in `emit.rs` (the `author_declaration` and `Some("\\author")` arms) — confirm the captured span is exactly the `\author{...}` group and not trailing preamble. This is a capture-boundary bug.

- [ ] **Step 3: Fix only if it failed** — scope the `\author` raw capture to its own braced group (the arms use `first_curly_group`/inner-bytes slicing; ensure the slice ends at the matched close brace, not a later one). Make the minimal change that makes the probe pass. If the root cause is larger than a brace-bound tweak, DELETE the probe test, leave a one-line `// TODO(fidelity-backlog #1b): \newcolumntype capture-boundary` note, and log it as a new backlog row in `docs/fidelity-backlog.md` instead — do NOT expand scope here.

- [ ] **Step 4: Run**

Run: `cargo test -p byetex-core --test author_block_sanitize`
Expected: PASS (probe green, or removed-and-logged).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "fix(authors): scope \\author capture to its braced group" # or "docs: log \\newcolumntype capture as backlog #1b"
```

---

## Task 6: Full verification + re-grade

- [ ] **Step 1: Whole suite + lints**

Run: `cargo test --workspace`  → all green.
Run: `cargo clippy --workspace`  → clean (ignore the pre-existing `table_float_kind.rs` unused-import warning).

- [ ] **Step 2: Acceptance gate (no compile regression)**

```bash
cargo build --release -p byetex-cli
BYETEX_BIN="$PWD/target/release/byetex" ../../../scripts/acceptance.sh   # run from repo root with this binary
```
Expected: `OK: no compile regression in known_pass set.` (45/45). The corpus payloads must be symlinked into the worktree's `corpus/` first (the pinned + audit IDs).

- [ ] **Step 3: Re-grade the worst papers** — regenerate packets with the new binary and view before/after front-matter crops:

```bash
for id in 2605.22507 2605.22765 2605.22159 2605.22820; do ln -sf /Users/zeyuyang42/Workspace/tools/ByeTex/corpus/$id corpus/$id; done
uv run --with requests --with Pillow --with numpy --with scikit-image \
  python scripts/visual_test.py --papers 2605.22507 2605.22765 2605.22159 2605.22820 --truth-source auto
```
Then Read `tests/visual/<id>/pages/frontmatter-typst.png` for each and confirm: no `%`/`\,`/`\}`/`\textbf` tokens, all authors present. (Optionally dispatch the `byetex-visual-grading` skill on the new packets and diff the `front-matter/author-block` finding vs the merged backlog.)

- [ ] **Step 4: Commit any doc updates** (e.g. mark backlog #1 resolved):

```bash
git add -A
git commit -m "docs: mark author-block leakage (backlog #1) resolved + re-grade evidence"
```

---

## Self-review notes (done by the planner)

- **Spec coverage:** sanitize (Task 1) ✓; wire-in (Task 2) ✓; comma/shared-line + `\quad` separators (Task 3) ✓; `\thanks`→affiliation (Task 4) ✓; `\newcolumntype` flag (Task 5) ✓; invariants + regression + re-grade (Tasks 2-6) ✓.
- **Type consistency:** `sanitize_author_block`/`sanitize_macros`/`strip_latex_comments`/`split_top_level_commas`/`parse_shared_lines`/`extract_email_token`/`strip_email_token` are used consistently; `Author`/`Affiliation`/`Content::Typst` match the existing definitions; `matched_close_brace(s, pos)` signature matches.
- **Gotcha:** Task 1 keeps `\quad`/`\qquad` (separators) so Task 3 can split on them; Task 3 then strips residual `\quad` from single-author names. Don't add `\quad` to a drop-set.
