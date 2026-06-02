//! AST/node classification, curly-group access, label extraction & misc string helpers, extracted from emit.rs (pure code motion).

use tree_sitter::Node;

use super::sanitize_label_key;
use crate::warnings::Range;

// ─── Node classification helpers ──────────────────────────────────────────────

pub(in crate::emit) fn is_comment(kind: &str) -> bool {
    matches!(kind, "line_comment" | "block_comment" | "comment")
}

pub(in crate::emit) fn is_section_kind(kind: &str) -> bool {
    matches!(
        kind,
        "part"
            | "chapter"
            | "section"
            | "subsection"
            | "subsubsection"
            | "paragraph"
            | "subparagraph"
    )
}

pub(in crate::emit) fn section_level(kind: &str) -> u8 {
    // LaTeX has \part > \chapter > \section > ... \subparagraph. Typst has a
    // single integer level. We collapse part/chapter/section to level 1 for
    // article-class compatibility; M2 doesn't target book/report yet, so this
    // is acceptable until M4. Within the M2 article subset, level 1 = section.
    match kind {
        "part" | "chapter" | "section" => 1,
        "subsection" => 2,
        "subsubsection" => 3,
        "paragraph" => 4,
        "subparagraph" => 5,
        _ => 1,
    }
}

pub(in crate::emit) fn is_command(kind: &str) -> bool {
    matches!(
        kind,
        "generic_command"
            | "class_include"
            | "package_include"
            | "latex_include"
            | "graphics_include"
            | "citation"
            | "label_definition"
            | "label_reference"
            | "new_command_definition"
            | "title_declaration"
            | "author_declaration"
            | "counter_declaration"
            | "hyperlink"
            | "todo"
    )
}

// ─── Node span & text utilities ───────────────────────────────────────────────

pub(in crate::emit) fn range_of(node: Node<'_>) -> Range {
    let start = node.start_position();
    let end = node.end_position();
    Range {
        start_line: start.row as u32 + 1,
        start_col: start.column as u32 + 1,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32 + 1,
        byte_start: node.start_byte() as u32,
        byte_end: node.end_byte() as u32,
    }
}

pub(in crate::emit) fn command_name_of(snippet: &str) -> String {
    let mut chars = snippet.char_indices();
    if chars.next().map(|(_, c)| c) != Some('\\') {
        return snippet.to_string();
    }
    let end = chars
        .find(|(_, c)| !c.is_ascii_alphabetic() && *c != '*')
        .map(|(i, _)| i)
        .unwrap_or(snippet.len());
    snippet[..end].to_string()
}

/// Read the `command_name` child of a `generic_command` and return its text.
pub(in crate::emit) fn command_name_text(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "command_name" {
            return Some(src[child.start_byte()..child.end_byte()].to_string());
        }
    }
    None
}

/// If `node` is a `curly_group` whose first named child is a declarative
/// bold/italic font switch (`{\bf ..}`, `{\em ..}`), return the Typst wrap
/// markup and the byte just after the switch command. Other family switches
/// (`\sc`, `\tt`, ...) have no clean inline equivalent and are not wrapped.
pub(in crate::emit) fn leading_font_switch(node: Node<'_>, src: &str) -> Option<((&'static str, &'static str), usize)> {
    let mut cursor = node.walk();
    let first = node.named_children(&mut cursor).next()?;
    if first.kind() != "generic_command" {
        return None;
    }
    let wrap = match command_name_text(first, src)?.as_str() {
        "\\bf" | "\\bfseries" => ("*", "*"),
        "\\em" | "\\it" | "\\itshape" | "\\sl" | "\\slshape" => ("_", "_"),
        _ => return None,
    };
    Some((wrap, first.end_byte()))
}

/// First `curly_group` child of `node`, if any.
pub(in crate::emit) fn first_curly_group(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .find(|child| child.kind() == "curly_group");
    result
}

/// The `n`-th (0-based) `curly_group`-family child of `node`. Used to read
/// `\captionof{type}{caption}`: arg 0 is the type, arg 1 the caption.
pub(in crate::emit) fn nth_curly_group(node: Node<'_>, n: usize) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .filter(|child| child.kind().starts_with("curly_group"))
        .nth(n);
    result
}

/// Recursively flatten transparent `text` containers in a slice of
/// math nodes. tree-sitter-latex groups adjacent words and commands
/// into a single `text` node, hiding nested font declarations from a
/// shallow sibling scan. Flattening lifts those grandchildren up so a
/// caller can iterate uniformly.
pub(in crate::emit) fn flatten_text_children<'a>(body: &[Node<'a>]) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    for child in body {
        push_flat(*child, &mut out);
    }
    out
}

pub(in crate::emit) fn push_flat<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "text" {
        let mut cursor = node.walk();
        for c in node.children(&mut cursor) {
            push_flat(c, out);
        }
    } else {
        out.push(node);
    }
}

// ─── Math / text font helpers ─────────────────────────────────────────────────

/// If `node` is a TeX font-style declaration (`\bf`, `\it`, `\rm`,
/// `\sf`, `\tt`, ...), return the Typst math wrapper that approximates
/// its effect. These commands are *declarations* — they scope from
/// their position to the end of the enclosing group — so the caller
/// is expected to wrap subsequent siblings, not the declaration node
/// itself. Returns `None` for any other node.
///
/// We map to the math-mode wrappers that Typst's standard library
/// provides: `bold(...)`, `italic(...)`, `upright(...)`, `mono(...)`.
/// Slant/small-caps don't have direct math equivalents — folded onto
/// `italic`/`upright` to keep a single round-trip output.
pub(in crate::emit) fn math_font_decl_wrapper(node: Node<'_>, src: &str) -> Option<&'static str> {
    if node.kind() != "generic_command" {
        return None;
    }
    let mut cursor = node.walk();
    let name_node = node
        .children(&mut cursor)
        .find(|c| c.kind() == "command_name")?;
    let name = src.get(name_node.start_byte()..name_node.end_byte())?;
    match name {
        "\\bf" | "\\bfseries" | "\\boldmath" => Some("bold"),
        "\\it" | "\\itshape" | "\\sl" | "\\slshape" => Some("italic"),
        "\\rm" | "\\rmfamily" | "\\sc" | "\\scshape" => Some("upright"),
        "\\sf" | "\\sffamily" => Some("upright"),
        "\\tt" | "\\ttfamily" => Some("mono"),
        _ => None,
    }
}

/// First child whose kind starts with `curly_group` — matches all the
/// specialized variants tree-sitter-latex uses (`curly_group_author_list`,
/// `curly_group_text`, `curly_group_path`, etc.).
pub(in crate::emit) fn first_curly_like(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .find(|child| child.kind().starts_with("curly_group"));
    result
}

/// True if the last emitted bytes look like "there is no base symbol for a
/// following attachment", in which case a subscript or superscript needs an
/// empty-string base to be valid Typst.
///
/// Two cases: (1) we just opened math (`$`) and haven't written a base yet
/// (a floating `{}^{a}` footnote marker); (2) the attachment directly follows
/// an opening delimiter `(`/`[`/`{` — the isotope/prescript idiom
/// `\mu(^{233}\mathrm{U})` (corpus 2605.31203), where `^` would otherwise be
/// `(^...)` and Typst rejects it with `unexpected hat`.
pub(in crate::emit) fn needs_empty_base(out: &str) -> bool {
    let trimmed = out.trim_end_matches([' ', '\t']);
    trimmed.ends_with('$')
        || trimmed.ends_with("$ ")
        || trimmed.ends_with('(')
        || trimmed.ends_with('[')
        || trimmed.ends_with('{')
}


// ─── Label, citation & graphics extraction ────────────────────────────────────


/// Whether `node`'s reference command takes a comma-separated LIST of labels in
/// one brace — i.e. cleveref's `\cref{a,b}` → two refs. For every other
/// reference command (`\ref`, `\eqref`, `\pageref`, `\autoref`, …) a comma is
/// part of a single literal label name (`\label{calc_annihi,crea}`), so the
/// brace must NOT be split — otherwise the ref keys diverge from the sanitized
/// label key (`calc_annihi-crea`).
///
/// The starred forms (`\cref*`, `\Cref*`, …) behave identically to the
/// unstarred ones, so the trailing `*` is ignored — matching against the bare
/// name keeps every starred variant covered without enumerating each one.
///
/// NOTE: `emit_label_reference` separately switches on the same command kind to
/// pick the render form (`\eqref` paren-wrap, etc.); a new comma-list command
/// must be added here, and given a render arm there if it needs special output.
pub(in crate::emit) fn label_ref_splits_on_comma(node: Node<'_>) -> bool {
    let Some(kind) = node.child(0).map(|c| c.kind()) else {
        return false;
    };
    let base = kind.strip_suffix('*').unwrap_or(kind);
    matches!(
        base,
        "\\cref"
            | "\\Cref"
            | "\\cpageref"
            | "\\Cpageref"
            | "\\labelcref"
            | "\\labelcpageref"
            | "\\namecrefs"
            | "\\nameCrefs"
            | "\\lcnamecrefs"
    )
}

/// Extract the key(s) from a `label_reference` node (`\ref{x}`, `\cref{a,b}`)
/// plus the byte offset just past the closing `}` so callers can `skip_until`
/// over the part of the source tree-sitter dropped when the key contains
/// underscores (same bug as `extract_label_name`).
///
/// Returns all comma-separated keys for a comma-list command (see
/// [`label_ref_splits_on_comma`]); every other ref returns a one-element Vec
/// with the comma kept as a literal label char.
pub(in crate::emit) fn extract_label_ref_keys_and_end(node: Node<'_>, src: &str) -> Option<(Vec<String>, usize)> {
    let split = label_ref_splits_on_comma(node);
    let bytes = src.as_bytes();
    let mut cursor = node.walk();
    let mut open: Option<usize> = None;
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "curly_group_label_list" | "curly_group_label") {
            open = Some(child.start_byte());
            break;
        }
    }
    let open = open?;
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 1i32;
    let mut i = open + 1;
    let start = i;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let inner = &src[start..i];
                    let keys: Vec<String> = if split {
                        inner
                            .split(',')
                            .map(|k| normalize_label_key(k.trim()))
                            .filter(|k| !k.is_empty())
                            .collect()
                    } else {
                        // Single literal key: keep any comma (it becomes `-` via
                        // sanitize, matching the `\label` key).
                        let key = normalize_label_key(inner.trim());
                        if key.is_empty() {
                            Vec::new()
                        } else {
                            vec![key]
                        }
                    };
                    return Some((keys, i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}



// ─── Tabular, math rows & math sanitization ───────────────────────────────────


/// Skip a `{...}` balanced-brace group starting at `start` (where `src[start] == '{'`).
/// Returns the index one past the closing `}`.
pub(in crate::emit) fn skip_balanced_braces(src: &str, start: usize) -> usize {
    let bytes = src.as_bytes();
    if bytes.get(start) != Some(&b'{') {
        return start;
    }
    let mut depth = 1usize;
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            b'\\' => i += 1, // skip escaped char
            _ => {}
        }
        i += 1;
    }
    i
}

/// Split a rendered math body into row segments at every `\\`
/// row-break. The row-break is the single backslash char that
/// `emit_math_command`'s `\\` arm writes (optionally followed by a
/// `\n` per Bug #20). Other backslashes in the body (`\{`, `\}`,
/// `\#`, `\$`, etc.) are escape sequences and must be preserved.
///
/// Heuristic: a `\` is a row-break iff the next character is
/// whitespace (space, tab, newline) OR end-of-body. Escape sequences
/// (`\{`, `\}`, `\#`, ...) all have a non-whitespace second char.
pub(in crate::emit) fn split_math_rows(body: &str) -> Vec<&str> {
    let bytes = body.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let next = bytes.get(i + 1).copied();
            let is_row_break = match next {
                None => true, // trailing backslash at end of body
                Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => true,
                _ => false,
            };
            if is_row_break {
                out.push(&body[start..i]);
                // Skip the `\` and any following whitespace so the
                // next row segment doesn't start with stray whitespace.
                i += 1;
                while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                    i += 1;
                }
                start = i;
                continue;
            }
            // Escape sequence: skip the `\X` pair so we don't mistake
            // it for a row-break on the next iteration.
            i += 2;
            continue;
        }
        i += 1;
    }
    if start <= bytes.len() {
        out.push(&body[start..]);
    }
    out
}



/// Find the byte index one past the `}` that closes the `{` at `start`.
/// Returns `None` if `bytes[start]` is not `{` or braces are unbalanced.
/// Skips `\{` and `\}` so escaped braces don't affect the depth count.
pub(in crate::emit) fn brace_balanced_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1, // skip the escaped byte
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}


// ─── Command dispatch helpers ──────────────────────────────────────────────────


/// Decide whether a Typst math-mode subscript/superscript argument
/// needs an explicit `(...)` wrapper. A single token (one letter or
/// digit, optionally with one trailing `prime`-style suffix) parses
/// correctly as `_x` / `^x`; anything more compound (function call,
/// space-separated tokens, multi-char identifier) needs `_(...)`
/// because `_cal(T)` reads as `_c · al(T)`.
pub(in crate::emit) fn needs_subscript_parens(rendered: &str) -> bool {
    if rendered.is_empty() {
        return false;
    }
    // Already explicitly parenthesised — leave alone.
    if rendered.starts_with('(') && rendered.ends_with(')') {
        return false;
    }
    // A single character (letter or digit) is safe as-is.
    if rendered.chars().count() == 1 {
        return false;
    }
    // Single escaped char (`\#`, `\&`, `\,`) is safe.
    if rendered.starts_with('\\') && rendered.chars().count() <= 2 {
        return false;
    }
    true
}

// ─── Label extraction & normalization ─────────────────────────────────────────

/// Read the environment name from a `generic_environment` node. Looks for
/// `begin > curly_group_text > text|word` and returns its source text.
pub(in crate::emit) fn environment_name(env: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = env.walk();
    for child in env.children(&mut cursor) {
        if child.kind() != "begin" {
            continue;
        }
        let mut begin_cursor = child.walk();
        for grandchild in child.children(&mut begin_cursor) {
            if grandchild.kind() == "curly_group_text" {
                // Inside the curly_group_text, the env name is the inner text
                // span. Easiest: take everything between `{` and `}` and trim.
                let s = grandchild.start_byte();
                let e = grandchild.end_byte();
                let raw = &src[s..e];
                let trimmed = raw.trim_start_matches('{').trim_end_matches('}').trim();
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Extract the label key from a `label_definition` node like `\label{sec:foo}`.
pub(in crate::emit) fn extract_label_name(node: Node<'_>, src: &str) -> Option<String> {
    extract_label_name_and_end(node, src).map(|(n, _)| n)
}

/// Same as `extract_label_name`, but also returns the byte offset
/// immediately past the closing `}` of the label argument. The caller
/// uses that offset to set `skip_until` so the leaked tail (when
/// tree-sitter truncates the label at `_`) isn't re-emitted as
/// stray math content.
pub(in crate::emit) fn extract_label_name_and_end(node: Node<'_>, src: &str) -> Option<(String, usize)> {
    // tree-sitter-latex stops the `label` token at the first `_`, which
    // means `\label{eq:edl_objective}` parses with `label = "eq:edl"`
    // plus a synthesized closing brace and the rest of the name
    // (`_objective}`) leaks into the parent text as a subscript +
    // word + ERROR `}`. Recovering the full name reliably means
    // ignoring the truncated grammar token and scanning the raw
    // source bytes for the brace span instead.
    let bytes = src.as_bytes();
    let mut cursor = node.walk();
    let mut open: Option<usize> = None;
    for child in node.children(&mut cursor) {
        if child.kind() == "curly_group_label" {
            open = Some(child.start_byte());
            break;
        }
    }
    let open = open?;
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 1i32;
    let mut i = open + 1;
    let start = i;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((normalize_label_key(&src[start..i]), i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Normalise a LaTeX label key to a form Typst accepts as a `<...>`
/// label name. Typst labels can't contain whitespace; collapse runs
/// of internal whitespace to a single hyphen.
pub(in crate::emit) fn normalize_label_key(raw: &str) -> String {
    // First collapse whitespace runs to a single `-`, then sanitize any
    // remaining chars that are not valid in a Typst `<label>` token.
    let mut out = String::with_capacity(raw.len());
    let mut prev_was_dash = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            if !prev_was_dash && !out.is_empty() {
                out.push('-');
                prev_was_dash = true;
            }
        } else {
            out.push(c);
            prev_was_dash = false;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    sanitize_label_key(&out)
}
