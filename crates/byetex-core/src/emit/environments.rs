//! Environment emission (theorem/proof/lists/minipage/subequations) + theorem-definition harvesting, extracted from emit.rs (pure code motion).

use std::collections::HashMap;
use std::fmt::Write;

use tree_sitter::Node;

use super::{
    command_name_text, environment_name, extract_def_and_record, extract_environment_def,
    extract_label_name_and_end, extract_let, extract_newcommand,
    extract_newcommandx, extract_theorem_def, let_alias_def, range_of, read_newif_flag,
    sanitize_label_key, strip_trailing_typst_label, Emitter, MacroDef,
};
use crate::warnings::{Category, Severity, Warning};

impl<'a> Emitter<'a> {
    /// Render an environment's body into a fresh `String` (no side effect on
    /// `self.out`). Used by `abstract` capture when a class template wants
    /// the body as a content field rather than inline document text.
    pub(in crate::emit) fn render_env_body_to_string(&mut self, env: Node<'_>) -> String {
        self.with_sub_buffer(|emitter| {
            emitter.emit_environment_body(env);
        })
    }

    /// Emit a `\begin{subequations}...\end{subequations}` env. Bug #44:
    /// a `\label{...}` that's a direct child of `subequations` (e.g.
    /// `\begin{subequations}\label{eqn:AMP}\begin{align}...\end{align}
    /// \end{subequations}`) targets the equation group as a whole. By
    /// the time we visit the inner math env, that text-mode label has
    /// already been emitted as a stray `<key>` and Typst attaches it
    /// to the wrong content. Pre-stage direct-child labels into
    /// `pending_math_labels` so the inner math env's close-flush
    /// attaches them all together.
    pub(in crate::emit) fn emit_subequations_env(&mut self, env: Node<'_>) -> usize {
        let mut cursor = env.walk();
        for child in env.children(&mut cursor) {
            if child.kind() == "label_definition" {
                if let Some((key, end)) = extract_label_name_and_end(child, self.src) {
                    if !self.pending_math_labels.iter().any(|x| x == &key) {
                        self.pending_math_labels.push(key);
                    }
                    self.skip_until = self.skip_until.max(end);
                }
            }
        }
        self.emit_environment_body(env)
    }

    /// `\begin{minipage}[pos][height][inner-pos]{width} ... \end{minipage}` —
    /// transparent body wrapper, but the leading optional `[...]` position
    /// groups and the mandatory `{width}` group must be dropped (not emitted as
    /// content). Drops everything up to and including the first curly group (the
    /// width), then emits the remaining children as the body.
    pub(in crate::emit) fn emit_minipage(&mut self, env: Node<'_>) -> usize {
        while self.out.ends_with('\n') || self.out.ends_with(' ') || self.out.ends_with('\t') {
            self.out.pop();
        }
        // If stripping whitespace exposed a trailing `\` row-break marker (the
        // previous table row ended with `\\` right before this minipage), keep a
        // space after it: `split_math_rows` only treats `\` as a row break when
        // it is followed by whitespace, and fusing `\` into the minipage's
        // leading `#raw(...)` would both swallow the row break and corrupt the
        // call into `\#raw(...)`.
        if self.out.ends_with('\\') {
            self.out.push(' ');
        }

        let mut cursor = env.walk();
        let children: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();

        // The mandatory `{width}` is the FIRST curly group among the children;
        // any optional `[pos]`/`[height]`/`[inner-pos]` groups precede it. The
        // first such bracket is folded into the `begin` node by tree-sitter, but
        // additional ones surface as bare `[` / text / `]` tokens — so rather
        // than enumerate bracket node kinds, drop everything up to AND INCLUDING
        // the first curly group, then emit the rest as the body. A well-formed
        // minipage always has the width first, so the first curly group is never
        // body content; a body that itself starts with `{...}` is preserved
        // because only that first group is dropped. If there is no curly group
        // (malformed / width omitted), emit all children.
        let body_start = children
            .iter()
            .position(|c| {
                matches!(
                    c.kind(),
                    "curly_group" | "curly_group_text" | "curly_group_word"
                )
            })
            .map(|i| i + 1)
            .unwrap_or(0);

        let body = &children[body_start..];
        if body.is_empty() {
            return env.end_byte();
        }
        // Within the minipage body a `\\` is an intra-box line break, not a
        // table row separator — flag it so the `\\` handler emits `#linebreak()`.
        // Save/restore for correct nesting (minipage inside minipage).
        let was_in_minipage = self.in_minipage;
        self.in_minipage = true;
        let mut last = body[0].start_byte();
        for child in body {
            let cs = child.start_byte();
            self.safe_copy(last, cs);
            last = self.emit_node(*child);
        }
        let end = body.last().unwrap().end_byte();
        self.safe_copy(last, end);
        self.in_minipage = was_in_minipage;
        env.end_byte()
    }

    /// Emit a beamer `block`/`alertblock`/`exampleblock` as a titled `#block`:
    /// the `{Title}` argument becomes a bold accent-colored header, the body the
    /// box content. `accent` is the theme-ish hex color for the title + left rule.
    pub(in crate::emit) fn emit_beamer_block(&mut self, env: Node<'_>, accent: &str) -> usize {
        let mut cursor = env.walk();
        let children: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();

        // First curly group is the `{Title}` argument.
        let mut body_start = 0;
        let mut title = String::new();
        if let Some(first) = children.first() {
            if matches!(
                first.kind(),
                "curly_group" | "curly_group_text" | "curly_group_word"
            ) {
                title = self.render_curly_group_content(*first).trim().to_string();
                body_start = 1;
            }
        }

        let body_nodes = &children[body_start..];
        let body = if body_nodes.is_empty() {
            String::new()
        } else {
            // A native Typst `#block` (NOT a touying context) tolerates a reveal, so a
            // beamer `\only<…>` in the block body still becomes a real `#only`/`#uncover`
            // (no `touying-fn-wrapper` panic; verified).
            self.with_sub_buffer(|emitter| {
                let mut last = body_nodes[0].start_byte();
                for child in body_nodes {
                    emitter.safe_copy(last, child.start_byte());
                    last = emitter.emit_node(*child);
                }
                emitter.safe_copy(last, body_nodes.last().unwrap().end_byte());
            })
        };

        self.ensure_paragraph_break();
        let _ = write!(
            self.out,
            "#block(width: 100%, inset: 8pt, radius: 2pt, fill: rgb(\"{accent}\").lighten(88%), \
             stroke: (left: 2pt + rgb(\"{accent}\")))[\n"
        );
        if !title.is_empty() {
            let _ = write!(
                self.out,
                "  #text(weight: \"bold\", fill: rgb(\"{accent}\"))[{title}]\n\n"
            );
        }
        let _ = write!(self.out, "  {}\n]\n", body.trim());
        env.end_byte()
    }

    /// Emit a beamer `columns` block as a Typst `#grid`: one track per inner
    /// `column`, its `{width}` mapped to a Typst column spec, its body rendered as
    /// a content cell. Falls back to a transparent body emit if no `column` is found.
    pub(in crate::emit) fn emit_beamer_columns(&mut self, env: Node<'_>) -> usize {
        let mut cursor = env.walk();
        let columns: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| environment_name(*c, self.src).as_deref() == Some("column"))
            .collect();
        if columns.is_empty() {
            return self.emit_environment_body(env);
        }

        let mut specs: Vec<String> = Vec::new();
        let mut cells: Vec<String> = Vec::new();
        for col in &columns {
            let mut ccur = col.walk();
            let children: Vec<Node<'_>> = col
                .children(&mut ccur)
                .filter(|c| !matches!(c.kind(), "begin" | "end"))
                .collect();
            // First curly group is the mandatory `{width}` (like minipage).
            let mut body_start = 0;
            let mut spec = "1fr".to_string();
            if let Some(first) = children.first() {
                if matches!(
                    first.kind(),
                    "curly_group" | "curly_group_text" | "curly_group_word"
                ) {
                    spec = column_width_to_typst(self.curly_group_inner_trimmed(*first));
                    body_start = 1;
                }
            }
            let body = &children[body_start..];
            let cell = if body.is_empty() {
                String::new()
            } else {
                // The cell is a native Typst `#grid` cell (NOT touying's `#cols`), which
                // tolerates a reveal — so a beamer `\only<…>` here still becomes a real
                // `#only`/`#uncover` (no `touying-fn-wrapper` panic; verified).
                self.with_sub_buffer(|emitter| {
                    let mut last = body[0].start_byte();
                    for child in body {
                        emitter.safe_copy(last, child.start_byte());
                        last = emitter.emit_node(*child);
                    }
                    emitter.safe_copy(last, body.last().unwrap().end_byte());
                })
            };
            specs.push(spec);
            cells.push(cell.trim().to_string());
        }

        self.ensure_paragraph_break();
        let _ = write!(self.out, "#grid(columns: ({}), gutter: 1em,\n", specs.join(", "));
        for cell in &cells {
            let _ = write!(self.out, "  [{cell}],\n");
        }
        self.out.push_str(")\n");
        env.end_byte()
    }

    /// Emit a beamer `frame` as a slide: a `#pagebreak()` before all but the
    /// first frame, the `\begin{frame}{Title}` argument (if present) as a bold
    /// slide title, then the frame body. A `\frametitle{…}` inside the body is
    /// handled by its own command arm.
    pub(in crate::emit) fn emit_beamer_frame(&mut self, env: Node<'_>) -> usize {
        self.ensure_paragraph_break();

        let mut cursor = env.walk();
        let all: Vec<Node<'_>> = env.children(&mut cursor).collect();
        // End of `\begin{frame}` — the title argument (if any) starts right after it.
        let begin_end = all
            .iter()
            .find(|c| c.kind() == "begin")
            .map_or_else(|| env.start_byte(), |c| c.end_byte());
        let children: Vec<Node<'_>> = all
            .into_iter()
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();

        // `\begin{frame}{Title}{Subtitle}` — the title (and optional subtitle) are the
        // leading curly group(s) on the SAME LINE as `\begin{frame}` (only `[opts]` /
        // spaces in the gap). A group on a NEW line is body content, not a title — so
        // a frame whose body opens with `{...}` (or uses `\frametitle`) keeps it.
        let mut body_start = 0;
        let mut prev_end = begin_end;
        // i==0 → frame title, i==1 → optional frame subtitle.
        for i in 0..2 {
            let Some(child) = children.get(i) else { break };
            if !matches!(
                child.kind(),
                "curly_group" | "curly_group_text" | "curly_group_word"
            ) {
                break;
            }
            // Same line as `\begin{frame}` (or the previous title group)?
            if self
                .src
                .get(prev_end..child.start_byte())
                .is_none_or(|gap| gap.contains('\n'))
            {
                break;
            }
            let text = self.render_curly_group_content(*child).trim().to_string();
            if !text.is_empty() {
                // touying: a level-2 heading IS a slide (metropolis renders the
                // dark header bar from it). The optional second group is a frame
                // subtitle — a small line just under the title heading.
                if i == 0 {
                    let _ = write!(self.out, "== {text}\n\n");
                } else {
                    let _ = write!(self.out, "#text(size: 0.9em)[{text}]\n\n");
                }
            }
            body_start = i + 1;
            prev_end = child.end_byte();
        }

        let body = &children[body_start..];
        if body.is_empty() {
            return env.end_byte();
        }
        let mut last = body[0].start_byte();
        for child in body {
            let cs = child.start_byte();
            self.safe_copy(last, cs);
            last = self.emit_node(*child);
        }
        let end = body.last().unwrap().end_byte();
        self.safe_copy(last, end);
        env.end_byte()
    }

    /// Emit just the body of an environment (skip `begin` and `end` children).
    /// Strips trailing whitespace already in `self.out` so that preamble noise
    /// (dropped `\documentclass`, `\usepackage`, blank lines) doesn't leak
    /// in as leading newlines.
    pub(in crate::emit) fn emit_environment_body(&mut self, env: Node<'_>) -> usize {
        while self.out.ends_with('\n') || self.out.ends_with(' ') || self.out.ends_with('\t') {
            self.out.pop();
        }
        // A transparent wrapper (`center`, `document`, …) emits its body inline
        // after the trim above. But if the buffer now ends with a Typst set-rule
        // statement (e.g. `\appendix` → `#set heading(numbering: "A.1")`,
        // corpus 2605.31603), inline body content would glue onto it ("expected
        // semicolon or line break"). Re-add the line break the statement needs.
        if let Some(last_line) = self.out.rsplit('\n').next() {
            let s = last_line.trim_start();
            if s.starts_with("#set ")
                || s.starts_with("#show ")
                || s.starts_with("#let ")
                || s.starts_with("#import ")
            {
                self.out.push('\n');
            }
        }

        let mut cursor = env.walk();
        let mut body: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();

        // For a custom `\newenvironment` that takes mandatory arguments, the
        // use-site args (`\begin{name}{a}{b}`) appear as leading `curly_group`
        // children. Drop the first N (N = declared arg count) so they don't
        // leak into the passed-through body. Only a contiguous leading run is
        // skipped, so a `curly_group` that is real content (after any text) is
        // never mistaken for an argument. (Optional `[..]` args parse inside the
        // `begin` node and are already excluded.)
        if let Some(name) = environment_name(env, self.src) {
            if let Some(&nargs) = self.env_arg_counts.get(&name) {
                let mut remaining = nargs;
                let mut leading = true;
                body.retain(|c| {
                    if leading && remaining > 0 && c.kind() == "curly_group" {
                        remaining -= 1;
                        false
                    } else {
                        leading = false;
                        true
                    }
                });
            }
        }

        if body.is_empty() {
            return env.end_byte();
        }
        let mut last = body[0].start_byte();
        for child in &body {
            let cs = child.start_byte();
            self.safe_copy(last, cs);
            last = self.emit_node(*child);
        }
        let end = body.last().unwrap().end_byte();
        self.safe_copy(last, end);
        env.end_byte()
    }

    pub(in crate::emit) fn emit_simple_list(&mut self, env: Node<'_>, marker: &str) -> usize {
        // An enumerate with an enumitem counter-FORMAT optional arg
        // (`[(a)]`, `[label=(\roman*)]`, …) maps to a Typst `#enum(numbering: …)`,
        // which the `+` markup can't express. Pure-option specs (`[noitemsep]`)
        // and itemize are unaffected.
        if marker == "+" {
            if let Some(numbering) = enumerate_numbering(env, self.src) {
                return self.emit_enum_with_numbering(env, &numbering);
            }
        }
        let is_beamer = self.detected_class == crate::class_map::DocClass::Beamer;
        let mut cursor = env.walk();
        let mut first = true;
        // Count of spec-bearing items already emitted — a beamer `\item<n->` reveals
        // one sub-slide later than the previous one, so emit a `#pause` BEFORE every
        // spec-bearing item after the first (the cleanest touying idiom for sequential
        // item reveals). Gated on beamer so non-beamer lists are unchanged.
        let mut overlay_items_seen = 0usize;
        for child in env.children(&mut cursor) {
            if child.kind() != "enum_item" {
                continue;
            }
            if !first {
                self.out.push('\n');
            }
            if is_beamer && item_has_overlay_spec(child, self.src) {
                if overlay_items_seen > 0 {
                    self.out.push_str("#pause\n");
                }
                overlay_items_seen += 1;
            }
            // A custom `\item[label]` replaces the auto marker in LaTeX. The
            // Typst `+`/`-` shorthand can't carry a per-item label (and the
            // bracket would otherwise leak as escaped `\[label\]` after a wrong
            // auto number), so render it as a term item — the same mechanism a
            // `description` list uses.
            match self.render_enum_item_term(child) {
                Some(label) if !label.trim().is_empty() => {
                    let body = self.render_enum_item_body(child, /* description: */ true);
                    let _ = write!(self.out, "/ {}: {}", label.trim(), body.trim());
                }
                _ => {
                    let body = self.render_enum_item_body(child, /* description: */ false);
                    let _ = write!(self.out, "{} {}", marker, body.trim());
                }
            }
            first = false;
        }
        env.end_byte()
    }

    /// Emit a styled enumerate as `#enum(numbering: "<fmt>", [item], …)` — the
    /// function form, since the `+` markup can't carry a numbering format.
    fn emit_enum_with_numbering(&mut self, env: Node<'_>, numbering: &str) -> usize {
        self.ensure_paragraph_break();
        let _ = write!(self.out, "#enum(numbering: \"{}\",", numbering);
        let mut cursor = env.walk();
        let items: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| c.kind() == "enum_item")
            .collect();
        for item in &items {
            let body = self.render_enum_item_body(*item, false);
            let _ = write!(self.out, "\n  [{}],", body.trim());
        }
        self.out.push_str("\n)");
        env.end_byte()
    }

    pub(in crate::emit) fn emit_description(&mut self, env: Node<'_>) -> usize {
        let mut cursor = env.walk();
        let mut first = true;
        for child in env.children(&mut cursor) {
            if child.kind() != "enum_item" {
                continue;
            }
            if !first {
                self.out.push('\n');
            }
            let term = self.render_enum_item_term(child).unwrap_or_default();
            let body = self.render_enum_item_body(child, /* description: */ true);
            let _ = write!(self.out, "/ {}: {}", term.trim(), body.trim());
            first = false;
        }
        env.end_byte()
    }

    /// Render the body of an `enum_item` — everything after `\item` and after
    /// any optional `[label]`. The label is matched from SOURCE, not the AST:
    /// tree-sitter doesn't parse `[(a)]` (leading paren) as a `brack_group_text`,
    /// so it would otherwise leak into the body as escaped `\[(a)\]`. The label
    /// byte range is skipped via `skip_until` so a single `text` node spanning
    /// `[label] body` emits only its `body` tail.
    pub(in crate::emit) fn render_enum_item_body(
        &mut self,
        item: Node<'_>,
        _is_description: bool,
    ) -> String {
        let mut body_start = item_body_start(item, self.src);
        // Beamer `\item<1->` — drop the leading overlay spec (gated on the class so a
        // non-beamer `\item <0,1>` keeps its literal text).
        if self.detected_class == crate::class_map::DocClass::Beamer {
            body_start = skip_leading_overlay_spec(self.src, body_start);
        }
        let mut cursor = item.walk();
        let body: Vec<Node<'_>> = item
            .children(&mut cursor)
            .filter(|c| c.kind() != "\\item" && c.end_byte() > body_start)
            .collect();
        if body.is_empty() {
            return String::new();
        }
        self.with_sub_buffer(|emitter| {
            let saved = emitter.skip_until;
            emitter.skip_until = emitter.skip_until.max(body_start);
            let mut last = body_start;
            for child in &body {
                let cs = child.start_byte();
                if cs > last {
                    emitter.safe_copy(last, cs);
                }
                last = emitter.emit_node(*child);
            }
            let end = body.last().unwrap().end_byte();
            emitter.safe_copy(last, end);
            emitter.skip_until = saved;
        })
    }

    /// The optional `\item[label]` content, rendered, or `None` if absent/empty.
    pub(in crate::emit) fn render_enum_item_term(&mut self, item: Node<'_>) -> Option<String> {
        let (label, _) = item_bracket(item, self.src)?;
        let label = label.trim();
        if label.is_empty() {
            return None;
        }
        Some(
            self.render_in_sub_emitter(label, false, false)
                .trim()
                .to_string(),
        )
    }

    // ─── Theorem / proof / bibliography environments ──────────────────────────

    /// `\begin{theorem}[note]\label{X} body \end{theorem}` →
    /// `#figure(kind: "<name>", supplement: [Name], [body]) <X>`.
    pub(in crate::emit) fn emit_theorem_env(&mut self, env: Node<'_>, name: &str) -> usize {
        let mut cursor = env.walk();
        let mut label: Option<String> = None;
        let body: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| {
                if c.kind() == "label_definition" {
                    // Use the `_and_end` form + `skip_until`: tree-sitter truncates the
                    // label key at the first `_`, so the tail (`_to_denoiser}`) parses as
                    // separate sibling nodes that would otherwise leak into the body
                    // (dogfood A2). Skip past the real closing brace.
                    if let Some((key, end)) = extract_label_name_and_end(*c, self.src) {
                        if label.is_none() {
                            label = Some(key);
                        }
                        self.skip_until = self.skip_until.max(end);
                    }
                    return false;
                }
                // tree-sitter-latex may parse `\label{key}` as a
                // `generic_command` rather than `label_definition` when it
                // appears inside a user-defined theorem environment body.
                // Detect and consume it here so it doesn't leak into the
                // figure content as a text-mode label.
                if c.kind() == "generic_command" {
                    let text = self.src.get(c.start_byte()..c.end_byte()).unwrap_or("");
                    if text.starts_with("\\label") {
                        if label.is_none() {
                            if let (Some(s), Some(e)) =
                                (text.find('{').map(|i| i + 1), text.rfind('}'))
                            {
                                let key = text[s..e].trim();
                                if !key.is_empty() {
                                    label = Some(sanitize_label_key(key));
                                }
                            }
                        }
                        return false;
                    }
                }
                !matches!(c.kind(), "begin" | "end")
            })
            .collect();

        let mut inner = if body.is_empty() {
            String::new()
        } else {
            self.with_sub_buffer(|emitter| {
                let mut last = body[0].start_byte();
                for child in &body {
                    let cs = child.start_byte();
                    emitter.safe_copy(last, cs);
                    last = emitter.emit_node(*child);
                }
                let end = body.last().unwrap().end_byte();
                emitter.safe_copy(last, end);
            })
        };

        // If the label wasn't found as a direct child, it may still have been
        // emitted into `inner` by a nested environment handler (e.g. a `\label`
        // inside an `enumerate` item).  Extract any trailing Typst label from
        // the rendered content and hoist it outside the #figure() call.
        if label.is_none() {
            if let (cleaned, Some(key)) = strip_trailing_typst_label(&inner) {
                inner = cleaned;
                label = Some(key);
            }
        }

        self.ensure_paragraph_break();
        // Bug #39: the `kind:` string must be a plain identifier
        // (used for `@xxx` cross-references). When the `\newtheorem`
        // display contains math like `Theorem A$^\star_{\mathrm{global}}$`,
        // the raw lowercased name leaks `$`, `\`, and `^` into a
        // Typst STRING literal and breaks parsing. Sanitize to
        // ASCII alphanumeric + hyphens; the `supplement: [...]`
        // content block (which IS markup, math allowed) keeps the
        // full display unchanged.
        let kind = name
            .chars()
            .filter_map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    Some(c.to_ascii_lowercase())
                } else if c == ' ' || c == '_' {
                    Some('-')
                } else {
                    None
                }
            })
            .collect::<String>();
        let kind = if kind.is_empty() {
            "theorem".to_string()
        } else {
            kind
        };
        // Convert the display name through the sub-emitter so LaTeX math
        // (e.g. `Theorem A$^\star_{\mathrm{global}}$`) becomes valid Typst.
        let converted_name = self.render_in_sub_emitter(name, false, true);
        // The optional `\begin{theorem}[Note]` becomes the figure caption; a
        // per-kind head show rule (emitted in finish()) renders it as
        // "Theorem N (Note). body".
        let note = theorem_note(env, self.src)
            .map(|n| {
                self.render_in_sub_emitter(&n, false, true)
                    .trim()
                    .to_string()
            })
            .filter(|n| !n.is_empty());
        self.used_theorem_kinds.insert(kind.clone());
        let _ = write!(
            self.out,
            "#figure(kind: \"{}\", supplement: [{}]",
            kind,
            converted_name.trim()
        );
        if let Some(n) = &note {
            let _ = write!(self.out, ", caption: [{}]", n);
        }
        let _ = write!(self.out, ", [{}])", inner.trim());
        if let Some(l) = label {
            if self.label_first_use(&l) {
                let _ = write!(self.out, " <{}>", l);
            }
        }
        env.end_byte()
    }

    /// `\begin{proof}...\end{proof}` → `*Proof.* body` as a paragraph block.
    pub(in crate::emit) fn emit_proof_env(&mut self, env: Node<'_>) -> usize {
        let mut cursor = env.walk();
        let body: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();
        let inner = if body.is_empty() {
            String::new()
        } else {
            self.with_sub_buffer(|emitter| {
                let mut last = body[0].start_byte();
                for child in &body {
                    let cs = child.start_byte();
                    emitter.safe_copy(last, cs);
                    last = emitter.emit_node(*child);
                }
                let end = body.last().unwrap().end_byte();
                emitter.safe_copy(last, end);
            })
        };
        self.ensure_paragraph_break();
        let _ = write!(self.out, "*Proof.* {}", inner.trim());
        env.end_byte()
    }

    pub(in crate::emit) fn warn_unsupported_environment(
        &mut self,
        node: Node<'_>,
        env_name: Option<&str>,
    ) {
        let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
        let name = env_name.unwrap_or("?").to_string();
        self.warnings.push(Warning {
            range: range_of(node),
            category: Category::UnsupportedEnvironment { name },
            severity: Severity::Warning,
            message: "environment not yet supported by ByeTex; raw source dropped".to_string(),
            snippet,
            suggested_skill: None,
        });
    }

    // ─── Theorem & tcolorbox macro harvesting ─────────────────────────────────

    /// Harvest a `theorem_definition` node (`\newtheorem{name}{Display}` and
    /// variants) into `self.theorem_kinds` so that `emit_generic_environment`
    /// can route unknown environment names to `emit_theorem_env` instead of
    /// warning.
    pub(in crate::emit) fn harvest_theorem_definition(&mut self, node: Node<'_>) {
        if let Some((name, display)) = extract_theorem_def(node, self.src) {
            self.theorem_kinds.entry(name).or_insert(display);
        }
    }

    /// Harvest every definition (macros, `\def`, `\let`, theorems, `\newif`
    /// flags, tcolorbox/siam envs) from a standalone `source` fragment into the
    /// emitter's tables, parent-wins (existing entries are never overwritten).
    ///
    /// Used to register the contents of a skipped `\makeatletter ... \makeatother`
    /// region: the normal emit walk performs these registrations node-by-node as
    /// it renders, but the region is skipped from rendering, and `\input` child
    /// emitters run no prepass to fall back on. `source` is parsed as its own
    /// fragment, so all byte offsets stay self-consistent.
    pub(in crate::emit) fn harvest_definitions(&mut self, source: &str) {
        let tree = crate::parser::parse(source);
        let mut stack: Vec<Node<'_>> = vec![tree.root_node()];
        while let Some(n) = stack.pop() {
            match n.kind() {
                "new_command_definition" => {
                    if let Some((name, def)) = extract_newcommand(n, source) {
                        self.macros.entry(name).or_insert(def);
                    }
                }
                "old_command_definition" => {
                    let mut harvested: HashMap<String, MacroDef> = HashMap::new();
                    let _ = extract_def_and_record(n, source, &mut harvested);
                    for (k, v) in harvested {
                        self.macros.entry(k).or_insert(v);
                    }
                }
                "let_command_definition" => {
                    if let Some((new_name, old_name)) = extract_let(n, source) {
                        let def = let_alias_def(&old_name, &self.macros);
                        self.macros.entry(new_name).or_insert(def);
                    }
                }
                "theorem_definition" => {
                    if let Some((name, display)) = extract_theorem_def(n, source) {
                        self.theorem_kinds.entry(name).or_insert(display);
                    }
                }
                "generic_command" => {
                    match command_name_text(n, source).as_deref() {
                        Some("\\newif") => {
                            if let Some((flag, _)) = read_newif_flag(source, n.end_byte()) {
                                self.newif_flags.entry(flag).or_insert(false);
                            }
                        }
                        Some("\\newcommandx") => {
                            if let Some((name, def)) = extract_newcommandx(n, source) {
                                self.macros.entry(name).or_insert(def);
                            }
                        }
                        Some("\\newtcolorbox") | Some("\\newmdenv") => {
                            self.harvest_tcolorbox_decl(n, source);
                        }
                        Some("\\newsiamremark") | Some("\\newsiamthm") => {
                            self.harvest_generic_theorem_cmd(n, source);
                        }
                        _ => {}
                    }
                    let mut cursor = n.walk();
                    for c in n.children(&mut cursor) {
                        stack.push(c);
                    }
                }
                _ => {
                    let mut cursor = n.walk();
                    for c in n.children(&mut cursor) {
                        stack.push(c);
                    }
                }
            }
        }
    }

    /// Harvest `\newtcolorbox{name}{opts}` and `\newmdenv{name}{opts}`: record
    /// the env name in `theorem_kinds` with an empty-string sentinel so that
    /// any `\begin{name}...\end{name}` is treated as transparent (body
    /// pass-through) rather than triggering an `UnsupportedEnvironment` warning.
    pub(in crate::emit) fn harvest_tcolorbox_decl(&mut self, node: Node<'_>, source: &str) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if matches!(
                child.kind(),
                "curly_group" | "curly_group_text" | "curly_group_text_list"
            ) {
                let raw = &source[child.start_byte()..child.end_byte()];
                let name = raw
                    .trim_matches(|c: char| c == '{' || c == '}')
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    // Empty display = transparent sentinel (not a theorem block).
                    self.theorem_kinds.entry(name).or_default();
                }
                break;
            }
        }
    }

    /// Harvest a `\newenvironment{name}{begindef}{enddef}` /
    /// `\renewenvironment` node (tree-sitter `environment_definition`). The env
    /// `name` is registered with the empty-display sentinel so the env body is
    /// passed through transparently when used (see [`harvest_tcolorbox_decl`]).
    pub(in crate::emit) fn harvest_environment_definition(&mut self, node: Node<'_>) {
        if let Some((name, nargs)) = extract_environment_def(node, self.src) {
            if nargs > 0 {
                self.env_arg_counts.entry(name.clone()).or_insert(nargs);
            }
            self.theorem_kinds.entry(name).or_default();
        }
    }

    /// Harvest a SIAM-style theorem declaration (`\newsiamremark{name}{Display}`,
    /// `\newsiamthm{name}{Display}`) from a generic_command node into
    /// `self.theorem_kinds`. These commands share the same two-curly-group
    /// signature as `\newtheorem`.
    pub(in crate::emit) fn harvest_generic_theorem_cmd(&mut self, node: Node<'_>, source: &str) {
        let mut cursor = node.walk();
        let mut groups: Vec<(usize, usize)> = Vec::new();
        for child in node.children(&mut cursor) {
            if matches!(
                child.kind(),
                "curly_group" | "curly_group_text" | "curly_group_text_list"
            ) {
                groups.push((child.start_byte(), child.end_byte()));
                if groups.len() == 2 {
                    break;
                }
            }
        }
        let [name_range, display_range] = groups.as_slice() else {
            return;
        };
        let name = source[name_range.0..name_range.1]
            .trim_matches(|c: char| c == '{' || c == '}')
            .trim()
            .to_string();
        let display = source[display_range.0..display_range.1]
            .trim_matches(|c: char| c == '{' || c == '}')
            .trim()
            .to_string();
        if !name.is_empty() && !display.is_empty() {
            self.theorem_kinds.entry(name).or_insert(display);
        }
    }
}

/// Map a beamer `column` `{width}` to a Typst grid column spec. A
/// `\textwidth`/`\linewidth`/`\columnwidth`-relative width (`0.5\textwidth`)
/// becomes an `fr` ratio (`0.5fr`); a bare absolute length (`3cm`, `40mm`) passes
/// through; anything else falls back to `1fr`.
fn column_width_to_typst(raw: &str) -> String {
    let s = raw.trim();
    for unit in ["\\textwidth", "\\linewidth", "\\columnwidth", "\\paperwidth"] {
        if let Some(num) = s.strip_suffix(unit) {
            let n = num.trim().trim_end_matches('*').trim();
            let n = if n.is_empty() { "1" } else { n };
            return format!("{}fr", normalize_leading_dot(n));
        }
    }
    // Absolute length like `3cm` / `.5cm` / `40mm` / `2in` — keep for Typst.
    if s.starts_with(|c: char| c.is_ascii_digit() || c == '.')
        && s.ends_with(|c: char| c.is_ascii_alphabetic())
    {
        return normalize_leading_dot(s);
    }
    "1fr".to_string()
}

/// `.5` → `0.5`: Typst number literals require a leading digit, but LaTeX widths
/// idiomatically drop it (`{.45\textwidth}`). Leaves other values unchanged.
fn normalize_leading_dot(s: &str) -> String {
    match s.strip_prefix('.') {
        Some(rest) => format!("0.{rest}"),
        None => s.to_string(),
    }
}

/// Byte offset where an `enum_item`'s body begins: after `\item` and any
/// optional `[label]`.
fn item_body_start(item: Node<'_>, src: &str) -> usize {
    if let Some((_, body_start)) = item_bracket(item, src) {
        return body_start;
    }
    let mut cursor = item.walk();
    let start = item
        .children(&mut cursor)
        .find(|c| c.kind() == "\\item")
        .map(|c| c.end_byte())
        .unwrap_or_else(|| item.start_byte());
    start
}

/// True if an `enum_item` carries a beamer overlay spec right after `\item`
/// (`\item<1-> body`). Used to decide whether to inject a `#pause` before the item
/// for a touying sequential reveal.
pub(in crate::emit) fn item_has_overlay_spec(item: Node<'_>, src: &str) -> bool {
    let body_start = item_body_start(item, src);
    skip_leading_overlay_spec(src, body_start) != body_start
}

/// If `src[start..]` (after optional spaces) begins with a beamer `<overlay-spec>`
/// (`<` + only `0-9 + - . , | space` + `>`), return the byte just past `>`; else `start`.
/// Used (beamer-only) to drop the `<…>` of `\item<1->`.
pub(in crate::emit) fn skip_leading_overlay_spec(src: &str, start: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = start;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'<') {
        return start;
    }
    let mut j = i + 1;
    while j < bytes.len() && bytes[j] != b'>' {
        if !matches!(bytes[j], b'0'..=b'9' | b'+' | b'-' | b'.' | b',' | b'|' | b' ') {
            return start;
        }
        j += 1;
    }
    if bytes.get(j) == Some(&b'>') {
        j + 1
    } else {
        start
    }
}

/// The optional `[label]` of an `enum_item`, matched from source (depth-aware
/// over nested `[]`): `(raw_label, body_start_byte)`. Source-based because
/// tree-sitter doesn't model `\item[(a)]` (leading paren) as a bracket group.
fn item_bracket(item: Node<'_>, src: &str) -> Option<(String, usize)> {
    let mut cursor = item.walk();
    let item_end = item
        .children(&mut cursor)
        .find(|c| c.kind() == "\\item")?
        .end_byte();
    let bytes = src.as_bytes();
    let mut i = item_end;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if bytes.get(i) != Some(&b'[') {
        return None;
    }
    let content_start = i + 1;
    let mut depth = 1usize;
    i += 1;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    if depth != 0 {
        return None;
    }
    let label = src.get(content_start..i - 1)?.to_string();
    Some((label, i))
}

/// If `\begin{enumerate}[...]` carries an enumitem counter FORMAT, return the
/// equivalent Typst numbering pattern (`(a)`, `(i)`, `1.`, …). `None` for a
/// plain enumerate or pure-option specs (`[noitemsep]`, `[leftmargin=*]`).
fn enumerate_numbering(env: Node<'_>, src: &str) -> Option<String> {
    let s = src.get(env.start_byte()..env.end_byte())?;
    let after = s.strip_prefix("\\begin")?.trim_start().strip_prefix('{')?;
    let close = after.find('}')?;
    let rest = after.get(close + 1..)?.trim_start().strip_prefix('[')?;
    // matching `]` (depth-aware over nested brackets)
    let bytes = rest.as_bytes();
    let mut depth = 1usize;
    let mut j = 0usize;
    while j < bytes.len() && depth > 0 {
        match bytes[j] {
            b'[' => depth += 1,
            b']' => depth -= 1,
            _ => {}
        }
        j += 1;
    }
    if depth != 0 {
        return None;
    }
    numbering_from_spec(&rest[..j - 1])
}

/// Map an enumitem label spec to a Typst numbering pattern. Accepts the
/// shortcut form (`(a)`) and `label=…`; converts counter macros (`\alph*`→`a`,
/// `\roman*`→`i`, `\arabic*`→`1`, upper variants → `A`/`I`). Returns `None`
/// unless the result is a single counter symbol wrapped in punctuation.
fn numbering_from_spec(spec: &str) -> Option<String> {
    let cand = if let Some(pos) = spec.find("label=") {
        spec.get(pos + 6..)?.split(',').next().unwrap_or("").trim()
    } else {
        let first = spec.split(',').next().unwrap_or("").trim();
        if first.contains('=') {
            return None; // first segment is an option key, no label shortcut
        }
        first
    };
    let conv = cand
        .replace("\\alph*", "a")
        .replace("\\alph", "a")
        .replace("\\Alph*", "A")
        .replace("\\Alph", "A")
        .replace("\\roman*", "i")
        .replace("\\roman", "i")
        .replace("\\Roman*", "I")
        .replace("\\Roman", "I")
        .replace("\\arabic*", "1")
        .replace("\\arabic", "1");
    let conv = conv.trim();
    // Exactly one alphanumeric, and it must be a Typst counter symbol; the rest
    // is punctuation (parens/dots). Rejects words like `noitemsep`.
    let alnum: Vec<char> = conv.chars().filter(|c| c.is_alphanumeric()).collect();
    if alnum.len() == 1 && matches!(alnum[0], 'a' | 'A' | 'i' | 'I' | '1') && !conv.contains('"') {
        Some(conv.to_string())
    } else {
        None
    }
}

/// The optional `\begin{<thm>}[Note]` argument, matched from source. Requires
/// the `[` on the same line as `\begin{…}` (only spaces/tabs between), so a body
/// that starts with `[` on a new line is not mistaken for a note. Depth-aware
/// over nested `[]`.
fn theorem_note(env: Node<'_>, src: &str) -> Option<String> {
    let s = src.get(env.start_byte()..env.end_byte())?;
    let after = s.strip_prefix("\\begin")?.trim_start().strip_prefix('{')?;
    let close = after.find('}')?;
    let gap = after.get(close + 1..)?;
    let rest = gap.trim_start_matches([' ', '\t']).strip_prefix('[')?;
    let bytes = rest.as_bytes();
    let mut depth = 1usize;
    let mut j = 0usize;
    while j < bytes.len() && depth > 0 {
        match bytes[j] {
            b'[' => depth += 1,
            b']' => depth -= 1,
            _ => {}
        }
        j += 1;
    }
    if depth != 0 {
        return None;
    }
    let note = rest[..j - 1].trim();
    if note.is_empty() {
        None
    } else {
        Some(note.to_string())
    }
}
