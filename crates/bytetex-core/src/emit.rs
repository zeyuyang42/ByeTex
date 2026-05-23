//! Emitter — walks the tree-sitter AST and produces Typst source plus warnings.
//!
//! ## Scope
//!
//! - M1: plain text passthrough, `%`-comment dropping, generic warning for any
//!   unrecognised backslash command.
//! - M2: sectioning (`\section`..`\subparagraph`, starred forms, attached labels).
//!   Inline formatting + lists come in subsequent M2 sub-tasks; this file is
//!   structured around a dispatch-by-kind pattern so each batch is additive.

use std::fmt::Write;

use tree_sitter::Node;

use crate::warnings::{Category, Range, Severity, Warning};

pub(crate) struct Emitter<'a> {
    out: String,
    warnings: Vec<Warning>,
    src: &'a str,
    #[allow(dead_code)]
    source_name: &'a str,
    /// True while emitting the interior of a math container. Affects how
    /// commands (e.g. `\alpha` → `alpha`) and subscripts (`_{x}` → `_(x)`)
    /// are rendered.
    in_math: bool,
    /// While emitting inside a math container, `\label{x}` is recorded here
    /// and later attached to the enclosing equation/figure as a Typst label.
    /// Cleared by the container emitter after attachment.
    pending_math_label: Option<String>,
    /// Captures `\bibliographystyle{plain}` so a following `\bibliography{refs}`
    /// can attach the style. Cleared after use.
    pending_bib_style: Option<String>,
    /// Set when we emit a numbered heading reference (`@sec:...`). Typst
    /// refuses to reference headings without numbering, so we prepend a
    /// `#set heading(numbering: ...)` to the output in `finish()`.
    needs_heading_numbering: bool,
    /// Same for equations / `@eq:...` references.
    needs_equation_numbering: bool,
    /// Tracks the key of an in-flight `\bibitem{key}` so we can close its
    /// content wrapper and attach `<key>` at the right place.
    pending_bibitem_key: Option<String>,
    /// Byte offset up to which the emitter should skip nodes — used by the
    /// `\verb|...|` handler, since the tree-sitter grammar does not model
    /// verb delimiters and would otherwise re-emit the inner tokens.
    skip_until: usize,
}

impl<'a> Emitter<'a> {
    pub(crate) fn new(src: &'a str, source_name: &'a str) -> Self {
        Self {
            out: String::new(),
            warnings: Vec::new(),
            src,
            source_name,
            in_math: false,
            pending_math_label: None,
            pending_bib_style: None,
            needs_heading_numbering: false,
            needs_equation_numbering: false,
            pending_bibitem_key: None,
            skip_until: 0,
        }
    }

    pub(crate) fn emit_root(&mut self, root: Node<'_>) {
        let _ = self.emit_node(root);
    }

    pub(crate) fn finish(mut self) -> (String, Vec<Warning>) {
        // Conditional Typst preamble for documents that use references. LaTeX
        // numbers sections and equations by default; Typst does not. Without
        // this preamble, `@sec:foo` etc. fail with "cannot reference X without
        // numbering".
        let mut preamble = String::new();
        if self.needs_heading_numbering {
            preamble.push_str("#set heading(numbering: \"1.\")\n");
        }
        if self.needs_equation_numbering {
            preamble.push_str("#set math.equation(numbering: \"(1)\")\n");
        }
        if !preamble.is_empty() {
            preamble.push('\n');
            preamble.push_str(&self.out);
            self.out = preamble;
        }
        (self.out, self.warnings)
    }

    /// Push `src[from..to]` to the output, but only when the range is valid.
    /// Some emitters (notably comment-drop with newline consumption) advance
    /// the cursor past a node's `end_byte`; downstream trailing-copy logic
    /// must tolerate the resulting reverse range as a no-op.
    fn safe_copy(&mut self, from: usize, to: usize) {
        if from < to {
            self.out.push_str(&self.src[from..to]);
        }
    }

    /// Emit `node` and return the source byte offset to resume after.
    fn emit_node(&mut self, node: Node<'_>) -> usize {
        // Skip nodes that fall inside a region already consumed (e.g. by the
        // `\verb|...|` handler, which slurps tokens the grammar parsed as if
        // they were live LaTeX).
        if node.start_byte() < self.skip_until {
            return self.skip_until.max(node.end_byte());
        }

        // Comments: drop the comment AND the trailing newline (LaTeX `%` semantics).
        if is_comment(node.kind()) {
            let end = node.end_byte();
            return if self.src.as_bytes().get(end) == Some(&b'\n') {
                end + 1
            } else {
                end
            };
        }

        // Math containers — switch to math mode and render the body.
        match node.kind() {
            "inline_formula" => return self.emit_inline_math(node),
            "displayed_equation" => return self.emit_display_math(node),
            "math_environment" => return self.emit_math_environment(node),
            "subscript" if self.in_math => return self.emit_subscript(node, "_"),
            "superscript" if self.in_math => return self.emit_subscript(node, "^"),
            _ => {}
        }

        // Sectioning: \section, \subsection, ...; starred forms preserved.
        if is_section_kind(node.kind()) {
            return self.emit_section(node);
        }

        // Backslash commands: look up by name, fall through to warn-and-drop.
        if node.kind() == "generic_command" {
            return self.emit_generic_command(node);
        }

        // \begin{X} ... \end{X}: dispatch by environment name.
        if node.kind() == "generic_environment" {
            return self.emit_generic_environment(node);
        }

        // Inside math, `\label{...}` is silently lifted out and attached to
        // the enclosing math container as a Typst `<label>`.
        if self.in_math && node.kind() == "label_definition" {
            if let Some(l) = extract_label_name(node, self.src) {
                self.pending_math_label = Some(l);
            }
            return node.end_byte();
        }

        // M4 dedicated node kinds.
        match node.kind() {
            "citation" => return self.emit_citation(node),
            "label_reference" => return self.emit_label_reference(node),
            "bibtex_include" => return self.emit_bibliography(node),
            "bibstyle_include" => return self.emit_bibstyle(node),
            "graphics_include" => return self.emit_graphics_include(node),
            "caption" => {
                // Standalone caption (outside a figure) — drop with warning.
                self.warn_unsupported_command(node);
                return node.end_byte();
            }
            _ => {}
        }

        // Outside math, escape Typst-special characters that appear bare in
        // the LaTeX source. In LaTeX text mode these are literal characters;
        // Typst would interpret them as markup (bold, italic, brackets, etc.).
        if !self.in_math {
            if let Some(escaped) = needs_text_escape(node.kind()) {
                self.out.push_str(escaped);
                return node.end_byte();
            }
        }

        // Other "command-shaped" nodes (citation, includes, etc.) — warn until
        // the relevant later milestone implements them.
        if is_command(node.kind()) {
            self.warn_unsupported_command(node);
            return node.end_byte();
        }

        self.emit_recursive_with_gaps(node)
    }

    /// Default emission: copy source bytes between sibling children, recursing
    /// into each child. Leaves (no children) emit their full source span.
    fn emit_recursive_with_gaps(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let mut last = node.start_byte();
        for child in node.children(&mut cursor) {
            let cs = child.start_byte();
            self.safe_copy(last, cs);
            last = self.emit_node(child);
        }
        self.safe_copy(last, node.end_byte());
        node.end_byte()
    }

    fn emit_generic_command(&mut self, node: Node<'_>) -> usize {
        let name = command_name_text(node, self.src);
        if self.in_math {
            return self.emit_math_command(node, name.as_deref());
        }
        // `\verb<delim>content<delim>`: tree-sitter does not model the verb
        // delimiter scope, so we manually consume the source from the byte
        // after `\verb` to the next occurrence of the delimiter, and skip any
        // tokens the grammar produced inside.
        if name.as_deref() == Some("\\verb") {
            let bytes = self.src.as_bytes();
            let end = node.end_byte();
            if let Some(&delim) = bytes.get(end) {
                if let Some(rel) = bytes[end + 1..].iter().position(|&b| b == delim) {
                    let close = end + 1 + rel;
                    let content = &self.src[end + 1..close];
                    let _ = write!(self.out, "`{}`", content);
                    self.skip_until = close + 1;
                    return close + 1;
                }
            }
            self.warn_unsupported_command(node);
            return node.end_byte();
        }

        // `\bibitem{key}` inside `thebibliography` becomes a `#figure(...)`
        // with a custom kind so that `@key` references resolve. Typst only
        // allows labels to be referenced on a few element kinds — `figure`
        // with `supplement: none` is the least-intrusive.
        if name.as_deref() == Some("\\bibitem") {
            if let Some(arg) = first_curly_group(node) {
                let key = self
                    .src
                    .get(arg.start_byte() + 1..arg.end_byte() - 1)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !key.is_empty() {
                    self.close_bibitem();
                    if !self.out.ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out
                        .push_str("#figure(kind: \"bibitem\", supplement: none, [");
                    self.pending_bibitem_key = Some(key);
                    return node.end_byte();
                }
            }
        }
        match name.as_deref() {
            // Italic / emphasis
            Some("\\emph") | Some("\\textit") => self.emit_inline_wrap(node, "_", "_"),
            // Bold
            Some("\\textbf") => self.emit_inline_wrap(node, "*", "*"),
            // Monospace / typewriter
            Some("\\texttt") => self.emit_inline_wrap(node, "`", "`"),
            // Underline
            Some("\\underline") => self.emit_inline_wrap(node, "#underline[", "]"),
            // Small caps
            Some("\\textsc") => self.emit_inline_wrap(node, "#smallcaps[", "]"),
            // Roman / default — no formatting, just render the body
            Some("\\textrm") | Some("\\textnormal") | Some("\\textmd") | Some("\\textup") => {
                self.emit_inline_unwrap(node)
            }
            // Forced line break: Typst uses `\` followed by whitespace.
            Some("\\\\") => {
                if !self.out.ends_with(' ') && !self.out.ends_with('\n') {
                    self.out.push(' ');
                }
                self.out.push('\\');
                node.end_byte()
            }
            // Layout-only commands: drop silently and eat the trailing space
            // that LaTeX would consume after a command-without-args.
            Some("\\noindent") | Some("\\indent") => {
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            _ => {
                self.warn_unsupported_command(node);
                node.end_byte()
            }
        }
    }

    /// Find the first `curly_group` child of `node` and render its inner
    /// content wrapped between `left` and `right`. Falls back to dropping
    /// the command if no argument is present.
    fn emit_inline_wrap(&mut self, node: Node<'_>, left: &str, right: &str) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let content = self.render_curly_group_content(arg);
            self.out.push_str(left);
            self.out.push_str(&content);
            self.out.push_str(right);
        }
        node.end_byte()
    }

    /// Emit just the body of `\textrm{X}` etc. — strips the command, keeps `X`.
    fn emit_inline_unwrap(&mut self, node: Node<'_>) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let content = self.render_curly_group_content(arg);
            self.out.push_str(&content);
        }
        node.end_byte()
    }

    fn emit_generic_environment(&mut self, node: Node<'_>) -> usize {
        let env = environment_name(node, self.src);
        match env.as_deref() {
            Some("itemize") => self.emit_simple_list(node, "-"),
            Some("enumerate") => self.emit_simple_list(node, "+"),
            Some("description") => self.emit_description(node),
            // Transparent wrappers: emit body, no markup. `\documentclass` etc.
            // already produced warnings as separate top-level commands.
            Some("document") | Some("abstract") => self.emit_environment_body(node),
            // Matrix family — handled wherever we encounter them. If we're
            // not already in math mode, the surrounding container will wrap
            // us; pmatrix() etc. assume math context.
            Some("pmatrix") | Some("bmatrix") | Some("vmatrix") | Some("Vmatrix")
            | Some("Bmatrix") | Some("matrix") => self.emit_matrix_env(node, env.as_deref()),
            // `cases` env produces piecewise display.
            Some("cases") => self.emit_cases_env(node),
            // M4: tables and figure floats.
            Some("tabular") | Some("tabular*") | Some("array") => self.emit_tabular(node),
            Some("figure") | Some("figure*") | Some("table") => self.emit_figure(node),
            // IEEE/thebibliography style: emit each \bibitem as a labeled
            // numbered-list entry so `@bN` references resolve.
            Some("thebibliography") => self.emit_thebibliography(node),
            _ => {
                self.warn_unsupported_environment(node, env.as_deref());
                node.end_byte()
            }
        }
    }

    /// Emit just the body of an environment (skip `begin` and `end` children).
    /// Strips trailing whitespace already in `self.out` so that preamble noise
    /// (dropped `\documentclass`, `\usepackage`, blank lines) doesn't leak
    /// in as leading newlines.
    fn emit_environment_body(&mut self, env: Node<'_>) -> usize {
        while self.out.ends_with('\n') || self.out.ends_with(' ') || self.out.ends_with('\t') {
            self.out.pop();
        }

        let mut cursor = env.walk();
        let body: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();

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

    fn emit_simple_list(&mut self, env: Node<'_>, marker: &str) -> usize {
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

    fn emit_description(&mut self, env: Node<'_>) -> usize {
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
    fn render_enum_item_body(&mut self, item: Node<'_>, is_description: bool) -> String {
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

    fn render_enum_item_term(&mut self, item: Node<'_>) -> Option<String> {
        let mut cursor = item.walk();
        for child in item.children(&mut cursor) {
            if child.kind() == "brack_group_text" {
                return Some(self.render_curly_group_content(child));
            }
        }
        None
    }

    /// Close any in-flight `\bibitem{key}` by emitting `]) <key>` so the label
    /// attaches to the entry's `#figure[...]` wrapper.
    fn close_bibitem(&mut self) {
        if let Some(key) = self.pending_bibitem_key.take() {
            // Trim trailing whitespace so the closing bracket sits flush.
            while self.out.ends_with(' ') || self.out.ends_with('\n') {
                self.out.pop();
            }
            let _ = write!(self.out, "]) <{}>\n", key);
        }
    }

    /// `\begin{thebibliography}{99}...\bibitem{k} entry text...\end{thebibliography}`
    /// → numbered list with `<k>` labels per entry. The `{99}` width spec is
    /// dropped.
    fn emit_thebibliography(&mut self, env: Node<'_>) -> usize {
        self.ensure_paragraph_break();
        let mut cursor = env.walk();
        // Skip begin/end. Also skip the leading curly_group_text (the width
        // spec like `{99}`) that lives right after `begin`.
        let mut seen_first_curly_after_begin = false;
        let body: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| {
                if matches!(c.kind(), "begin" | "end") {
                    return false;
                }
                if !seen_first_curly_after_begin
                    && matches!(
                        c.kind(),
                        "curly_group_text" | "curly_group" | "curly_group_word"
                    )
                {
                    seen_first_curly_after_begin = true;
                    return false;
                }
                true
            })
            .collect();
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
        // Close the final pending bibitem (no following bibitem to do it).
        self.close_bibitem();
        env.end_byte()
    }

    fn warn_unsupported_environment(&mut self, node: Node<'_>, env_name: Option<&str>) {
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

    // ===== Math mode =====

    fn emit_inline_math(&mut self, node: Node<'_>) -> usize {
        self.out.push('$');
        let was = self.in_math;
        self.in_math = true;
        self.emit_math_children(node);
        self.in_math = was;
        self.out.push('$');
        node.end_byte()
    }

    fn emit_display_math(&mut self, node: Node<'_>) -> usize {
        // Typst block math wants a blank line before the `$ ... $`.
        self.ensure_paragraph_break();
        self.out.push_str("$ ");
        let was = self.in_math;
        self.in_math = true;
        self.emit_math_children(node);
        self.in_math = was;
        // Trim trailing whitespace we accumulated inside (newlines from layout) so
        // the closing `$` follows directly after the content.
        while self.out.ends_with(' ') || self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out.push_str(" $");
        node.end_byte()
    }

    /// `\begin{equation}...\end{equation}` and friends. The grammar tags these
    /// as `math_environment` (distinct from `generic_environment`). We treat
    /// numbered/unnumbered forms the same and let Typst handle numbering.
    fn emit_math_environment(&mut self, node: Node<'_>) -> usize {
        let _env_name = environment_name(node, self.src).unwrap_or_default();
        self.ensure_paragraph_break();
        self.out.push_str("$ ");
        let was = self.in_math;
        self.in_math = true;

        let prev_label = self.pending_math_label.take();
        let mut cursor = node.walk();
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();

        if !body.is_empty() {
            let mut last = body[0].start_byte();
            for child in &body {
                let cs = child.start_byte();
                self.safe_copy(last, cs);
                last = self.emit_node(*child);
            }
            let end = body.last().unwrap().end_byte();
            self.safe_copy(last, end);
        }

        self.in_math = was;
        while self.out.ends_with(' ') || self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out.push_str(" $");
        if let Some(l) = self.pending_math_label.take() {
            let _ = write!(self.out, " <{}>", l);
        }
        self.pending_math_label = prev_label;
        node.end_byte()
    }

    /// Skip the math delimiters (`$`, `$$`, `\[`, `\]`) and emit interior
    /// children with the usual gap-copy mechanism.
    fn emit_math_children(&mut self, node: Node<'_>) {
        let mut cursor = node.walk();
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "$" | "$$" | "\\[" | "\\]"))
            .collect();
        if body.is_empty() {
            return;
        }
        let mut last = body[0].start_byte();
        for child in &body {
            let cs = child.start_byte();
            self.safe_copy(last, cs);
            last = self.emit_node(*child);
        }
        let end = body.last().unwrap().end_byte();
        self.safe_copy(last, end);
    }

    /// Emit a math command inside `$...$`. Looks up name in the symbol table;
    /// if not found, falls back to structural commands (\frac, \sqrt, ...);
    /// if still not found, emits an `AmbiguousMath` warning and a placeholder.
    fn emit_math_command(&mut self, node: Node<'_>, name: Option<&str>) -> usize {
        let n = match name {
            Some(s) => s,
            None => {
                self.out
                    .push_str(&self.src[node.start_byte()..node.end_byte()]);
                return node.end_byte();
            }
        };
        if let Some(typst) = lookup_math_symbol(n) {
            self.out.push_str(typst);
            return node.end_byte();
        }
        match n {
            "\\frac" => self.emit_math_frac(node),
            "\\sqrt" => self.emit_math_sqrt(node),
            "\\binom" => self.emit_math_binom(node),
            // Row break inside math envs. We emit just `\`; the source's
            // surrounding whitespace (gap-copied by the parent) takes care of
            // spacing around it.
            "\\\\" => {
                self.out.push('\\');
                node.end_byte()
            }
            // Thin/medium/thick math spaces.
            "\\," => {
                self.out.push_str("thin");
                node.end_byte()
            }
            "\\;" => {
                self.out.push_str("thick");
                node.end_byte()
            }
            "\\:" => {
                self.out.push_str("med");
                node.end_byte()
            }
            _ => {
                self.warn_ambiguous_math(node, n);
                // Inside math, Typst accepts `"text"` as a literal text node.
                // Strip the leading backslash for readability.
                let display = n.strip_prefix('\\').unwrap_or(n);
                let _ = write!(self.out, " \"{}\" ", display);
                node.end_byte()
            }
        }
    }

    /// `\frac{a}{b}` → `(a) / (b)` per the M3 plan.
    fn emit_math_frac(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let args: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "curly_group")
            .collect();
        if args.len() < 2 {
            self.warn_ambiguous_math(node, "\\frac (missing args)");
            return node.end_byte();
        }
        let num = self.render_math_group(args[0]);
        let den = self.render_math_group(args[1]);
        let _ = write!(self.out, "({}) / ({})", num.trim(), den.trim());
        node.end_byte()
    }

    fn emit_math_sqrt(&mut self, node: Node<'_>) -> usize {
        let arg = first_curly_group(node);
        match arg {
            Some(g) => {
                let inner = self.render_math_group(g);
                let _ = write!(self.out, "sqrt({})", inner.trim());
            }
            None => {
                self.warn_ambiguous_math(node, "\\sqrt (missing arg)");
            }
        }
        node.end_byte()
    }

    fn emit_math_binom(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let args: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "curly_group")
            .collect();
        if args.len() < 2 {
            self.warn_ambiguous_math(node, "\\binom (missing args)");
            return node.end_byte();
        }
        let n = self.render_math_group(args[0]);
        let k = self.render_math_group(args[1]);
        let _ = write!(self.out, "binom({}, {})", n.trim(), k.trim());
        node.end_byte()
    }

    /// Subscript/superscript: emit the marker, then the argument. Single-char
    /// args go through unwrapped; multi-char args wrap in parens.
    fn emit_subscript(&mut self, node: Node<'_>, marker: &str) -> usize {
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        // Typst requires a base before `_` or `^`. If the previous character in
        // our output is whitespace or the opening math delimiter, the original
        // LaTeX had a bare attachment (e.g. `${}^{a}$` for a floating footnote
        // marker); prepend an empty string base so Typst accepts it.
        if needs_empty_base(&self.out) {
            self.out.push_str("\"\"");
        }
        let arg = children.iter().find(|c| !matches!(c.kind(), "_" | "^"));
        self.out.push_str(marker);
        if let Some(arg) = arg {
            if arg.kind() == "curly_group" {
                let inner = self.render_math_group(*arg);
                let _ = write!(self.out, "({})", inner.trim());
            } else {
                let _ = self.emit_node(*arg);
            }
        }
        node.end_byte()
    }

    /// Render the inside of a math `{ ... }` group into a fresh sub-string,
    /// preserving math mode.
    fn render_math_group(&mut self, group: Node<'_>) -> String {
        let mut cursor = group.walk();
        let children: Vec<Node<'_>> = group.children(&mut cursor).collect();
        let start_skip = usize::from(matches!(
            children.first().map(|n| n.kind()),
            Some("{") | Some("[")
        ));
        let end_skip = usize::from(matches!(
            children.last().map(|n| n.kind()),
            Some("}") | Some("]")
        ));
        let inner_len = children.len().saturating_sub(start_skip + end_skip);
        if inner_len == 0 {
            return String::new();
        }
        let inner = &children[start_skip..start_skip + inner_len];
        self.with_sub_buffer(|emitter| {
            let was = emitter.in_math;
            emitter.in_math = true;
            let mut last = inner[0].start_byte();
            for child in inner {
                let cs = child.start_byte();
                emitter.safe_copy(last, cs);
                last = emitter.emit_node(*child);
            }
            let end = inner.last().unwrap().end_byte();
            emitter.safe_copy(last, end);
            emitter.in_math = was;
        })
    }

    /// `\begin{pmatrix} a & b \\ c & d \end{pmatrix}` → `mat(a, b; c, d)`.
    fn emit_matrix_env(&mut self, node: Node<'_>, _env: Option<&str>) -> usize {
        let was = self.in_math;
        self.in_math = true;
        // Collect body source bytes between begin and end, then parse cells.
        let mut cursor = node.walk();
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();
        // Render the body, then split on `\\` for rows, then `&` within rows.
        let body_str = if body.is_empty() {
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

        // Split on the `\` token that our math-mode `\\` emitter writes
        // (always surrounded by a single space on the left).
        let rows: Vec<&str> = body_str.split(" \\").collect();
        let rendered: Vec<String> = rows
            .into_iter()
            .map(|row| {
                row.split('&')
                    .map(|cell| cell.trim().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect();
        let _ = write!(self.out, "mat({})", rendered.join("; "));
        self.in_math = was;
        node.end_byte()
    }

    /// `\begin{cases} ... \end{cases}` → `cases(... ; ... ; ...)`.
    fn emit_cases_env(&mut self, node: Node<'_>) -> usize {
        let was = self.in_math;
        self.in_math = true;
        let mut cursor = node.walk();
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end"))
            .collect();
        let body_str = if body.is_empty() {
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
        let rows: Vec<&str> = body_str.split(" \\").map(|r| r.trim()).collect();
        let _ = write!(self.out, "cases({})", rows.join("; "));
        self.in_math = was;
        node.end_byte()
    }

    /// Ensure two trailing newlines for a Typst paragraph break before a block.
    fn ensure_paragraph_break(&mut self) {
        if self.out.is_empty() {
            return;
        }
        while self.out.ends_with(' ') || self.out.ends_with('\t') {
            self.out.pop();
        }
        if !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        if !self.out.ends_with("\n\n") {
            self.out.push('\n');
        }
    }

    // ===== M4: refs, citations, floats =====

    fn emit_citation(&mut self, node: Node<'_>) -> usize {
        // Keys are inside `curly_group_text_list`, possibly with `,` separators.
        let keys = extract_citation_keys(node, self.src);
        if keys.is_empty() {
            self.warn_unsupported_command(node);
            return node.end_byte();
        }
        let typst = keys
            .iter()
            .map(|k| format!("@{}", k))
            .collect::<Vec<_>>()
            .join(" ");
        self.out.push_str(&typst);
        // See `emit_label_reference` for why we sometimes append a separator
        // after `@key`. Same logic applies to citations.
        let end = node.end_byte();
        if let Some(&b) = self.src.as_bytes().get(end) {
            if matches!(b, b'-' | b'_' | b':' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z') {
                self.out.push(' ');
            }
        }
        node.end_byte()
    }

    fn emit_label_reference(&mut self, node: Node<'_>) -> usize {
        // `label_reference` children are `\ref`, `\eqref`, `\pageref` as the
        // first child (with the backslash literally in the kind), then the
        // curly group with the label.
        let mut cursor = node.walk();
        let first_kind = node
            .children(&mut cursor)
            .next()
            .map(|c| c.kind().to_string());
        let key = extract_label_ref_key(node, self.src).unwrap_or_default();
        if key.is_empty() {
            self.warn_unsupported_command(node);
            return node.end_byte();
        }
        match first_kind.as_deref() {
            Some("\\eqref") => {
                self.needs_equation_numbering = true;
                let _ = write!(self.out, "(@{})", key);
            }
            Some("\\pageref") => {
                // Typst doesn't have a direct equivalent; warn once and emit `@key`.
                let _ = write!(self.out, "@{}", key);
                self.warnings.push(Warning {
                    range: range_of(node),
                    category: Category::NeedsManualReview {
                        reason: "\\pageref has no direct Typst equivalent".to_string(),
                    },
                    severity: Severity::Info,
                    message: "rendered as a normal reference; page numbers are limited in Typst"
                        .to_string(),
                    snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                    suggested_skill: None,
                });
            }
            _ => {
                // Heuristic: prefix tells us what the ref targets. Figures and
                // tables are auto-numbered by Typst; only headings/equations
                // need an explicit `#set ... (numbering: ...)` preamble.
                if key.starts_with("eq:") {
                    self.needs_equation_numbering = true;
                } else if !key.starts_with("fig:") && !key.starts_with("tab:") {
                    self.needs_heading_numbering = true;
                }
                let _ = write!(self.out, "@{}", key);
            }
        }
        // Typst labels include `-`, `.`, `:`, etc. If the source has
        // `\ref{A}--\ref{B}` with no space between, Typst will glue the dashes
        // onto the label. Append an explicit space when the next source byte
        // would form an identifier-continuation character.
        let end = node.end_byte();
        if let Some(&b) = self.src.as_bytes().get(end) {
            // `-`, `_`, `:`, digit, and lowercase/uppercase letters all extend
            // a Typst label identifier. `.` does NOT extend it, so we don't
            // append a separator before a sentence-final period.
            if matches!(b, b'-' | b'_' | b':' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z') {
                self.out.push(' ');
            }
        }
        node.end_byte()
    }

    fn emit_bibliography(&mut self, node: Node<'_>) -> usize {
        let path = extract_bib_path(node, self.src).unwrap_or_default();
        if path.is_empty() {
            self.warn_unsupported_command(node);
            return node.end_byte();
        }
        let style = self.pending_bib_style.take();
        // Convention: append `.bib` if no extension supplied.
        let path_with_ext = if path.contains('.') {
            path.clone()
        } else {
            format!("{}.bib", path)
        };
        self.ensure_paragraph_break();
        if let Some(s) = style {
            let _ = write!(
                self.out,
                "#bibliography(\"{}\", style: \"{}\")",
                path_with_ext, s
            );
        } else {
            let _ = write!(self.out, "#bibliography(\"{}\")", path_with_ext);
        }
        node.end_byte()
    }

    fn emit_bibstyle(&mut self, node: Node<'_>) -> usize {
        if let Some(style) = extract_bib_path(node, self.src) {
            self.pending_bib_style = Some(style);
        }
        // Style on its own doesn't emit anything; it attaches to the next
        // `\bibliography{...}`. Consume trailing newline so we don't leave a
        // blank line where the style used to be.
        let end = node.end_byte();
        let bytes = self.src.as_bytes();
        if bytes.get(end) == Some(&b'\n') {
            end + 1
        } else {
            end
        }
    }

    fn emit_graphics_include(&mut self, node: Node<'_>) -> usize {
        let path = extract_graphics_path(node, self.src).unwrap_or_default();
        let opts = extract_graphics_options(node, self.src);
        let mut args = format!("\"{}\"", path);
        if let Some(width) = opts.iter().find(|(k, _)| k == "width") {
            // Translate `0.5\textwidth` → `50%`. Other forms (e.g. `3cm`) pass through.
            let v = normalize_graphics_length(&width.1);
            args.push_str(&format!(", width: {}", v));
        }
        if let Some(height) = opts.iter().find(|(k, _)| k == "height") {
            let v = normalize_graphics_length(&height.1);
            args.push_str(&format!(", height: {}", v));
        }
        let _ = write!(self.out, "image({})", args);
        node.end_byte()
    }

    /// `\begin{figure}...\caption{X}...\label{fig:y}...\end{figure}` →
    /// `#figure(image(...), caption: [X]) <fig:y>`.
    fn emit_figure(&mut self, node: Node<'_>) -> usize {
        let mut graphics: Option<Node<'_>> = None;
        let mut caption: Option<Node<'_>> = None;
        let mut label: Option<String> = None;
        let mut nested_tabular: Option<Node<'_>> = None;

        // Walk the entire subtree because IEEE-style templates often wrap
        // `\includegraphics` in `\centerline{...}` or `\centering{...}`.
        let mut stack: Vec<Node<'_>> = vec![node];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                match child.kind() {
                    "graphics_include" if graphics.is_none() => graphics = Some(child),
                    "caption" if caption.is_none() => caption = Some(child),
                    "label_definition" if label.is_none() => {
                        label = extract_label_name(child, self.src)
                    }
                    "generic_environment" => {
                        if matches!(
                            environment_name(child, self.src).as_deref(),
                            Some("tabular") | Some("tabular*") | Some("array")
                        ) && nested_tabular.is_none()
                        {
                            nested_tabular = Some(child);
                        }
                        stack.push(child);
                    }
                    _ => stack.push(child),
                }
            }
        }

        let body_str = if let Some(g) = graphics {
            self.with_sub_buffer(|emitter| {
                emitter.emit_graphics_include(g);
            })
        } else if let Some(t) = nested_tabular {
            // `\begin{table}` wrapping a `tabular` (common IEEE pattern).
            // emit_tabular writes `#table(...)`; strip the leading `#` since
            // inside a `#figure(...)` argument the function call must be bare.
            let s = self
                .with_sub_buffer(|emitter| {
                    emitter.emit_tabular(t);
                })
                .trim()
                .to_string();
            s.strip_prefix('#').map(|s| s.to_string()).unwrap_or(s)
        } else {
            // Neither graphics nor a tabular body — warn and placeholder.
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::NeedsManualReview {
                    reason:
                        "figure has no \\includegraphics or tabular body — content not auto-translated"
                            .to_string(),
                },
                severity: Severity::Warning,
                message: "figure body needs manual review".to_string(),
                snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                suggested_skill: None,
            });
            "image(\"???\")".to_string()
        };

        self.ensure_paragraph_break();
        self.out.push_str("#figure(\n  ");
        self.out.push_str(&body_str);
        if let Some(c) = caption {
            if let Some(arg) = first_curly_group(c) {
                let text = self.render_curly_group_content(arg);
                let _ = write!(self.out, ",\n  caption: [{}]", text);
            }
        }
        self.out.push_str(",\n)");
        if let Some(l) = label {
            let _ = write!(self.out, " <{}>", l);
        }
        node.end_byte()
    }

    /// `\begin{tabular}{lcr} a & b \\ c & d \end{tabular}` →
    /// `#table(columns: 3, align: (left, center, right), [a], [b], [c], [d])`.
    fn emit_tabular(&mut self, node: Node<'_>) -> usize {
        // Column spec is the first `curly_group` child of the env.
        let col_spec = first_curly_group(node)
            .map(|g| self.src[g.start_byte() + 1..g.end_byte() - 1].to_string())
            .unwrap_or_default();
        let (count, aligns) = parse_column_spec(&col_spec);

        // Collect body children (everything between begin and end, excluding
        // the column-spec curly_group, but including the text children that
        // contain cells and row separators).
        let mut cursor = node.walk();
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end" | "curly_group"))
            .collect();

        // Render body to a string, then parse rows + cells.
        let body_str = if body.is_empty() {
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

        // Strip \hline (already emitted as raw text by the default emitter).
        let cleaned = body_str.replace("\\hline", "");
        // Rows are separated by ` \` (the math-mode-style row break we emit).
        // Outside math, we emit `\\\\` → ` \` too in `\\` emitter; same shape.
        // We split on the literal `\` token, then on `&` for cells.
        let rows: Vec<&str> = cleaned
            .split('\\')
            .filter(|r| !r.trim().is_empty())
            .collect();
        let mut cells = Vec::new();
        for row in &rows {
            for cell in row.split('&') {
                cells.push(cell.trim().to_string());
            }
        }

        self.ensure_paragraph_break();
        let _ = write!(
            self.out,
            "#table(\n  columns: {},\n  align: ({}),\n",
            count,
            aligns.join(", ")
        );
        // Emit cells grouped by row for readability.
        let mut idx = 0;
        for _ in 0..rows.len() {
            self.out.push_str("  ");
            for c in 0..count {
                if idx >= cells.len() {
                    break;
                }
                if c > 0 {
                    self.out.push_str(", ");
                }
                let _ = write!(self.out, "[{}]", cells[idx]);
                idx += 1;
            }
            self.out.push_str(",\n");
        }
        self.out.push(')');
        node.end_byte()
    }

    fn warn_ambiguous_math(&mut self, node: Node<'_>, reason: &str) {
        let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
        self.warnings.push(Warning {
            range: range_of(node),
            category: Category::AmbiguousMath {
                reason: reason.to_string(),
            },
            severity: Severity::Warning,
            message: format!("math command '{}' is not in the supported table", reason),
            snippet,
            suggested_skill: None,
        });
    }

    fn emit_section(&mut self, node: Node<'_>) -> usize {
        let kind = node.kind();
        let level = section_level(kind);

        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();

        // The header zone of a section node is: command_name [+ optional brack_group
        // for optional arg] + curly_group (title) + optional label_definition.
        // Everything after the first body-shaped child is the section's content.
        let mut starred = false;
        let mut title = String::new();
        let mut label: Option<String> = None;
        let mut body_start_idx = children.len();

        for (i, child) in children.iter().enumerate() {
            match child.kind() {
                k if k.starts_with('\\') && k.ends_with('*') => starred = true,
                k if k.starts_with('\\') => {}
                "curly_group" if title.is_empty() => {
                    title = self.render_curly_group_content(*child);
                }
                "brack_group" => {
                    // Optional short-title arg, e.g. \section[Short]{Long}. Ignore.
                }
                "label_definition" if label.is_none() => {
                    label = extract_label_name(*child, self.src);
                }
                _ => {
                    body_start_idx = i;
                    break;
                }
            }
        }

        if starred {
            if level == 1 {
                let _ = write!(self.out, "#heading(numbering: none)[{}]", title);
            } else {
                let _ = write!(
                    self.out,
                    "#heading(level: {}, numbering: none)[{}]",
                    level, title
                );
            }
        } else {
            for _ in 0..level {
                self.out.push('=');
            }
            let _ = write!(self.out, " {}", title);
        }
        if let Some(l) = &label {
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

    /// Render the inside of a `{ ... }` or `[ ... ]` group into a fresh
    /// sub-string. The leading and trailing delimiter tokens are stripped if
    /// present, so callers can pass `curly_group`, `curly_group_label`,
    /// `brack_group_text`, etc.
    fn render_curly_group_content(&mut self, group: Node<'_>) -> String {
        let mut cursor = group.walk();
        let children: Vec<Node<'_>> = group.children(&mut cursor).collect();

        let start_skip = usize::from(matches!(
            children.first().map(|n| n.kind()),
            Some("{") | Some("[")
        ));
        let end_skip = usize::from(matches!(
            children.last().map(|n| n.kind()),
            Some("}") | Some("]")
        ));
        let inner_len = children.len().saturating_sub(start_skip + end_skip);
        if inner_len == 0 {
            return String::new();
        }
        let inner = &children[start_skip..start_skip + inner_len];

        self.with_sub_buffer(|emitter| {
            let mut last = inner[0].start_byte();
            for child in inner {
                let cs = child.start_byte();
                emitter.safe_copy(last, cs);
                last = emitter.emit_node(*child);
            }
            // Trailing gap inside the inner range.
            let end = inner.last().unwrap().end_byte();
            emitter.safe_copy(last, end);
        })
        .trim()
        .to_string()
    }

    /// Capture whatever `f` writes to `self.out` and return it; restore the
    /// previous buffer when done. Caller is free to mutate everything else.
    fn with_sub_buffer<F: FnOnce(&mut Self)>(&mut self, f: F) -> String {
        let original = std::mem::take(&mut self.out);
        f(self);
        let captured = std::mem::take(&mut self.out);
        self.out = original;
        captured
    }

    fn warn_unsupported_command(&mut self, node: Node<'_>) {
        let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
        let name = command_name_of(&snippet);
        self.warnings.push(Warning {
            range: range_of(node),
            category: Category::UnsupportedCommand { name },
            severity: Severity::Warning,
            message: "command not yet supported by ByeTex; raw source dropped".to_string(),
            snippet,
            suggested_skill: None,
        });
    }
}

fn is_comment(kind: &str) -> bool {
    matches!(kind, "line_comment" | "block_comment" | "comment")
}

fn is_section_kind(kind: &str) -> bool {
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

fn section_level(kind: &str) -> u8 {
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

fn is_command(kind: &str) -> bool {
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

fn range_of(node: Node<'_>) -> Range {
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

fn command_name_of(snippet: &str) -> String {
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
fn command_name_text(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "command_name" {
            return Some(src[child.start_byte()..child.end_byte()].to_string());
        }
    }
    None
}

/// First `curly_group` child of `node`, if any.
fn first_curly_group(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .find(|child| child.kind() == "curly_group");
    result
}

/// True if the last emitted bytes look like "we just opened math and haven't
/// written a base symbol yet", in which case a subscript or superscript needs
/// an empty-string base to be valid Typst.
fn needs_empty_base(out: &str) -> bool {
    let trimmed = out.trim_end_matches([' ', '\t']);
    trimmed.ends_with('$') || trimmed.ends_with("$ ")
}

/// If `kind` is a token that's literal in LaTeX text mode but markup in
/// Typst, return the escaped Typst form. Returns `None` for unaffected kinds.
fn needs_text_escape(kind: &str) -> Option<&'static str> {
    match kind {
        "*" => Some("\\*"),
        "_" => Some("\\_"),
        "[" => Some("\\["),
        "]" => Some("\\]"),
        "#" => Some("\\#"),
        "@" => Some("\\@"),
        "<" => Some("\\<"),
        "`" => Some("\\`"),
        _ => None,
    }
}

/// Extract the list of citation keys from a `citation` node. Keys are
/// children of `curly_group_text_list`, separated by `,`.
fn extract_citation_keys(node: Node<'_>, src: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "curly_group_text_list" {
            let inner = src[child.start_byte() + 1..child.end_byte() - 1].to_string();
            for k in inner.split(',') {
                let t = k.trim();
                if !t.is_empty() {
                    keys.push(t.to_string());
                }
            }
        }
    }
    keys
}

/// Extract the key from a `label_reference` node (`\ref{x}`, `\eqref{x}`).
fn extract_label_ref_key(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "curly_group_label_list" || child.kind() == "curly_group_label" {
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "label" {
                    return Some(src[grandchild.start_byte()..grandchild.end_byte()].to_string());
                }
            }
        }
    }
    None
}

/// Extract the path argument from a `bibtex_include` (`\bibliography{x}`) or
/// `bibstyle_include` (`\bibliographystyle{x}`) node.
fn extract_bib_path(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "curly_group_path" | "curly_group_path_list") {
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "path" {
                    return Some(src[grandchild.start_byte()..grandchild.end_byte()].to_string());
                }
            }
        }
    }
    None
}

/// Extract the path argument from a `graphics_include` (`\includegraphics{X}`).
fn extract_graphics_path(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "curly_group_path" {
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "path" {
                    return Some(src[grandchild.start_byte()..grandchild.end_byte()].to_string());
                }
            }
        }
    }
    None
}

/// Extract key-value options from `\includegraphics[width=0.5\textwidth]`.
/// Each pair lives inside `brack_group_key_value > key_value_pair`.
fn extract_graphics_options(node: Node<'_>, src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "brack_group_key_value" {
            continue;
        }
        let mut sub = child.walk();
        for grandchild in child.children(&mut sub) {
            if grandchild.kind() != "key_value_pair" {
                continue;
            }
            let mut kv_cursor = grandchild.walk();
            let mut k = String::new();
            let mut v = String::new();
            let mut after_eq = false;
            for kv_child in grandchild.children(&mut kv_cursor) {
                match kv_child.kind() {
                    "=" => after_eq = true,
                    _ => {
                        let s = &src[kv_child.start_byte()..kv_child.end_byte()];
                        if after_eq {
                            v.push_str(s);
                        } else {
                            k.push_str(s);
                        }
                    }
                }
            }
            out.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}

/// Translate LaTeX length expressions to Typst.
/// - `0.5\textwidth` → `50%`
/// - `3cm`, `2in`, `100pt` → as-is (Typst accepts these units)
fn normalize_graphics_length(v: &str) -> String {
    let v = v.trim();
    if let Some(num) = v.strip_suffix("\\textwidth") {
        if let Ok(f) = num.trim().parse::<f64>() {
            return format!("{}%", (f * 100.0).round() as i64);
        }
    }
    if let Some(num) = v.strip_suffix("\\linewidth") {
        if let Ok(f) = num.trim().parse::<f64>() {
            return format!("{}%", (f * 100.0).round() as i64);
        }
    }
    v.to_string()
}

/// Parse a LaTeX tabular column spec like `lcr` or `|l|c|r|` into a count and
/// a vector of Typst alignment names (`"left"`, `"center"`, `"right"`).
fn parse_column_spec(spec: &str) -> (usize, Vec<String>) {
    let mut aligns = Vec::new();
    for c in spec.chars() {
        match c {
            'l' => aligns.push("left".to_string()),
            'c' => aligns.push("center".to_string()),
            'r' => aligns.push("right".to_string()),
            // Ignore vertical bars and other spec characters.
            _ => {}
        }
    }
    (aligns.len(), aligns)
}

/// Translate a LaTeX math command (with the leading backslash) into the
/// corresponding Typst math fragment. Returns `None` for unknown commands so
/// callers can decide between structural emission, warning, or pass-through.
fn lookup_math_symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        // Lowercase Greek
        "\\alpha" => "alpha",
        "\\beta" => "beta",
        "\\gamma" => "gamma",
        "\\delta" => "delta",
        "\\epsilon" => "epsilon",
        "\\varepsilon" => "epsilon.alt",
        "\\zeta" => "zeta",
        "\\eta" => "eta",
        "\\theta" => "theta",
        "\\vartheta" => "theta.alt",
        "\\iota" => "iota",
        "\\kappa" => "kappa",
        "\\lambda" => "lambda",
        "\\mu" => "mu",
        "\\nu" => "nu",
        "\\xi" => "xi",
        "\\pi" => "pi",
        "\\varpi" => "pi.alt",
        "\\rho" => "rho",
        "\\varrho" => "rho.alt",
        "\\sigma" => "sigma",
        "\\varsigma" => "sigma.alt",
        "\\tau" => "tau",
        "\\upsilon" => "upsilon",
        "\\phi" => "phi",
        "\\varphi" => "phi.alt",
        "\\chi" => "chi",
        "\\psi" => "psi",
        "\\omega" => "omega",
        // Uppercase Greek
        "\\Gamma" => "Gamma",
        "\\Delta" => "Delta",
        "\\Theta" => "Theta",
        "\\Lambda" => "Lambda",
        "\\Xi" => "Xi",
        "\\Pi" => "Pi",
        "\\Sigma" => "Sigma",
        "\\Upsilon" => "Upsilon",
        "\\Phi" => "Phi",
        "\\Psi" => "Psi",
        "\\Omega" => "Omega",
        // Operators
        "\\cdot" => "dot.c",
        "\\times" => "times",
        "\\div" => "div",
        "\\pm" => "plus.minus",
        "\\mp" => "minus.plus",
        "\\leq" | "\\le" => "<=",
        "\\geq" | "\\ge" => ">=",
        "\\neq" | "\\ne" => "!=",
        "\\equiv" => "equiv",
        "\\approx" => "approx",
        "\\sim" => "tilde",
        "\\propto" => "prop",
        "\\to" | "\\rightarrow" => "arrow.r",
        "\\leftarrow" => "arrow.l",
        "\\leftrightarrow" => "arrow.l.r",
        "\\Rightarrow" => "arrow.r.double",
        "\\Leftarrow" => "arrow.l.double",
        "\\Leftrightarrow" => "arrow.l.r.double",
        "\\mapsto" => "arrow.r.bar",
        "\\circ" => "circle.small",
        "\\bullet" => "bullet",
        "\\star" => "star.op",
        "\\ast" => "ast",
        // Sets and logic
        "\\in" => "in",
        "\\notin" => "in.not",
        "\\subset" => "subset",
        "\\supset" => "supset",
        "\\subseteq" => "subset.eq",
        "\\supseteq" => "supset.eq",
        "\\cup" => "union",
        "\\cap" => "inter",
        "\\setminus" => "without",
        "\\emptyset" => "nothing",
        "\\forall" => "forall",
        "\\exists" => "exists",
        "\\neg" | "\\lnot" => "not",
        "\\land" | "\\wedge" => "and",
        "\\lor" | "\\vee" => "or",
        "\\implies" => "==>",
        "\\iff" => "<==>",
        // Sums / products / integrals
        "\\sum" => "sum",
        "\\prod" => "product",
        "\\int" => "integral",
        "\\iint" => "integral.double",
        "\\iiint" => "integral.triple",
        "\\oint" => "integral.cont",
        "\\lim" => "lim",
        "\\sup" => "sup",
        "\\inf" => "inf",
        "\\max" => "max",
        "\\min" => "min",
        // Number sets (require amsfonts in LaTeX). \mathbb{R} is handled
        // elsewhere; common shorthand commands below.
        "\\R" => "RR",
        "\\Z" => "ZZ",
        "\\N" => "NN",
        "\\Q" => "QQ",
        "\\C" => "CC",
        // Special
        "\\infty" => "infinity",
        "\\partial" => "diff",
        "\\nabla" => "nabla",
        "\\hbar" => "planck.reduce",
        "\\ell" => "ell",
        "\\dots" | "\\ldots" => "dots.h",
        "\\cdots" => "dots.c",
        "\\vdots" => "dots.v",
        "\\ddots" => "dots.down",
        "\\angle" => "angle",
        "\\degree" => "degree",
        // Trig and log functions — Typst recognises these by name in math.
        "\\sin" => "sin",
        "\\cos" => "cos",
        "\\tan" => "tan",
        "\\cot" => "cot",
        "\\sec" => "sec",
        "\\csc" => "csc",
        "\\arcsin" => "arcsin",
        "\\arccos" => "arccos",
        "\\arctan" => "arctan",
        "\\sinh" => "sinh",
        "\\cosh" => "cosh",
        "\\tanh" => "tanh",
        "\\log" => "log",
        "\\ln" => "ln",
        "\\exp" => "exp",
        // Other small bits
        "\\pmod" => "mod",
        "\\bmod" => "mod",
        "\\gcd" => "gcd",
        _ => return None,
    })
}

/// LaTeX commands consume the run of spaces/tabs that immediately follow
/// them (the rationale being that an argument-less command without a brace
/// group can't otherwise be separated from the next token). When we drop a
/// command we mirror that consumption so we don't leave a stray leading space.
fn consume_trailing_inline_space(src: &str, mut pos: usize) -> usize {
    let bytes = src.as_bytes();
    while bytes.get(pos) == Some(&b' ') || bytes.get(pos) == Some(&b'\t') {
        pos += 1;
    }
    pos
}

/// Read the environment name from a `generic_environment` node. Looks for
/// `begin > curly_group_text > text|word` and returns its source text.
fn environment_name(env: Node<'_>, src: &str) -> Option<String> {
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
fn extract_label_name(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "curly_group_label" {
            // The `label` token is the key without the braces.
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "label" {
                    return Some(src[grandchild.start_byte()..grandchild.end_byte()].to_string());
                }
            }
        }
    }
    None
}
