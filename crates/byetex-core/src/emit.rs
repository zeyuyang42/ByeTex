//! Emitter — walks the tree-sitter AST and produces Typst source plus warnings.
//!
//! ## Scope
//!
//! - M1: plain text passthrough, `%`-comment dropping, generic warning for any
//!   unrecognised backslash command.
//! - M2: sectioning (`\section`..`\subparagraph`, starred forms, attached labels).
//!   Inline formatting + lists come in subsequent M2 sub-tasks; this file is
//!   structured around a dispatch-by-kind pattern so each batch is additive.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::class_map::DocClass;
use crate::document::{Content, DocumentMetadata};
use crate::warnings::{Category, Range, Severity, Warning};

/// A `\newcommand` definition harvested from the input. `body` is the
/// raw LaTeX source between the outer curly braces; expansion inlines
/// the body at every call site, substituting `#1` / `#2` / … with the
/// raw source of the call's curly_group arguments before re-parsing.
#[derive(Debug, Clone)]
pub(crate) struct MacroDef {
    /// Number of `#N` parameters expected. Zero for no-arg macros.
    pub params: usize,
    /// Raw LaTeX body, brace-stripped.
    pub body: String,
}

/// Walk `source` once and collect every `\newcommand` / `\def`
/// declaration into a fresh table. Used by the project-mode pre-scan
/// (see `project::harvest_project_macros`) so macros defined in
/// `.cls`/`.sty` files or in sibling `.tex` files unreached by `\input`
/// are still available when the entry file is converted.
pub(crate) fn harvest_macros_from_source(source: &str) -> HashMap<String, MacroDef> {
    let tree = crate::parser::parse(source);
    let mut out: HashMap<String, MacroDef> = HashMap::new();
    let root = tree.root_node();
    let mut stack: Vec<Node<'_>> = vec![root];
    while let Some(n) = stack.pop() {
        match n.kind() {
            "new_command_definition" => {
                if let Some((name, def)) = extract_newcommand(n, source) {
                    out.insert(name, def);
                }
            }
            "old_command_definition" => {
                let _ = extract_def_and_record(n, source, &mut out);
            }
            _ => {
                let mut cursor = n.walk();
                for c in n.children(&mut cursor) {
                    stack.push(c);
                }
            }
        }
    }
    out
}

/// Sentinel character emitted by `push_math_symbol` immediately after a
/// multi-character math identifier so that `collapse_math_spaces` can
/// later decide whether to insert a real separator (when the next char
/// would fuse — letter or digit) or drop it (when Typst already breaks
/// — `_`, `^`, `,`, `(`, `)`). Chosen as U+0017 ETB which has no
/// legitimate use in either LaTeX source or rendered Typst.
const MATH_WORD_BOUNDARY: char = '\u{17}';

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
    /// Structured title-block + metadata accumulated during the AST walk.
    /// `\title{X}`, `\author{X}`, `\date{X}`, `\begin{abstract}…\end{abstract}`,
    /// `\keywords{X}` etc. populate this. Per-class extractors in
    /// `class_map.rs` post-process raw `\author{...}` strings into
    /// structured `Author` records (with affiliation, email, orcid).
    metadata: DocumentMetadata,
    /// Raw author strings (one per `\author{...}` call) captured as the
    /// AST walks. Converted to structured `metadata.authors` records in
    /// `finish()`, where the per-class parser runs after the class
    /// detection and `\input` expansion have completed.
    raw_authors: Vec<String>,
    /// LaTeX document class detected from `\documentclass[opts]{class}` and
    /// refined by `\usepackage{...}` calls. Drives the Typst Universe template
    /// import emitted in `finish()`.
    detected_class: DocClass,
    /// Directory used to resolve `\input{...}` / `\include{...}` paths. When
    /// `Some`, the emitter expands those directives inline; when `None`, it
    /// drops them with a `needs_manual_review` warning (the v0.1 behaviour
    /// that runs when `convert()` is called with bare source and no file).
    base_dir: Option<PathBuf>,
    /// Canonicalised paths of files already expanded along the current
    /// expansion chain — used to break `\input` cycles. Each successful
    /// recursive expansion inserts the resolved file's canonical path before
    /// recursing and is left in the set so a sibling include of the same
    /// file is treated as a duplicate (warn) rather than an infinite loop.
    visited_includes: HashSet<PathBuf>,
    /// `\newcommand` definitions harvested as we walk the source. Each
    /// matching call site is later expanded inline by re-parsing the
    /// substituted body. Out-of-scope forms (`\def`, `\providecommand`,
    /// optional-default `\newcommand[1][default]`) are not entered into
    /// this map.
    macros: HashMap<String, MacroDef>,
    /// Assets (images, bib files) resolved on disk during this emit pass.
    /// Populated only when `base_dir` is `Some`. Bubbled up to `ConvertOutput`
    /// by `finish()` so the project layer can copy them to the output dir.
    asset_refs: Vec<crate::AssetRef>,
    /// Current `\newcommand` expansion depth. A self-referential macro
    /// (`\newcommand{\foo}{\foo}`) would otherwise recurse without bound
    /// and overflow the stack. The cap is generous enough for legitimate
    /// nested expansions but stops adversarial inputs cold.
    macro_depth: u32,
}

/// Maximum allowed `\newcommand` expansion depth (see `Emitter::macro_depth`).
const MAX_MACRO_DEPTH: u32 = 64;

impl<'a> Emitter<'a> {
    /// Constructor variant used by the public `convert()` entry point and by
    /// recursive `\input` expansion. `base_dir` enables include resolution;
    /// `visited` is the cycle-detection set carried across the chain of
    /// nested includes.
    pub(crate) fn with_includes(
        src: &'a str,
        source_name: &'a str,
        base_dir: Option<PathBuf>,
        visited: HashSet<PathBuf>,
    ) -> Self {
        Self::with_includes_and_macros(src, source_name, base_dir, visited, HashMap::new())
    }

    /// Same as `with_includes` but lets the caller seed the macro table.
    /// Used by the folder-input path (`plan_project_from_dir`) which
    /// pre-scans every `.tex`/`.sty`/`.cls` for `\newcommand`/`\def`
    /// before converting the entry file. This guarantees that a macro
    /// defined in a sibling source file never reached via `\input` is
    /// still available at every call site in the entry's expansion tree.
    pub(crate) fn with_includes_and_macros(
        src: &'a str,
        source_name: &'a str,
        base_dir: Option<PathBuf>,
        visited: HashSet<PathBuf>,
        mut preseeded_macros: HashMap<String, MacroDef>,
    ) -> Self {
        // Seed always-on KaTeX built-in macros before the prepass runs.
        // or_insert ensures preseeded entries (from project-mode harvest) and
        // later user \newcommand definitions (which use insert) always win.
        for (name, seed) in crate::package_macros::KATEX_BUILTIN {
            if lookup_math_symbol(name).is_none() {
                preseeded_macros
                    .entry(name.to_string())
                    .or_insert_with(|| MacroDef {
                        params: seed.params,
                        body: seed.body.to_string(),
                    });
            }
        }
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
            metadata: DocumentMetadata::default(),
            raw_authors: Vec::new(),
            detected_class: DocClass::Unknown,
            base_dir,
            visited_includes: visited,
            macros: preseeded_macros,
            asset_refs: Vec::new(),
            macro_depth: 0,
        }
    }

    pub(crate) fn emit_root(&mut self, root: Node<'_>) {
        let _ = self.emit_node(root);
    }

    /// Walk the entire AST *before* `emit_root`, harvesting ALL macro
    /// definitions into `self.macros`. This ensures macros used before their
    /// definition (forward references) are available at emit time.
    pub(crate) fn prepass_collect(&mut self, root: Node<'_>) {
        let mut stack: Vec<Node<'_>> = vec![root];
        while let Some(n) = stack.pop() {
            match n.kind() {
                "new_command_definition" => {
                    // Tree-sitter uses `new_command_definition` for \newcommand,
                    // \renewcommand, \providecommand, \DeclareMathOperator, etc.
                    // Dispatch on the first child token to distinguish them.
                    // Collect kinds into a Vec to avoid borrow-checker issues with
                    // holding a cursor reference across the match.
                    let cmd_token = new_command_token_kind(n);
                    match cmd_token.as_deref() {
                        Some("\\newcommand") | Some("\\newcommand*") | None => {
                            if let Some((name, def)) = extract_newcommand(n, self.src) {
                                self.macros.insert(name, def);
                            }
                        }
                        Some("\\renewcommand") | Some("\\renewcommand*") => {
                            if let Some((name, def)) = extract_newcommand(n, self.src) {
                                self.macros.insert(name, def);
                            }
                        }
                        Some("\\providecommand") | Some("\\providecommand*") => {
                            if let Some((name, def)) = extract_newcommand(n, self.src) {
                                // \providecommand: no-op if name is already defined
                                if !self.macros.contains_key(&name)
                                    && lookup_math_symbol(&name).is_none()
                                {
                                    self.macros.insert(name, def);
                                }
                            }
                        }
                        Some("\\DeclareMathOperator") | Some("\\DeclareMathOperator*") => {
                            let starred = cmd_token
                                .as_deref()
                                .map_or(false, |s| s.ends_with('*'));
                            if let Some((name, def)) =
                                extract_declare_math_operator_from_newcmd(n, self.src, starred)
                            {
                                self.macros.insert(name, def);
                            }
                        }
                        _ => {
                            // Other new_command_definition variants — try generic extract
                            if let Some((name, def)) = extract_newcommand(n, self.src) {
                                self.macros.insert(name, def);
                            }
                        }
                    }
                }
                "old_command_definition" => {
                    let _ = extract_def_and_record(n, self.src, &mut self.macros);
                }
                "generic_command" => {
                    // generic_command does NOT produce \renewcommand/\providecommand/
                    // \DeclareMathOperator (those are new_command_definition in tree-sitter).
                    // Just recurse into children in case there's a nested definition.
                    let mut cursor = n.walk();
                    for c in n.children(&mut cursor) {
                        stack.push(c);
                    }
                }
                "package_include" => {
                    if let Some(pkg) = extract_package_name(n, self.src) {
                        // Local .sty first so it beats bundled seeds
                        self.expand_local_package(&pkg);
                        // Then seed bundled macros — or_insert loses to any existing entry
                        if let Some(seeds) = crate::package_macros::package_macros(&pkg) {
                            for (macro_name, seed) in seeds {
                                if lookup_math_symbol(macro_name).is_none() {
                                    self.macros
                                        .entry(macro_name.to_string())
                                        .or_insert_with(|| MacroDef {
                                            params: seed.params,
                                            body: seed.body.to_string(),
                                        });
                                }
                            }
                        }
                    }
                    // Do NOT recurse into children of package_include
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

    pub(crate) fn finish(
        mut self,
    ) -> (
        String,
        Vec<Warning>,
        Vec<crate::AssetRef>,
        std::collections::HashMap<String, String>,
    ) {
        // If `\documentclass` mapped to a known Typst Universe template,
        // prepend the `#import` + `#show:` pair so the converted PDF gets
        // that class's full visual identity (columns, font, headings, title
        // block). Otherwise fall back to the hand-rolled centered title.
        let template_preamble = self.build_template_preamble();
        if let Some(p) = template_preamble {
            let body = std::mem::take(&mut self.out);
            self.out.push_str(&p);
            self.out.push_str(&body);
        } else if !self.metadata.is_title_block_empty() || !self.raw_authors.is_empty() {
            // Pre-pend rather than append: insert at the start of `out` so the
            // title block lives at the top of the document.
            let body = std::mem::take(&mut self.out);
            self.flush_title_block();
            self.out.push_str(&body);
        }

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

        // Typographic substitutions: LaTeX `---` / `--` → em-/en-dash;
        // LaTeX-style double quotes ``X'' → ASCII "X" (Typst will smart-quote).
        // Done as a final string pass so we don't have to wrangle token-level
        // detection for adjacent `-` / backtick / apostrophe runs.
        self.out = post_process_typography(&self.out);
        let class_metadata = self.metadata.class_metadata;
        (self.out, self.warnings, self.asset_refs, class_metadata)
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
        // they were live LaTeX, or by emit_math_wrap consuming a
        // brace-less arg).
        if node.start_byte() < self.skip_until {
            // Partial overlap: the head of this node is already
            // emitted, but the tail still needs to come out. Two
            // cases:
            //   - leaf (no children): emit the tail bytes verbatim.
            //   - has children: recurse — each child either falls
            //     fully before skip_until (re-checks and skips), or
            //     fully after (emits normally), or straddles
            //     (recursive partial-skip).
            if self.skip_until < node.end_byte() {
                if node.child_count() == 0 {
                    self.safe_copy(self.skip_until, node.end_byte());
                    return node.end_byte();
                }
                let mut cursor = node.walk();
                let kids: Vec<Node<'_>> = node.children(&mut cursor).collect();
                let mut last = self.skip_until.max(node.start_byte());
                for child in &kids {
                    let cs = child.start_byte();
                    if cs >= last {
                        self.safe_copy(last, cs);
                    }
                    last = self.emit_node(*child);
                }
                self.safe_copy(last, node.end_byte());
                return node.end_byte();
            }
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
            // `\left( ... \right)` in math: tree-sitter packages the whole
            // span as a single `math_delimiter` with `left_command`,
            // `left_delimiter`, body, `right_command`, `right_delimiter`
            // fields. Emit the delimiters directly and recurse into the
            // body — Typst auto-pairs the symbols.
            "math_delimiter" if self.in_math => return self.emit_math_delimiter(node),
            "subscript" if self.in_math => return self.emit_subscript(node, "_"),
            "superscript" if self.in_math => return self.emit_subscript(node, "^"),
            // `\text{X}` inside math — the grammar tags this as `text_mode`.
            // Emit the inner content as a quoted Typst string so it renders
            // upright. Don't recurse (we'd otherwise split letters).
            "text_mode" if self.in_math => {
                if let Some(arg) = first_curly_group(node) {
                    let inner = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    let _ = write!(self.out, "\"{}\"", inner);
                }
                return node.end_byte();
            }
            // A bare `command_name` (e.g. `_\theta`) inside math — look it up
            // in the math symbol table. Without this branch, the default
            // recursive walker would copy `\theta` verbatim.
            "command_name" if self.in_math => {
                let text = &self.src[node.start_byte()..node.end_byte()];
                if let Some(typst) = lookup_math_symbol(text) {
                    self.push_math_symbol(typst);
                    return node.end_byte();
                }
                // A bare wrap command (e.g. `_\mathcal{T}` parses with
                // `\mathcal` as just a `command_name`, dropping the
                // `{T}` to a sibling). Use the brace-less wrap helper
                // to consume the next source token directly.
                if let Some((l, r)) = wrap_for_command_name(text) {
                    return self.emit_math_wrap(node, l, r);
                }
                // Emit a placeholder rather than leaking raw `\name` into
                // the Typst output (which would fail to compile).
                return self.emit_unknown_math_command(node, text);
            }
            _ => {}
        }

        // Multi-letter math identifier splitting. LaTeX math reads consecutive
        // letters as implicit products (e.g. `mc` = m·c); Typst reads them as a
        // single identifier. Inside math, split a multi-letter `word` into
        // single chars separated by spaces, unless the word is a known
        // function name.
        if self.in_math && node.kind() == "word" {
            let text = &self.src[node.start_byte()..node.end_byte()];
            // tree-sitter-latex sometimes appends trailing punctuation (`.`, `!`,
            // `?`) to the word token (e.g. `dt.` is one node, not `dt` + `.`).
            // Split at the first non-alphabetic char to get the identifier prefix.
            let alpha_end = text.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(text.len());
            let alpha = &text[..alpha_end];
            let tail = &text[alpha_end..];
            // Guard: keep the preceding identifier from fusing with this word's
            // first letter (e.g. `t` + `dt` → `tdt`). The helper is a no-op
            // when the previous output char is not a letter.
            self.ensure_math_letter_boundary(text);
            if should_split_math_word(alpha) {
                let mut first = true;
                for c in alpha.chars() {
                    if !first {
                        self.out.push(' ');
                    }
                    self.out.push(c);
                    first = false;
                }
                self.out.push_str(tail);
                return node.end_byte();
            }
            // Non-split path: we own the write so the default walker below
            // doesn't double-emit the same bytes.
            self.out.push_str(text);
            return node.end_byte();
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
            if let Some((l, end)) = extract_label_name_and_end(node, self.src) {
                self.pending_math_label = Some(l);
                // tree-sitter-latex truncates the label key at `_` and
                // leaks the rest into the surrounding text. Skip past
                // the real closing brace so we don't re-emit the
                // leaked `_objective}` etc.
                self.skip_until = self.skip_until.max(end);
                return end;
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
            // Orphan `\label{X}` outside any section/equation/figure — emit
            // the Typst label syntax so subsequent `@X` references resolve.
            "label_definition" => {
                if let Some((key, end)) = extract_label_name_and_end(node, self.src) {
                    let _ = write!(self.out, " <{}>", key);
                    self.skip_until = self.skip_until.max(end);
                    return end;
                }
                return node.end_byte();
            }
            // `\href{url}{display}` — Typst `#link("url")[display]`.
            "hyperlink" => {
                let mut cursor = node.walk();
                let mut url: Option<String> = None;
                let mut display: Option<Node<'_>> = None;
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "curly_group_uri" => {
                            let mut sub = child.walk();
                            for gc in child.children(&mut sub) {
                                if gc.kind() == "uri" {
                                    url =
                                        Some(self.src[gc.start_byte()..gc.end_byte()].to_string());
                                }
                            }
                        }
                        "curly_group" if display.is_none() => {
                            display = Some(child);
                        }
                        _ => {}
                    }
                }
                if let Some(u) = url {
                    if let Some(d) = display {
                        let rendered = self.render_curly_group_content(d);
                        let _ = write!(self.out, "#link(\"{}\")[{}]", u, rendered);
                    } else {
                        let _ = write!(self.out, "#link(\"{}\")", u);
                    }
                }
                return node.end_byte();
            }
            // `\input` / `\include` — when the caller supplied a base
            // directory (the file's parent, in the CLI), resolve the path
            // and recursively convert the included source so its body
            // appears inline at this point. Without a base directory we
            // can't do filesystem I/O safely; fall back to the
            // needs_manual_review warning that documented the v0.1
            // behaviour.
            "latex_include" => {
                if self.base_dir.is_some() {
                    if self.expand_latex_include(node) {
                        return node.end_byte();
                    }
                    // expand_latex_include returns false only when it
                    // already pushed a more specific warning (missing
                    // file, cycle, read error). Don't double-warn.
                    return node.end_byte();
                }
                let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
                self.warnings.push(Warning {
                    range: range_of(node),
                    category: Category::NeedsManualReview {
                        reason: "multi-file include (\\input/\\include) is out of scope"
                            .to_string(),
                    },
                    severity: Severity::Warning,
                    message: "ByeTex converts one file at a time. Concatenate \
                              the included sources before running, or rewrite \
                              using Typst's `#include` directive."
                        .to_string(),
                    snippet,
                    suggested_skill: Some("byetex-unsupported-environment".to_string()),
                });
                return node.end_byte();
            }
            "title_declaration" => {
                if let Some(arg) = first_curly_group(node) {
                    self.metadata.title =
                        Some(Content::Typst(self.render_curly_group_content(arg)));
                }
                return node.end_byte();
            }
            "author_declaration" => {
                // The grammar uses `curly_group_author_list` for the author arg.
                if let Some(arg) = first_curly_like(node) {
                    let rendered = self.render_curly_group_content(arg);
                    self.raw_authors.push(rendered);
                }
                return node.end_byte();
            }
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

        // `\usepackage{...}` — drop silently for packages that have no Typst
        // equivalent (Typst's defaults cover them or they're style-only).
        // Also: certain ML-conference style packages (`neurips_2024`,
        // `iclr2025_conference`, `icml*`) imply a document class, so we
        // refine `detected_class` even though we drop the package itself.
        if node.kind() == "package_include" {
            if let Some(pkg) = extract_package_name(node, self.src) {
                self.detected_class =
                    std::mem::replace(&mut self.detected_class, DocClass::Unknown)
                        .refine_from_package(&pkg);
                // BEFORE the noop-list check: if a local `<pkg>.sty`
                // (or `.cls`) sits in the source directory, parse it
                // for `\newcommand` / `\def` and merge the macros
                // into `self.macros`. This means an
                // `\usepackage{neurips_2026}` whose `.sty` lives next
                // to the paper contributes its `\acksection`, etc.
                // System packages (no local file) are unaffected.
                self.expand_local_package(&pkg);
                if is_known_noop_package(&pkg) {
                    return node.end_byte();
                }
            }
            self.warn_unsupported_command(node);
            return node.end_byte();
        }

        // `\documentclass[opts]{class}` — capture class + options so we can
        // emit the matching Typst Universe template in `finish()`. The
        // source line itself is dropped from the output.
        if node.kind() == "class_include" {
            let (class, opts) = extract_class_and_options(node, self.src);
            if let Some(c) = class {
                self.detected_class = DocClass::from_class(&c, &opts);
            }
            return node.end_byte();
        }

        // `\newcommand{\name}[N]{body}` (and related forms) — harvest the macro
        // into `self.macros` so subsequent calls to `\name` get expanded inline.
        // Tree-sitter also uses `new_command_definition` for `\renewcommand`,
        // `\providecommand`, and `\DeclareMathOperator`. The prepass already
        // seeded them; here we just ensure the table is up to date (emit-order
        // definitions also need to land for the forward-reference case).
        if node.kind() == "new_command_definition" {
            let cmd_token = new_command_token_kind(node);
            match cmd_token.as_deref() {
                Some("\\renewcommand") | Some("\\renewcommand*") => {
                    // \renewcommand always overwrites.
                    if let Some((name, def)) = extract_newcommand(node, self.src) {
                        self.macros.insert(name, def);
                    }
                }
                Some("\\providecommand") | Some("\\providecommand*") => {
                    // \providecommand: no-op if already defined or is a built-in.
                    if let Some((name, def)) = extract_newcommand(node, self.src) {
                        if !self.macros.contains_key(&name)
                            && lookup_math_symbol(&name).is_none()
                        {
                            self.macros.insert(name, def);
                        }
                    }
                }
                Some("\\DeclareMathOperator") | Some("\\DeclareMathOperator*") => {
                    // Harvested in prepass_collect with correct \operatorname body.
                    // Do not re-harvest here — extract_newcommand would give the wrong
                    // body (just the display text, not \operatorname{...}).
                }
                _ => {
                    // \newcommand (and any other variant) — always overwrites.
                    if let Some((name, def)) = extract_newcommand(node, self.src) {
                        self.macros.insert(name, def);
                    }
                }
            }
            return node.end_byte();
        }
        // `\def\name<params>{body}` is `old_command_definition`. The
        // tree-sitter grammar packages just `\def\name` as the node;
        // the params placeholders and the body curly_group land as
        // SIBLINGS in the parent. Harvest the full definition by
        // scanning source bytes, and skip past the body so it
        // doesn't leak into the output as raw text.
        if node.kind() == "old_command_definition" {
            if let Some(end) = extract_def_and_record(node, self.src, &mut self.macros) {
                self.skip_until = self.skip_until.max(end);
                return end;
            }
            return node.end_byte();
        }
        if matches!(node.kind(), "counter_declaration" | "theorem_definition") {
            return node.end_byte();
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
            // Use the `#raw(...)` function rather than the `` `…` `` literal
            // syntax. The function form composes cleanly with any backticks
            // in the surrounding body (post_process_typography escapes lone
            // source backticks at the end), and avoids the "unclosed raw"
            // error when source text mixes `\texttt{X}` with stray ` `` ` from
            // LaTeX left-single-quote.
            Some("\\texttt") => self.emit_inline_raw(node),
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
            // Table rule commands have no Typst equivalent in our default
            // table emission (Typst auto-styles rules). Drop silently.
            Some("\\hline") | Some("\\toprule") | Some("\\midrule") | Some("\\bottomrule")
            | Some("\\cmidrule") => node.end_byte(),
            // Sizing-delimiter commands escaping their math container —
            // tree-sitter constructs a `math_delimiter` only when the
            // matching pair is present; when one half is missing the
            // bare `\left` / `\right` ends up here in text mode. Drop
            // silently so the literal backslash doesn't leak into the
            // Typst output.
            Some("\\left") | Some("\\right") | Some("\\middle") | Some("\\bigl")
            | Some("\\Bigl") | Some("\\biggl") | Some("\\Biggl") | Some("\\bigr")
            | Some("\\Bigr") | Some("\\biggr") | Some("\\Biggr") | Some("\\bigm")
            | Some("\\Bigm") | Some("\\biggm") | Some("\\Biggm") | Some("\\big")
            | Some("\\Big") | Some("\\bigg") | Some("\\Bigg") => node.end_byte(),
            // `\xspace` (from the xspace package) auto-inserts a space
            // when not followed by punctuation. Typst already
            // separates command-following-letter via whitespace, so
            // dropping the call is invisible. Same for `\notag`,
            // `\nonumber` outside math (rare but seen).
            Some("\\xspace")
            | Some("\\notag")
            | Some("\\nonumber")
            | Some("\\protect")
            | Some("\\ignorespaces") => node.end_byte(),
            // Common text-mode symbols.
            Some("\\S") => {
                self.out.push('§');
                node.end_byte()
            }
            Some("\\P") => {
                self.out.push('¶');
                node.end_byte()
            }
            Some("\\copyright") => {
                self.out.push('©');
                node.end_byte()
            }
            Some("\\textregistered") => {
                self.out.push('®');
                node.end_byte()
            }
            // Deprecated font-switching commands. These change the style of
            // all following text until the group ends — Typst would need a
            // #strong[…]/#emph[…] scope wrap, which requires end-of-group
            // tracking we don't yet have. Warn so the caller can see the loss.
            Some("\\bf") | Some("\\sf") | Some("\\rm") | Some("\\it") | Some("\\tt")
            | Some("\\sl") | Some("\\sc") | Some("\\em") => {
                self.warn_unsupported_command(node);
                node.end_byte()
            }
            // Vertical-skip primitives.
            Some("\\smallskip") => {
                self.out.push_str("#v(0.5em)");
                node.end_byte()
            }
            Some("\\medskip") => {
                self.out.push_str("#v(1em)");
                node.end_byte()
            }
            Some("\\bigskip") => {
                self.out.push_str("#v(1.5em)");
                node.end_byte()
            }
            // Horizontal-fill.
            Some("\\hfill") | Some("\\hfil") => {
                self.out.push_str("#h(1fr)");
                node.end_byte()
            }
            Some("\\centerline") => self.emit_inline_wrap(node, "#align(center)[", "]"),
            // Text-mode super/subscript wrappers.
            Some("\\textsuperscript") => self.emit_inline_wrap(node, "#super[", "]"),
            Some("\\textsubscript") => self.emit_inline_wrap(node, "#sub[", "]"),
            // Spacing primitives with no Typst equivalent — drop silently.
            // (`\smallskip`/`\medskip`/`\bigskip` are handled above with
            // explicit `#v(...)` emission and take precedence over this
            // catch-all.)
            Some("\\kern")
            | Some("\\vspace")
            | Some("\\hspace")
            | Some("\\vspace*")
            | Some("\\hspace*")
            | Some("\\quad")
            | Some("\\qquad")
            | Some("\\,")
            | Some("\\;")
            | Some("\\:")
            | Some("\\!")
            | Some("\\enspace")
            | Some("\\thinspace")
            | Some("\\linebreak")
            | Some("\\pagebreak")
            | Some("\\nopagebreak")
            | Some("\\newpage")
            | Some("\\clearpage")
            | Some("\\cleardoublepage")
                if !self.in_math =>
            {
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            // Layout-only directives.
            Some("\\centering")
            | Some("\\raggedright")
            | Some("\\raggedleft")
            | Some("\\justify")
            | Some("\\flushleft")
            | Some("\\flushright") => consume_trailing_inline_space(self.src, node.end_byte()),
            // Float/figure placement specifiers + page-style controls.
            Some("\\setcounter")
            | Some("\\renewcommand")
            | Some("\\providecommand")
            | Some("\\pagestyle")
            | Some("\\thispagestyle")
            | Some("\\pagenumbering")
            | Some("\\addtocounter")
            | Some("\\stepcounter")
            | Some("\\refstepcounter")
            | Some("\\setlength")
            | Some("\\addtolength")
            | Some("\\settowidth")
            | Some("\\bibliographystyle") => {
                // These take args we don't translate. Drop silently.
                node.end_byte()
            }
            // ACM publication-metadata (conference/journal machinery); no visible
            // author content — drop silently.
            Some("\\setcopyright")
            | Some("\\copyrightyear")
            | Some("\\acmYear")
            | Some("\\acmConference")
            | Some("\\acmBooktitle")
            | Some("\\acmDOI")
            | Some("\\acmISBN")
            | Some("\\acmPrice")
            | Some("\\acmSubmissionID")
            | Some("\\affiliation") => node.end_byte(),
            // ACM author-info fields. Capture into class_metadata so callers
            // and class templates can access the values, and warn so the user
            // knows these fields are not yet fully rendered.
            Some("\\institution")
            | Some("\\city")
            | Some("\\country")
            | Some("\\state")
            | Some("\\streetaddress")
            | Some("\\postcode")
            | Some("\\email")
            | Some("\\orcid")
            | Some("\\authornote")
            | Some("\\additionalaffiliation")
            | Some("\\ccsdesc")
            | Some("\\shortauthors") => {
                if let Some(key) = command_name_text(node, self.src) {
                    let field = key.trim_start_matches('\\').to_string();
                    if let Some(arg) = first_curly_like(node) {
                        let content = self.render_curly_group_content(arg);
                        self.metadata.class_metadata.entry(field).or_insert(content);
                    }
                }
                self.warn_unsupported_command(node);
                node.end_byte()
            }
            // `\keywords{a, b, c}` and `\IEEEkeywords{...}` — capture into the
            // title-block field when the class template wants it; otherwise
            // silently drop.
            Some("\\keywords") | Some("\\IEEEkeywords") => {
                if self.detected_class.import_line().is_some() {
                    if let Some(arg) = first_curly_like(node) {
                        let rendered = self.render_curly_group_content(arg);
                        self.metadata.keywords = rendered
                            .split(',')
                            .map(|k| k.trim().to_string())
                            .filter(|k| !k.is_empty())
                            .collect();
                    }
                }
                node.end_byte()
            }
            // IEEEtran-specific — preamble flag, no visible content.
            Some("\\IEEEoverridecommandlockouts") => node.end_byte(),
            // IEEEtran commands that carry visible content (page footer, footnote
            // markers, abstract body, acknowledgement list items). Warn.
            Some("\\IEEEpubid")
            | Some("\\IEEEauthorrefmark")
            | Some("\\IEEEcompsoctitleabstractindextext")
            | Some("\\IEEEcompsocthanksitem") => {
                self.warn_unsupported_command(node);
                node.end_byte()
            }
            // NeurIPS-specific.
            Some("\\And") | Some("\\AND") | Some("\\PassOptionsToPackage") | Some("\\And ") => {
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            // Tables-of-contents et al. — drop, will be re-added with Typst syntax later if needed.
            Some("\\tableofcontents")
            | Some("\\listoffigures")
            | Some("\\listoftables")
            | Some("\\printbibliography")
            | Some("\\printindex") => consume_trailing_inline_space(self.src, node.end_byte()),
            // `\multicolumn{n}{spec}{content}` → `table.cell(colspan: n)[content]`.
            // The surrounding emit_tabular's body splitter will treat the whole
            // thing as one cell, which is the intended outcome.
            Some("\\multicolumn") => {
                let mut cursor = node.walk();
                let groups: Vec<Node<'_>> = node
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "curly_group")
                    .collect();
                if groups.len() < 3 {
                    self.warn_unsupported_command(node);
                    return node.end_byte();
                }
                let n = self
                    .src
                    .get(groups[0].start_byte() + 1..groups[0].end_byte() - 1)
                    .unwrap_or("1")
                    .trim();
                let content = self.render_curly_group_content(groups[2]);
                let _ = write!(self.out, "table.cell(colspan: {})[{}]", n, content);
                node.end_byte()
            }
            // LaTeX text-mode literal escapes for special characters.
            // Typst needs its own escape syntax for some of these.
            Some("\\{") => {
                self.out.push_str("\\{");
                node.end_byte()
            }
            Some("\\}") => {
                self.out.push_str("\\}");
                node.end_byte()
            }
            Some("\\#") => {
                self.out.push_str("\\#");
                node.end_byte()
            }
            Some("\\$") => {
                self.out.push_str("\\$");
                node.end_byte()
            }
            Some("\\&") => {
                self.out.push('&');
                node.end_byte()
            }
            Some("\\%") => {
                self.out.push('%');
                node.end_byte()
            }
            Some("\\_") => {
                self.out.push_str("\\_");
                node.end_byte()
            }
            // `\ ` (backslash-space) is LaTeX's literal-space marker; emit a
            // regular space and consume the original.
            Some("\\ ") => {
                self.out.push(' ');
                node.end_byte()
            }
            // TeX micro-typography primitives — no visible effect in Typst's
            // layout model (italic correction, discretionary hyphen,
            // sentence-end tweak). Drop silently.
            Some("\\/") | Some("\\-") | Some("\\@") => node.end_byte(),
            // Accent operators: map to precomposed Unicode (\'e → é, \"o → ö,
            // \^a → â, \`e → è, \~n → ñ).
            Some("\\'") => self.emit_text_accent(node, '\''),
            Some("\\\"") => self.emit_text_accent(node, '"'),
            Some("\\^") => self.emit_text_accent(node, '^'),
            Some("\\`") => self.emit_text_accent(node, '`'),
            Some("\\~") => self.emit_text_accent(node, '~'),
            // Typographic logos — drop the styling, keep the text.
            Some("\\LaTeX") => self.emit_logo(node, "LaTeX"),
            Some("\\TeX") => self.emit_logo(node, "TeX"),
            Some("\\BibTeX") => self.emit_logo(node, "BibTeX"),
            Some("\\eTeX") => self.emit_logo(node, "eTeX"),
            Some("\\XeLaTeX") => self.emit_logo(node, "XeLaTeX"),
            Some("\\LuaLaTeX") => self.emit_logo(node, "LuaLaTeX"),
            // Title-block accumulators. `\title`, `\author`, `\date` capture
            // their argument; `\maketitle` emits the assembled block. If
            // \maketitle is never called the block is flushed in `finish()`.
            Some("\\title") => {
                if let Some(arg) = first_curly_group(node) {
                    self.metadata.title =
                        Some(Content::Typst(self.render_curly_group_content(arg)));
                }
                node.end_byte()
            }
            Some("\\author") => {
                if let Some(arg) = first_curly_group(node) {
                    let rendered = self.render_curly_group_content(arg);
                    self.raw_authors.push(rendered);
                }
                node.end_byte()
            }
            Some("\\date") => {
                if let Some(arg) = first_curly_group(node) {
                    self.metadata.date = Some(self.render_curly_group_content(arg));
                }
                node.end_byte()
            }
            Some("\\maketitle") => {
                // No-op at the source position; `finish()` always pre-pends
                // the assembled title block at the document head so the visual
                // result matches LaTeX irrespective of where `\maketitle` lives.
                node.end_byte()
            }
            // `\thanks{X}` attaches a footnote to whatever preceded it. We
            // render inline as a Typst footnote at the current position.
            Some("\\thanks") | Some("\\footnote") | Some("\\footnotetext") => {
                if let Some(arg) = first_curly_group(node) {
                    let content = self.render_curly_group_content(arg);
                    let _ = write!(self.out, "#footnote[{}]", content);
                }
                node.end_byte()
            }
            // `\mbox{X}` (and `\hbox{X}`, `\fbox{X}`, `\framebox{X}`) — boxing
            // primitives that just render their content; emit X as-is.
            Some("\\mbox") | Some("\\hbox") | Some("\\fbox") | Some("\\framebox") => {
                self.emit_inline_unwrap(node)
            }
            // `\multirow{n}{w}{X}` → `table.cell(rowspan: n)[X]`. The third
            // `{X}` argument is the cell content; the second is column width.
            Some("\\multirow") => {
                let mut cursor = node.walk();
                let groups: Vec<Node<'_>> = node
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "curly_group")
                    .collect();
                if groups.len() < 3 {
                    self.warn_unsupported_command(node);
                    return node.end_byte();
                }
                let n = self
                    .src
                    .get(groups[0].start_byte() + 1..groups[0].end_byte() - 1)
                    .unwrap_or("1")
                    .trim();
                let content = self.render_curly_group_content(groups[2]);
                let _ = write!(self.out, "table.cell(rowspan: {})[{}]", n, content);
                node.end_byte()
            }
            // `\makecell[opts]{X}` — render the content; the [opts] are layout
            // hints we ignore.
            Some("\\makecell") => self.emit_inline_unwrap(node),
            // `\lipsum[N]` and `\blindtext` — placeholder-text generators.
            // Drop silently; the user added them as filler.
            Some("\\lipsum") | Some("\\blindtext") | Some("\\Blindtext") => node.end_byte(),
            // `\href{url}{display}` → Typst `#link("url")[display]`.
            Some("\\href") => {
                let mut cursor = node.walk();
                let groups: Vec<Node<'_>> = node
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "curly_group")
                    .collect();
                if groups.len() >= 2 {
                    let url = self
                        .src
                        .get(groups[0].start_byte() + 1..groups[0].end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    let display = self.render_curly_group_content(groups[1]);
                    let _ = write!(self.out, "#link(\"{}\")[{}]", url, display);
                } else if let Some(arg) = first_curly_group(node) {
                    let url = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    let _ = write!(self.out, "#link(\"{}\")", url);
                }
                node.end_byte()
            }
            // `\url{X}` → bare link in Typst.
            Some("\\url") => {
                if let Some(arg) = first_curly_group(node) {
                    let url = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    let _ = write!(self.out, "#link(\"{}\")", url);
                }
                node.end_byte()
            }
            // Font-size directives — unscoped toggles. Typst's equivalent
            // would be a #text(size: …)[…] wrap but that needs end-of-group
            // tracking we don't yet have. Warn so the caller can see the loss.
            Some("\\small")
            | Some("\\large")
            | Some("\\Large")
            | Some("\\LARGE")
            | Some("\\huge")
            | Some("\\Huge")
            | Some("\\normalsize")
            | Some("\\footnotesize")
            | Some("\\scriptsize")
            | Some("\\tiny") => {
                self.warn_unsupported_command(node);
                node.end_byte()
            }
            // `\appendix` toggles section-number style to letters; emit as a
            // set rule.
            Some("\\appendix") => {
                self.out.push_str("\n#set heading(numbering: \"A.1\")\n");
                node.end_byte()
            }
            // `\label{X}` outside any section/equation context — keep the
            // label so subsequent `\ref{X}` resolves. Typst syntax: `<x>`.
            Some("\\label") => {
                if let Some(arg) = first_curly_group(node) {
                    let key = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    let _ = write!(self.out, " <{}>", key);
                }
                node.end_byte()
            }
            // `\DeclareMathOperator{\name}{display}` — harvested in
            // `prepass_collect`; emit-time uses `expand_user_macro` via the
            // user-macro fallback. Drop the definition node silently here.
            Some("\\DeclareMathOperator") | Some("\\DeclareMathOperator*") => {
                node.end_byte()
            }
            // `\input{file}` / `\include{file}` / `\subfile{file}` — when the
            // caller supplied a base directory, expand inline by parsing and
            // converting the referenced file. Without a base directory, fall
            // back to a needs_manual_review warning (the v0.1 behaviour for
            // callers that pass raw source without a containing file).
            Some("\\input") | Some("\\include") | Some("\\subfile") => {
                if self.base_dir.is_some() {
                    let _ = self.expand_latex_include(node);
                    return node.end_byte();
                }
                let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
                self.warnings.push(Warning {
                    range: range_of(node),
                    category: Category::NeedsManualReview {
                        reason: "multi-file include (\\input/\\include) is out of scope"
                            .to_string(),
                    },
                    severity: Severity::Warning,
                    message: "ByeTex converts one file at a time. Concatenate \
                              your inputs before running, or rewrite using \
                              Typst's `#include` directive."
                        .to_string(),
                    snippet,
                    suggested_skill: Some("byetex-unsupported-environment".to_string()),
                });
                node.end_byte()
            }
            _ => {
                // Last-chance: maybe this is a `\newcommand` we
                // harvested earlier. Expand it inline and let the
                // re-parse pick up nested commands.
                if let Some(n) = name.as_deref() {
                    if self.macros.contains_key(n) {
                        return self.expand_user_macro(node, n);
                    }
                }
                self.warn_unsupported_command(node);
                node.end_byte()
            }
        }
    }

    /// Expand a user-defined `\newcommand` at its call site.
    ///
    /// Reads the macro's stored body, substitutes `#1`..`#N` placeholders
    /// with the raw source of each `curly_group` argument of the call,
    /// re-parses the resulting LaTeX with `parser::parse`, and emits it
    /// via a child `Emitter` that inherits the parent's math context,
    /// macro table, and `base_dir`. The child's body output is appended
    /// to the parent's; warnings are merged. If the parameter count
    /// doesn't match, fall back to warn-and-drop.
    ///
    /// Brace-less calls (`\mat X`, `\mat \alpha`) are also supported:
    /// when the AST has fewer `curly_group` children than the macro
    /// expects, the missing args are consumed from raw source bytes via
    /// [`consume_braceless_arg`]. `self.skip_until` is bumped so the
    /// parent walker doesn't re-emit the consumed tokens.
    fn expand_user_macro(&mut self, node: Node<'_>, name: &str) -> usize {
        if self.macro_depth >= MAX_MACRO_DEPTH {
            // Bail out and emit a warning. A self-referential or mutually
            // recursive `\newcommand` would otherwise overflow the stack.
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::CustomMacro {
                    name: name.to_string(),
                },
                severity: Severity::Warning,
                message: format!(
                    "\\newcommand `{}` expansion exceeded depth {} — aborting expansion (possible recursion)",
                    name, MAX_MACRO_DEPTH
                ),
                snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                suggested_skill: None,
            });
            return node.end_byte();
        }
        let macro_def = match self.macros.get(name).cloned() {
            Some(d) => d,
            None => return node.end_byte(),
        };
        // Collect the call's curly_group arguments in order.
        let mut cursor = node.walk();
        let mut args: Vec<String> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "curly_group")
            .map(|c| {
                // Strip the outer `{` and `}` from each arg.
                self.src
                    .get(c.start_byte() + 1..c.end_byte() - 1)
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        // If the call site has fewer curly_groups than the macro expects,
        // try LaTeX's brace-less calling convention: read the next N
        // tokens from the raw source (`\name`, `{group}`, or one char).
        // Real arXiv papers heavily rely on this — `$\mat X$`, `\vec a`,
        // `\rvec \alpha`. Without it every such call site is dropped with
        // a `custom_macro` warning.
        let mut consumed_end = node.end_byte();
        while args.len() < macro_def.params {
            match consume_braceless_arg(self.src, consumed_end) {
                Some((arg, end)) => {
                    args.push(arg.as_substitution().to_string());
                    consumed_end = end;
                }
                None => break, // EOF / only whitespace — fall through to warn.
            }
        }
        if args.len() < macro_def.params {
            // Genuine missing-arg case: the source really doesn't have
            // enough tokens after the macro call. Emit a warning and
            // drop the call so the raw `\name` doesn't bleed into the
            // Typst output.
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::CustomMacro {
                    name: name.to_string(),
                },
                severity: Severity::Warning,
                message: format!(
                    "\\newcommand call `{}` expected {} arg(s), found {}",
                    name,
                    macro_def.params,
                    args.len()
                ),
                snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                suggested_skill: None,
            });
            return node.end_byte();
        }
        // Mark the consumed brace-less range as already-emitted so the
        // parent walker doesn't re-emit those source bytes after we
        // append the expansion.
        if consumed_end > node.end_byte() {
            self.skip_until = self.skip_until.max(consumed_end);
        }
        // Substitute `#1`..`#N` in the body. We can't naively call
        // `str::replace("#1", arg)` — that would also rewrite `#10`,
        // `#11`, ... as `<arg>0`, `<arg>1` for any macro with ≥10
        // parameters. Walk the body and replace `#<digits>` tokens
        // greedily instead.
        let expanded = substitute_macro_args(&macro_def.body, &args[..macro_def.params]);
        // Re-parse and emit. Use a sub-emitter so we don't disturb
        // our `out` cursor management — its output is appended.
        let tree = crate::parser::parse(&expanded);
        let visited = std::mem::take(&mut self.visited_includes);
        let macros = self.macros.clone();
        let mut sub =
            Emitter::with_includes(&expanded, self.source_name, self.base_dir.clone(), visited);
        sub.in_math = self.in_math;
        sub.macros = macros;
        sub.macro_depth = self.macro_depth + 1;
        sub.emit_root(tree.root_node());
        // Merge child state back.
        self.visited_includes = std::mem::take(&mut sub.visited_includes);
        // The macro's expansion may have defined additional macros
        // (rare but allowed). Pull those back to the parent.
        for (k, v) in sub.macros.drain() {
            self.macros.entry(k).or_insert(v);
        }
        let body_out = sub.out;
        // Trim the trailing newline the child may have added if the
        // body is a one-liner; otherwise math expansions get
        // unwanted line breaks.
        let body_out = body_out.trim_end_matches('\n');
        self.out.push_str(body_out);
        self.warnings.append(&mut sub.warnings);
        // A `\includegraphics` or `\bibliography` reached via a macro body
        // must still bubble up so the project materialiser copies the file.
        self.asset_refs.append(&mut sub.asset_refs);
        // Return the end of the consumed range so the AST walker resumes
        // past any brace-less args we ate. For purely curly-group calls,
        // `consumed_end == node.end_byte()` and this matches the prior
        // behaviour.
        consumed_end
    }

    /// Expand a `\input{...}` / `\include{...}` directive inline.
    ///
    /// Looks up the referenced file relative to `self.base_dir`, parses it
    /// with the same tree-sitter LaTeX grammar, runs a child `Emitter` over
    /// its content, and appends the child's body to `self.out`. Pending
    /// title-block fields (title, authors, abstract, keywords), document
    /// class, and the numbering flags are merged so that an `\input` that
    /// contains `\title{...}` or sets a class still drives the parent's
    /// preamble.
    ///
    /// Cycle detection uses canonical paths: a file already on the include
    /// chain is reported via a `needs_manual_review` warning rather than
    /// re-expanded.
    ///
    /// Returns true when the include resolved and was expanded; false when
    /// the resolution failed (a more specific warning has been pushed).
    /// Try to read `<pkg>.sty` (or `<pkg>.cls`) sitting next to the
    /// paper's source files and harvest any `\newcommand` / `\def`
    /// definitions into `self.macros`. Subsequent calls to those
    /// macros in the body get expanded by `expand_user_macro`.
    ///
    /// Silent no-op when no local file is found (system packages like
    /// `amsmath`, `tikz`, `geometry`) — the caller still falls back
    /// to the no-op-allowlist drop. Failures inside the sub-parse
    /// are absorbed (a malformed `.sty` shouldn't bring down the
    /// parent conversion).
    fn expand_local_package(&mut self, pkg: &str) {
        let base_dir = match self.base_dir.clone() {
            Some(b) => b,
            None => return,
        };
        let resolved = match resolve_package_path(&base_dir, pkg) {
            Some(p) => p,
            None => return,
        };
        let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if !self.visited_includes.insert(canonical.clone()) {
            return; // already harvested on this chain
        }
        let source = match std::fs::read_to_string(&resolved) {
            Ok(s) => s,
            Err(_) => return,
        };
        // Walk the file's AST looking for `new_command_definition`
        // and `old_command_definition` nodes; harvest each one's
        // definition into a fresh map, then merge.
        let tree = crate::parser::parse(&source);
        let mut harvested: HashMap<String, MacroDef> = HashMap::new();
        let root = tree.root_node();
        let mut stack: Vec<Node<'_>> = vec![root];
        while let Some(n) = stack.pop() {
            match n.kind() {
                "new_command_definition" => {
                    if let Some((name, def)) = extract_newcommand(n, &source) {
                        harvested.insert(name, def);
                    }
                }
                "old_command_definition" => {
                    let _ = extract_def_and_record(n, &source, &mut harvested);
                }
                _ => {
                    let mut cursor = n.walk();
                    for c in n.children(&mut cursor) {
                        stack.push(c);
                    }
                }
            }
        }
        // Merge into self.macros, parent-wins.
        for (k, v) in harvested {
            self.macros.entry(k).or_insert(v);
        }
    }

    fn expand_latex_include(&mut self, node: Node<'_>) -> bool {
        let base_dir = match self.base_dir.clone() {
            Some(b) => b,
            None => return false,
        };
        let raw_path = match extract_latex_include_path(node, self.src) {
            Some(p) => p,
            None => return false,
        };
        let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
        let resolved = match resolve_input_path(&base_dir, &raw_path) {
            Some(p) => p,
            None => {
                self.warnings.push(Warning {
                    range: range_of(node),
                    category: Category::NeedsManualReview {
                        reason: format!("included file not found relative to base: {}", raw_path),
                    },
                    severity: Severity::Warning,
                    message: format!(
                        "could not resolve `{}` against base directory `{}` (tried `{0}` and `{0}.tex`)",
                        raw_path,
                        base_dir.display()
                    ),
                    snippet,
                    suggested_skill: Some("byetex-unsupported-environment".to_string()),
                });
                return false;
            }
        };
        let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if self.visited_includes.contains(&canonical) {
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::NeedsManualReview {
                    reason: "circular \\input / \\include chain".to_string(),
                },
                severity: Severity::Warning,
                message: format!(
                    "`{}` is already on the include chain — skipping to avoid an infinite loop",
                    canonical.display()
                ),
                snippet,
                suggested_skill: None,
            });
            return false;
        }
        let source = match std::fs::read_to_string(&resolved) {
            Ok(s) => s,
            Err(e) => {
                self.warnings.push(Warning {
                    range: range_of(node),
                    category: Category::NeedsManualReview {
                        reason: format!("failed to read included file: {}", e),
                    },
                    severity: Severity::Warning,
                    message: format!("could not read `{}`: {}", resolved.display(), e),
                    snippet,
                    suggested_skill: Some("byetex-unsupported-environment".to_string()),
                });
                return false;
            }
        };
        let new_base = resolved.parent().map(Path::to_path_buf).unwrap_or(base_dir);
        let source_name = resolved.display().to_string();
        // Move the visited set into the child so the chain is shared. Insert
        // before recursing so the child's own includes see the parent in
        // its chain.
        let mut visited = std::mem::take(&mut self.visited_includes);
        visited.insert(canonical);
        let tree = crate::parser::parse(&source);
        let mut sub = Emitter::with_includes(&source, &source_name, Some(new_base), visited);
        sub.emit_root(tree.root_node());
        // Merge the child's body and state back into the parent.
        if !self.out.ends_with('\n') && !self.out.is_empty() {
            self.out.push('\n');
        }
        self.out.push_str(&sub.out);
        self.warnings.append(&mut sub.warnings);
        self.asset_refs.append(&mut sub.asset_refs);
        self.needs_heading_numbering |= sub.needs_heading_numbering;
        self.needs_equation_numbering |= sub.needs_equation_numbering;
        // Merge the included file's metadata into the parent, parent
        // taking priority for fields it already owns.
        self.metadata.merge_from(&mut sub.metadata);
        if self.raw_authors.is_empty() {
            self.raw_authors.append(&mut sub.raw_authors);
        }
        if matches!(self.detected_class, DocClass::Unknown) {
            self.detected_class = std::mem::replace(&mut sub.detected_class, DocClass::Unknown);
        }
        // Take the (possibly extended) visited set back so siblings see
        // the chain. Drop the canonical insert that belonged to *this*
        // include so a sibling `\input{x}` after the current one is still
        // detected as a duplicate (which it is — the rest of the chain
        // remains).
        self.visited_includes = std::mem::take(&mut sub.visited_includes);
        true
    }

    /// Emit a typographic-logo command (`\LaTeX`, `\TeX`, etc.) as plain text.
    /// LaTeX users normally write `\LaTeX{}` so the empty group blocks LaTeX
    /// from swallowing the following space. tree-sitter parses that `{}` as a
    /// `curly_group` child of the command — when we see it, the caller's
    /// intent was to preserve the following space, so we do.
    fn emit_logo(&mut self, node: Node<'_>, name: &str) -> usize {
        self.out.push_str(name);
        // If the generic_command has any `curly_group` child, the user wrote
        // `\LaTeX{}` (or `\LaTeX{x}`) and the space-eating safeguard is in
        // place. Otherwise consume the trailing space, matching LaTeX.
        if first_curly_group(node).is_some() {
            return node.end_byte();
        }
        consume_trailing_inline_space(self.src, node.end_byte())
    }

    /// Emit the centered Typst title block from any captured \title/\author/\date.
    fn flush_title_block(&mut self) {
        // Promote any raw author strings to structured records first,
        // so the fallback path renders the same set of authors that the
        // template path would have used.
        self.materialize_authors();
        if self.metadata.is_title_block_empty() {
            return;
        }
        self.ensure_paragraph_break();
        self.out.push_str("#align(center)[\n");
        if let Some(title) = self.metadata.title.take() {
            let _ = writeln!(
                self.out,
                "  #text(size: 1.5em, weight: \"bold\")[{}]",
                title.as_content()
            );
        }
        if !self.metadata.authors.is_empty() {
            self.out.push_str("  #v(0.6em)\n  ");
            let names: Vec<String> = self
                .metadata
                .authors
                .iter()
                .map(|a| a.name.as_content().to_string())
                .collect();
            self.out.push_str(&names.join(", "));
            self.out.push('\n');
            self.metadata.authors.clear();
        }
        if let Some(date) = self.metadata.date.take() {
            let _ = write!(self.out, "  #v(0.4em)\n  {}\n", date);
        }
        self.out.push_str("]\n\n");
    }

    /// Convert the raw `\author{...}` strings collected during the AST
    /// walk into structured `Author` records by running the per-class
    /// parser from `class_map.rs`. Idempotent — calling it twice is a
    /// no-op.
    fn materialize_authors(&mut self) {
        if self.raw_authors.is_empty() {
            return;
        }
        let raw = std::mem::take(&mut self.raw_authors);
        let mut parsed = crate::class_map::parse_authors(&raw, &self.detected_class);
        self.metadata.authors.append(&mut parsed);
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

    /// `\texttt{X}` → `#raw("X")` (Typst's function form of inline raw text).
    /// We deliberately avoid the `` `…` `` literal syntax so that the
    /// surrounding output's backtick handling — see
    /// [`post_process_typography`] — can safely escape stray source
    /// backticks without breaking us.
    fn emit_inline_raw(&mut self, node: Node<'_>) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let content = self
                .src
                .get(arg.start_byte() + 1..arg.end_byte() - 1)
                .unwrap_or("")
                .trim();
            // Escape only the characters Typst's string literal must
            // escape; everything else (including `_`, `*`, `#`) stays
            // literal because `#raw(...)` doesn't reparse the content.
            let escaped = content.replace('\\', "\\\\").replace('"', "\\\"");
            let _ = write!(self.out, "#raw(\"{}\")", escaped);
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
            // Abstract: capture into a title-block field when the class's
            // template accepts an `abstract:` parameter (IEEE, NeurIPS).
            // For acmart and unknown classes the abstract stays inline.
            Some("abstract") => {
                if self.detected_class.wants_abstract_field() && self.metadata.r#abstract.is_none()
                {
                    let body = self.render_env_body_to_string(node);
                    self.metadata.r#abstract = Some(Content::Typst(body.trim().to_string()));
                    node.end_byte()
                } else {
                    self.emit_environment_body(node)
                }
            }
            // IEEEtran's keywords env. Same capture-or-drop dance as abstract.
            Some("IEEEkeywords") => {
                if self.detected_class.import_line().is_some() && self.metadata.keywords.is_empty()
                {
                    let body = self.render_env_body_to_string(node);
                    self.metadata.keywords = body
                        .trim()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    node.end_byte()
                } else {
                    self.emit_environment_body(node)
                }
            }
            // Transparent wrappers: emit body, no markup. `\documentclass` etc.
            // already produced warnings as separate top-level commands.
            Some("document") | Some("subequations") | Some("center") | Some("flushleft")
            | Some("flushright") | Some("quote") | Some("quotation") | Some("verse")
            | Some("titlepage") | Some("minipage") => self.emit_environment_body(node),
            // Matrix family — handled wherever we encounter them. If we're
            // not already in math mode, the surrounding container will wrap
            // us; pmatrix() etc. assume math context.
            Some("pmatrix") | Some("bmatrix") | Some("vmatrix") | Some("Vmatrix")
            | Some("Bmatrix") | Some("matrix") => self.emit_matrix_env(node, env.as_deref()),
            // `cases` env produces piecewise display.
            Some("cases") => self.emit_cases_env(node),
            // M4: tables and figure floats.
            // `array` is dispatched specially: when nested inside a math
            // container (`align*`, `gather`, `\left\{...\right\}`, etc.)
            // it should render as Typst `cases(...)`, not as a `#table(...)`
            // (which is text-mode-only and breaks the surrounding `$...$`).
            Some("array") if self.in_math => self.emit_array_in_math(node),
            Some("tabular") | Some("tabular*") | Some("array") => self.emit_tabular(node),
            Some("figure") | Some("figure*") | Some("table") => self.emit_figure(node),
            // IEEE/thebibliography style: emit each \bibitem as a labeled
            // numbered-list entry so `@bN` references resolve.
            Some("thebibliography") => self.emit_thebibliography(node),
            // Theorem-family envs from amsthm. Emit as labeled figures with a
            // custom kind so `@thm:foo` resolves.
            Some("theorem") => self.emit_theorem_env(node, "Theorem"),
            Some("lemma") => self.emit_theorem_env(node, "Lemma"),
            Some("corollary") => self.emit_theorem_env(node, "Corollary"),
            Some("proposition") => self.emit_theorem_env(node, "Proposition"),
            Some("definition") => self.emit_theorem_env(node, "Definition"),
            Some("example") => self.emit_theorem_env(node, "Example"),
            Some("remark") => self.emit_theorem_env(node, "Remark"),
            // Proof env — no label-targeting needed; emit as a block.
            Some("proof") => self.emit_proof_env(node),
            _ => {
                self.warn_unsupported_environment(node, env.as_deref());
                node.end_byte()
            }
        }
    }

    /// Render an environment's body into a fresh `String` (no side effect on
    /// `self.out`). Used by `abstract` capture when a class template wants
    /// the body as a content field rather than inline document text.
    fn render_env_body_to_string(&mut self, env: Node<'_>) -> String {
        self.with_sub_buffer(|emitter| {
            emitter.emit_environment_body(env);
        })
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

    /// Build the `#import "@preview/X:V": fn` + `#show: fn.with(...)` pair
    /// that styles the converted document as the LaTeX class would. Returns
    /// `None` for unknown classes (caller falls back to the hand-rolled
    /// centered title block).
    fn build_template_preamble(&mut self) -> Option<String> {
        let import_line = self.detected_class.import_line()?;
        // Run the class-aware author parser now that detection has
        // settled (it may have been refined by a `\usepackage{neurips_*}`).
        self.materialize_authors();
        // Bare-bones documents (no title, no authors) skip the template
        // preamble. Emitting an empty `arkheion.with(title: [], authors:
        // ())` block would force a blank title page and trip the
        // template's required-field assertions.
        if self.metadata.is_title_block_empty() {
            return None;
        }
        let show_call = self.detected_class.show_call(&self.metadata)?;
        let mut s = String::new();
        s.push_str(import_line);
        s.push('\n');
        s.push_str(&show_call);
        s.push('\n');
        Some(s)
    }

    /// Close any in-flight `\bibitem{key}` by emitting `]) <key>` so the label
    /// attaches to the entry's `#figure[...]` wrapper.
    fn close_bibitem(&mut self) {
        if let Some(key) = self.pending_bibitem_key.take() {
            // Trim trailing whitespace so the closing bracket sits flush.
            while self.out.ends_with(' ') || self.out.ends_with('\n') {
                self.out.pop();
            }
            let _ = writeln!(self.out, "]) <{}>", key);
        }
    }

    /// `\begin{theorem}[note]\label{X} body \end{theorem}` →
    /// `#figure(kind: "<name>", supplement: [Name], [body]) <X>`.
    fn emit_theorem_env(&mut self, env: Node<'_>, name: &'static str) -> usize {
        let mut cursor = env.walk();
        let mut label: Option<String> = None;
        let body: Vec<Node<'_>> = env
            .children(&mut cursor)
            .filter(|c| {
                if c.kind() == "label_definition" {
                    label = extract_label_name(*c, self.src);
                    return false;
                }
                !matches!(c.kind(), "begin" | "end")
            })
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
        let _ = write!(
            self.out,
            "#figure(kind: \"{}\", supplement: [{}], [{}])",
            name.to_lowercase(),
            name,
            inner.trim()
        );
        if let Some(l) = label {
            let _ = write!(self.out, " <{}>", l);
        }
        env.end_byte()
    }

    /// `\begin{proof}...\end{proof}` → `*Proof.* body` as a paragraph block.
    fn emit_proof_env(&mut self, env: Node<'_>) -> usize {
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

    /// Push a math-symbol replacement into `self.out`, prepending a space
    /// when the symbol starts with a letter and the last emitted character
    /// is also a letter. LaTeX writes `t\in[0,T]` with no separator; the
    /// LaTeX tokenizer treats the `\` as a word boundary. Typst reads
    /// adjacent letters as a single identifier, so `t` + `in` collapses to
    /// the unknown variable `tin`. Inserting a space recovers the boundary.
    ///
    /// Symbols that contain a `.` (e.g. `arrow.r`, `dots.h`, `chevron.l`)
    /// get an additional *trailing* space: Typst treats `arrow.r0` as
    /// `arrow.r` with an unknown `0` modifier, so we need to break the
    /// `0` (or letter) away from the dotted suffix on the right too.
    fn push_math_symbol(&mut self, typst: &str) {
        if typst.is_empty() {
            return;
        }
        self.ensure_math_letter_boundary(typst);
        self.out.push_str(typst);
        // For multi-character symbols whose last character could fuse
        // with a following alphanumeric (`approx22`, `dot.c y`,
        // `arrow.r0`), drop a `MATH_WORD_BOUNDARY` sentinel here. The
        // sentinel is rewritten at the math container's exit:
        //
        //   sentinel followed by `_`/`^`/punct/`(` → drop (no separator
        //   needed; Typst already token-breaks at those).
        //   sentinel followed by anything else (letter/digit/end of
        //   buffer) → replace with a single ASCII space so the two
        //   identifiers stay separate.
        let multi_char_letterish = typst.chars().count() > 1
            && typst
                .chars()
                .last()
                .is_some_and(|c| c.is_ascii_alphanumeric());
        if multi_char_letterish {
            self.out.push(MATH_WORD_BOUNDARY);
        }
    }

    /// Insert a single space into `self.out` when needed to keep a letter
    /// at the end of the current output from fusing with a letter at the
    /// start of `next`. The same fusion bites every math emitter that
    /// writes a function-call wrapper (`bb(`, `sqrt(`, `binom(`, `op(`,
    /// …) — e.g. `\in\mathbb{R}` was emitting `inbb(R)` because
    /// `emit_math_wrap`'s `bb(` followed the `in` from `\in` with no
    /// separator. Callers invoke this before the letter-starting prefix.
    fn ensure_math_letter_boundary(&mut self, next: &str) {
        let starts_with_letter = next.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
        let prev_is_letter = self
            .out
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphabetic());
        if starts_with_letter && prev_is_letter {
            self.out.push(' ');
        }
    }

    /// Replace the in-progress math body (output bytes from `body_start` to
    /// the current end of `self.out`) with a copy where unbalanced `[` / `]`
    /// have been escaped as `\[` / `\]`. Balanced pairs are left as-is. See
    /// [`escape_unbalanced_math_brackets`] for the rationale.
    fn balance_math_brackets(&mut self, body_start: usize) {
        if body_start > self.out.len() {
            return;
        }
        let body_len = self.out.len() - body_start;
        let escaped = escape_unbalanced_math_brackets(&self.out[body_start..]);
        if escaped.len() != body_len {
            self.out.truncate(body_start);
            self.out.push_str(&escaped);
        }
    }

    /// Collapse runs of two or more ASCII spaces in the in-progress math
    /// body to a single space. `push_math_symbol` appends a trailing space
    /// to multi-character word-like symbols (`approx`, `dot.c`, `arrow.r`)
    /// so they don't fuse with a following digit or letter; when the source
    /// already had whitespace between the LaTeX command and the next token,
    /// the two spaces collide. Math rendering treats `a  b` and `a b`
    /// identically, so collapsing keeps the output tidy and avoids
    /// snapshot churn.
    /// Resolve `MATH_WORD_BOUNDARY` sentinels that `push_math_symbol`
    /// dropped into the in-progress math body. Each sentinel becomes a
    /// space when the following character would fuse with the preceding
    /// math identifier (`approx` + `22` → `approx 22`), and is dropped
    /// otherwise (`sum` + `_` → `sum_`).
    fn collapse_math_spaces(&mut self, body_start: usize) {
        // Guard: the surrounding math-container emitters sometimes pop
        // trailing whitespace from `self.out` before calling us. If
        // they popped past `body_start` the slice would panic; treat
        // that as "body empty, nothing to do".
        if body_start > self.out.len() {
            return;
        }
        let body = &self.out[body_start..];
        if !body.contains(MATH_WORD_BOUNDARY) && !body.contains("  ") && !body.ends_with(' ') {
            return;
        }
        let mut out = String::with_capacity(body.len());
        let chars: Vec<char> = body.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == MATH_WORD_BOUNDARY {
                // Look ahead at the next non-sentinel character.
                let mut j = i + 1;
                while j < chars.len() && chars[j] == MATH_WORD_BOUNDARY {
                    j += 1;
                }
                if let Some(&next) = chars.get(j) {
                    // For dotted symbols (e.g. `arrow.r`, `dots.h`) a following
                    // `(` would be parsed by Typst as a function-call argument,
                    // turning the symbol into an unknown function. Emit a space to
                    // break the call syntax. Non-dotted symbols (`sum`, `int`, …)
                    // are fine: Typst already tokenises `sum(` as subscript-less
                    // sum followed by a group.
                    let prev_token_dotted = {
                        let s = out.as_str();
                        let last_ws = s
                            .rfind(|c: char| c.is_whitespace())
                            .map(|p| p + 1)
                            .unwrap_or(0);
                        s[last_ws..].contains('.')
                    };
                    // Only `(` can make Typst interpret the dotted symbol as a
                    // function call (e.g. `arrow.r(` → function call). `)`, `,`
                    // and other punct are fine without a separator.
                    let next_is_call_open = next == '(';
                    if next.is_ascii_alphanumeric() || (prev_token_dotted && next_is_call_open) {
                        out.push(' ');
                    }
                    // else: drop the sentinel — Typst already tokenizes
                    // at `_`, `^`, `(`, `)`, `,`, etc.
                }
                i = j;
                continue;
            }
            // Collapse runs of ASCII spaces to one.
            if c == ' ' {
                out.push(' ');
                i += 1;
                while i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                continue;
            }
            out.push(c);
            i += 1;
        }
        while out.ends_with(' ') {
            out.pop();
        }
        self.out.truncate(body_start);
        self.out.push_str(&out);
    }

    fn emit_inline_math(&mut self, node: Node<'_>) -> usize {
        self.out.push('$');
        let body_start = self.out.len();
        let was = self.in_math;
        self.in_math = true;
        self.emit_math_children(node);
        self.in_math = was;
        self.collapse_math_spaces(body_start);
        self.balance_math_brackets(body_start);
        self.out.push('$');
        node.end_byte()
    }

    fn emit_display_math(&mut self, node: Node<'_>) -> usize {
        // Typst block math wants a blank line before the `$ ... $`.
        self.ensure_paragraph_break();
        self.out.push_str("$ ");
        let body_start = self.out.len();
        let was = self.in_math;
        self.in_math = true;
        self.emit_math_children(node);
        self.in_math = was;
        // Trim trailing whitespace we accumulated inside (newlines from layout) so
        // the closing `$` follows directly after the content. Guard against
        // popping past body_start when the math body is empty.
        while self.out.len() > body_start && (self.out.ends_with(' ') || self.out.ends_with('\n')) {
            self.out.pop();
        }
        self.collapse_math_spaces(body_start);
        self.balance_math_brackets(body_start);
        self.out.push_str(" $");
        node.end_byte()
    }

    /// `\begin{equation}...\end{equation}` and friends. The grammar tags these
    /// as `math_environment` (distinct from `generic_environment`). We treat
    /// numbered/unnumbered forms the same and let Typst handle numbering.
    fn emit_math_environment(&mut self, node: Node<'_>) -> usize {
        let env_name = environment_name(node, self.src).unwrap_or_default();
        // `array` parses as a math_environment in tree-sitter-latex (not
        // as a generic_environment). When we hit one and we're already
        // inside another math container, render via the
        // `array → cases(...)` helper instead of opening a new `$...$`
        // block (which would break the parent math). The dispatcher in
        // emit_generic_environment never sees this node — it's all on
        // the math path.
        if env_name == "array" && self.in_math {
            return self.emit_array_in_math(node);
        }
        // Guard: if we are already inside a math container (e.g. a math_environment
        // nested under an outer `$...$`), do NOT open a fresh `$ ... $`. Opening a
        // new `$` would close the outer math in Typst's parser, leaving the outer
        // closing `$` dangling. Instead, just inline the body children.
        if self.in_math {
            let prev_label = self.pending_math_label.take();
            let mut cursor = node.walk();
            let body: Vec<Node<'_>> = node
                .children(&mut cursor)
                .filter(|c| !matches!(c.kind(), "begin" | "end"))
                .collect();
            if !body.is_empty() {
                let mut last = body[0].start_byte();
                for child in &body {
                    self.safe_copy(last, child.start_byte());
                    last = self.emit_node(*child);
                }
                self.safe_copy(last, body.last().unwrap().end_byte());
            }
            if let Some(l) = self.pending_math_label.take() {
                let _ = write!(self.out, " <{}>", l);
            }
            self.pending_math_label = prev_label;
            return node.end_byte();
        }
        self.ensure_paragraph_break();
        self.out.push_str("$ ");
        let body_start = self.out.len();
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
        while self.out.len() > body_start && (self.out.ends_with(' ') || self.out.ends_with('\n')) {
            self.out.pop();
        }
        self.collapse_math_spaces(body_start);
        self.balance_math_brackets(body_start);
        self.out.push_str(" $");
        if let Some(l) = self.pending_math_label.take() {
            let _ = write!(self.out, " <{}>", l);
        }
        self.pending_math_label = prev_label;
        node.end_byte()
    }

    /// Skip the math delimiters (`$`, `$$`, `\[`, `\]`) and emit interior
    /// children with the usual gap-copy mechanism.
    /// `\left<L> ... \right<R>` in math. tree-sitter packages the whole
    /// span as a `math_delimiter` node. We emit just the delimiter pair
    /// plus the body — Typst auto-pairs balanced delimiters and provides
    /// `lr(...)` for explicit stretching that we don't need here. Drop
    /// the `\left` / `\right` commands themselves (they'd otherwise leak
    /// into the output as literal `\left(`/`\right)` and Typst would
    /// read `\l` as the math escape for `l`, leaving `eft(` dangling).
    /// `\left.` and `\right.` (no-display delimiters in LaTeX) are
    /// emitted as empty so the body still pairs.
    fn emit_math_delimiter(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        for child in children {
            let kind = child.kind();
            // Skip the size commands themselves.
            if matches!(
                kind,
                "\\left"
                    | "\\right"
                    | "\\bigl"
                    | "\\Bigl"
                    | "\\biggl"
                    | "\\Biggl"
                    | "\\bigr"
                    | "\\Bigr"
                    | "\\biggr"
                    | "\\Biggr"
                    | "\\middle"
            ) {
                continue;
            }
            // `.` is LaTeX's "invisible delimiter" — drop.
            let text = &self.src[child.start_byte()..child.end_byte()];
            if text == "." {
                continue;
            }
            self.emit_node(child);
        }
        node.end_byte()
    }

    fn emit_math_children(&mut self, node: Node<'_>) {
        let mut cursor = node.walk();
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "$" | "$$" | "\\[" | "\\]" | "\\(" | "\\)"))
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
            self.push_math_symbol(typst);
            return node.end_byte();
        }
        match n {
            "\\frac" | "\\tfrac" | "\\dfrac" | "\\cfrac" => self.emit_math_frac(node),
            "\\sqrt" => self.emit_math_sqrt(node),
            "\\binom" | "\\dbinom" | "\\tbinom" => self.emit_math_binom(node),
            // Horizontal braces: `\overbrace{x}` → `overbrace(x)`.
            // If the user writes `\overbrace{x}^{text}`, the `^{text}` becomes a
            // Typst superscript on the overbrace call, which is correct.
            "\\overbrace" => self.emit_math_wrap(node, "overbrace(", ")"),
            "\\underbrace" => self.emit_math_wrap(node, "underbrace(", ")"),
            // Enclosures
            "\\cancel" => self.emit_math_wrap(node, "cancel(", ")"),
            "\\bcancel" => self.emit_math_wrap(node, "cancel(inverted: true, ", ")"),
            "\\xcancel" => self.emit_math_wrap(node, "cancel(cross: true, ", ")"),
            "\\sout" => self.emit_math_wrap(node, "strike(", ")"),
            // `\text{X}` and `\mathrm{X}` switch to upright text inside math.
            // Typst renders quoted strings as upright text in math context.
            "\\text" | "\\mathrm" | "\\textrm" | "\\mathnormal" => {
                if let Some(arg) = first_curly_group(node) {
                    // Take the raw inner source — we want literal text, not
                    // a recursively-emitted (and possibly mangled) sub-render.
                    let inner = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    let _ = write!(self.out, "\"{}\"", inner);
                }
                node.end_byte()
            }
            // `\mathbf{X}` → bold math; `\mathbb{X}` → blackboard bold (`bb(X)`).
            "\\mathbf" | "\\bm" | "\\bs" | "\\bold" => self.emit_math_wrap(node, "bold(", ")"),
            "\\mathbb" | "\\mathbbm" | "\\Bbb" => self.emit_math_wrap(node, "bb(", ")"),
            "\\mathcal" => self.emit_math_wrap(node, "cal(", ")"),
            "\\mathfrak" | "\\frak" => self.emit_math_wrap(node, "frak(", ")"),
            "\\mathscr" => self.emit_math_wrap(node, "scr(", ")"),
            "\\mathsf" => self.emit_math_wrap(node, "sans(", ")"),
            "\\mathit" => self.emit_math_wrap(node, "italic(", ")"),
            "\\mathtt" => self.emit_math_wrap(node, "mono(", ")"),
            "\\boldsymbol" | "\\pmb" => self.emit_math_wrap(node, "bold(", ")"),
            // Math accents
            "\\bar" | "\\overline" => self.emit_math_wrap(node, "overline(", ")"),
            "\\underline" => self.emit_math_wrap(node, "underline(", ")"),
            "\\hat" | "\\widehat" => self.emit_math_wrap(node, "hat(", ")"),
            "\\tilde" | "\\widetilde" => self.emit_math_wrap(node, "tilde(", ")"),
            "\\vec" | "\\overrightarrow" | "\\Overrightarrow" => {
                self.emit_math_wrap(node, "arrow(", ")")
            }
            "\\dot" => self.emit_math_wrap(node, "dot(", ")"),
            "\\ddot" => self.emit_math_wrap(node, "dot.double(", ")"),
            "\\acute" => self.emit_math_wrap(node, "acute(", ")"),
            "\\grave" => self.emit_math_wrap(node, "grave(", ")"),
            "\\check" | "\\widecheck" => self.emit_math_wrap(node, "caron(", ")"),
            "\\breve" => self.emit_math_wrap(node, "breve(", ")"),
            "\\mathring" => self.emit_math_wrap(node, "circle(", ")"),
            "\\phantom" => self.emit_math_wrap(node, "hide(", ")"),
            // `\operatorname{name}` → `op("name")` — upright math text.
            "\\operatorname" => self.emit_math_operatorname(node),
            // Math-mode spacing primitives — drop silently.
            "\\hspace" | "\\vspace" | "\\!" | "\\linebreak" | "\\nobreak" => node.end_byte(),
            // `\tag{...}` adds LaTeX equation labels for presentation only;
            // Typst handles equation numbering itself. Drop the command and its
            // curly-group argument (the generic_command node covers both).
            "\\tag" => node.end_byte(),
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
            _ => self.emit_unknown_math_command(node, n),
        }
    }

    /// Fallback for an unrecognised command inside math. Emits a Typst
    /// string-literal placeholder (`"name"`) so the output stays valid, and
    /// records an `ambiguous_math` warning. Both the `command_name` walker
    /// arm and `emit_math_command`'s catch-all delegate here so the two paths
    /// cannot drift apart.
    /// Emit a LaTeX text accent (`\'`, `\"`, `\^`, `` \` ``, `\~`) as the
    /// correct Unicode character.
    ///
    /// - Brace form `\'{e}`: the curly_group child provides the letter.
    /// - Bare form `\'e`: the first source byte after the command node is the
    ///   letter; it is consumed via `skip_until` so the parent walker doesn't
    ///   re-emit it.
    fn emit_text_accent(&mut self, node: Node<'_>, accent: char) -> usize {
        // Brace form: curly_group child.
        if let Some(group) = first_curly_group(node) {
            let inner = &self.src[group.start_byte() + 1..group.end_byte() - 1];
            if let Some(letter) = inner.chars().next() {
                let rest = &inner[letter.len_utf8()..];
                self.out.push_str(&apply_text_accent(accent, letter));
                self.out.push_str(rest);
                return node.end_byte();
            }
            // Empty braces — emit nothing.
            return node.end_byte();
        }
        // Bare form: peek at the next byte in source.
        let rest = &self.src[node.end_byte()..];
        if let Some(letter) = rest.chars().next() {
            let new_end = node.end_byte() + letter.len_utf8();
            self.out.push_str(&apply_text_accent(accent, letter));
            self.skip_until = self.skip_until.max(new_end);
            return new_end;
        }
        node.end_byte()
    }

    fn emit_unknown_math_command(&mut self, node: Node<'_>, name: &str) -> usize {
        if self.macros.contains_key(name) {
            return self.expand_user_macro(node, name);
        }
        self.warn_ambiguous_math(node, name);
        let display = name.strip_prefix('\\').unwrap_or(name);
        let _ = write!(self.out, " \"{}\" ", display);
        node.end_byte()
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
                self.ensure_math_letter_boundary("sqrt(");
                let _ = write!(self.out, "sqrt({})", inner.trim());
            }
            None => {
                self.warn_ambiguous_math(node, "\\sqrt (missing arg)");
            }
        }
        node.end_byte()
    }

    /// `\operatorname{X}` → `op("X")` — render the literal name as upright text.
    fn emit_math_operatorname(&mut self, node: Node<'_>) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let inner = self
                .src
                .get(arg.start_byte() + 1..arg.end_byte() - 1)
                .unwrap_or("")
                .trim();
            self.ensure_math_letter_boundary("op(");
            let _ = write!(self.out, "op(\"{}\")", inner);
        } else {
            self.warn_ambiguous_math(node, "\\operatorname (missing arg)");
        }
        node.end_byte()
    }

    /// Wrap the first curly_group argument in a Typst math function call:
    /// `\mathbf{X}` → `bold(X)`. Recursively renders the inner content in
    /// math mode so nested commands are translated.
    fn emit_math_wrap(&mut self, node: Node<'_>, left: &str, right: &str) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let inner = self.render_math_group(arg);
            self.ensure_math_letter_boundary(left);
            self.out.push_str(left);
            self.out.push_str(inner.trim());
            self.out.push_str(right);
            return node.end_byte();
        }
        // Brace-less form — LaTeX permits `\hat x`, `\mathcal A`,
        // `\bar\alpha` etc. The argument is the next non-whitespace
        // token in the source; tree-sitter parses it as a sibling of
        // this command, not a child. Consume it via the shared
        // `consume_braceless_arg` helper, then route per variant:
        // commands lookup_math_symbol → user macros → raw; groups go
        // through a math sub-emitter; chars pass through.
        let (parsed_arg, arg_end) = match consume_braceless_arg(self.src, node.end_byte()) {
            Some(pair) => pair,
            None => {
                self.warn_ambiguous_math(node, "missing argument");
                return node.end_byte();
            }
        };
        let arg_render = match parsed_arg {
            BracelessArg::Command(cmd) => {
                // Resolution order: math symbol table → user macros → raw.
                // Without the macro check, `\widehat\HSIC` (where `\HSIC`
                // is a `\newcommand` defined in the source) emits the
                // literal `\HSIC` instead of the expansion.
                if let Some(typst) = lookup_math_symbol(&cmd) {
                    typst.to_string()
                } else if let Some(macro_def) = self.macros.get(&cmd).cloned() {
                    // Expand: re-parse the macro body and render via a
                    // sub-emitter inheriting the math context + macro table.
                    let body = macro_def.body.clone();
                    let tree = crate::parser::parse(&body);
                    let visited = std::mem::take(&mut self.visited_includes);
                    let macros = self.macros.clone();
                    let mut sub = Emitter::with_includes(
                        &body,
                        self.source_name,
                        self.base_dir.clone(),
                        visited,
                    );
                    sub.in_math = true;
                    sub.macros = macros;
                    sub.macro_depth = self.macro_depth + 1;
                    sub.emit_root(tree.root_node());
                    self.visited_includes = std::mem::take(&mut sub.visited_includes);
                    for (k, v) in sub.macros.drain() {
                        self.macros.entry(k).or_insert(v);
                    }
                    self.warnings.append(&mut sub.warnings);
                    self.asset_refs.append(&mut sub.asset_refs);
                    sub.out.trim().to_string()
                } else {
                    cmd
                }
            }
            BracelessArg::Group(inner_src) => {
                // Brace group as the arg: re-parse inner content with the
                // current math context so nested commands render properly.
                let tree = crate::parser::parse(&inner_src);
                let visited = std::mem::take(&mut self.visited_includes);
                let macros = self.macros.clone();
                let mut sub = Emitter::with_includes(
                    &inner_src,
                    self.source_name,
                    self.base_dir.clone(),
                    visited,
                );
                sub.in_math = true;
                sub.macros = macros;
                sub.emit_root(tree.root_node());
                self.visited_includes = std::mem::take(&mut sub.visited_includes);
                for (k, v) in sub.macros.drain() {
                    self.macros.entry(k).or_insert(v);
                }
                self.warnings.append(&mut sub.warnings);
                self.asset_refs.append(&mut sub.asset_refs);
                sub.out.trim().to_string()
            }
            BracelessArg::Char(c) => c,
        };
        self.ensure_math_letter_boundary(left);
        self.out.push_str(left);
        self.out.push_str(arg_render.trim());
        self.out.push_str(right);
        // Mark the consumed argument range as already-emitted.
        self.skip_until = self.skip_until.max(arg_end);
        arg_end
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
        self.ensure_math_letter_boundary("binom(");
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
                // Render the arg into a scratch buffer so we can decide
                // whether to wrap. Typst parses `_cal(T)` as `_c · al(T)`
                // (the `c` is the subscript, the rest is a separate
                // expression); we need `_(cal(T))` to keep the whole
                // wrap as the subscript group. Wrap whenever the
                // rendered text would otherwise parse as more than a
                // single token.
                let rendered = self.with_sub_buffer(|emitter| {
                    let _ = emitter.emit_node(*arg);
                });
                let trimmed = rendered.trim();
                if needs_subscript_parens(trimmed) {
                    let _ = write!(self.out, "({})", trimmed);
                } else {
                    self.out.push_str(trimmed);
                }
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

    /// `\begin{cases} ... \end{cases}` → `cases(...)`. Each LaTeX row maps
    /// to one Typst cases argument. Rows are separated in the source by
    /// `\\`, and inside each row the value and condition are separated by
    /// `&` (e.g. `value & condition \\`). Typst's `cases()` only takes a
    /// list of expressions, so we collapse the row's value and condition
    /// with a `quad` space between them, then wrap the entire row in a
    /// math grouping construct that preserves nested commas — without it,
    /// commas inside `\max\{a, 0\}` are read as cases separators.
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
        // Row break in LaTeX cases is `\\`. The render walker emitted that
        // as ` \` (trailing space then backslash) at the end of each line,
        // matching the source convention. Split on it.
        let rows: Vec<String> = body_str
            .split(" \\")
            .map(|r| {
                let r = r.trim();
                // Inside a row, `&` separates value from condition.
                // Replace with ` quad ` (an em of horizontal space) and
                // wrap the row in `lr(...)` so internal commas are
                // preserved as content, not parsed as cases separators.
                let row = r.replace('&', " quad ");
                // `lr(...)` accepts arbitrary content; the leading and
                // trailing single-char delim positions don't matter when
                // the content already pairs. Use empty fences to keep the
                // grouping invisible.
                format!("[{}]", row)
            })
            .filter(|r| r != "[]")
            .collect();
        let _ = write!(self.out, "cases({})", rows.join(", "));
        self.in_math = was;
        node.end_byte()
    }

    /// `\begin{array}{cols} ... \end{array}` when nested inside a math
    /// container (the only reasonable Typst rendering for math-mode
    /// arrays). The dispatcher routes here when `self.in_math == true`;
    /// the text-mode `array` case still goes through `emit_tabular`.
    ///
    /// LaTeX `array` envs differ from `cases` only in the column
    /// specifier — `cases` is implicitly `{ll}`, array exposes it.
    /// For two-column arrays (the common piecewise form) the output
    /// is identical to `cases`. For wider arrays we collapse all
    /// cells with `quad` and let cases render them as one stacked
    /// expression per row.
    fn emit_array_in_math(&mut self, node: Node<'_>) -> usize {
        // Skip the column-spec curly_group (the first one); body
        // children are the rest.
        let mut cursor = node.walk();
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| !matches!(c.kind(), "begin" | "end" | "curly_group"))
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
        // Rows are split on the rendered row-break (` \` from the
        // emit_math_command `\\` handler) — same as emit_cases_env.
        let rows: Vec<String> = body_str
            .split(" \\")
            .map(|r| {
                let r = r.trim();
                // Cells: `&` separator gets collapsed to `quad`. Wrap
                // the whole row in `[content]` so internal commas
                // don't get read as cases() argument separators.
                let row = r.replace('&', " quad ");
                format!("[{}]", row)
            })
            .filter(|r| r != "[]")
            .collect();
        let _ = write!(self.out, "cases({})", rows.join(", "));
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
        let (key, end_after_brace) = match extract_label_ref_key_and_end(node, self.src) {
            Some(x) => x,
            None => {
                self.warn_unsupported_command(node);
                return node.end_byte();
            }
        };
        if key.is_empty() {
            self.warn_unsupported_command(node);
            return node.end_byte();
        }
        // Cover the truncated-grammar tail (`_objective}`) when the key
        // contains underscores — same as `\label{...}` handling above.
        self.skip_until = self.skip_until.max(end_after_brace);
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
                } else if !key.starts_with("fig:")
                    && !key.starts_with("tab:")
                    && !key.starts_with("thm:")
                    && !key.starts_with("lem:")
                    && !key.starts_with("cor:")
                    && !key.starts_with("def:")
                    && !key.starts_with("prop:")
                {
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
        let paths = extract_bib_paths(node, self.src);
        if paths.is_empty() {
            self.warn_unsupported_command(node);
            return node.end_byte();
        }
        let style = self.pending_bib_style.take();
        // Convention: append `.bib` if no extension supplied.
        let paths_with_ext: Vec<String> = paths
            .iter()
            .map(|p| {
                if p.contains('.') {
                    p.clone()
                } else {
                    format!("{}.bib", p)
                }
            })
            .collect();
        // Record an AssetRef for each bib that resolves on disk so the
        // project materialiser copies every entry, not just the first.
        if let Some(ref base) = self.base_dir.clone() {
            for (raw, with_ext) in paths.iter().zip(paths_with_ext.iter()) {
                if let Some(source_path) = probe_bib_on_disk(base, raw) {
                    self.asset_refs.push(crate::AssetRef {
                        kind: crate::AssetKind::Bibliography,
                        typst_path: with_ext.clone(),
                        source_path,
                    });
                }
            }
        }
        self.ensure_paragraph_break();
        let mapped = style.as_deref().and_then(map_bibliography_style);
        // Typst's `#bibliography` takes either a single path string or a
        // tuple of paths. Emit the tuple form when we have multiple.
        let path_arg = if paths_with_ext.len() == 1 {
            format!("\"{}\"", paths_with_ext[0])
        } else {
            let joined = paths_with_ext
                .iter()
                .map(|p| format!("\"{}\"", p))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({},)", joined)
        };
        if let Some(s) = mapped {
            let _ = write!(self.out, "#bibliography({}, style: \"{}\")", path_arg, s);
        } else {
            let _ = write!(self.out, "#bibliography({})", path_arg);
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
        // Typst supports PNG/JPG/GIF/SVG and PDF (>=0.10), but NOT EPS or
        // PS — many older arxiv preprints ship `.eps` figures. Emit a
        // labelled placeholder rect rather than a hard image() call so
        // the rest of the document compiles. Same fallback for `.ps`
        // and `.tikz`-style includes that masquerade as graphics.
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".eps")
            || lower.ends_with(".ps")
            || lower.ends_with(".tikz")
            || lower.ends_with(".pgf")
        {
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::NeedsManualReview {
                    reason: format!("unsupported image format: {}", path),
                },
                severity: Severity::Warning,
                message: format!(
                    "Typst cannot render `{}` — emitting a placeholder. Convert the asset to PDF, PNG, or SVG and rerun.",
                    path
                ),
                snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                suggested_skill: None,
            });
            let _ = write!(
                self.out,
                "rect(width: 60%, height: 4em, stroke: 0.5pt, fill: luma(240))[#align(center + horizon)[`{}`]]",
                path
            );
            return node.end_byte();
        }
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
        // Record the asset ref if the image exists on disk. The typst_path is
        // whatever path string the Typst source references (used for relocation
        // by the project layer). When the file can't be probed, emit a
        // NeedsManualReview warning so callers know the `image(...)` call in
        // the Typst body has no matching AssetRef in the project plan.
        if let Some(ref base) = self.base_dir.clone() {
            match probe_image_on_disk(base, &path) {
                Some(source_path) => {
                    // Build the typst_path: use the resolved filename so it has an
                    // extension even if the LaTeX source omitted it.
                    let typst_path = if std::path::Path::new(&path).extension().is_some() {
                        path.clone()
                    } else {
                        source_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|name| {
                                // Re-join the directory component from the original path.
                                let dir = std::path::Path::new(&path).parent()
                                    .and_then(|p| p.to_str())
                                    .unwrap_or("");
                                if dir.is_empty() { name.to_string() }
                                else { format!("{}/{}", dir, name) }
                            })
                            .unwrap_or_else(|| path.clone())
                    };
                    self.asset_refs.push(crate::AssetRef {
                        kind: crate::AssetKind::Image,
                        typst_path,
                        source_path,
                    });
                }
                None => {
                    self.warnings.push(Warning {
                        range: range_of(node),
                        category: Category::NeedsManualReview {
                            reason: format!("image not found relative to base: {}", path),
                        },
                        severity: Severity::Warning,
                        message: format!(
                            "could not resolve `\\includegraphics{{{}}}` against `{}` — the Typst body still references it, so `typst compile` will fail until the file is provided.",
                            path,
                            base.display()
                        ),
                        snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                        suggested_skill: None,
                    });
                }
            }
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
            // No image / tabular and no other recoverable body —
            // emit a placeholder rect that compiles without referring
            // to a missing file. The labelled rect plays the same
            // role as the EPS fallback in emit_graphics_include.
            "rect(width: 60%, height: 4em, stroke: 0.5pt, fill: luma(240))[#align(center + horizon)[(figure)]]"
                .to_string()
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
        // Column spec is the first `curly_group` child of the env —
        // except for `tabular*` / `tabularx` which take a width
        // argument first; in that case the column spec is the SECOND
        // curly group.
        let env = environment_name(node, self.src).unwrap_or_default();
        let needs_skip = matches!(env.as_str(), "tabular*" | "tabularx" | "tabulary");
        let mut cursor = node.walk();
        let curly_groups: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "curly_group")
            .collect();
        let spec_node = if needs_skip {
            curly_groups.get(1).copied()
        } else {
            curly_groups.first().copied()
        };
        let col_spec = spec_node
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
        // Emit cells grouped by row for readability. Skip rows that have no
        // cells (avoids a trailing lone-comma artifact).
        let mut idx = 0;
        for _ in 0..rows.len() {
            if idx >= cells.len() {
                break;
            }
            self.out.push_str("  ");
            let mut emitted_any = false;
            for _ in 0..count {
                if idx >= cells.len() {
                    break;
                }
                if emitted_any {
                    self.out.push_str(", ");
                }
                // Cells produced by `\multicolumn` are already a function call
                // (`table.cell(colspan: N)[...]`) and must not get wrapped in
                // another `[...]`. Recognize the prefix and emit verbatim.
                let cell = &cells[idx];
                if cell.starts_with("table.cell(") {
                    self.out.push_str(cell);
                } else {
                    let _ = write!(self.out, "[{}]", escape_text_cell(cell));
                }
                emitted_any = true;
                idx += 1;
            }
            if emitted_any {
                self.out.push_str(",\n");
            } else {
                // Roll back the leading two-space indent if no cell emitted.
                self.out.pop();
                self.out.pop();
            }
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

/// First child whose kind starts with `curly_group` — matches all the
/// specialized variants tree-sitter-latex uses (`curly_group_author_list`,
/// `curly_group_text`, `curly_group_path`, etc.).
fn first_curly_like(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .find(|child| child.kind().starts_with("curly_group"));
    result
}

/// True if the last emitted bytes look like "we just opened math and haven't
/// written a base symbol yet", in which case a subscript or superscript needs
/// an empty-string base to be valid Typst.
fn needs_empty_base(out: &str) -> bool {
    let trimmed = out.trim_end_matches([' ', '\t']);
    trimmed.ends_with('$') || trimmed.ends_with("$ ")
}

/// Map a LaTeX text accent + base letter to the precomposed Unicode codepoint.
///
/// `accent` is the accent character: `'\''` acute, '`' grave, `'"'` diaeresis,
/// `'^'` circumflex, `'~'` tilde. Returns a `String` so the combining-mark
/// fallback path (two code points) is representable.
fn apply_text_accent(accent: char, letter: char) -> String {
    let precomposed: Option<char> = match (accent, letter) {
        // Acute (')
        ('\'', 'a') => Some('á'), ('\'', 'A') => Some('Á'),
        ('\'', 'e') => Some('é'), ('\'', 'E') => Some('É'),
        ('\'', 'i') => Some('í'), ('\'', 'I') => Some('Í'),
        ('\'', 'o') => Some('ó'), ('\'', 'O') => Some('Ó'),
        ('\'', 'u') => Some('ú'), ('\'', 'U') => Some('Ú'),
        ('\'', 'y') => Some('ý'), ('\'', 'Y') => Some('Ý'),
        ('\'', 'n') => Some('ń'), ('\'', 'N') => Some('Ń'),
        ('\'', 'c') => Some('ć'), ('\'', 'C') => Some('Ć'),
        ('\'', 's') => Some('ś'), ('\'', 'S') => Some('Ś'),
        ('\'', 'z') => Some('ź'), ('\'', 'Z') => Some('Ź'),
        ('\'', 'l') => Some('ĺ'), ('\'', 'L') => Some('Ĺ'),
        ('\'', 'r') => Some('ŕ'), ('\'', 'R') => Some('Ŕ'),
        // Grave (`)
        ('`', 'a') => Some('à'), ('`', 'A') => Some('À'),
        ('`', 'e') => Some('è'), ('`', 'E') => Some('È'),
        ('`', 'i') => Some('ì'), ('`', 'I') => Some('Ì'),
        ('`', 'o') => Some('ò'), ('`', 'O') => Some('Ò'),
        ('`', 'u') => Some('ù'), ('`', 'U') => Some('Ù'),
        ('`', 'n') => Some('ǹ'), ('`', 'N') => Some('Ǹ'),
        // Diaeresis (")
        ('"', 'a') => Some('ä'), ('"', 'A') => Some('Ä'),
        ('"', 'e') => Some('ë'), ('"', 'E') => Some('Ë'),
        ('"', 'i') => Some('ï'), ('"', 'I') => Some('Ï'),
        ('"', 'o') => Some('ö'), ('"', 'O') => Some('Ö'),
        ('"', 'u') => Some('ü'), ('"', 'U') => Some('Ü'),
        ('"', 'y') => Some('ÿ'), ('"', 'Y') => Some('Ÿ'),
        // Circumflex (^)
        ('^', 'a') => Some('â'), ('^', 'A') => Some('Â'),
        ('^', 'e') => Some('ê'), ('^', 'E') => Some('Ê'),
        ('^', 'i') => Some('î'), ('^', 'I') => Some('Î'),
        ('^', 'o') => Some('ô'), ('^', 'O') => Some('Ô'),
        ('^', 'u') => Some('û'), ('^', 'U') => Some('Û'),
        ('^', 'c') => Some('ĉ'), ('^', 'C') => Some('Ĉ'),
        ('^', 'g') => Some('ĝ'), ('^', 'G') => Some('Ĝ'),
        ('^', 'h') => Some('ĥ'), ('^', 'H') => Some('Ĥ'),
        ('^', 'j') => Some('ĵ'), ('^', 'J') => Some('Ĵ'),
        ('^', 's') => Some('ŝ'), ('^', 'S') => Some('Ŝ'),
        ('^', 'w') => Some('ŵ'), ('^', 'W') => Some('Ŵ'),
        ('^', 'y') => Some('ŷ'), ('^', 'Y') => Some('Ŷ'),
        // Tilde (~)
        ('~', 'a') => Some('ã'), ('~', 'A') => Some('Ã'),
        ('~', 'e') => Some('ẽ'), ('~', 'E') => Some('Ẽ'),
        ('~', 'i') => Some('ĩ'), ('~', 'I') => Some('Ĩ'),
        ('~', 'n') => Some('ñ'), ('~', 'N') => Some('Ñ'),
        ('~', 'o') => Some('õ'), ('~', 'O') => Some('Õ'),
        ('~', 'u') => Some('ũ'), ('~', 'U') => Some('Ũ'),
        _ => None,
    };
    if let Some(c) = precomposed {
        return c.to_string();
    }
    // Combining-mark fallback: letter + Unicode combining diacritic.
    let combining: Option<char> = match accent {
        '\'' => Some('\u{0301}'),
        '`' => Some('\u{0300}'),
        '"' => Some('\u{0308}'),
        '^' => Some('\u{0302}'),
        '~' => Some('\u{0303}'),
        _ => None,
    };
    let mut s = letter.to_string();
    if let Some(m) = combining {
        s.push(m);
    }
    s
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

/// Extract the key from a `label_reference` node (`\ref{x}`, `\eqref{x}`)
/// plus the byte offset just past the closing `}` so callers can
/// `skip_until` over the part of the source tree-sitter dropped when
/// the key contains underscores (same bug as `extract_label_name`).
fn extract_label_ref_key_and_end(node: Node<'_>, src: &str) -> Option<(String, usize)> {
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
                    return Some((normalize_label_key(&src[start..i]), i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Extract the path argument from a `bibtex_include` (`\bibliography{x}`) or
/// `bibstyle_include` (`\bibliographystyle{x}`) node.
fn extract_bib_path(node: Node<'_>, src: &str) -> Option<String> {
    extract_bib_paths(node, src).into_iter().next()
}

/// Collect every comma-separated bib path in a `\bibliography{a,b,c}`
/// call. The pre-2026-05 helper returned only the first match, so
/// multi-bib papers silently lost every entry after the first; project
/// mode then failed to copy those files and `typst compile` died with
/// `file not found`.
fn extract_bib_paths(node: Node<'_>, src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "curly_group_path" | "curly_group_path_list") {
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "path" {
                    out.push(src[grandchild.start_byte()..grandchild.end_byte()].to_string());
                }
            }
        }
    }
    out
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
/// - `\linewidth` / `\textwidth` / `\columnwidth` → `100%`
/// - `0.5\textwidth` → `50%`
/// - `3cm`, `2in`, `100pt` → as-is (Typst accepts these units)
///
/// Bare width tokens with no numeric coefficient previously fell through
/// verbatim — Typst then rejected the `\` in code context, blocking
/// compilation. Treat the bare form as the full container width.
fn normalize_graphics_length(v: &str) -> String {
    let v = v.trim();
    for kw in ["\\textwidth", "\\linewidth", "\\columnwidth"] {
        if let Some(num) = v.strip_suffix(kw) {
            let num = num.trim();
            if num.is_empty() {
                return "100%".to_string();
            }
            if let Ok(f) = num.parse::<f64>() {
                return format!("{}%", (f * 100.0).round() as i64);
            }
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

/// Escape unbalanced paired delimiters (`[`, `]`, `(`, `)`) in a Typst math
/// body. LaTeX half-open intervals such as `(0, s_*]` or `[a, b)` mix
/// delimiter kinds: Typst pairs `[..]` and `(..)` independently, so when one
/// kind doesn't balance, both the orphan close (`]`) AND the partner of the
/// other kind that no longer has a matching close (`(`) need escaping —
/// otherwise Typst complains about an unclosed delimiter on the *other* one.
/// Balanced pairs are left untouched. Pre-existing backslash escapes are
/// skipped so we never double-escape.
fn escape_unbalanced_math_brackets(body: &str) -> String {
    let bytes = body.as_bytes();
    let mut bracket_opens: Vec<usize> = Vec::new();
    let mut paren_opens: Vec<usize> = Vec::new();
    let mut escapes: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // Skip any backslash-escaped character (including pre-existing
        // `\[` / `\(` from `\left[`-style emissions we may add later).
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        match bytes[i] {
            b'[' => bracket_opens.push(i),
            b']' if bracket_opens.pop().is_none() => escapes.push(i),
            b'(' => paren_opens.push(i),
            b')' if paren_opens.pop().is_none() => escapes.push(i),
            _ => {}
        }
        i += 1;
    }
    escapes.extend(bracket_opens);
    escapes.extend(paren_opens);
    if escapes.is_empty() {
        return body.to_string();
    }
    escapes.sort_unstable();
    let mut out = String::with_capacity(body.len() + escapes.len());
    let mut last = 0;
    for pos in escapes {
        out.push_str(&body[last..pos]);
        out.push('\\');
        out.push(bytes[pos] as char);
        last = pos + 1;
    }
    out.push_str(&body[last..]);
    out
}

/// Translate a LaTeX math command (with the leading backslash) into the
/// corresponding Typst math fragment. Returns `None` for unknown commands so
/// callers can decide between structural emission, warning, or pass-through.
pub(crate) fn lookup_math_symbol(name: &str) -> Option<&'static str> {
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
        "\\sim" => "tilde.op",
        "\\simeq" => "tilde.eq",
        "\\cong" => "tilde.equiv",
        "\\asymp" => "≍",
        "\\propto" => "prop",
        "\\ngeq" => "gt.eq.not",
        "\\ngtr" => "gt.not",
        "\\nleq" => "lt.eq.not",
        "\\nless" => "lt.not",
        "\\coloneqq" | "\\coloneq" | "\\defeq" => "colon.eq",
        "\\eqqcolon" | "\\eqcolon" => "eq.colon",
        "\\bowtie" => "join",
        "\\to" | "\\rightarrow" => "arrow.r",
        "\\leftarrow" => "arrow.l",
        "\\leftrightarrow" => "arrow.l.r",
        "\\Rightarrow" => "arrow.r.double",
        "\\Leftarrow" => "arrow.l.double",
        "\\Leftrightarrow" => "arrow.l.r.double",
        "\\mapsto" => "arrow.r.bar",
        "\\hookrightarrow" => "arrow.r.hook",
        "\\hookleftarrow" => "arrow.l.hook",
        "\\uparrow" => "arrow.t",
        "\\downarrow" => "arrow.b",
        "\\updownarrow" => "arrow.t.b",
        "\\Uparrow" => "arrow.t.double",
        "\\Downarrow" => "arrow.b.double",
        "\\circ" => "circle.small",
        "\\bullet" => "bullet",
        "\\star" => "star.op",
        "\\ast" => "ast.op",
        // Circled / boxed operators
        "\\otimes" => "times.circle",
        "\\oplus" => "plus.circle",
        "\\ominus" => "minus.circle",
        "\\odot" => "dot.circle",
        "\\oslash" => "slash.circle",
        "\\boxtimes" => "times.square",
        "\\boxplus" => "plus.square",
        // Geometric / order
        "\\Box" | "\\square" => "square",
        "\\diamond" | "\\Diamond" | "\\diamondsuit" => "diamond",
        "\\triangle" | "\\bigtriangleup" => "triangle",
        "\\bigtriangledown" => "triangle.b",
        "\\angle" => "angle",
        "\\perp" => "perp",
        "\\parallel" => "parallel",
        "\\top" => "top",
        "\\bot" => "bot",
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
        "\\partial" => "partial",
        "\\nabla" => "nabla",
        "\\hbar" | "\\hslash" => "planck",
        "\\ell" => "ell",
        "\\dots" | "\\ldots" => "dots.h",
        "\\cdots" => "dots.c",
        "\\vdots" => "dots.v",
        "\\ddots" => "dots.down",
        "\\degree" => "degree",
        "\\dagger" => "dagger",
        "\\ddagger" => "dagger.double",
        "\\prime" => "prime",
        "\\Re" => "Re",
        "\\Im" => "Im",
        // `\notag` / `\nonumber` suppress equation numbering. Typst
        // doesn't number untagged equations either, so drop silently.
        "\\notag" | "\\nonumber" => "",
        // `\colon` is the typed colon glyph in amsmath; in Typst math
        // a plain `:` renders identically.
        "\\colon" => ":",
        "\\aleph" => "aleph",
        "\\beth" => "beth",
        "\\gimel" => "gimel",
        "\\imath" => "dotless.i",
        "\\jmath" => "dotless.j",
        "\\backslash" => "backslash",
        "\\flat" => "♭",
        "\\sharp" => "♯",
        "\\natural" => "♮",
        "\\clubsuit" => "♣",
        "\\spadesuit" => "♠",
        "\\heartsuit" => "♥",
        // `\not` as a slash overlay — Typst's `cancel(...)` is the
        // closest match but takes an argument; for the bare-prefix
        // form (`\not =`) drop the prefix and let the `=` render
        // unmodified.
        "\\not" => "",
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
        // Norm / bar delimiters
        "\\|" | "\\Vert" => "||",
        "\\vert" => "|",
        "\\lvert" => "|",
        "\\rvert" => "|",
        "\\lVert" | "\\rVert" => "||",
        // Typst 0.13+ deprecated `angle.l` / `angle.r` in favour of
        // `chevron.l` / `chevron.r`; emitting the new names keeps the
        // compile clean of deprecation warnings.
        "\\langle" => "chevron.l",
        "\\rangle" => "chevron.r",
        "\\lceil" => "ceil.l",
        "\\rceil" => "ceil.r",
        "\\lfloor" => "floor.l",
        "\\rfloor" => "floor.r",
        // Math spacing — LaTeX positive-space commands. Typst's `thin`,
        // `med`, `thick`, `quad` are the equivalent named symbols. Without
        // these the bare LaTeX command name leaked into the output and
        // fused with the next identifier (`\thinspace` adjacent to `d`
        // would produce the unknown variable `thinspaced`).
        "\\qquad" => "quad quad",
        "\\quad" => "quad",
        "\\," | "\\thinspace" => "thin",
        "\\:" | "\\medspace" => "med",
        "\\;" | "\\thickspace" => "thick",
        // Negative spacing — Typst has no direct named equivalent. Drop;
        // the visual difference at the call sites is sub-em.
        "\\!" | "\\negthinspace" | "\\negmedspace" | "\\negthickspace" => "",
        // Delimiter-size commands (Typst auto-sizes via `lr(...)`); drop.
        "\\big" | "\\Big" | "\\bigg" | "\\Bigg" | "\\bigl" | "\\Bigl" | "\\biggl" | "\\Biggl"
        | "\\bigr" | "\\Bigr" | "\\biggr" | "\\Biggr" | "\\bigm" | "\\Bigm" | "\\biggm"
        | "\\Biggm" => "",
        // `\left` / `\right` in math — Typst's math grammar auto-pairs
        // `(`, `[`, `\{` style delimiters and provides `lr(...)` for
        // explicit stretching. Dropping the command keeps the following
        // delimiter character (which is emitted as its own node) intact.
        // Previously `\left(V-G\right)` leaked into the output as raw
        // `\left(V-G\right)`, and Typst read `\l` as the math escape for
        // `l`, leaving the unknown identifier `eft(...)`.
        "\\left" | "\\right" => "",
        // `\middle` is the same pattern for mid-fence stretching; no
        // Typst equivalent for the bare form, so drop and let the
        // following delimiter render literally.
        "\\middle" => "",
        // Operator-display modifiers — Typst always places sub/super
        // in display position for `lim`, `sum`, `int`, etc., so the
        // explicit force is a no-op. Drop the command name itself; if
        // left in, `\limits` was emitted as the literal word and the
        // subscript that followed became an unknown symbol modifier.
        "\\limits"
        | "\\nolimits"
        | "\\displaystyle"
        | "\\textstyle"
        | "\\scriptstyle"
        | "\\scriptscriptstyle" => "",
        // Math escapes for ASCII chars. Keep the leading backslash so Typst
        // treats them as math escapes — emitting the bare character would
        // trigger Typst's own special handling: `#` opens code context,
        // `$` toggles math, `&` is alignment, `_` / `^` are sub/superscript,
        // `{` / `}` are paired delimiters. Concretely, `f_\#` previously
        // emitted as `f_(#)` was parsed as `(code)` and failed with
        // "unexpected closing paren".
        "\\#" => "\\#",
        "\\$" => "\\$",
        "\\%" => "\\%",
        "\\&" => "\\&",
        "\\_" => "\\_",
        "\\{" => "\\{",
        "\\}" => "\\}",
        // === AMS subset/supset relations (Phase 1a) ===
        "\\subsetneq" | "\\varsubsetneq" => "subset.neq",
        "\\supsetneq" | "\\varsupsetneq" => "supset.neq",
        "\\nsubseteq" => "subset.eq.not",
        "\\nsupseteq" => "supset.eq.not",
        "\\sqsubseteq" => "subset.eq.sq",
        "\\sqsupseteq" => "supset.eq.sq",
        "\\Subset" => "subset.double",
        "\\Supset" => "supset.double",
        // === AMS ordering relations (Phase 1a) ===
        "\\prec" => "prec",
        "\\succ" => "succ",
        "\\preceq" => "prec.eq",
        "\\succeq" => "succ.eq",
        "\\ll" => "lt.double",
        "\\gg" => "gt.double",
        "\\lll" | "\\llless" => "lt.triple",
        "\\ggg" | "\\gggtr" => "gt.triple",
        "\\doteq" => "eq.dots",
        "\\nsim" => "tilde.not",
        "\\nequiv" => "equiv.not",
        // === AMS turnstile / logic (Phase 1a) ===
        "\\vdash" => "tack.r",
        "\\dashv" => "tack.l",
        "\\Vdash" | "\\vDash" => "tack.r.double",
        "\\models" => "models",
        "\\mid" => "divides",
        "\\nmid" => "divides.not",
        "\\nparallel" | "\\nshortparallel" => "parallel.not",
        // === Long arrows (Phase 1a) ===
        "\\longrightarrow" => "arrow.r.long",
        "\\longleftarrow" => "arrow.l.long",
        "\\longleftrightarrow" => "arrow.l.r.long",
        "\\Longrightarrow" => "arrow.r.double.long",
        "\\Longleftarrow" => "arrow.l.double.long",
        "\\Longleftrightarrow" => "arrow.l.r.double.long",
        "\\longmapsto" => "arrow.r.bar.long",
        // === Harpoons and diagonal arrows (Phase 1a) ===
        "\\rightharpoonup" => "harpoon.rt",
        "\\leftharpoonup" => "harpoon.lt",
        "\\rightharpoondown" => "harpoon.rb",
        "\\leftharpoondown" => "harpoon.lb",
        "\\rightleftharpoons" | "\\rightleftarrows" => "harpoons.rtlb",
        "\\nearrow" => "arrow.tr",
        "\\searrow" => "arrow.br",
        "\\nwarrow" => "arrow.tl",
        "\\swarrow" => "arrow.bl",
        "\\Lsh" => "arrow.l.hook",
        "\\Rsh" => "arrow.r.hook",
        "\\Lleftarrow" => "arrow.l.triple",
        "\\Rrightarrow" => "arrow.r.triple",
        // === Big operators (Phase 1a) ===
        "\\bigcup" => "union.big",
        "\\bigcap" => "inter.big",
        "\\bigvee" => "or.big",
        "\\bigwedge" => "and.big",
        "\\bigoplus" => "plus.o.big",
        "\\bigotimes" => "times.o.big",
        "\\bigodot" => "dot.o.big",
        "\\coprod" => "product.co",
        // === Binary operators (Phase 1a) ===
        "\\rtimes" => "times.r",
        "\\ltimes" => "times.l",
        "\\circledast" => "ast.op.o",
        "\\circledcirc" => "compose.o",
        "\\wr" => "wreath",
        "\\uplus" => "union.plus",
        "\\sqcup" => "union.sq",
        "\\sqcap" => "inter.sq",
        // === Misc AMS symbols (Phase 1a) ===
        "\\therefore" => "therefore",
        "\\because" => "because",
        "\\complement" => "complement",
        "\\daleth" => "daleth",
        "\\backprime" => "prime.rev",
        "\\varkappa" => "kappa.alt",
        "\\digamma" => "digamma",
        // === Additional AMS relations (Phase 1a) ===
        "\\approxeq" => "approx.eq",
        "\\backsim" => "tilde.rev",
        "\\backsimeq" => "tilde.rev.eq",
        "\\eqcirc" => "eq.o",
        "\\Cap" | "\\doublecap" => "inter.double",
        "\\Cup" | "\\doublecup" => "union.double",
        "\\backepsilon" => "in.rev",
        // === Extended ordering relations (Phase 1b) ===
        "\\geqq" => "gt.equiv",
        "\\leqq" => "lt.equiv",
        "\\geqslant" => "gt.eq.slant",
        "\\leqslant" => "lt.eq.slant",
        "\\gtrsim" => "gt.tilde",
        "\\lesssim" => "lt.tilde",
        "\\gtrapprox" => "gt.approx",
        "\\lessapprox" => "lt.approx",
        "\\gtrdot" => "gt.dot",
        "\\lessdot" => "lt.dot",
        "\\gtrless" => "gt.lt",
        "\\lessgtr" => "lt.gt",
        "\\gtreqless" => "gt.eq.lt",
        "\\lesseqgtr" => "lt.eq.gt",
        // \gtreqqless / \lesseqqgtr: no Typst named symbol, defer
        // "\\gtreqqless" => "gt.equiv.lt",  // invalid
        // "\\lesseqqgtr" => "lt.equiv.gt",  // invalid
        "\\gnapprox" => "gt.napprox",
        "\\lnapprox" => "lt.napprox",
        "\\gneq" => "gt.nequiv",
        "\\lneq" => "lt.nequiv",
        // \gneqq / \lneqq: gt.nequiv.double / lt.nequiv.double do not exist in Typst 0.14.2, defer
        // "\\gneqq" => "gt.nequiv.double",
        // "\\lneqq" => "lt.nequiv.double",
        "\\gnsim" => "gt.ntilde",
        "\\lnsim" => "lt.ntilde",
        // \eqsim: eq.tilde does not exist in Typst 0.14.2, defer
        // \eqslantgtr / \eqslantless: eq.slant.gt / eq.slant.lt do not exist, defer
        // \fallingdotseq / \risingdotseq: eq.dots.fall / eq.dots.rise do not exist, defer
        "\\precapprox" => "prec.approx",
        "\\succapprox" => "succ.approx",
        "\\preccurlyeq" | "\\curlyeqprec" => "prec.curly.eq",
        "\\succcurlyeq" | "\\curlyeqsucc" => "succ.curly.eq",
        "\\precnapprox" => "prec.napprox",
        "\\succnapprox" => "succ.napprox",
        "\\precneqq" => "prec.nequiv",
        "\\succneqq" => "succ.nequiv",
        "\\precnsim" => "prec.ntilde",
        "\\succnsim" => "succ.ntilde",
        "\\precsim" => "prec.tilde",
        "\\succsim" => "succ.tilde",
        // === Triangle symbols (Phase 1b) ===
        "\\triangleleft" | "\\vartriangleleft" => "lt.tri",
        "\\triangleright" | "\\vartriangleright" => "gt.tri",
        "\\trianglelefteq" => "lt.tri.eq",
        "\\trianglerighteq" => "gt.tri.eq",
        "\\ntriangleleft" => "lt.tri.not",
        "\\ntriangleright" => "gt.tri.not",
        "\\ntrianglelefteq" => "lt.tri.eq.not",
        "\\ntrianglerighteq" => "gt.tri.eq.not",
        "\\vartriangle" => "triangle.t",
        "\\triangledown" => "triangle.b",
        "\\blacktriangle" => "triangle.filled.t",
        "\\blacktriangledown" => "triangle.filled.b",
        "\\blacktriangleleft" => "triangle.filled.l",
        "\\blacktriangleright" => "triangle.filled.r",
        // === Extended arrows (Phase 1b) ===
        "\\rightsquigarrow" | "\\leadsto" => "arrow.r.squiggly",
        // \leftrightsquigarrow: arrow.l.r.squiggly does not exist in Typst 0.14.2, defer
        "\\twoheadrightarrow" => "arrow.r.twohead",
        "\\twoheadleftarrow" => "arrow.l.twohead",
        "\\rightarrowtail" => "arrow.r.tail",
        "\\leftarrowtail" => "arrow.l.tail",
        "\\multimap" => "multimap",
        "\\upuparrows" => "arrows.tt",
        "\\downdownarrows" => "arrows.bb",
        "\\leftrightarrows" => "arrows.lr",
        "\\leftleftarrows" => "arrows.ll",
        "\\rightrightarrows" => "arrows.rr",
        "\\Updownarrow" => "arrows.tb",
        "\\looparrowleft" => "arrow.l.loop",
        "\\looparrowright" => "arrow.r.loop",
        "\\curvearrowleft" => "arrow.l.curve",
        "\\curvearrowright" => "arrow.r.curve",
        "\\dashleftarrow" => "arrow.l.dashed",
        "\\dashrightarrow" => "arrow.r.dashed",
        // === Vertical harpoons (Phase 1b) ===
        "\\upharpoonright" | "\\restriction" => "harpoon.tr",
        "\\upharpoonleft" => "harpoon.tl",
        "\\downharpoonright" => "harpoon.br",
        "\\downharpoonleft" => "harpoon.bl",
        "\\leftrightharpoons" => "harpoons.ltrb",
        // === Negated arrows (Phase 1b) ===
        "\\nleftarrow" => "arrow.l.not",
        "\\nrightarrow" => "arrow.r.not",
        "\\nleftrightarrow" => "arrow.l.r.not",
        "\\nLeftarrow" => "arrow.l.double.not",
        "\\nRightarrow" => "arrow.r.double.not",
        "\\nLeftrightarrow" => "arrow.l.r.double.not",
        // === Negated turnstile / logic (Phase 1b) ===
        "\\nvdash" => "tack.r.not",
        "\\nvDash" => "tack.r.double.not",
        "\\nVdash" => "tack.r.not.double",
        "\\nVDash" => "tack.r.double.not.double",
        "\\ncong" => "tilde.nequiv",
        "\\nprec" => "prec.not",
        "\\nsucc" => "succ.not",
        "\\npreceq" => "prec.eq.not",
        "\\nsucceq" => "succ.eq.not",
        // === Misc AMS (Phase 1b) ===
        "\\varnothing" => "emptyset",
        "\\nexists" => "exists.not",
        "\\ni" | "\\owns" => "in.rev",
        "\\smallsetminus" => "without",
        // \intercal: "intercal" is not a Typst named symbol in 0.14.2, defer
        "\\checkmark" => "checkmark",
        "\\lozenge" => "lozenge.stroked",
        "\\blacklozenge" => "lozenge.filled",
        "\\blacksquare" => "square.filled",
        "\\bigstar" => "star.filled",
        "\\yen" => "yen",
        "\\sphericalangle" => "angle.spheric",
        "\\measuredangle" => "angle.arc",
        "\\frown" | "\\smallfrown" => "frown",
        "\\smile" | "\\smallsmile" => "smile",
        "\\varpropto" => "prop",
        "\\dotplus" => "plus.dot",
        "\\divideontimes" => "times.div",
        // \veebar: or.excl does not exist in Typst 0.14.2, defer
        "\\boxminus" => "minus.square",
        "\\boxdot" => "dot.square",
        "\\circleddash" => "minus.o",
        "\\varvdots" => "dots.v",
        "\\mathellipsis" => "dots.h",
        "\\shortmid" => "divides",
        "\\shortparallel" => "parallel",
        "\\smallint" => "integral",
        "\\gets" => "arrow.l",
        "\\lhd" | "\\unlhd" => "lt.tri.eq",
        "\\rhd" | "\\unrhd" => "gt.tri.eq",
        "\\imageof" => "image",
        "\\origof" => "original",
        "\\cdotp" | "\\centerdot" => "dot.c",
        "\\circledR" => "circle.stroked.small",
        "\\circledS" => "circle.small.filled",
        "\\leftthreetimes" => "times.three.l",
        "\\rightthreetimes" => "times.three.r",
        "\\dag" | "\\textdagger" => "dagger",
        "\\ddag" | "\\textdaggerdbl" => "dagger.double",
        // \wp: "weierp" is not a Typst named symbol in 0.14.2, defer
        // Bold variants
        // `\\boldsymbol` / `\\pmb` are handled as wraps in
        // `emit_math_command`; keeping a symbol-table entry would mask
        // the wrap dispatch by returning early.
        // Common math fonts not handled by emit_math_wrap
        _ => return None,
    })
}

/// Extract the class name and option list from a `class_include` node.
/// `\documentclass[opt1,opt2]{class}` → (Some("class"), ["opt1", "opt2"]).
/// Pull `(\name, MacroDef)` out of a `new_command_definition` node.
/// AST shape (`\newcommand{\name}[N]{body}`):
///
/// ```text
/// new_command_definition
///   \newcommand                      (literal)
///   curly_group_command_name         contains `{ command_name "\\name" }`
///   brack_group_argc (optional)      contains `[ argc "N" ]`
///   brack_group (optional, skipped)  the optional-default form — unsupported
///   curly_group                      the macro body
/// ```
/// The three shapes a brace-less LaTeX argument can take. See
/// [`consume_braceless_arg`].
#[derive(Debug, Clone)]
pub(crate) enum BracelessArg {
    /// A `\command-name` (with the leading backslash). Letters-only run;
    /// for single-character escapes like `\%` or `\é` the next char is
    /// included regardless of class.
    Command(String),
    /// The inner content of a balanced `{...}` group, sans braces.
    Group(String),
    /// A single Unicode codepoint argument (letter, digit, punctuation).
    Char(String),
}

impl BracelessArg {
    /// The textual representation used as a substitution body for
    /// `\newcommand` expansion. For `Command` this is the literal
    /// `\name`; for `Group` it's the inner content; for `Char` it's the
    /// single codepoint.
    pub(crate) fn as_substitution(&self) -> &str {
        match self {
            BracelessArg::Command(s) | BracelessArg::Group(s) | BracelessArg::Char(s) => s,
        }
    }
}

/// Consume one LaTeX argument starting at byte offset `start` in `src`,
/// LaTeX-style: leading ASCII whitespace is skipped, then the next token
/// is read as either a `\command` run, a balanced `{group}`, or one
/// Unicode codepoint.
///
/// Returns `Some((arg, end_byte))` on success, where `end_byte` is the
/// byte index immediately past the consumed token. Returns `None` only
/// when `start` lies past EOF or the remaining bytes are pure whitespace
/// — the caller decides whether that's an error condition.
///
/// Used by both [`Emitter::emit_math_wrap`] (math accents like `\hat x`,
/// `\bar\alpha`, `\mathbf{X}`) and [`Emitter::expand_user_macro`] so
/// `\newcommand`s called brace-less (`\mat X`, `\rvec\alpha`) work the
/// same way LaTeX expects.
pub(crate) fn consume_braceless_arg(src: &str, start: usize) -> Option<(BracelessArg, usize)> {
    let bytes = src.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
        // `\name` — ASCII-letter run, OR single-char escape (`\%`, `\é`).
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j == i + 1 {
            // Single-char escape. Advance by codepoint length so we
            // never split a multi-byte UTF-8 sequence mid-byte.
            let after = &src[i + 1..];
            let step = after.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            j = i + 1 + step;
        }
        return Some((BracelessArg::Command(src[i..j].to_string()), j));
    }
    if bytes[i] == b'{' {
        // Balanced `{...}` group; depth-track, ignore `\{` and `\}`.
        let inner_start = i + 1;
        let mut depth = 1i32;
        let mut j = inner_start;
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
        if j >= bytes.len() {
            // Unbalanced — fail closed so the caller can warn.
            return None;
        }
        return Some((
            BracelessArg::Group(src[inner_start..j].to_string()),
            j + 1,
        ));
    }
    // Single Unicode codepoint.
    let rest = &src[i..];
    let c = rest.chars().next()?;
    let end = i + c.len_utf8();
    Some((BracelessArg::Char(c.to_string()), end))
}

/// Substitute `#1`..`#N` placeholders in a `\newcommand` body. Walks
/// the body character-by-character so `#10` doesn't accidentally match
/// `#1`+`0` and an unmatched `#<digit>` (outside the param range) is
/// passed through unchanged.
fn substitute_macro_args(body: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' {
            // Consume a run of digits and look up the parameter index.
            let mut digits = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    digits.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if digits.is_empty() {
                out.push('#');
            } else if let Ok(idx) = digits.parse::<usize>() {
                // `\newcommand` parameters are 1-indexed.
                if idx >= 1 && idx <= args.len() {
                    out.push_str(&args[idx - 1]);
                } else {
                    // No matching arg — keep the placeholder verbatim.
                    out.push('#');
                    out.push_str(&digits);
                }
            } else {
                out.push('#');
                out.push_str(&digits);
            }
        } else {
            out.push(c);
        }
    }
    out
}

///
/// Returns `None` when the new command has an optional-default
/// argument (the form `\newcommand{\name}[1][default]{body}`) — we
/// don't model defaults yet, so let those fall through to silent drop.
fn extract_newcommand(node: Node<'_>, src: &str) -> Option<(String, MacroDef)> {
    let mut cursor = node.walk();
    let mut name: Option<String> = None;
    let mut params: usize = 0;
    let mut body_group: Option<Node<'_>> = None;
    let mut brack_groups: usize = 0;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "curly_group_command_name" => {
                let mut sub = child.walk();
                for gc in child.children(&mut sub) {
                    if gc.kind() == "command_name" {
                        name = Some(src[gc.start_byte()..gc.end_byte()].to_string());
                    }
                }
            }
            "brack_group_argc" => {
                let mut sub = child.walk();
                for gc in child.children(&mut sub) {
                    if gc.kind() == "argc" {
                        if let Ok(n) = src[gc.start_byte()..gc.end_byte()].parse::<usize>() {
                            params = n;
                        }
                    }
                }
            }
            "brack_group" => {
                // Optional-default form — bail. We'd need to honour the
                // default in the placeholder substitution; defer.
                brack_groups += 1;
            }
            "curly_group" if body_group.is_none() => {
                body_group = Some(child);
            }
            _ => {}
        }
    }
    if brack_groups > 0 {
        return None;
    }
    let name = name?;
    let body_node = body_group?;
    // Strip the outer `{` and `}` to leave the body source.
    let body = src
        .get(body_node.start_byte() + 1..body_node.end_byte() - 1)
        .unwrap_or("")
        .to_string();
    Some((name, MacroDef { params, body }))
}

/// Return the kind-string of the first child of a `new_command_definition` node
/// whose kind starts with `\` (e.g. `"\\newcommand"`, `"\\renewcommand"`,
/// `"\\DeclareMathOperator"`). Returns `None` if no such child exists.
///
/// We copy the kind into an owned `String` to avoid keeping a `TreeCursor`
/// alive across caller logic, which would trigger borrow-checker errors.
fn new_command_token_kind(node: Node<'_>) -> Option<String> {
    let mut cursor = node.walk();
    let mut result = None;
    for child in node.children(&mut cursor) {
        if child.kind().starts_with('\\') {
            result = Some(child.kind().to_string());
            break;
        }
    }
    result
}

/// Extract the macro name and body from a `\DeclareMathOperator` node that
/// tree-sitter has classified as `new_command_definition`.
///
/// The node structure is:
/// ```text
/// new_command_definition
///   \DeclareMathOperator          (token)
///   curly_group_command_name      contains { command_name "\\name" }
///   curly_group                   the display text, e.g. "{sinc}"
/// ```
///
/// Returns `(macro_name, MacroDef)` where the body is `\operatorname{display}`
/// (or `\operatorname*{display}` for the starred form).
fn extract_declare_math_operator_from_newcmd(
    node: Node<'_>,
    src: &str,
    starred: bool,
) -> Option<(String, MacroDef)> {
    let mut cursor = node.walk();
    let mut name: Option<String> = None;
    let mut display: Option<String> = None;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "curly_group_command_name" => {
                let mut inner = child.walk();
                for c in child.children(&mut inner) {
                    if c.kind() == "command_name" {
                        name = Some(src[c.start_byte()..c.end_byte()].to_string());
                    }
                }
            }
            "curly_group" => {
                // The display text group (e.g. "{sinc}")
                if display.is_none() {
                    let body_src = &src[child.start_byte()..child.end_byte()];
                    display = Some(if body_src.starts_with('{') && body_src.ends_with('}') {
                        body_src[1..body_src.len() - 1].to_string()
                    } else {
                        body_src.to_string()
                    });
                }
            }
            _ => {}
        }
    }
    let name = name?;
    let display = display?;
    let body = if starred {
        format!(r"\operatorname*{{{}}}", display)
    } else {
        format!(r"\operatorname{{{}}}", display)
    };
    Some((name, MacroDef { params: 0, body }))
}

/// Maps the well-known math wrap commands to their Typst `(left, right)`
/// delimiter pair. Used by the bare `command_name` branch of
/// `emit_node` to recover the brace-less form (e.g. `_\mathcal{T}` —
/// tree-sitter parses the `{T}` as a sibling of the enclosing
/// subscript, so the command_name itself reaches us without a child).
/// Decide whether a Typst math-mode subscript/superscript argument
/// needs an explicit `(...)` wrapper. A single token (one letter or
/// digit, optionally with one trailing `prime`-style suffix) parses
/// correctly as `_x` / `^x`; anything more compound (function call,
/// space-separated tokens, multi-char identifier) needs `_(...)`
/// because `_cal(T)` reads as `_c · al(T)`.
/// Escape the handful of ASCII characters that have special meaning
/// inside a Typst `[...]` content block but commonly appear unescaped
/// in table cells / caption text: `_` opens italic, `*` opens bold,
/// `#` opens code context, `<` opens labels, `@` opens references.
/// Already-escaped `\_` / `\*` / etc. are left alone so this is
/// idempotent. We don't touch `[`, `]`, `{`, `}` here — the caller
/// already balances those, and over-escaping breaks the surrounding
/// content block.
fn escape_text_cell(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Preserve any backslash-escape verbatim — `\_` etc.
                out.push('\\');
                if let Some(&next) = chars.peek() {
                    out.push(next);
                    chars.next();
                }
            }
            '_' | '*' | '#' | '@' | '<' | '`' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn needs_subscript_parens(rendered: &str) -> bool {
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

/// Harvest a `\def\name<params>{body}` definition by scanning raw
/// source bytes from the end of the `old_command_definition` node
/// forward. Tree-sitter packages only `\def\name` as the node; the
/// `#1` placeholders and the body `{...}` are emitted as siblings,
/// so we have to find them ourselves.
///
/// Returns the byte offset just past the closing `}` of the body
/// (callers set `skip_until` here so the body bytes aren't re-emitted
/// as raw text). Returns `None` when the syntax can't be parsed —
/// the caller falls back to drop-without-harvest in that case.
fn extract_def_and_record(
    node: Node<'_>,
    src: &str,
    macros: &mut HashMap<String, MacroDef>,
) -> Option<usize> {
    // Pull the `\name` from the command_name child.
    let mut cursor = node.walk();
    let name = node
        .children(&mut cursor)
        .find(|c| c.kind() == "command_name")
        .map(|c| src[c.start_byte()..c.end_byte()].to_string())?;
    let bytes = src.as_bytes();
    let mut i = node.end_byte();
    // Count `#1`..`#9` placeholders before the body.
    let mut params: usize = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i + 1 < bytes.len() && bytes[i] == b'#' && bytes[i + 1].is_ascii_digit() {
            let n = (bytes[i + 1] - b'0') as usize;
            if n > params {
                params = n;
            }
            i += 2;
        } else {
            break;
        }
    }
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if bytes.get(i) != Some(&b'{') {
        return None;
    }
    // Balance braces to find the body's closing `}`.
    let inner_start = i + 1;
    let mut depth = 1i32;
    let mut j = inner_start;
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
                    let body = src[inner_start..j].to_string();
                    macros.insert(name, MacroDef { params, body });
                    return Some(j + 1);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

pub(crate) fn wrap_for_command_name(name: &str) -> Option<(&'static str, &'static str)> {
    Some(match name {
        "\\mathbb" | "\\mathbbm" | "\\Bbb" => ("bb(", ")"),
        "\\mathcal" => ("cal(", ")"),
        "\\mathfrak" | "\\frak" => ("frak(", ")"),
        "\\mathscr" => ("scr(", ")"),
        "\\mathsf" => ("sans(", ")"),
        "\\mathit" => ("italic(", ")"),
        "\\mathtt" => ("mono(", ")"),
        "\\mathbf" | "\\bm" | "\\bs" | "\\boldsymbol" | "\\pmb" | "\\bold" => ("bold(", ")"),
        "\\bar" | "\\overline" => ("overline(", ")"),
        "\\underline" => ("underline(", ")"),
        "\\hat" | "\\widehat" => ("hat(", ")"),
        "\\tilde" | "\\widetilde" => ("tilde(", ")"),
        "\\vec" | "\\overrightarrow" | "\\Overrightarrow" => ("arrow(", ")"),
        "\\dot" => ("dot(", ")"),
        "\\ddot" => ("dot.double(", ")"),
        "\\acute" => ("acute(", ")"),
        "\\grave" => ("grave(", ")"),
        "\\check" | "\\widecheck" => ("caron(", ")"),
        "\\breve" => ("breve(", ")"),
        "\\mathring" => ("circle(", ")"),
        "\\phantom" => ("hide(", ")"),
        "\\overbrace" => ("overbrace(", ")"),
        "\\underbrace" => ("underbrace(", ")"),
        "\\cancel" => ("cancel(", ")"),
        _ => return None,
    })
}

fn extract_class_and_options(node: Node<'_>, src: &str) -> (Option<String>, Vec<String>) {
    let mut class: Option<String> = None;
    let mut opts: Vec<String> = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "curly_group_path" | "curly_group_path_list" => {
                let mut sub = child.walk();
                for gc in child.children(&mut sub) {
                    if gc.kind() == "path" {
                        class = Some(src[gc.start_byte()..gc.end_byte()].to_string());
                    }
                }
            }
            "brack_group_key_value" => {
                let mut sub = child.walk();
                for gc in child.children(&mut sub) {
                    if gc.kind() == "key_value_pair" {
                        let mut kv_cursor = gc.walk();
                        let mut key_buf = String::new();
                        for kc in gc.children(&mut kv_cursor) {
                            if kc.kind() == "=" {
                                break;
                            }
                            key_buf.push_str(&src[kc.start_byte()..kc.end_byte()]);
                        }
                        let k = key_buf.trim().to_string();
                        if !k.is_empty() {
                            opts.push(k);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (class, opts)
}

/// Extract the file path argument from `\input{...}` / `\include{...}` /
/// `\subfile{...}`. Both the dedicated `latex_include` node kind and the
/// generic-command variant share the same `curly_group_path > path`
/// substructure, so a single helper covers both call sites.
fn extract_latex_include_path(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "curly_group_path" | "curly_group") {
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "path" {
                    return Some(src[grandchild.start_byte()..grandchild.end_byte()].to_string());
                }
            }
            // Fallback: strip the literal braces. Covers shapes where the
            // grammar tagged the curly contents as a generic node.
            let raw = &src[child.start_byte()..child.end_byte()];
            let inner = raw
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if let Some(s) = inner {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Resolve an `\input{rel}` style path against `base`. LaTeX accepts both
/// `\input{foo}` (no extension; the `.tex` is implicit) and `\input{foo.tex}`
/// — try the literal first, then the `.tex`-appended form.
/// Probe the base directory for an image asset with the given stem/path.
/// Tries the path as-is first; if it has no extension, probes common formats.
/// Returns the resolved path on disk, or `None` if nothing is found.
fn probe_image_on_disk(base: &Path, path: &str) -> Option<PathBuf> {
    let direct = base.join(path);
    if direct.is_file() {
        return Some(direct);
    }
    if std::path::Path::new(path).extension().is_none() {
        for ext in &["pdf", "png", "jpg", "jpeg", "svg", "gif"] {
            let candidate = base.join(format!("{}.{}", path, ext));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Probe the base directory for a BibTeX file. Appends `.bib` if the stem has
/// no extension. Returns the resolved path on disk, or `None`.
fn probe_bib_on_disk(base: &Path, path: &str) -> Option<PathBuf> {
    let direct = base.join(path);
    if direct.is_file() {
        return Some(direct);
    }
    if !path.contains('.') {
        let with_ext = base.join(format!("{}.bib", path));
        if with_ext.is_file() {
            return Some(with_ext);
        }
    }
    None
}

fn resolve_input_path(base: &Path, raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let direct = base.join(raw);
    if direct.is_file() {
        return Some(direct);
    }
    if !raw.ends_with(".tex") {
        let with_ext = base.join(format!("{}.tex", raw));
        if with_ext.is_file() {
            return Some(with_ext);
        }
    }
    None
}

/// Resolve a `\usepackage{X}` reference to a local `X.sty` or `X.cls`
/// file. Probes the base directory and common style subdirectories
/// (`style/`, `macros/`, `tex/`, `sty/`). Returns `None` when the
/// package is a system package (no local file), in which case the
/// caller falls back to the no-op allowlist / warn-and-drop path.
fn resolve_package_path(base: &Path, pkg: &str) -> Option<PathBuf> {
    let pkg = pkg.trim();
    if pkg.is_empty() {
        return None;
    }
    let candidates = ["", "style/", "macros/", "tex/", "sty/"];
    for sub in &candidates {
        for ext in &[".sty", ".cls"] {
            let p = base.join(format!("{}{}{}", sub, pkg, ext));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Extract the package name from `\usepackage[opts]{name}`. The container is
/// `curly_group_path` for single-package form and `curly_group_path_list`
/// when options are present — accept either.
fn extract_package_name(node: Node<'_>, src: &str) -> Option<String> {
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

/// LaTeX packages that don't need translation — either their behavior is the
/// Typst default, or they only affect rendering (font, color, layout) and
/// have no semantic impact on the converted document.
fn is_known_noop_package(name: &str) -> bool {
    matches!(
        name,
        // Math / fonts
        "amsmath" | "amssymb" | "amsfonts" | "amsthm" | "amsopn"
        | "mathtools" | "mathrsfs" | "dsfont" | "stmaryrd" | "bm" | "bbm"
        | "physics"
        | "accents" | "nicefrac" | "siunitx" | "rsfso"
        // Graphics / color / layout
        | "graphicx" | "graphics" | "xcolor" | "color" | "tikz"
        | "geometry" | "microtype" | "fancyhdr" | "setspace" | "indentfirst"
        | "adjustbox" | "float" | "wrapfig" | "placeins" | "subfigure" | "subcaption"
        | "lineno" | "rotating" | "subfig"
        // Tables
        | "booktabs" | "array" | "tabularx" | "longtable" | "arydshln"
        | "colortbl" | "multirow" | "makecell"
        // Encoding / fonts
        | "inputenc" | "fontenc" | "lmodern" | "times" | "helvet" | "courier"
        | "mathptmx" | "newtxtext" | "newtxmath" | "fontspec" | "babel"
        | "T1" | "utf8"
        // Bibliography / refs
        | "cite" | "natbib" | "biblatex" | "hyperref" | "url"
        | "cleveref" | "varioref" | "nameref" | "backref" | "footmisc"
        // Verb / code
        | "verbatim" | "fancyvrb" | "listings" | "minted" | "ulem"
        // Algorithms
        | "algorithm" | "algorithmic" | "algorithmicx" | "algpseudocode"
        // Misc utilities
        | "enumitem" | "etoolbox" | "xparse" | "ifthen" | "ifpdf" | "iftex"
        | "textcomp" | "lipsum" | "blindtext" | "authblk" | "caption"
        | "tcolorbox" | "framed" | "mdframed" | "epstopdf" | "pgf" | "pgfplots"
        | "comment" | "xspace" | "pifont" | "xurl" | "xr" | "xr-hyper"
        | "xfrac" | "type1cm" | "titlesec" | "soul" | "multicol"
        | "makeidx" | "dirtytalk" | "changepage" | "afterpage" | "ragged2e"
        | "xstring" | "calc" | "currfile" | "kvoptions" | "fp"
        // Conference/journal style files commonly preloaded by templates.
        | "neurips_2022" | "neurips_2023" | "neurips_2024" | "neurips_2025"
        | "neurips_2026" | "iclr2024_conference" | "iclr2025_conference"
        | "icml2024" | "icml2025" | "icml2026"
        | "acmart" | "IEEEtran" | "spconf"
    )
}

/// Map a LaTeX `\bibliographystyle{X}` name to the nearest Typst built-in
/// style. Returns `None` for unknown styles so the caller can omit the
/// `style:` argument and let Typst use its default.
fn map_bibliography_style(latex: &str) -> Option<&'static str> {
    // Typst's `alphanumeric` is for inline citation labels only and rejects
    // bibliography lists. We pick `ieee` as the safest default for the
    // numeric/order-of-appearance LaTeX styles since most academic templates
    // use a numeric bibliography. Author-year variants map to APA.
    match latex {
        "plain" | "alpha" | "abbrv" | "unsrt" => Some("ieee"),
        "plainnat"
        | "abbrvnat"
        | "unsrtnat"
        | "apa"
        | "apalike"
        | "apacite"
        | "chicago"
        | "chicagoa"
        | "chicago-author-date" => Some("apa"),
        "ieee" | "ieeetr" | "IEEEtran" => Some("ieee"),
        "mla" => Some("mla"),
        "ACM-Reference-Format" | "acm" | "acmauthoryear" | "acmnumeric" => Some("ieee"),
        _ => None,
    }
}

/// Decide whether a word inside math should be split into single characters.
/// LaTeX semantics: consecutive letters are implicit products (`mc` = m·c).
/// Typst semantics: consecutive letters form an identifier (`mc` = variable mc).
/// We split iff the word is more than one ASCII letter long and is not a
/// recognized math function name.
fn should_split_math_word(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    if !bytes.iter().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    if is_math_function_name(s) {
        return false;
    }
    true
}

/// Common LaTeX math functions that Typst also renders as upright identifiers.
/// Words matching these don't get character-split.
fn is_math_function_name(s: &str) -> bool {
    matches!(
        s,
        "sin"
            | "cos"
            | "tan"
            | "cot"
            | "sec"
            | "csc"
            | "arcsin"
            | "arccos"
            | "arctan"
            | "sinh"
            | "cosh"
            | "tanh"
            | "log"
            | "ln"
            | "exp"
            | "min"
            | "max"
            | "inf"
            | "sup"
            | "lim"
            | "det"
            | "arg"
            | "deg"
            | "dim"
            | "gcd"
            | "hom"
            | "ker"
            | "lg"
            | "mod"
            | "Pr"
            | "Re"
            | "Im"
            | "argmin"
            | "argmax"
            | "limsup"
            | "liminf"
            | "var"
            | "cov"
    )
}

/// Replace LaTeX typographic conventions with their Typst equivalents:
/// - `---` → `—` (em-dash)
/// - `--` → `–` (en-dash)
/// - ` `` `…`'' ` → `"…"` (LaTeX-style double quotes become ASCII doubles,
///   which Typst auto-smart-quotes)
///
/// Single-character contexts inside ``backticked raw blocks'' would normally
/// be preserved, but in our text-mode output the only `\`` we emit comes from
/// `\texttt{...}` — those wrappers are short and don't typically contain
/// `--`/`---`/``''. We accept the small risk in v0.2 and revisit if a real
/// template triggers it.
fn post_process_typography(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut prev: Option<char> = None;
    while let Some(c) = chars.next() {
        match c {
            '`' if chars.peek() == Some(&'`') => {
                chars.next();
                out.push('"');
                prev = Some('"');
            }
            // Lone backtick — LaTeX uses it as a left single quote (`'X'`)
            // and authors sometimes paste markdown-style code spans into
            // the source. Either way, Typst reads `` ` `` as the opener of
            // a raw block and fails with "unclosed raw text". Escape it so
            // it renders as a literal backtick. `\texttt{X}` no longer emits
            // backticks (it uses `#raw(...)` instead), so this pass only
            // ever sees backticks that came from the source.
            '`' => {
                out.push_str("\\`");
                prev = Some('`');
            }
            '\'' if chars.peek() == Some(&'\'') => {
                chars.next();
                out.push('"');
                prev = Some('"');
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                if chars.peek() == Some(&'-') {
                    chars.next();
                    out.push('\u{2014}');
                    prev = Some('\u{2014}');
                } else {
                    out.push('\u{2013}');
                    prev = Some('\u{2013}');
                }
            }
            // `@` is Typst's reference operator. When the previous emitted
            // character is alphanumeric, the `@` is clearly mid-word (email
            // address, twitter handle, etc.) and must be escaped to keep
            // Typst from parsing it as `@label`.
            '@' if prev.is_some_and(|p| p.is_ascii_alphanumeric()) => {
                out.push_str("\\@");
                prev = Some('@');
            }
            other => {
                out.push(other);
                prev = Some(other);
            }
        }
    }
    out
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
    extract_label_name_and_end(node, src).map(|(n, _)| n)
}

/// Same as `extract_label_name`, but also returns the byte offset
/// immediately past the closing `}` of the label argument. The caller
/// uses that offset to set `skip_until` so the leaked tail (when
/// tree-sitter truncates the label at `_`) isn't re-emitted as
/// stray math content.
fn extract_label_name_and_end(node: Node<'_>, src: &str) -> Option<(String, usize)> {
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
fn normalize_label_key(raw: &str) -> String {
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
    out
}
