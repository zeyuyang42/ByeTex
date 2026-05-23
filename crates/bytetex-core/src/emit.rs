//! Emitter — walks the tree-sitter AST and produces Typst source plus warnings.
//!
//! ## Scope
//!
//! - M1: plain text passthrough, `%`-comment dropping, generic warning for any
//!   unrecognised backslash command.
//! - M2: sectioning (`\section`..`\subparagraph`, starred forms, attached labels).
//!   Inline formatting + lists come in subsequent M2 sub-tasks; this file is
//!   structured around a dispatch-by-kind pattern so each batch is additive.

use std::collections::HashSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::class_map::DocClass;
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
    /// Title-block accumulators. `\title{X}`, `\author{X}`, `\date{X}` store
    /// rendered content here; `\maketitle` flushes them into a centered block
    /// at the document head. If `\maketitle` never appears but `pending_title`
    /// is set, the block is flushed in `finish()`.
    pending_title: Option<String>,
    pending_authors: Vec<String>,
    pending_date: Option<String>,
    pending_abstract: Option<String>,
    pending_keywords: Option<String>,
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
}

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
            pending_title: None,
            pending_authors: Vec::new(),
            pending_date: None,
            pending_abstract: None,
            pending_keywords: None,
            detected_class: DocClass::Unknown,
            base_dir,
            visited_includes: visited,
        }
    }

    pub(crate) fn emit_root(&mut self, root: Node<'_>) {
        let _ = self.emit_node(root);
    }

    pub(crate) fn finish(mut self) -> (String, Vec<Warning>) {
        // If `\documentclass` mapped to a known Typst Universe template,
        // prepend the `#import` + `#show:` pair so the converted PDF gets
        // that class's full visual identity (columns, font, headings, title
        // block). Otherwise fall back to the hand-rolled centered title.
        let template_preamble = self.build_template_preamble();
        if let Some(p) = template_preamble {
            let body = std::mem::take(&mut self.out);
            self.out.push_str(&p);
            self.out.push_str(&body);
        } else if self.pending_title.is_some() || !self.pending_authors.is_empty() {
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
            if should_split_math_word(text) {
                let mut first = true;
                for c in text.chars() {
                    if !first {
                        self.out.push(' ');
                    }
                    self.out.push(c);
                    first = false;
                }
                return node.end_byte();
            }
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
            // Orphan `\label{X}` outside any section/equation/figure — emit
            // the Typst label syntax so subsequent `@X` references resolve.
            "label_definition" => {
                if let Some(key) = extract_label_name(node, self.src) {
                    let _ = write!(self.out, " <{}>", key);
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
                    suggested_skill: Some("bytetex-unsupported-environment".to_string()),
                });
                return node.end_byte();
            }
            "title_declaration" => {
                if let Some(arg) = first_curly_group(node) {
                    self.pending_title = Some(self.render_curly_group_content(arg));
                }
                return node.end_byte();
            }
            "author_declaration" => {
                // The grammar uses `curly_group_author_list` for the author arg.
                if let Some(arg) = first_curly_like(node) {
                    let rendered = self.render_curly_group_content(arg);
                    self.pending_authors.push(rendered);
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

        // Macro / theorem / counter definitions — drop silently. We don't
        // expand `\newcommand` bodies (v0.2 non-goal) but the *definition*
        // itself shouldn't show up as a warning every time. `theorem_definition`
        // covers `\newtheorem` / `\newtheorem*` / `\declaretheorem(*)` (the
        // tree-sitter-latex grammar dedicates a node kind to them) — leaving
        // it unhandled previously emitted the source verbatim and broke the
        // compile with a backslash-in-code error.
        if matches!(
            node.kind(),
            "new_command_definition" | "counter_declaration" | "theorem_definition"
        ) {
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
            // Text-mode super/subscript wrappers.
            Some("\\textsuperscript") => self.emit_inline_wrap(node, "#super[", "]"),
            Some("\\textsubscript") => self.emit_inline_wrap(node, "#sub[", "]"),
            // Spacing primitives with no Typst equivalent — drop silently.
            Some("\\kern")
            | Some("\\vspace")
            | Some("\\hspace")
            | Some("\\vspace*")
            | Some("\\hspace*")
            | Some("\\smallskip")
            | Some("\\medskip")
            | Some("\\bigskip")
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
            // ACM-specific copyright / metadata; drop silently.
            Some("\\setcopyright")
            | Some("\\copyrightyear")
            | Some("\\acmYear")
            | Some("\\acmConference")
            | Some("\\acmBooktitle")
            | Some("\\acmDOI")
            | Some("\\acmISBN")
            | Some("\\acmPrice")
            | Some("\\acmSubmissionID")
            | Some("\\affiliation")
            | Some("\\institution")
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
            | Some("\\shortauthors") => node.end_byte(),
            // `\keywords{a, b, c}` and `\IEEEkeywords{...}` — capture into the
            // title-block field when the class template wants it; otherwise
            // silently drop.
            Some("\\keywords") | Some("\\IEEEkeywords") => {
                if self.detected_class.import_line().is_some() {
                    if let Some(arg) = first_curly_like(node) {
                        let rendered = self.render_curly_group_content(arg);
                        self.pending_keywords = Some(rendered);
                    }
                }
                node.end_byte()
            }
            // IEEEtran-specific.
            Some("\\IEEEoverridecommandlockouts")
            | Some("\\IEEEpubid")
            | Some("\\IEEEauthorrefmark")
            | Some("\\IEEEcompsoctitleabstractindextext")
            | Some("\\IEEEcompsocthanksitem") => node.end_byte(),
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
            // Other TeX-isms that should drop silently:
            //  \/  italic correction
            //  \-  discretionary hyphen
            //  \@  spacing tweak before sentence-ending period
            //  \~  tilde accent (would translate the following letter; rare)
            //  \'  \"  \^  accent-on-next-letter; complex; drop for now
            Some("\\/") | Some("\\-") | Some("\\@") | Some("\\~") | Some("\\'") | Some("\\\"")
            | Some("\\^") | Some("\\`") => node.end_byte(),
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
                    self.pending_title = Some(self.render_curly_group_content(arg));
                }
                node.end_byte()
            }
            Some("\\author") => {
                if let Some(arg) = first_curly_group(node) {
                    let rendered = self.render_curly_group_content(arg);
                    self.pending_authors.push(rendered);
                }
                node.end_byte()
            }
            Some("\\date") => {
                if let Some(arg) = first_curly_group(node) {
                    self.pending_date = Some(self.render_curly_group_content(arg));
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
            // Font-size directives — drop silently. They're scoped commands
            // (no argument) that change subsequent text style; Typst's
            // equivalent would be a #set text(size: ...) wrapper, but that
            // needs proper grouping tracking we don't yet have.
            Some("\\small")
            | Some("\\large")
            | Some("\\Large")
            | Some("\\LARGE")
            | Some("\\huge")
            | Some("\\Huge")
            | Some("\\normalsize")
            | Some("\\footnotesize")
            | Some("\\scriptsize")
            | Some("\\tiny") => node.end_byte(),
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
            // `\DeclareMathOperator{\name}{display}` — macro definition for a
            // new math operator. We don't yet expand custom macros; drop the
            // definition silently and let any uses of `\name` warn as usual.
            Some("\\DeclareMathOperator") | Some("\\DeclareMathOperator*") => node.end_byte(),
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
                    suggested_skill: Some("bytetex-unsupported-environment".to_string()),
                });
                node.end_byte()
            }
            _ => {
                self.warn_unsupported_command(node);
                node.end_byte()
            }
        }
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
                    suggested_skill: Some("bytetex-unsupported-environment".to_string()),
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
                    suggested_skill: Some("bytetex-unsupported-environment".to_string()),
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
        self.needs_heading_numbering |= sub.needs_heading_numbering;
        self.needs_equation_numbering |= sub.needs_equation_numbering;
        if self.pending_title.is_none() {
            self.pending_title = sub.pending_title.take();
        }
        if self.pending_authors.is_empty() {
            self.pending_authors.append(&mut sub.pending_authors);
        }
        if self.pending_date.is_none() {
            self.pending_date = sub.pending_date.take();
        }
        if self.pending_abstract.is_none() {
            self.pending_abstract = sub.pending_abstract.take();
        }
        if self.pending_keywords.is_none() {
            self.pending_keywords = sub.pending_keywords.take();
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
        if self.pending_title.is_none() && self.pending_authors.is_empty() {
            return;
        }
        self.ensure_paragraph_break();
        self.out.push_str("#align(center)[\n");
        if let Some(title) = self.pending_title.take() {
            let _ = writeln!(
                self.out,
                "  #text(size: 1.5em, weight: \"bold\")[{}]",
                title
            );
        }
        if !self.pending_authors.is_empty() {
            self.out.push_str("  #v(0.6em)\n  ");
            let authors = std::mem::take(&mut self.pending_authors);
            self.out.push_str(&authors.join(", "));
            self.out.push('\n');
        }
        if let Some(date) = self.pending_date.take() {
            let _ = write!(self.out, "  #v(0.4em)\n  {}\n", date);
        }
        self.out.push_str("]\n\n");
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
                if self.detected_class.wants_abstract_field() && self.pending_abstract.is_none() {
                    let body = self.render_env_body_to_string(node);
                    self.pending_abstract = Some(body.trim().to_string());
                    node.end_byte()
                } else {
                    self.emit_environment_body(node)
                }
            }
            // IEEEtran's keywords env. Same capture-or-drop dance as abstract.
            Some("IEEEkeywords") => {
                if self.detected_class.import_line().is_some() && self.pending_keywords.is_none() {
                    let body = self.render_env_body_to_string(node);
                    self.pending_keywords = Some(body.trim().to_string());
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
        let title = self.pending_title.take().unwrap_or_default();
        let authors = std::mem::take(&mut self.pending_authors);
        let abstract_ = self.pending_abstract.take().unwrap_or_default();
        let keywords = self.pending_keywords.take().unwrap_or_default();
        let show_call = self
            .detected_class
            .show_call(&title, &authors, &abstract_, &keywords)?;
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
    fn push_math_symbol(&mut self, typst: &str) {
        if typst.is_empty() {
            return;
        }
        self.ensure_math_letter_boundary(typst);
        self.out.push_str(typst);
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
        let body_len = self.out.len() - body_start;
        let escaped = escape_unbalanced_math_brackets(&self.out[body_start..]);
        if escaped.len() != body_len {
            self.out.truncate(body_start);
            self.out.push_str(&escaped);
        }
    }

    fn emit_inline_math(&mut self, node: Node<'_>) -> usize {
        self.out.push('$');
        let body_start = self.out.len();
        let was = self.in_math;
        self.in_math = true;
        self.emit_math_children(node);
        self.in_math = was;
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
        // the closing `$` follows directly after the content.
        while self.out.ends_with(' ') || self.out.ends_with('\n') {
            self.out.pop();
        }
        self.balance_math_brackets(body_start);
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
        while self.out.ends_with(' ') || self.out.ends_with('\n') {
            self.out.pop();
        }
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
            "\\frac" => self.emit_math_frac(node),
            "\\sqrt" => self.emit_math_sqrt(node),
            "\\binom" => self.emit_math_binom(node),
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
            "\\mathbf" | "\\bm" | "\\bs" => self.emit_math_wrap(node, "bold(", ")"),
            "\\mathbb" | "\\mathbbm" => self.emit_math_wrap(node, "bb(", ")"),
            "\\mathcal" => self.emit_math_wrap(node, "cal(", ")"),
            "\\mathfrak" => self.emit_math_wrap(node, "frak(", ")"),
            "\\mathsf" => self.emit_math_wrap(node, "sans(", ")"),
            "\\mathit" => self.emit_math_wrap(node, "italic(", ")"),
            "\\mathtt" => self.emit_math_wrap(node, "mono(", ")"),
            // Math accents
            "\\bar" | "\\overline" => self.emit_math_wrap(node, "overline(", ")"),
            "\\underline" => self.emit_math_wrap(node, "underline(", ")"),
            "\\hat" | "\\widehat" => self.emit_math_wrap(node, "hat(", ")"),
            "\\tilde" | "\\widetilde" => self.emit_math_wrap(node, "tilde(", ")"),
            "\\vec" => self.emit_math_wrap(node, "arrow(", ")"),
            "\\dot" => self.emit_math_wrap(node, "dot(", ")"),
            "\\ddot" => self.emit_math_wrap(node, "dot.double(", ")"),
            "\\acute" => self.emit_math_wrap(node, "acute(", ")"),
            "\\grave" => self.emit_math_wrap(node, "grave(", ")"),
            "\\check" => self.emit_math_wrap(node, "caron(", ")"),
            "\\breve" => self.emit_math_wrap(node, "breve(", ")"),
            // `\operatorname{name}` → `op("name")` — upright math text.
            "\\operatorname" => self.emit_math_operatorname(node),
            // Math-mode spacing primitives — drop silently.
            "\\hspace" | "\\vspace" | "\\!" | "\\linebreak" | "\\nobreak" => node.end_byte(),
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
        } else {
            self.warn_ambiguous_math(node, "missing argument");
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
        let mapped = style.as_deref().and_then(map_bibliography_style);
        if let Some(s) = mapped {
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
                    let _ = write!(self.out, "[{}]", cell);
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
        "\\partial" => "partial",
        "\\nabla" => "nabla",
        "\\hbar" => "planck.reduce",
        "\\ell" => "ell",
        "\\dots" | "\\ldots" => "dots.h",
        "\\cdots" => "dots.c",
        "\\vdots" => "dots.v",
        "\\ddots" => "dots.down",
        "\\angle" => "angle",
        "\\degree" => "degree",
        "\\dagger" => "dagger",
        "\\ddagger" => "dagger.double",
        "\\prime" => "prime",
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
        // Math spacing
        "\\qquad" => "quad quad",
        "\\quad" => "quad",
        // Delimiter-size commands (Typst auto-sizes via `lr(...)`); drop.
        "\\big" | "\\Big" | "\\bigg" | "\\Bigg" | "\\bigl" | "\\Bigl" | "\\biggl" | "\\Biggl"
        | "\\bigr" | "\\Bigr" | "\\biggr" | "\\Biggr" | "\\bigm" | "\\Bigm" | "\\biggm"
        | "\\Biggm" => "",
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
        // Bold variants
        "\\boldsymbol" | "\\pmb" => "bold",
        // Common math fonts not handled by emit_math_wrap
        _ => return None,
    })
}

/// Extract the class name and option list from a `class_include` node.
/// `\documentclass[opt1,opt2]{class}` → (Some("class"), ["opt1", "opt2"]).
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
