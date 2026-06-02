//! Environment emission (theorem/proof/lists/minipage/subequations) + theorem-definition harvesting, extracted from emit.rs (pure code motion).

use std::collections::HashMap;
use std::fmt::Write;

use tree_sitter::Node;

use super::{
    command_name_text, environment_name, extract_def_and_record, extract_environment_def,
    extract_label_name, extract_label_name_and_end, extract_let, extract_newcommand,
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

    /// Emit just the body of an environment (skip `begin` and `end` children).
    /// Strips trailing whitespace already in `self.out` so that preamble noise
    /// (dropped `\documentclass`, `\usepackage`, blank lines) doesn't leak
    /// in as leading newlines.
    pub(in crate::emit) fn emit_environment_body(&mut self, env: Node<'_>) -> usize {
        while self.out.ends_with('\n') || self.out.ends_with(' ') || self.out.ends_with('\t') {
            self.out.pop();
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
        let mut cursor = env.walk();
        let mut first = true;
        for child in env.children(&mut cursor) {
            if child.kind() != "enum_item" {
                continue;
            }
            if !first {
                self.out.push('\n');
            }
            let body = self.render_enum_item_body(child, /* description: */ false);
            let _ = write!(self.out, "{} {}", marker, body.trim());
            first = false;
        }
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

    /// Render the body of an `enum_item` (everything after `\item` and after
    /// the optional `[term]` bracket group). If `is_description` is true, the
    /// `brack_group_text` child is treated as the term and not included.
    pub(in crate::emit) fn render_enum_item_body(&mut self, item: Node<'_>, is_description: bool) -> String {
        let mut cursor = item.walk();
        let children: Vec<Node<'_>> = item.children(&mut cursor).collect();

        let body: Vec<Node<'_>> = children
            .into_iter()
            .filter(|c| {
                let k = c.kind();
                if k == "\\item" {
                    return false;
                }
                if is_description && k == "brack_group_text" {
                    return false;
                }
                true
            })
            .collect();

        if body.is_empty() {
            return String::new();
        }

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
    }

    pub(in crate::emit) fn render_enum_item_term(&mut self, item: Node<'_>) -> Option<String> {
        let mut cursor = item.walk();
        for child in item.children(&mut cursor) {
            if child.kind() == "brack_group_text" {
                return Some(self.render_curly_group_content(child));
            }
        }
        None
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
                    label = extract_label_name(*c, self.src);
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
        let _ = write!(
            self.out,
            "#figure(kind: \"{}\", supplement: [{}], [{}])",
            kind,
            converted_name.trim(),
            inner.trim()
        );
        if let Some(l) = label {
            let _ = write!(self.out, " <{}>", l);
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


    pub(in crate::emit) fn warn_unsupported_environment(&mut self, node: Node<'_>, env_name: Option<&str>) {
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
