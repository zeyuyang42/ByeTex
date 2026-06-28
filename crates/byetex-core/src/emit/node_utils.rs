//! AST/node classification, curly-group access, label extraction & misc string helpers, extracted from emit.rs (pure code motion).

use crate::ir::Node;

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

pub(in crate::emit) fn section_level(kind: &str, chapter_based: bool) -> u8 {
    // LaTeX has \part > \chapter > \section > ... \subparagraph; Typst has a single
    // integer level. In a chapter-bearing class (book/report/thesis) `\chapter` is the
    // top level, so `\section` and below shift down by one (section = 2). In the article
    // family there are no chapters, so `\section` = level 1 (the legacy mapping).
    if chapter_based {
        match kind {
            "part" | "chapter" => 1,
            "section" => 2,
            "subsection" => 3,
            "subsubsection" => 4,
            "paragraph" => 5,
            "subparagraph" => 6,
            _ => 1,
        }
    } else {
        match kind {
            "part" | "chapter" | "section" => 1,
            "subsection" => 2,
            "subsubsection" => 3,
            "paragraph" => 4,
            "subparagraph" => 5,
            _ => 1,
        }
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
pub(in crate::emit) fn leading_font_switch(
    node: Node<'_>,
    src: &str,
) -> Option<((&'static str, &'static str), usize)> {
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

// ─── Colour helpers (xcolor → Typst) ──────────────────────────────────────────

/// Map an xcolor named colour to the equivalent Typst colour identifier, or its
/// closest built-in (Typst lacks `cyan`/`magenta`/`violet`/… — fold onto the
/// exact RGB equivalent). Returns `None` for names Typst can't represent, so the
/// caller falls back to plain content rather than emitting an invalid `fill:`.
pub(in crate::emit) fn named_color(name: &str) -> Option<&'static str> {
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "red" => "red",
        "green" => "green",
        "blue" => "blue",
        "black" => "black",
        "white" => "white",
        "yellow" => "yellow",
        "orange" => "orange",
        "purple" => "purple",
        "teal" => "teal",
        "olive" => "olive",
        "lime" => "lime",
        "navy" => "navy",
        "maroon" => "maroon",
        "aqua" => "aqua",
        "fuchsia" => "fuchsia",
        "silver" => "silver",
        "eastern" => "eastern",
        "gray" | "grey" => "gray",
        // xcolor names without a direct Typst built-in → exact/closest match
        "cyan" => "aqua",
        "magenta" => "fuchsia",
        "violet" => "purple",
        "pink" => "fuchsia",
        "darkgray" | "darkgrey" => "gray",
        "lightgray" | "lightgrey" => "silver",
        _ => return None,
    })
}

/// Convert an xcolor `[model]{spec}` pair to a Typst colour expression
/// (`rgb("#..")`, `rgb(..)`, `luma(..)`, `cmyk(..)`). `None` if the spec doesn't
/// parse, so the caller drops the colour rather than emit invalid Typst.
pub(in crate::emit) fn color_from_model_spec(model: &str, spec: &str) -> Option<String> {
    let m = model.trim();
    let spec = spec.trim();
    let nums = |n: usize| -> Option<Vec<f64>> {
        let v: Vec<f64> = spec
            .split(',')
            .map(|s| s.trim().parse::<f64>().ok())
            .collect::<Option<Vec<f64>>>()?;
        if v.len() == n {
            Some(v)
        } else {
            None
        }
    };
    if m.eq_ignore_ascii_case("html") {
        // Emit DECIMAL `rgb(r, g, b)`, not `rgb("#RRGGBB")`: the `#` inside a
        // Typst string gets escaped to `\#` by the table-cell escaping pass,
        // producing an invalid colour string (corpus 2605.31586). Decimal ints
        // carry no `#`/`"` and survive every escaping context.
        let hex = spec.trim_start_matches('#');
        if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(format!("rgb({r}, {g}, {b})"));
        }
        return None;
    }
    if m == "RGB" {
        let v = nums(3)?;
        return Some(format!(
            "rgb({}, {}, {})",
            v[0] as i64, v[1] as i64, v[2] as i64
        ));
    }
    if m.eq_ignore_ascii_case("rgb") {
        let v = nums(3)?;
        return Some(format!(
            "rgb({:.0}%, {:.0}%, {:.0}%)",
            v[0] * 100.0,
            v[1] * 100.0,
            v[2] * 100.0
        ));
    }
    if m.eq_ignore_ascii_case("gray") {
        let v = nums(1)?;
        return Some(format!("luma({:.0}%)", v[0] * 100.0));
    }
    if m.eq_ignore_ascii_case("cmyk") {
        let v = nums(4)?;
        return Some(format!(
            "cmyk({:.0}%, {:.0}%, {:.0}%, {:.0}%)",
            v[0] * 100.0,
            v[1] * 100.0,
            v[2] * 100.0,
            v[3] * 100.0
        ));
    }
    if m.eq_ignore_ascii_case("named") {
        return named_color(spec).map(|s| s.to_string());
    }
    None
}

/// Extract up to `max` brace-group bodies from `s` (depth-aware). Used to read
/// the `{name}{model}{spec}` args of `\definecolor` straight from source.
pub(in crate::emit) fn brace_groups(s: &str, max: usize) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() && out.len() < max {
        if bytes[i] == b'{' {
            let start = i + 1;
            let mut depth = 1;
            i += 1;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            out.push(s[start..i - 1].to_string());
        } else {
            i += 1;
        }
    }
    out
}

/// Split a colour command's source (`\textcolor[model]{a}{b}…`) into its
/// optional `[model]` and the brace-group contents (up to 3). Used by
/// `\textcolor`/`\colorbox`/`\fcolorbox` to read their colour args from source.
pub(in crate::emit) fn color_command_parts(text: &str, cmd: &str) -> (Option<String>, Vec<String>) {
    let body = text
        .trim_start()
        .strip_prefix(cmd)
        .unwrap_or("")
        .trim_start();
    let (model, rest) = match body.strip_prefix('[') {
        Some(r) => match r.find(']') {
            Some(close) => (Some(r[..close].to_string()), &r[close + 1..]),
            None => (None, body),
        },
        None => (None, body),
    };
    (model, brace_groups(rest, 3))
}

/// Parse a `\definecolor{name}{model}{spec}` source span into `(name, typst)`.
pub(in crate::emit) fn parse_definecolor(node: Node<'_>, src: &str) -> Option<(String, String)> {
    let text = src.get(node.start_byte()..node.end_byte())?;
    let groups = brace_groups(text, 3);
    if groups.len() < 3 {
        return None;
    }
    let typst = color_from_model_spec(&groups[1], &groups[2])?;
    Some((groups[0].trim().to_string(), typst))
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
/// plus the byte offset just past the closing `}`.
///
/// The label group's span is authoritative: `ir::lower`'s
/// `normalize_truncated_labels` already repaired the underscore-truncation quirk,
/// so the `curly_group_label[_list]` node covers the full `{...}` and we can read
/// the key straight from its span — no source byte-scan needed.
///
/// Returns all comma-separated keys for a comma-list command (see
/// [`label_ref_splits_on_comma`]); every other ref returns a one-element Vec
/// with the comma kept as a literal label char.
pub(in crate::emit) fn extract_label_ref_keys_and_end(
    node: Node<'_>,
    src: &str,
) -> Option<(Vec<String>, usize)> {
    let split = label_ref_splits_on_comma(node);
    let mut cursor = node.walk();
    let group = node
        .children(&mut cursor)
        .find(|c| matches!(c.kind(), "curly_group_label_list" | "curly_group_label"))?;
    let (open, end) = (group.start_byte(), group.end_byte());
    let bytes = src.as_bytes();
    if bytes.get(open) != Some(&b'{') || bytes.get(end.checked_sub(1)?) != Some(&b'}') {
        return None;
    }
    let inner = &src[open + 1..end - 1];
    let keys: Vec<String> = if split {
        inner
            .split(',')
            .map(|k| normalize_label_key(k.trim()))
            .filter(|k| !k.is_empty())
            .collect()
    } else {
        // Single literal key: keep any comma (it becomes `-` via sanitize,
        // matching the `\label` key).
        let key = normalize_label_key(inner.trim());
        if key.is_empty() {
            Vec::new()
        } else {
            vec![key]
        }
    };
    Some((keys, end))
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

/// Same as `extract_label_name`, but also returns the byte offset immediately
/// past the closing `}` of the label argument.
///
/// `ir::lower`'s `normalize_truncated_labels` already repaired the
/// underscore-truncation quirk (tree-sitter-latex used to stop the `label` token
/// at the first `_` and leak the tail as `subscript`/`word`/`ERROR` siblings), so
/// the `curly_group_label` node now spans the full `{...}` and the key reads
/// straight off its span. The returned end offset is still used to set
/// `skip_until` (now a harmless no-op for labels, since nothing leaks).
pub(in crate::emit) fn extract_label_name_and_end(
    node: Node<'_>,
    src: &str,
) -> Option<(String, usize)> {
    let mut cursor = node.walk();
    let group = node
        .children(&mut cursor)
        .find(|c| c.kind() == "curly_group_label")?;
    let (open, end) = (group.start_byte(), group.end_byte());
    let bytes = src.as_bytes();
    if bytes.get(open) != Some(&b'{') || bytes.get(end.checked_sub(1)?) != Some(&b'}') {
        return None;
    }
    Some((normalize_label_key(&src[open + 1..end - 1]), end))
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

/// Detect a TeX register/penalty/dimen/skip assignment tail immediately following
/// a control word — `\clubpenalty=300`, `\interfootnotelinepenalty=10000`,
/// `\parindent=2em`, `\parskip=0pt plus 1pt minus 1pt`. Given the byte offset just
/// past the command, returns the offset past the whole `=<value>[ plus <d>][ minus
/// <d>]` tail, or `None` when no numeric assignment follows. The leading `=` plus a
/// numeric value is required, so ordinary text and non-assignment commands never
/// match.
pub(in crate::emit) fn peek_tex_assignment_end(src: &str, after_cmd: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut i = after_cmd;
    let skip_blanks = |i: &mut usize| {
        while *i < bytes.len() && (bytes[*i] == b' ' || bytes[*i] == b'\t') {
            *i += 1;
        }
    };
    skip_blanks(&mut i);
    if bytes.get(i) != Some(&b'=') {
        return None;
    }
    i += 1;
    skip_blanks(&mut i);
    // A dimension/number: optional sign, digits with optional decimal point, then
    // an optional unit (letters: pt/em/cm/in/sp/ex/bp/pc/mu/dd/cc…). Returns the
    // end offset, or None if no digit is present.
    let read_number = |start: usize| -> Option<usize> {
        let mut i = start;
        if matches!(bytes.get(i), Some(b'-') | Some(b'+')) {
            i += 1;
        }
        let mut saw_digit = false;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            if bytes[i].is_ascii_digit() {
                saw_digit = true;
            }
            i += 1;
        }
        if !saw_digit {
            return None;
        }
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        Some(i)
    };
    i = read_number(i)?;
    // Optional glue stretch/shrink: ` plus <dim>` / ` minus <dim>`, repeatable.
    loop {
        let mut j = i;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        let rest = &src[j..];
        let kw_len = if rest.starts_with("plus") {
            4
        } else if rest.starts_with("minus") {
            5
        } else {
            break;
        };
        let mut k = j + kw_len;
        while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
            k += 1;
        }
        match read_number(k) {
            Some(end) => i = end,
            None => break,
        }
    }
    Some(i)
}
