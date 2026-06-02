//! Section/heading emission, extracted from emit.rs (pure code motion).

use std::fmt::Write;

use tree_sitter::Node;

use super::{collapse_inline_whitespace, extract_label_name, sanitize_label_key, section_level, Emitter};

impl<'a> Emitter<'a> {
    // ─── Sectioning ───────────────────────────────────────────────────────────

    /// Of several `\label` aliases on one element, choose the one to attach
    /// (Typst keeps a single label per element): the first alias that is
    /// referenced anywhere by a `\ref`-family command, else the first label.
    /// Matching is on the sanitized key, since that's what both `<label>` and
    /// `@ref` use.
    pub(in crate::emit) fn pick_label_to_attach(&self, labels: &[String]) -> Option<String> {
        labels
            .iter()
            .find(|l| self.referenced_labels.contains(&sanitize_label_key(l)))
            .or_else(|| labels.first())
            .cloned()
    }

    pub(in crate::emit) fn emit_section(&mut self, node: Node<'_>) -> usize {
        let kind = node.kind();
        let level = section_level(kind);

        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();

        // The header zone of a section node is: command_name [+ optional brack_group
        // for optional arg] + curly_group (title) + optional label_definition.
        // Everything after the first body-shaped child is the section's content.
        let mut starred = false;
        let mut title = String::new();
        // All `\label` aliases on this heading. Typst keeps only one label per
        // element, so after collection we attach the alias that is actually
        // `\ref`'d (or the first when none is referenced).
        let mut labels: Vec<String> = Vec::new();
        let mut body_start_idx = children.len();

        for (i, child) in children.iter().enumerate() {
            match child.kind() {
                k if k.starts_with('\\') && k.ends_with('*') => starred = true,
                k if k.starts_with('\\') => {}
                "curly_group" if title.is_empty() => {
                    // Bug #35: section titles that span multiple source
                    // lines (e.g. `\section{Foo bar\nbaz}`) used to emit
                    // a heading where only the first line started with
                    // `= `; the continuation became a plain paragraph
                    // followed by `<label>` attached to *text* rather
                    // than the heading. Typst then aborted with
                    // `cannot reference text` on any `@label` to that
                    // section. Collapse internal whitespace runs
                    // (including newlines) to a single space so the
                    // entire title sits on one line.
                    let raw = self.render_curly_group_content(*child);
                    title = collapse_inline_whitespace(&raw);
                }
                "brack_group" => {
                    // Optional short-title arg, e.g. \section[Short]{Long}. Ignore.
                }
                "line_comment" | "block_comment" | "comment" => {
                    // LaTeX `%` comments between the title and the label:
                    // e.g. `\subsection{T}%\n\label{k}`. Skip so the loop
                    // reaches the label_definition that follows.
                }
                "label_definition" => {
                    // Collect every label alias (the chosen one is decided
                    // after the full sweep — see pick_label_to_attach).
                    if let Some(k) = extract_label_name(*child, self.src) {
                        if !labels.contains(&k) {
                            labels.push(k);
                        }
                    }
                }
                // Bug #35: tree-sitter mis-parses curly groups whose
                // key contains `_` (e.g. `\ref{thm:UAP_general_dim}`
                // inside the title), leaving an orphan `}` as an
                // ERROR child between the title and the label. Skip
                // these so the title-extraction loop reaches the
                // `\label{...}` that follows.
                "ERROR" => {
                    let text = self.src[child.start_byte()..child.end_byte()].trim();
                    if text == "{" || text == "}" || text.is_empty() {
                        // Skip silently; orphan brace from mis-parse.
                    } else {
                        body_start_idx = i;
                        break;
                    }
                }
                _ => {
                    body_start_idx = i;
                    break;
                }
            }
        }

        // Bug #35 (sibling-label fallback): tree-sitter sometimes
        // truncates a curly group containing `_` (e.g.
        // `\ref{thm:UAP_general_dim}` inside the title) and the
        // section node ends prematurely. The associated `\label{...}`
        // then sits as a sibling of the section rather than a child,
        // and our AST-child-only sweep above doesn't find it.
        //
        // Additionally, LaTeX allows multiple consecutive \label{}
        // commands on the same section (all are aliases for the same
        // location). Typst's "last label wins" rule would silently drop
        // every label except the last, breaking any reference that used
        // an earlier label. We therefore scan ALL consecutive sibling
        // \label{...} commands: the first becomes the heading's label
        // (kept), and the rest are consumed/skipped so the walker never
        // re-emits them as standalone labels.
        //
        // Bug A (paper 2605.22814): only do this when the heading has NO body
        // children. When a `\Cref{fig:a_b}`-style underscore key truncates a
        // *sub*section title, the broken subsection (and its trailing
        // `\label`) get absorbed as children/siblings of the enclosing
        // section, which DOES have body children. In that case `node.end_byte()`
        // already sits past the body, so scanning forward would grab a distant
        // orphan label that belongs to the subsection, attach it to the wrong
        // heading, and — worse — advance `skip_until` past the body before the
        // body-emit loop runs, silently deleting every intervening node. The
        // Bug #35 sibling-orphan case this scanner exists for is always
        // body-less (the title-only node ends at the truncation point), so the
        // guard preserves it while letting the orphan label fall through to the
        // recursively-emitted (body-less) subsection that actually owns it.
        if body_start_idx == children.len() {
            let bytes = self.src.as_bytes();
            let mut i = self.skip_until.max(node.end_byte());
            // Skip whitespace, stray closing braces (the ERROR `}` left behind
            // by a truncated curly group), and LaTeX `%` line comments.
            // A `%` in LaTeX comments out the rest of the line including the
            // newline itself, so `\subsection{T}%\n\label{k}` is treated as if
            // the label is on the same logical line as the heading.
            while i < bytes.len() {
                if bytes[i].is_ascii_whitespace() || bytes[i] == b'}' {
                    i += 1;
                } else if bytes[i] == b'%' {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            const LABEL_TAG: &[u8] = b"\\label";
            loop {
                if bytes.len().saturating_sub(i) < LABEL_TAG.len()
                    || &bytes[i..i + LABEL_TAG.len()] != LABEL_TAG
                {
                    break;
                }
                let mut k = i + LABEL_TAG.len();
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k >= bytes.len() || bytes[k] != b'{' {
                    break;
                }
                let key_start = k + 1;
                let mut j = key_start;
                let mut depth = 1i32;
                while j < bytes.len() {
                    match bytes[j] {
                        b'\\' if j + 1 < bytes.len() => {
                            j += 2;
                            continue;
                        }
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] != b'}' {
                    break;
                }
                let key = self.src[key_start..j].trim().to_string();
                if key.is_empty() {
                    break;
                }
                // Collect every consecutive sibling label alias; consume them
                // all so none leaks as body content.
                if !labels.contains(&key) {
                    labels.push(key);
                }
                self.skip_until = self.skip_until.max(j + 1);
                i = j + 1;
                // Advance past whitespace and % comments to check for another \label.
                while i < bytes.len() {
                    if bytes[i].is_ascii_whitespace() {
                        i += 1;
                    } else if bytes[i] == b'%' {
                        while i < bytes.len() && bytes[i] != b'\n' {
                            i += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        // Pick the single label to attach: the first alias that is `\ref`'d
        // anywhere (so the reference resolves), else the first label.
        let chosen_label = self.pick_label_to_attach(&labels);

        // A heading's `==` markers are only recognised by Typst at the start of
        // a line. If the previous content left the cursor mid-line (e.g. a
        // theorem/remark env ending in `<rem:foo>`), the markers would be glued
        // on as plain text and any attached `<label>` would bind to text — so
        // `@label` would fail with `cannot reference text`. Force a break first.
        self.ensure_paragraph_break();

        if starred {
            // A starred section with a label must still be referenceable via
            // @label. Typst's `numbering: none` makes headings unreferenceable
            // in Typst 0.14+. A function `(..n) => none` is valid, keeps the
            // heading in the numbering counter (so @ref works), but renders
            // no visible number.
            let num_arg = if chosen_label.is_some() {
                "(..n) => none"
            } else {
                "none"
            };
            if level == 1 {
                let _ = write!(self.out, "#heading(numbering: {})[{}]", num_arg, title);
            } else {
                let _ = write!(
                    self.out,
                    "#heading(level: {}, numbering: {})[{}]",
                    level, num_arg, title
                );
            }
        } else {
            for _ in 0..level {
                self.out.push('=');
            }
            let _ = write!(self.out, " {}", title);
        }
        if let Some(l) = &chosen_label {
            let _ = write!(self.out, " <{}>", l);
        }
        self.out.push_str("\n\n");

        // Recurse into body children. We deliberately skip the gap between the
        // header and the first body child — we already emitted "\n\n".
        let body = &children[body_start_idx..];
        if !body.is_empty() {
            let mut last = body[0].start_byte();
            for child in body {
                let cs = child.start_byte();
                self.safe_copy(last, cs);
                last = self.emit_node(*child);
            }
            self.safe_copy(last, node.end_byte());
        }

        node.end_byte()
    }
}
