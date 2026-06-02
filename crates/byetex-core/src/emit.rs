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

mod boundary;
mod escape;
pub(crate) use escape::{escape_text_for_typst_content, needs_text_escape, is_typst_label_char, sanitize_label_key, escape_paren_semicolons, escape_unbalanced_math_brackets, strip_trailing_typst_label, escape_text_cell};

/// A `\newcommand` definition harvested from the input. `body` is the
/// raw LaTeX source between the outer curly braces; expansion inlines
/// the body at every call site, substituting `#1` / `#2` / … with the
/// raw source of the call's curly_group arguments before re-parsing.
///
/// `optional_defaults` models LaTeX2e `\newcommand\foo[N][default]` and
/// the `xargspec` package's `\newcommandx\foo[N][K=default]` form. The
/// map is keyed by 1-indexed position: position `K` is optional with
/// the given default string substituted when the call site omits the
/// `[arg]`. Empty map means all `params` positions are mandatory.
#[derive(Debug, Clone, Default)]
pub(crate) struct MacroDef {
    /// Number of `#N` parameters expected. Zero for no-arg macros.
    pub params: usize,
    /// Raw LaTeX body, brace-stripped.
    pub body: String,
    /// Position -> default-value source. Positions in this map are
    /// optional at the call site; absent positions are mandatory.
    pub optional_defaults: HashMap<usize, String>,
}

/// Walk `source` once and collect every label key referenced by a
/// `\ref`/`\cref`/`\eqref`/`\autoref`/`\pageref` (all `label_reference`
/// nodes), sanitized. Used by the project-mode pre-scan so a `\ref` in one
/// file is known when the labelled section in another file is emitted.
pub(crate) fn harvest_referenced_labels_from_source(source: &str) -> HashSet<String> {
    let tree = crate::parser::parse(source);
    let mut out: HashSet<String> = HashSet::new();
    let mut stack: Vec<Node<'_>> = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.kind() == "label_reference" {
            if let Some((keys, _)) = extract_label_ref_keys_and_end(n, source) {
                for k in keys {
                    let s = sanitize_label_key(&k);
                    if !s.is_empty() {
                        out.insert(s);
                    }
                }
            }
        }
        let mut cursor = n.walk();
        for c in n.children(&mut cursor) {
            stack.push(c);
        }
    }
    out
}

/// Walk `source` once and collect every `\newcommand` / `\def`
/// declaration into a fresh table. Used by the project-mode pre-scan
/// (see `project::harvest_project_macros`) so macros defined in
/// `.cls`/`.sty` files or in sibling `.tex` files unreached by `\input`
/// are still available when the entry file is converted.
pub(crate) fn harvest_macros_from_source(source: &str) -> HashMap<String, MacroDef> {
    let tree = crate::parser::parse(source);
    let mut out: HashMap<String, MacroDef> = HashMap::new();
    // `\let\new\old` pairs, resolved after the main pass so `\old` can refer
    // to a macro harvested later in the (DFS, unordered) walk.
    let mut lets: Vec<(String, String)> = Vec::new();
    let root = tree.root_node();
    let mut stack: Vec<Node<'_>> = vec![root];
    while let Some(n) = stack.pop() {
        match n.kind() {
            "new_command_definition" => {
                // tree-sitter uses `new_command_definition` for \newcommand,
                // \renewcommand, \providecommand AND \DeclareMathOperator. The
                // last needs its own extractor (operator body, not a `#1`-param
                // macro); without dispatching, an \input'd `\DeclareMathOperator`
                // was mis-harvested and the operator emitted `ambiguous_math` at
                // every use. Mirror `prepass_collect`'s dispatch here.
                let cmd_token = new_command_token_kind(n);
                match cmd_token.as_deref() {
                    Some("\\DeclareMathOperator") | Some("\\DeclareMathOperator*") => {
                        let starred = cmd_token.as_deref().is_some_and(|s| s.ends_with('*'));
                        if let Some((name, def)) =
                            extract_declare_math_operator_from_newcmd(n, source, starred)
                        {
                            out.insert(name, def);
                        }
                    }
                    Some("\\providecommand") | Some("\\providecommand*") => {
                        if let Some((name, def)) = extract_newcommand(n, source) {
                            // \providecommand: no-op if already defined.
                            if !out.contains_key(&name) && lookup_math_symbol(&name).is_none() {
                                out.insert(name, def);
                            }
                        }
                    }
                    _ => {
                        if let Some((name, def)) = extract_newcommand(n, source) {
                            out.insert(name, def);
                        }
                    }
                }
            }
            "let_command_definition" => {
                if let Some(pair) = extract_let(n, source) {
                    lets.push(pair);
                }
            }
            "old_command_definition" => {
                let _ = extract_def_and_record(n, source, &mut out);
            }
            "generic_command" => {
                // `\newcommandx` (xargspec) doesn't have a built-in
                // tree-sitter node — it parses as a generic_command.
                // Detect it explicitly and harvest the definition.
                if command_name_text_static(n, source).as_deref() == Some("\\newcommandx") {
                    if let Some((name, def)) = extract_newcommandx(n, source) {
                        out.insert(name, def);
                    }
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
    // Second pass: expand calls to wrapper-newcommand macros.
    // A "wrapper" is a macro whose body contains `\newcommand{#` — it
    // defines another macro from its first argument at LaTeX run time.
    // Example from arXiv/2605.22821:
    //   \newcommand{\mytoken}[2]{\newcommand{#1}{{\color{\c}#2}}}
    //   \mytoken{\token}{t}   →  would define \token at run time
    // The harvester sees \mytoken defined but never evaluates the call,
    // so \token never reaches self.macros and every `$\token$` emits
    // ambiguous_math. This pass closes the gap.
    harvest_wrapper_newcommands(tree.root_node(), source, &mut out);
    // Resolve `\let` aliases last, once every `\newcommand`/`\def` is in the
    // table. `or_insert` so an explicit definition always beats an alias.
    for (new_name, old_name) in lets {
        let def = let_alias_def(&old_name, &out);
        out.entry(new_name).or_insert(def);
    }
    out
}

/// Walk `root` and expand calls to macros whose body contains
/// `\newcommand{#` (the diagnostic of a wrapper that defines another
/// macro from argument #1). Expands each call with its source args and
/// re-harvests the resulting `\newcommand` definitions into `out`.
/// Uses `or_insert` so direct definitions always win over derived ones.
fn harvest_wrapper_newcommands(root: Node<'_>, src: &str, out: &mut HashMap<String, MacroDef>) {
    let mut stack: Vec<Node<'_>> = vec![root];
    while let Some(n) = stack.pop() {
        if n.kind() == "generic_command" {
            if let Some(cmd) = command_name_text_static(n, src) {
                // Clone so we don't hold a borrow of `out` while inserting.
                let wrapper = out
                    .get(&cmd)
                    .filter(|d| d.body.contains("\\newcommand{#"))
                    .cloned();
                if let Some(macro_def) = wrapper {
                    let args = collect_curly_args_static(n, src);
                    if args.len() >= macro_def.params && macro_def.params > 0 {
                        let expanded =
                            substitute_macro_args(&macro_def.body, &args[..macro_def.params]);
                        let sub_tree = crate::parser::parse(&expanded);
                        let mut sub_stack = vec![sub_tree.root_node()];
                        while let Some(sn) = sub_stack.pop() {
                            if sn.kind() == "new_command_definition" {
                                if let Some((nm, def)) = extract_newcommand(sn, &expanded) {
                                    out.entry(nm).or_insert(def);
                                }
                            } else {
                                let mut c = sn.walk();
                                for child in sn.children(&mut c) {
                                    sub_stack.push(child);
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut cursor = n.walk();
        for c in n.children(&mut cursor) {
            stack.push(c);
        }
    }
}

/// Collect the text content of each `curly_group` child of a
/// `generic_command` node (stripping the outer `{` / `}`). Used by
/// `harvest_wrapper_newcommands` to read call-site arguments without an
/// `Emitter` self.
fn collect_curly_args_static(node: Node<'_>, src: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "curly_group" {
            let start = child.start_byte() + 1;
            let end = child.end_byte().saturating_sub(1);
            args.push(src.get(start..end).unwrap_or("").to_string());
        }
    }
    args
}

/// Free-function variant of `command_name_text` for use inside
/// `harvest_macros_from_source` (which has no `Emitter` self). Returns
/// the source text of the first `command_name` child, or `None`.
fn command_name_text_static(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut result = None;
    for c in node.children(&mut cursor) {
        if c.kind() == "command_name" {
            result = Some(src[c.start_byte()..c.end_byte()].to_string());
            break;
        }
    }
    result
}

/// Pull (`\new`, `\old`) from a `let_command_definition` node. Both names
/// include the leading backslash. Tree-sitter produces this same node for
/// both `\let\new\old` and `\let\new=\old`, so the `=` form is free.
fn extract_let(node: Node<'_>, src: &str) -> Option<(String, String)> {
    let decl = node.child_by_field_name("declaration")?;
    let imp = node.child_by_field_name("implementation")?;
    Some((
        src[decl.start_byte()..decl.end_byte()].to_string(),
        src[imp.start_byte()..imp.end_byte()].to_string(),
    ))
}

/// The `MacroDef` that `\let\new\old` assigns to `\new`: copy `\old`'s
/// definition when it's a known user macro (preserves arity), otherwise a
/// zero-arg alias whose body is `\old`. The body form covers builtins,
/// math symbols, and forward references — they resolve when `\new` is later
/// expanded and `\old` is re-parsed in context.
fn let_alias_def(old_name: &str, table: &HashMap<String, MacroDef>) -> MacroDef {
    table.get(old_name).cloned().unwrap_or_else(|| MacroDef {
        params: 0,
        body: old_name.to_string(),
        optional_defaults: HashMap::new(),
    })
}

/// Byte bounds of a `\ifX ... [\else ...] \fi` conditional, found by scanning
/// raw source from just after the opening `\ifX`.
struct CondBounds {
    /// (start, end) of the matching depth-0 `\else`, if present.
    else_span: Option<(usize, usize)>,
    /// Byte where the matching depth-0 `\fi` begins.
    fi_start: usize,
    /// Byte just after the matching `\fi`.
    fi_end: usize,
}

/// Scan `src` from `start` (just after an opening `\ifX`) for its matching
/// depth-0 `\else` and `\fi`. Any `\if*` control word opens a nesting level
/// and `\fi` closes one; `%` line comments are skipped so a `\fi` mentioned
/// in a comment doesn't terminate the scan. Returns `None` if unbalanced.
fn find_conditional_bounds(src: &str, start: usize) -> Option<CondBounds> {
    let bytes = src.as_bytes();
    let mut i = start;
    let mut depth: i32 = 0;
    let mut else_span: Option<(usize, usize)> = None;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'\\' => {
                let cs_start = i;
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == i + 1 {
                    // Control symbol (`\\`, `\{`, `\%`, ...): consume both bytes.
                    i += 2;
                    continue;
                }
                let cs = &src[cs_start..j];
                if cs == "\\fi" {
                    if depth == 0 {
                        return Some(CondBounds {
                            else_span,
                            fi_start: cs_start,
                            fi_end: j,
                        });
                    }
                    depth -= 1;
                } else if cs == "\\else" && depth == 0 {
                    else_span = Some((cs_start, j));
                } else if cs.starts_with("\\if") {
                    depth += 1;
                }
                i = j;
            }
            _ => i += 1,
        }
    }
    None
}

/// Read the `\if<name>` control word following a `\newif` (skipping leading
/// whitespace). Returns the bare flag name (`foo` for `\iffoo`) and the byte
/// just after the flag token.
fn read_newif_flag(src: &str, after_newif: usize) -> Option<(String, usize)> {
    let bytes = src.as_bytes();
    let mut i = after_newif;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'\\' {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
        j += 1;
    }
    let name = src[i..j].strip_prefix("\\if")?;
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), j))
}

/// Sentinel character emitted by `push_math_symbol` immediately after a
/// multi-character math identifier so that `collapse_math_spaces` can
/// later decide whether to insert a real separator (when the next char
/// would fuse — letter or digit) or drop it (when Typst already breaks
/// — `_`, `^`, `,`, `(`, `)`). Chosen as U+0017 ETB which has no
/// legitimate use in either LaTeX source or rendered Typst.
const MATH_WORD_BOUNDARY: char = '\u{17}';

/// Self-contained "clean neutral article" preamble (Task 1). Emits only native
/// Typst set/show rules — no `@preview` imports, compiles on stock Typst with
/// no packages or `typst.toml`. Scalar layout (paper size, font size) is taken
/// from `layout` when the source's `\documentclass` requested it (Task 2),
/// otherwise the neutral defaults (us-letter, 11pt) are kept. Heading
/// *numbering* is set by `finish()`, not here, so there is a single
/// `#set heading(numbering)` site.
fn build_neutral_preamble(
    layout: &crate::class_map::Layout,
    class: &crate::class_map::DocClass,
) -> String {
    let paper = layout.paper.unwrap_or("us-letter");
    // LaTeX's default body size for `\documentclass{article}` (no size option)
    // is 10pt; byetex previously defaulted to 11pt, inflating page count ~10%.
    let font_size = layout.font_size.unwrap_or("10pt");
    // Margin: an explicit `geometry` value always wins. Otherwise the neutral
    // 1in default — EXCEPT for dense two-column conference classes, whose own
    // class geometry is far tighter than 1in; using 1in there narrows the
    // columns and inflates the page count (IEEEtran conference: 22779
    // page_ratio 1.38 at 1in). Approximate the IEEEtran text block on letter.
    let margin = if layout.margin.is_default() {
        match class {
            crate::class_map::DocClass::IeeeTran { .. } => {
                "(top: 0.75in, bottom: 1in, x: 0.62in)".to_string()
            }
            _ => layout.margin.to_typst_value(),
        }
    } else {
        layout.margin.to_typst_value()
    };
    format!(
        "#set page(paper: \"{paper}\", margin: {margin})\n\
         #set text(font: \"New Computer Modern\", size: {font_size})\n\
         #set par(justify: true, leading: 0.65em, spacing: 0.65em, first-line-indent: 1.2em)\n\
         #show heading.where(level: 1): set text(size: 1.44em, weight: \"bold\")\n\
         #show heading.where(level: 2): set text(size: 1.2em, weight: \"bold\")\n\
         #show heading.where(level: 3): set text(size: 1em, weight: \"bold\")\n\
         #show heading: it => block(above: if it.level == 1 {{ 1.5em }} else {{ 1.4em }}, below: if it.level == 1 {{ 1.0em }} else {{ 0.65em }}, it)\n\n"
    )
}

pub(crate) struct Emitter<'a> {
    out: String,
    warnings: Vec<Warning>,
    src: &'a str,
    source_name: &'a str,
    /// True while emitting the interior of a math container. Affects how
    /// commands (e.g. `\alpha` → `alpha`) and subscripts (`_{x}` → `_(x)`)
    /// are rendered.
    in_math: bool,
    /// While emitting inside a math container, `\label{x}` is recorded here
    /// and later attached to the enclosing equation/figure as a Typst label.
    /// Cleared by the container emitter after attachment.
    /// Labels collected from `\label{...}` calls while inside a math
    /// container. Multiple labels can attach to one math env (e.g.
    /// `\begin{subequations}\label{eqn:AMP}\begin{align}\label{eqn:AMPa}
    /// ...\label{eqn:AMPb}`). The math-env emitter flushes the first
    /// one as `<key>` next to the closing `$`, and emits each
    /// additional label as a hidden equation block so all `\ref{...}`
    /// targets still resolve.
    pending_math_labels: Vec<String>,
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
    /// refined by `\usepackage{...}` calls. Drives per-class author parsing and
    /// retained as a layout hint.
    detected_class: DocClass,
    /// Scalar layout overrides (font size, paper size) derived from the
    /// `\documentclass[opts]`; applied on top of the neutral preamble.
    layout: crate::class_map::Layout,
    /// Directory used to resolve `\input{...}` / `\include{...}` paths. When
    /// `Some`, the emitter expands those directives inline; when `None`, it
    /// drops them with a `needs_manual_review` warning (the v0.1 behaviour
    /// that runs when `convert()` is called with bare source and no file).
    base_dir: Option<PathBuf>,
    /// The project root directory — always the top-level document's directory.
    /// Used as a fallback when resolving `\input{path}` from sub-files, since
    /// LaTeX resolves include paths relative to the root (not the current file).
    root_dir: Option<PathBuf>,
    /// Search directories declared by `\graphicspath{{dir1/}{dir2/}}` (relative
    /// to the project root), in declaration order. `\includegraphics{name}` is
    /// probed against these (after base_dir/root_dir) so a bare image name whose
    /// file lives in a graphicspath dir resolves instead of going "missing".
    /// Collected during emission (the directive may sit in an `\input`-ed
    /// preamble) and merged across the `\input` sub-emitter boundary.
    graphics_paths: Vec<String>,
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
    /// `\newif\ifX` boolean flags and their current state. Keyed by the bare
    /// name (`X`, without the `\if` prefix). `\Xtrue`/`\Xfalse` update the
    /// state in document order; `\ifX ... \else ... \fi` emits only the taken
    /// branch. TeX's `\if`-family is otherwise out of scope.
    newif_flags: HashMap<String, bool>,
    /// Sanitized label keys referenced anywhere by `\ref`/`\cref`/`\eqref`/
    /// `\autoref`/`\pageref`. Populated before emit (prepass on the main tree
    /// plus a project-wide pre-scan). When a section/figure carries multiple
    /// `\label` aliases — and Typst keeps only one per element — we attach the
    /// alias that is actually referenced so the reference resolves.
    referenced_labels: HashSet<String>,
    /// True once a `\documentclass` is seen — i.e. this is a full document
    /// (not a bare fragment). Gates the self-generated neutral preamble so
    /// fragment conversions stay preamble-free.
    saw_document_class: bool,
    /// `\newtheorem{name}{Display}` declarations harvested as we walk the
    /// source. When `emit_generic_environment` encounters an unknown env name
    /// that matches a key here, it routes to `emit_theorem_env_dyn` instead of
    /// `warn_unsupported_environment`.
    theorem_kinds: HashMap<String, String>,
    /// Declared mandatory-argument count for custom `\newenvironment`s, keyed by
    /// env name. At a use site `\begin{name}{a}{b}` the args are leading
    /// `curly_group` children; `emit_environment_body` drops this many so they
    /// don't leak into the passed-through body.
    env_arg_counts: HashMap<String, usize>,
    /// Set of bibliography keys that are defined either by a `.bib`
    /// file in `base_dir` or by a `\bibitem{key}` somewhere in the
    /// document. Populated by `harvest_bib_keys_from_dir` in the
    /// prepass plus per-bibitem inserts during emit. Used by
    /// `emit_citation` to drop `\cite{key}` calls whose key isn't
    /// defined — otherwise Typst aborts with `label <key> does not
    /// exist`. Keys are stored sanitized (see `sanitize_label_key`).
    /// Empty set short-circuits validation (legacy convert path).
    bibliography_keys: std::collections::HashSet<String>,
    /// Assets (images, bib files) resolved on disk during this emit pass.
    /// Populated only when `base_dir` is `Some`. Bubbled up to `ConvertOutput`
    /// by `finish()` so the project layer can copy them to the output dir.
    asset_refs: Vec<crate::AssetRef>,
    /// Current `\newcommand` expansion depth. A self-referential macro
    /// (`\newcommand{\foo}{\foo}`) would otherwise recurse without bound
    /// and overflow the stack. The cap is generous enough for legitimate
    /// nested expansions but stops adversarial inputs cold.
    macro_depth: u32,
    /// True while emitting a `minipage` body. Inside a minipage a `\\` is an
    /// intra-box line break, not a table row separator, so the `\\` handler
    /// emits a Typst `#linebreak()` instead of the bare `\` that the table
    /// row-splitter (`split_math_rows`) keys on — otherwise a minipage used as
    /// a table cell mis-splits across rows.
    in_minipage: bool,
    /// Set when an inline `\label` in text/list context emits a hidden
    /// `kind: "anchor"` figure so the label is referenceable. Gates the
    /// `#show figure.where(kind: "anchor"): it => none` rule in `finish()`.
    used_text_label_anchor: bool,
    /// True when a `\bibliography{...}` (`bibtex_include`) command is present,
    /// detected in the prepass.
    has_bibtex_include: bool,
    /// True when a `.bib` file actually resolved on disk during the prepass
    /// (`bibliography_keys` was non-empty right after the directory harvest,
    /// before any `\bibitem` was added during emit). Together with
    /// `has_bibtex_include` this means a `#bibliography(.bib)` will render the
    /// full reference list, so any manual `\bibitem`/`thebibliography` entries
    /// are redundant and must be dropped — otherwise the keys they share with
    /// the .bib collide (`label <key> occurs both in the document and its
    /// bibliography`, corpus 2605.31440).
    had_bib_file: bool,
}

/// Maximum allowed `\newcommand` expansion depth (see `Emitter::macro_depth`).
/// Each level allocates a fresh sub-Emitter and re-parses the body, so the
/// per-level stack usage is high; values much above 24 can overflow test
/// threads' default 2 MB stack. Real papers rarely nest macros more than
/// 4-5 levels.
const MAX_MACRO_DEPTH: u32 = 24;

impl<'a> Emitter<'a> {
    // ─── Construction & lifecycle ──────────────────────────────────────────────

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
                        optional_defaults: HashMap::new(),
                    });
            }
        }
        Self {
            out: String::new(),
            warnings: Vec::new(),
            src,
            source_name,
            in_math: false,
            pending_math_labels: Vec::new(),
            pending_bib_style: None,
            needs_heading_numbering: false,
            needs_equation_numbering: false,
            pending_bibitem_key: None,
            skip_until: 0,
            metadata: DocumentMetadata::default(),
            raw_authors: Vec::new(),
            detected_class: DocClass::Unknown,
            layout: crate::class_map::Layout::default(),
            root_dir: base_dir.clone(),
            graphics_paths: Vec::new(),
            base_dir,
            visited_includes: visited,
            macros: preseeded_macros,
            newif_flags: HashMap::new(),
            referenced_labels: HashSet::new(),
            saw_document_class: false,
            theorem_kinds: HashMap::new(),
            env_arg_counts: HashMap::new(),
            bibliography_keys: std::collections::HashSet::new(),
            asset_refs: Vec::new(),
            macro_depth: 0,
            in_minipage: false,
            used_text_label_anchor: false,
            has_bibtex_include: false,
            had_bib_file: false,
        }
    }

    pub(crate) fn emit_root(&mut self, root: Node<'_>) {
        let _ = self.emit_node(root);
    }

    /// Walk the entire AST *before* `emit_root`, harvesting ALL macro
    /// definitions into `self.macros`. This ensures macros used before their
    /// definition (forward references) are available at emit time.
    /// Seed labels referenced across the whole project (from a pre-scan of
    /// every source file) so cross-file `\ref`s inform multi-label attachment
    /// even when the `\ref` and the labelled section live in different files.
    pub(crate) fn seed_referenced_labels(&mut self, refs: HashSet<String>) {
        self.referenced_labels.extend(refs);
    }

    pub(crate) fn prepass_collect(&mut self, root: Node<'_>) {
        // PR-3: harvest bibliography keys from any `.bib` file in
        // base_dir so `emit_citation` can validate `\cite{key}` calls
        // and drop refs to undefined keys (otherwise Typst aborts the
        // whole compile with `label <key> does not exist`).
        // `\bibitem{key}` keys discovered during the main emit pass
        // are added to the set incrementally.
        if let Some(ref base) = self.base_dir.clone() {
            harvest_bib_keys_from_dir(base, &mut self.bibliography_keys);
        }
        // A .bib resolved on disk iff the harvest added any keys (no `\bibitem`
        // keys are present yet — those are inserted during emit).
        self.had_bib_file = !self.bibliography_keys.is_empty();
        let mut stack: Vec<Node<'_>> = vec![root];
        while let Some(n) = stack.pop() {
            // A `\bibliography{...}` directive — paired with a resolvable .bib,
            // its `#bibliography(.bib)` is the authoritative reference list, so
            // any manual `\bibitem`/`thebibliography` entries are dropped.
            if n.kind() == "bibtex_include" {
                self.has_bibtex_include = true;
            }
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
                            let starred = cmd_token.as_deref().is_some_and(|s| s.ends_with('*'));
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
                "let_command_definition" => {
                    // `\let\new\old` — seed the alias so forward references to
                    // `\new` resolve. The emit pass re-applies in document order.
                    if let Some((new_name, old_name)) = extract_let(n, self.src) {
                        let def = let_alias_def(&old_name, &self.macros);
                        self.macros.entry(new_name).or_insert(def);
                    }
                }
                "label_reference" => {
                    // Record which labels are `\ref`'d so multi-label sections
                    // can attach the referenced alias (see emit_section).
                    if let Some((keys, _)) = extract_label_ref_keys_and_end(n, self.src) {
                        for k in keys {
                            let s = sanitize_label_key(&k);
                            if !s.is_empty() {
                                self.referenced_labels.insert(s);
                            }
                        }
                    }
                }
                "generic_command" => {
                    // generic_command does NOT produce \renewcommand/\providecommand/
                    // \DeclareMathOperator (those are new_command_definition in tree-sitter).
                    // BUT `\newcommandx` (xargspec package) parses as generic_command
                    // because tree-sitter-latex has no built-in keyword for it.
                    // Harvest it explicitly.
                    if command_name_text(n, self.src).as_deref() == Some("\\newcommandx") {
                        if let Some((name, def)) = extract_newcommandx(n, self.src) {
                            self.macros.insert(name, def);
                        }
                    }
                    let mut cursor = n.walk();
                    for c in n.children(&mut cursor) {
                        stack.push(c);
                    }
                }
                "package_include" => {
                    for pkg in extract_package_names(n, self.src) {
                        // Local .sty first so it beats bundled seeds
                        self.expand_local_package(&pkg);
                        // Then seed bundled macros — or_insert loses to any existing entry
                        if let Some(seeds) = crate::package_macros::package_macros(&pkg) {
                            for (macro_name, seed) in seeds {
                                if lookup_math_symbol(macro_name).is_none() {
                                    self.macros.entry(macro_name.to_string()).or_insert_with(
                                        || MacroDef {
                                            params: seed.params,
                                            body: seed.body.to_string(),
                                            optional_defaults: HashMap::new(),
                                        },
                                    );
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
        // A full document (had a `\documentclass`, or carries title/authors)
        // is rendered with the self-generated, self-contained neutral preamble
        // + generalized title block — no Typst Universe import. Bare fragments
        // (no documentclass, no title) get neither, so fragment conversions
        // stay preamble-free.
        let is_document = self.saw_document_class
            || !self.metadata.is_title_block_empty()
            || !self.raw_authors.is_empty();
        if is_document {
            let body = std::mem::take(&mut self.out);
            self.flush_title_block(); // prepends title/authors/abstract/keywords (no-op if empty)
            let title_block = std::mem::take(&mut self.out);
            // Self-contained preamble first, then this document's numbering
            // rules (LaTeX numbers sections by default), then title + body.
            self.out
                .push_str(&build_neutral_preamble(&self.layout, &self.detected_class));
            self.out.push_str("#set heading(numbering: \"1.\")\n");
            if self.used_text_label_anchor {
                self.out
                    .push_str("#show figure.where(kind: \"anchor\"): it => none\n");
                // Emitted here; clear so the fragment-preamble block below
                // (which runs unconditionally) doesn't prepend it a second time.
                self.used_text_label_anchor = false;
            }
            if self.needs_equation_numbering {
                self.out
                    .push_str("#set math.equation(numbering: \"(1)\")\n");
            }
            self.out.push('\n');
            // The title block stays full-width; a two-column document wraps only
            // the body in `#columns(2)[...]` (mirrors LaTeX's full-width title
            // over a two-column body).
            self.out.push_str(&title_block);
            if self.layout.is_two_column(&self.detected_class) {
                self.out.push_str("#columns(2)[\n");
                self.out.push_str(body.trim_start_matches('\n'));
                if !self.out.ends_with('\n') {
                    self.out.push('\n');
                }
                self.out.push_str("]\n");
            } else {
                self.out.push_str(&body);
            }
            // Numbering is fully emitted above; don't double-prepend below.
            self.needs_heading_numbering = false;
            self.needs_equation_numbering = false;
        }

        // Numbering preamble for bare fragments (no neutral preamble): heading
        // numbering only when a fragment references a heading; equation
        // numbering stays demand-driven.
        let mut preamble = String::new();
        if self.used_text_label_anchor {
            preamble.push_str("#show figure.where(kind: \"anchor\"): it => none\n");
        }
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
        self.out = break_raw_paren_chains(&self.out);
        self.out = break_math_comment_tokens(&self.out);

        // Backstop for dangling `\ref`/`\cref` targets. A label that LaTeX would
        // merely warn about — commented out (`% \label{x}`, corpus 2605.31586),
        // or in a dropped/unsupported environment — leaves a `@key` reference
        // with no `<key>` anchor, which Typst rejects outright (`label <key>
        // does not exist`). Emit a hidden anchor for every referenced key that
        // has neither a `<key>` label in the output nor a bibliography entry, so
        // the reference resolves and the document compiles (as LaTeX does).
        // Scoped to `referenced_labels` (the `\ref`/`\cref` family); citation
        // keys are resolved by `#bibliography`, so they are never anchored here
        // and the backstop cannot collide with the bibliography. Only for full
        // documents: a bare fragment may be embedded in a context that defines
        // the label, where a backstop anchor would itself become a duplicate.
        if is_document {
            let backstop = self.dangling_ref_anchors();
            if !backstop.is_empty() {
                self.out.push_str(&backstop);
            }
        }

        // Prepend `#set document(author: ...)` for PDF metadata. Must come
        // after authors are materialised (build_template_preamble or
        // flush_title_block already did that) and after the body is assembled.
        if !self.metadata.authors.is_empty() {
            let names: Vec<String> = self
                .metadata
                .authors
                .iter()
                .map(|a| {
                    let n = a.name.as_content();
                    let escaped = n.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("\"{}\"", escaped)
                })
                .collect();
            let set_doc = format!("#set document(author: ({},))\n", names.join(", "));
            self.out.insert_str(0, &set_doc);
        }

        let class_metadata = self.metadata.class_metadata;
        (self.out, self.warnings, self.asset_refs, class_metadata)
    }

    /// Push `src[from..to]` to the output, but only when the range is valid.
    /// Some emitters (notably comment-drop with newline consumption) advance
    /// the cursor past a node's `end_byte`; downstream trailing-copy logic
    /// must tolerate the resulting reverse range as a no-op.
    fn safe_copy(&mut self, from: usize, to: usize) {
        if from >= to {
            return;
        }
        let text = &self.src[from..to];
        if self.in_math {
            self.out.push_str(text);
            return;
        }
        // Escape bare '#' to '\#' for Typst (where '#' starts a code
        // expression). Already-escaped '\#' is preserved as-is.
        let mut prev_backslash = false;
        for ch in text.chars() {
            if ch == '#' && !prev_backslash {
                self.out.push('\\');
            }
            self.out.push(ch);
            prev_backslash = ch == '\\';
        }
    }

    /// Return the inner text of a `curly_group` node (the bytes between
    /// the outer `{` and `}`), trimmed of surrounding whitespace. Used
    /// when a caller wants the raw argument text without emitting it
    /// through the AST walker (e.g. URL extraction, path extraction).
    fn curly_group_inner_trimmed(&self, group: Node<'_>) -> &'a str {
        self.src
            .get(group.start_byte() + 1..group.end_byte() - 1)
            .unwrap_or("")
            .trim()
    }

    /// Run a child `Emitter` over `src` and merge its side-effects
    /// (warnings, asset_refs, newly-defined macros, returned visited
    /// set) back into the parent. Returns the child's body output.
    ///
    /// The child inherits the parent's `source_name`, `base_dir`,
    /// `visited_includes`, and `macros` table. `in_math` is passed
    /// explicitly because the caller knows its math context. When
    /// `increment_depth` is `true`, `macro_depth` is bumped so the
    /// recursion cap (`MAX_MACRO_DEPTH`) reaches into the child.
    /// Three call sites use it: `expand_user_macro` and both
    /// `emit_math_wrap` branches (Command + Group). `expand_latex_include`
    /// stays inline because it merges several additional fields
    /// (metadata, raw_authors, detected_class, needs_*_numbering)
    /// that aren't part of the common pattern.
    fn render_in_sub_emitter(&mut self, src: &str, in_math: bool, increment_depth: bool) -> String {
        let tree = crate::parser::parse(src);
        let visited = std::mem::take(&mut self.visited_includes);
        let macros = self.macros.clone();
        let mut sub = Emitter::with_includes(src, self.source_name, self.base_dir.clone(), visited);
        sub.in_math = in_math;
        sub.macros = macros;
        sub.newif_flags = self.newif_flags.clone();
        sub.referenced_labels = self.referenced_labels.clone();
        if increment_depth {
            sub.macro_depth = self.macro_depth + 1;
        }
        sub.bibliography_keys = self.bibliography_keys.clone();
        sub.emit_root(tree.root_node());
        // Merge side-effects back into the parent.
        self.visited_includes = std::mem::take(&mut sub.visited_includes);
        for (k, v) in sub.macros.drain() {
            self.macros.entry(k).or_insert(v);
        }
        // Bibitems discovered inside the sub-emitter (e.g. when an
        // inlined `.bbl` runs through here and emits `\bibitem`
        // calls) need to flow back so the parent's citations resolve.
        self.bibliography_keys.extend(sub.bibliography_keys.drain());
        self.warnings.append(&mut sub.warnings);
        self.asset_refs.append(&mut sub.asset_refs);
        sub.out
    }

    // ─── Node dispatch ────────────────────────────────────────────────────────

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
                    // Math-mode `word` tails (e.g. `Np` after `\frac12Np`)
                    // must go through letter-splitting instead of raw copy.
                    if self.in_math && node.kind() == "word" {
                        let tail = &self.src[self.skip_until..node.end_byte()];
                        let alpha_end = tail
                            .find(|c: char| !c.is_ascii_alphabetic())
                            .unwrap_or(tail.len());
                        let alpha = &tail[..alpha_end];
                        let rest = &tail[alpha_end..];
                        if should_split_math_word(alpha) {
                            self.ensure_math_letter_boundary(tail);
                            let mut first = true;
                            for c in alpha.chars() {
                                if !first {
                                    self.out.push(' ');
                                }
                                self.out.push(c);
                                first = false;
                            }
                            if rest.starts_with(|c: char| c.is_ascii_digit()) {
                                self.out.push(' ');
                            }
                            self.out.push_str(rest);
                        } else {
                            self.safe_copy(self.skip_until, node.end_byte());
                        }
                    } else {
                        self.safe_copy(self.skip_until, node.end_byte());
                    }
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
            // Bug #14b: sizing commands as bare token kinds — tree-sitter
            // gives `\left` and `\right` their own kind strings outside
            // of a matched `math_delimiter` (which `emit_math_delimiter`
            // already handles). When `\left` is unmatched (no `\right`
            // partner) or when the enclosing math span ended up as an
            // ERROR node, the raw `\left` token leaks through. Drop it
            // silently; Typst auto-pairs the bare delimiter that follows.
            "\\left" | "\\right" | "\\middle" | "\\bigl" | "\\Bigl" | "\\biggl" | "\\Biggl"
            | "\\bigr" | "\\Bigr" | "\\biggr" | "\\Biggr" | "\\bigm" | "\\Bigm" | "\\biggm"
            | "\\Biggm" | "\\big" | "\\Big" | "\\bigg" | "\\Bigg" => return node.end_byte(),
            // tree-sitter-latex frequently mis-parses keys that
            // contain `_` (e.g. inside `\ref{thm:UAP_general_dim}`)
            // by truncating the curly_group, leaving an *orphan*
            // closing brace as an `ERROR` node — which then leaks
            // into the output as a stray `}` and either breaks the
            // surrounding markdown (Bug #35 in 2605.22557) or
            // produces stray label/ref attachment. Drop ERROR nodes
            // that are just a single brace.
            "ERROR"
                if {
                    let text = &self.src[node.start_byte()..node.end_byte()];
                    let trimmed = text.trim();
                    trimmed == "{" || trimmed == "}"
                } =>
            {
                return node.end_byte()
            }
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
                // `\text{X}`-family also commonly parses as a bare
                // command_name with `{X}` attached as an AST sibling
                // (e.g. deep inside `_{\mathrm{n}_{\text{b}}}`). Route
                // through the source-byte fallback path so we don't
                // warn just because the curly group isn't a child.
                if matches!(
                    text,
                    "\\text"
                        | "\\mathrm"
                        | "\\textrm"
                        | "\\mathnormal"
                        | "\\mbox"
                        | "\\hbox"
                        | "\\textnormal"
                        | "\\texttt"
                        | "\\textbf"
                        | "\\textup"
                        | "\\textit"
                        | "\\textsc"
                        | "\\textsl"
                ) {
                    return self.emit_math_text_call(node);
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
            let alpha_end = text
                .find(|c: char| !c.is_ascii_alphabetic())
                .unwrap_or(text.len());
            let alpha = &text[..alpha_end];
            let tail = &text[alpha_end..];
            // Guard: keep the preceding identifier from fusing with this word's
            // first letter (e.g. `t` + `dt` → `tdt`). The helper is a no-op
            // when the previous output char is not a letter.
            self.ensure_math_letter_boundary(text);
            // LaTeX operators Typst lacks as built-ins (cov/var/argmax/argmin)
            // must be emitted via `op("…")` — bare `cov` parses as an unknown
            // variable (corpus 2605.31567). Like sin/cos they aren't split.
            if is_operatorname_only_function(alpha) {
                let _ = write!(self.out, "op(\"{}\")", alpha);
                self.out.push_str(tail);
                return node.end_byte();
            }
            if should_split_math_word(alpha) {
                let mut first = true;
                for c in alpha.chars() {
                    if !first {
                        self.out.push(' ');
                    }
                    self.out.push(c);
                    first = false;
                }
                // Bug #23: `i0`-style letter+digit identifiers (e.g.
                // `_{i0}`) become Typst identifier lookups that fail.
                // Insert a separator between alpha and digit tail so
                // they parse as separate atoms.
                if boundary::starts_with_digit(tail) {
                    self.out.push(' ');
                }
                self.out.push_str(tail);
                return node.end_byte();
            }
            // Bug #23 (single-letter alpha case): even if we don't enter
            // the splitting branch, an `i0`-style word with a 1-char
            // alpha prefix needs the same separator before the digit
            // tail to keep Typst from reading `i0` as an identifier.
            if !alpha.is_empty() && boundary::starts_with_digit(tail) {
                self.out.push_str(alpha);
                self.out.push(' ');
                self.out.push_str(tail);
                return node.end_byte();
            }
            // Digit-prefix words like "2JX" or "2kg": alpha_end==0 because the
            // word starts with a digit, so the alpha-split branch never fires.
            // Extract the digit prefix, then apply the same splitting logic to
            // the trailing alpha run.
            if alpha.is_empty() {
                let digit_end = text
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(text.len());
                let digit_prefix = &text[..digit_end];
                let rest = &text[digit_end..];
                let rest_alpha_end = rest
                    .find(|c: char| !c.is_ascii_alphabetic())
                    .unwrap_or(rest.len());
                let rest_alpha = &rest[..rest_alpha_end];
                let rest_tail = &rest[rest_alpha_end..];
                if should_split_math_word(rest_alpha) {
                    self.out.push_str(digit_prefix);
                    for c in rest_alpha.chars() {
                        self.out.push(' ');
                        self.out.push(c);
                    }
                    if boundary::starts_with_digit(rest_tail) {
                        self.out.push(' ');
                    }
                    self.out.push_str(rest_tail);
                    return node.end_byte();
                }
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

        // `\textcolor{color}{content}` — tree-sitter-latex parses this as a
        // dedicated `color_reference` node. Drop the color arg, emit content.
        if node.kind() == "color_reference" {
            return if self.in_math {
                self.emit_math_textcolor(node)
            } else {
                self.emit_textcolor(node)
            };
        }

        // `\definecolor{name}{model}{spec}` / `\definecolorset{...}` — tree-sitter
        // parses these as dedicated `color_definition` / `color_set_definition`
        // nodes (NOT generic_command), so they bypass the command-name drop list
        // and were safe-copied into the body verbatim (corpus 2605.22779 spilled a
        // block of `\definecolor{...}{HTML}{...}` next to the abstract). byetex
        // doesn't apply xcolor colours, so the definition is inert — drop it whole.
        if matches!(node.kind(), "color_definition" | "color_set_definition") {
            return node.end_byte();
        }

        // Backslash commands: look up by name, fall through to warn-and-drop.
        if node.kind() == "generic_command" {
            return self.emit_generic_command(node);
        }

        // \begin{X} ... \end{X}: dispatch by environment name.
        if node.kind() == "generic_environment" {
            return self.emit_generic_environment(node);
        }

        // Verbatim/listing environments — tree-sitter-latex gives these
        // their own special node kinds (not "generic_environment"), so
        // they must be intercepted here.
        if node.kind() == "listing_environment" {
            return self.emit_listing_environment(node);
        }

        // Inside math, `\label{...}` is silently lifted out and attached to
        // the enclosing math container as a Typst `<label>`.
        if self.in_math && node.kind() == "label_definition" {
            if let Some((l, end)) = extract_label_name_and_end(node, self.src) {
                // Bug #44: multiple `\label{...}` inside one math env
                // (e.g. `\begin{subequations}\label{eqn:AMP}\begin{align}
                // \label{eqn:AMPa}...\label{eqn:AMPb}`). Collect them
                // all; the env-closing flush emits the first as the
                // attached `<key>` and emits each extra as a hidden
                // equation block so every `\ref{...}` resolves.
                if !self
                    .pending_math_labels
                    .iter()
                    .any(|existing| existing == &l)
                {
                    self.pending_math_labels.push(l);
                }
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
            // Orphan `\label{X}` outside any section/equation/figure. A bare
            // `<X>` here would attach to the surrounding paragraph text or list
            // item, which Typst can't reference ("cannot reference text"), or —
            // when the enclosing env was dropped — never be emitted at all
            // ("label does not exist"). Emit a hidden, self-numbered anchor
            // figure instead: it IS referenceable and, via the `kind: "anchor"`
            // show rule added in `finish()`, renders nothing. Its own per-kind
            // counter leaves real figure/table numbering untouched. (Section,
            // figure, and math labels are absorbed by their own handlers and
            // never reach this arm.)
            "label_definition" => {
                if let Some((key, end)) = extract_label_name_and_end(node, self.src) {
                    self.used_text_label_anchor = true;
                    let _ = write!(
                        self.out,
                        " #box[#figure(kind: \"anchor\", supplement: none, numbering: \"1\", [])<{}>]",
                        key
                    );
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
                // Capture raw LaTeX bytes so sub-commands (\email, \thanks,
                // \And, \corref, \IEEEauthorblockN, …) reach the per-author
                // parser in class_map.rs intact instead of being intercepted
                // and consumed by the top-level dispatcher.
                if let Some(arg) = first_curly_like(node) {
                    let inner = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte().saturating_sub(1))
                        .unwrap_or("")
                        .to_string();
                    self.raw_authors.push(inner);
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
        //
        // Each package in a comma-separated list is handled independently so
        // that `\usepackage{amsmath,xeCJK}` drops `amsmath` silently and emits
        // exactly one warning for `xeCJK` (named `usepackage:xeCJK`), rather
        // than either silently losing the trailing packages or collapsing all
        // unknowns into a generic `\usepackage` warning.
        if node.kind() == "package_include" {
            let pkgs = extract_package_names(node, self.src);
            let opts = extract_package_options(node, self.src);
            if pkgs.is_empty() {
                // Couldn't parse a name at all (malformed node) — fall back.
                self.warn_unsupported_command(node);
            } else {
                for pkg in &pkgs {
                    // Class refinement: ml-conference style files can upgrade
                    // a generic `\documentclass{article}`.
                    let old = std::mem::replace(&mut self.detected_class, DocClass::Unknown);
                    self.detected_class = old.refine_from_package(pkg);
                    // `geometry` package options set the page margins / paper.
                    if pkg == "geometry" {
                        if let Some(o) = opts.as_deref() {
                            self.layout.apply_geometry(o);
                        }
                    }
                    // Harvest macros / theorems from a local `<pkg>.sty` if
                    // present next to the source file.
                    self.expand_local_package(pkg);
                    if !is_known_noop_package(pkg) {
                        self.warn_unsupported_package(node, pkg, opts.as_deref());
                    }
                }
            }
            return node.end_byte();
        }

        // `\usetikzlibrary{...}` — preamble plumbing for TikZ, no Typst
        // equivalent. tree-sitter-latex gives this its own `tikz_library_import`
        // node kind (not a `generic_command`), so without this arm it would
        // fall through to the default verbatim copy and leak into the body.
        // Drop it silently, like `\usepackage{tikz}`.
        if node.kind() == "tikz_library_import" {
            return node.end_byte();
        }

        // `\documentclass[opts]{class}` — capture the class (drives author
        // parsing) and scalar layout options (font/paper size). The source
        // line itself is dropped from the output.
        if node.kind() == "class_include" {
            self.saw_document_class = true;
            let (class, opts) = extract_class_and_options(node, self.src);
            self.layout = crate::class_map::Layout::from_class_options(&opts);
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
                        if !self.macros.contains_key(&name) && lookup_math_symbol(&name).is_none() {
                            self.macros.insert(name, def);
                        }
                    }
                }
                Some("\\DeclareMathOperator") | Some("\\DeclareMathOperator*") => {
                    // Harvest with the correct `\operatorname{...}` body (NOT
                    // `extract_newcommand`, which would keep only the display
                    // text). The top-level prepass also seeds this, but sub-
                    // emitters for `\input`ed files do not run a prepass — they
                    // rely on emit-time harvesting — so an operator defined in an
                    // included file (e.g. `newcommands.tex`) would otherwise never
                    // register and warn `ambiguous_math` at every use.
                    let starred = cmd_token.as_deref().is_some_and(|s| s.ends_with('*'));
                    if let Some((name, def)) =
                        extract_declare_math_operator_from_newcmd(node, self.src, starred)
                    {
                        self.macros.entry(name).or_insert(def);
                    }
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
        // `\let\new\old` is a definition (a dedicated `let_command_definition`
        // node). Apply the alias in document order — `\let` reassigns, so this
        // overwrites — and emit nothing. Prepass already seeded forward refs.
        if node.kind() == "let_command_definition" {
            if let Some((new_name, old_name)) = extract_let(node, self.src) {
                let def = let_alias_def(&old_name, &self.macros);
                self.macros.insert(new_name, def);
            }
            return node.end_byte();
        }
        if node.kind() == "counter_declaration" {
            return node.end_byte();
        }
        if node.kind() == "theorem_definition" {
            self.harvest_theorem_definition(node);
            return node.end_byte();
        }
        // `\newenvironment{name}{begindef}{enddef}` (and `\renewenvironment`)
        // parse as a dedicated `environment_definition` node. We can't replay
        // the LaTeX begin/end definitions in Typst, but dropping the env body
        // outright loses real content (text, `\label`s). Register `name` as a
        // transparent kind (empty-display sentinel) so any later
        // `\begin{name}...\end{name}` passes its body through instead of
        // warning + dropping. The definition node itself emits nothing (without
        // this arm its raw source leaks into the body).
        if node.kind() == "environment_definition" {
            self.harvest_environment_definition(node);
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
    ///
    /// In math mode, route a `curly_group` through the font-scope-aware
    /// slice walker so TeX font declarations (`\bf`, `\it`, ...) inside
    /// `{\bf X}` wrap subsequent siblings in `bold(...)` etc. The
    /// non-math path is unchanged.
    fn emit_recursive_with_gaps(&mut self, node: Node<'_>) -> usize {
        if self.in_math && node.kind() == "curly_group" {
            let mut cursor = node.walk();
            let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
            // The opening `{` and closing `}` need to land in the output
            // so Typst sees a balanced group. Emit them around the
            // scope-aware walk of the inner nodes.
            let start_skip = usize::from(matches!(children.first().map(|n| n.kind()), Some("{")));
            let end_skip = usize::from(matches!(children.last().map(|n| n.kind()), Some("}")));
            let inner_len = children.len().saturating_sub(start_skip + end_skip);
            if start_skip == 1 {
                self.out.push('{');
            }
            if inner_len > 0 {
                self.emit_math_node_slice(&children[start_skip..start_skip + inner_len]);
            }
            if end_skip == 1 {
                self.out.push('}');
            }
            return node.end_byte();
        }
        // Text-mode declarative font-switch group: `{\bf x}` / `{\em y}`. Wrap
        // the rest of the group in Typst markup and drop the pure-grouping
        // braces. Non-switch groups fall through to the default walk below.
        if node.kind() == "curly_group" {
            if let Some((wrap, switch_end)) = leading_font_switch(node, self.src) {
                return self.emit_font_switch_group(node, switch_end, wrap);
            }
        }
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

    // ─── Generic commands & macro expansion ───────────────────────────────────

    /// Handle `\newif` flag machinery: the `\newif\ifX` definition, the
    /// `\Xtrue`/`\Xfalse` setters, and `\ifX ... [\else ...] \fi` conditionals
    /// for flags defined via `\newif`. Returns `Some(resume_byte)` when `name`
    /// is newif machinery (emitting the taken branch and/or updating state),
    /// or `None` to fall through to normal command dispatch. TeX's builtin
    /// `\if`-family (`\ifx`, `\ifnum`, `\iftrue`, ...) is left untouched.
    fn try_newif_command(&mut self, node: Node<'_>, name: Option<&str>) -> Option<usize> {
        let name = name?;

        // Definition: `\newif\ifX` registers flag X (default false) and skips
        // past the `\ifX` token so it isn't emitted or warned on.
        if name == "\\newif" {
            if let Some((flag, flag_end)) = read_newif_flag(self.src, node.end_byte()) {
                self.newif_flags.entry(flag).or_insert(false);
                self.skip_until = self.skip_until.max(flag_end);
                return Some(flag_end);
            }
            return Some(node.end_byte());
        }

        let bare = name.strip_prefix('\\')?;

        // Setters: `\Xtrue` / `\Xfalse` for a known flag X. Emit nothing.
        if let Some(flag) = bare.strip_suffix("true") {
            if self.newif_flags.contains_key(flag) {
                self.newif_flags.insert(flag.to_string(), true);
                return Some(node.end_byte());
            }
        }
        if let Some(flag) = bare.strip_suffix("false") {
            if self.newif_flags.contains_key(flag) {
                self.newif_flags.insert(flag.to_string(), false);
                return Some(node.end_byte());
            }
        }

        // Conditional: `\ifX ... [\else ...] \fi` for a known flag X. Emit
        // only the taken branch (re-parsed) and skip the whole region.
        if let Some(flag) = bare.strip_prefix("if") {
            if let Some(&state) = self.newif_flags.get(flag) {
                if let Some(b) = find_conditional_bounds(self.src, node.end_byte()) {
                    let then_end = b.else_span.map(|(s, _)| s).unwrap_or(b.fi_start);
                    let kept = if state {
                        self.src[node.end_byte()..then_end].to_string()
                    } else if let Some((_, else_end)) = b.else_span {
                        self.src[else_end..b.fi_start].to_string()
                    } else {
                        String::new()
                    };
                    if !kept.trim().is_empty() {
                        let rendered = self.render_in_sub_emitter(&kept, self.in_math, false);
                        self.out.push_str(rendered.trim_end_matches('\n'));
                    }
                    self.skip_until = self.skip_until.max(b.fi_end);
                    return Some(b.fi_end);
                }
                // Unbalanced (no matching \fi): drop just the \ifX token.
                return Some(node.end_byte());
            }
        }

        None
    }

    fn emit_generic_command(&mut self, node: Node<'_>) -> usize {
        let name = command_name_text(node, self.src);

        if let Some(end) = self.try_newif_command(node, name.as_deref()) {
            return end;
        }

        // `\ensuremath{X}` — mode-aware inline math guard.
        // In math: render the argument directly (no extra `$` wrapper).
        // In text: wrap in Typst inline math `$...$`.
        // Previously handled as a macro seed with body `$#1$`, which caused
        // nested `$...$` when expanded inside math mode (Bug #49).
        if name.as_deref() == Some("\\ensuremath") {
            if let Some(arg) = first_curly_group(node) {
                let inner = self.render_math_group(arg);
                let inner = inner.trim();
                if self.in_math {
                    self.out.push_str(inner);
                } else {
                    self.out.push('$');
                    self.out.push_str(inner);
                    self.out.push('$');
                }
            }
            return node.end_byte();
        }

        if self.in_math {
            return self.emit_math_command(node, name.as_deref());
        }
        // `\verb<delim>content<delim>`: tree-sitter does not model the verb
        // delimiter scope, so we manually consume the source from the byte
        // after `\verb` to the next occurrence of the delimiter, and skip any
        // tokens the grammar produced inside.
        if name.as_deref() == Some("\\verb") || name.as_deref() == Some("\\verb*") {
            let bytes = self.src.as_bytes();
            let end = node.end_byte();
            if let Some(&delim) = bytes.get(end) {
                if let Some(rel) = bytes[end + 1..].iter().position(|&b| b == delim) {
                    let close = end + 1 + rel;
                    let content = &self.src[end + 1..close];
                    // Use #raw(...) rather than backtick syntax so the
                    // post_process_typography backtick-escape pass does not
                    // double-escape the delimiters.
                    let escaped = content.replace('\\', "\\\\").replace('"', "\\\"");
                    let _ = write!(self.out, "#raw(\"{}\")", escaped);
                    self.skip_until = close + 1;
                    return close + 1;
                }
            }
            self.warn_unsupported_command(node);
            return node.end_byte();
        }

        // `\path|...|` (path.sty) is verb-like: it typesets its
        // delimiter-bounded argument verbatim (allowing line breaks), so it is
        // rendered the same way as `\verb` — `#raw(...)`. Only the delimited
        // form is handled; tikz's `\path (a) -- (b);` form (whitespace, `(`, or
        // `{` immediately after the command) is left to warn rather than be
        // mis-read as verbatim. tikzpicture bodies are dropped elsewhere, so in
        // practice only path.sty's form reaches here.
        if name.as_deref() == Some("\\path") || name.as_deref() == Some("\\path*") {
            let bytes = self.src.as_bytes();
            let end = node.end_byte();
            if let Some(&delim) = bytes.get(end) {
                let is_verb_delim = !delim.is_ascii_whitespace()
                    && !delim.is_ascii_alphanumeric()
                    && !matches!(delim, b'{' | b'(' | b'[');
                if is_verb_delim {
                    if let Some(rel) = bytes[end + 1..].iter().position(|&b| b == delim) {
                        let close = end + 1 + rel;
                        let content = &self.src[end + 1..close];
                        let escaped = content.replace('\\', "\\\\").replace('"', "\\\"");
                        let _ = write!(self.out, "#raw(\"{}\")", escaped);
                        self.skip_until = close + 1;
                        return close + 1;
                    }
                }
            }
            self.warn_unsupported_command(node);
            return node.end_byte();
        }

        // `\lstinline` (listings) and `\mintinline` (minted) are INLINE verbatim
        // commands, like `\verb`, but with a prefix argument:
        //   `\lstinline[opts]{code}` / `\lstinline[opts]<delim>code<delim>`
        //   `\mintinline{lang}{code}` / `\mintinline{lang}<delim>code<delim>`
        // Emit inline `#raw("...")`, carrying `lang:` when a language is known
        // (lstlisting's block form is handled separately in
        // `emit_listing_environment`). The code is read straight from the source
        // bytes so tree-sitter never re-interprets it.
        if name.as_deref() == Some("\\lstinline") || name.as_deref() == Some("\\mintinline") {
            let is_mint = name.as_deref() == Some("\\mintinline");
            let bytes = self.src.as_bytes();
            // Scan from right after the command NAME, not `node.end_byte()`:
            // tree-sitter absorbs the first `{...}`/`[...]` group into the
            // generic_command node, so `end_byte()` already sits past the
            // `{lang}`/`{code}` we need to read. The command name in the source
            // is exactly the matched `name` string (`\lstinline`/`\mintinline`).
            let name_len = name.as_deref().map_or(0, str::len);
            let mut i = node.start_byte() + name_len;
            let mut lang: Option<String> = None;

            if is_mint {
                // minted: the first mandatory `{...}` group is the language.
                if bytes.get(i) == Some(&b'{') {
                    let close = skip_balanced_braces(self.src, i);
                    let l = self.src[i + 1..close.saturating_sub(1)].trim();
                    if !l.is_empty() {
                        lang = Some(l.to_lowercase());
                    }
                    i = close;
                }
            } else if bytes.get(i) == Some(&b'[') {
                // listings: optional `[key=val,...]`, possibly with `language=`.
                if let Some(rel) = self.src[i..].find(']') {
                    let close = i + rel;
                    lang = self.src[i + 1..close].split(',').find_map(|kv| {
                        kv.trim()
                            .strip_prefix("language")
                            .and_then(|r| r.trim().strip_prefix('='))
                            .map(|v| {
                                v.trim()
                                    .trim_matches(|c| c == '{' || c == '}')
                                    .to_lowercase()
                            })
                            .filter(|v| !v.is_empty())
                    });
                    i = close + 1;
                }
            }

            // Read the verbatim code: a balanced `{...}` group, or a verb-style
            // `<delim>...<delim>` run (any non-alphanumeric, non-space delimiter).
            let body: Option<(String, usize)> = match bytes.get(i) {
                Some(&b'{') => {
                    let close = skip_balanced_braces(self.src, i);
                    Some((self.src[i + 1..close.saturating_sub(1)].to_string(), close))
                }
                Some(&delim) if !delim.is_ascii_whitespace() && !delim.is_ascii_alphanumeric() => {
                    bytes[i + 1..].iter().position(|&b| b == delim).map(|rel| {
                        let close = i + 1 + rel;
                        (self.src[i + 1..close].to_string(), close + 1)
                    })
                }
                _ => None,
            };

            if let Some((code, end)) = body {
                let escaped = code.replace('\\', "\\\\").replace('"', "\\\"");
                match lang {
                    Some(l) => {
                        let _ = write!(self.out, "#raw(\"{}\", lang: \"{}\")", escaped, l);
                    }
                    None => {
                        let _ = write!(self.out, "#raw(\"{}\")", escaped);
                    }
                }
                // Skip past whatever we consumed AND whatever tree-sitter folded
                // into the node, so neither the code nor an absorbed `{...}` is
                // re-emitted.
                let consumed = end.max(node.end_byte());
                self.skip_until = consumed;
                return consumed;
            }
            self.warn_unsupported_command(node);
            return node.end_byte();
        }

        // `\bibitem{key}` inside `thebibliography` becomes a `#figure(...)`
        // with a custom kind so that `@key` references resolve. Typst only
        // allows labels to be referenced on a few element kinds — `figure`
        // with `supplement: none` is the least-intrusive.
        //
        // tree-sitter-latex parses `\bibitem[Agr02]{Agr:Foo}` (optional
        // bracket present) with the `[...]` and `{...}` as AST siblings
        // of the generic_command rather than children. Source-byte
        // peek catches that case — same shape as PR #27's
        // `\xrightarrow` fix.
        if name.as_deref() == Some("\\bibitem") {
            let mut key: Option<String> = None;
            let mut consumed_end = node.end_byte();
            if let Some(arg) = first_curly_group(node) {
                let k = self
                    .src
                    .get(arg.start_byte() + 1..arg.end_byte() - 1)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !k.is_empty() {
                    key = Some(k);
                }
            } else {
                // AST-sibling fallback: scan source bytes after the
                // command for optional `[...]` then mandatory `{...}`.
                let bytes = self.src.as_bytes();
                let mut i = node.end_byte();
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                // Skip optional `[label]`.
                if i < bytes.len() && bytes[i] == b'[' {
                    let mut j = i + 1;
                    let mut depth = 0i32;
                    while j < bytes.len() {
                        match bytes[j] {
                            b'\\' if j + 1 < bytes.len() => {
                                j += 2;
                                continue;
                            }
                            b'{' => depth += 1,
                            b'}' => depth -= 1,
                            b']' if depth == 0 => break,
                            _ => {}
                        }
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b']' {
                        i = j + 1;
                        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                            i += 1;
                        }
                    }
                }
                if i < bytes.len() && bytes[i] == b'{' {
                    let inner_start = i + 1;
                    let mut j = inner_start;
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
                    if j < bytes.len() && bytes[j] == b'}' {
                        let k = self.src[inner_start..j].trim().to_string();
                        if !k.is_empty() {
                            key = Some(k);
                            consumed_end = j + 1;
                        }
                    }
                }
            }
            if let Some(k) = key {
                // A resolvable `\bibliography{.bib}` is authoritative; drop this
                // manual entry so its `<key>` doesn't collide with the .bib
                // (the key is already validated via the .bib harvest).
                if self.bib_file_is_authoritative() {
                    self.close_bibitem();
                    if consumed_end > node.end_byte() {
                        self.skip_until = self.skip_until.max(consumed_end);
                    }
                    return consumed_end;
                }
                self.close_bibitem();
                if !self.out.ends_with('\n') {
                    self.out.push('\n');
                }
                self.out
                    .push_str("#figure(kind: \"bibitem\", supplement: none, [");
                // Sanitize the bibitem key so Typst accepts the `<key>`
                // label syntax; cite/ref use sites apply the same
                // transformation so the labels still match.
                let sanitized = sanitize_label_key(&k);
                // Record the key so `emit_citation` knows it's defined.
                self.bibliography_keys.insert(sanitized.clone());
                self.pending_bibitem_key = Some(sanitized);
                if consumed_end > node.end_byte() {
                    self.skip_until = self.skip_until.max(consumed_end);
                }
                return consumed_end;
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
            // `\tabularnewline` is a LaTeX synonym for `\\` inside tabular
            // environments — handled the same way. Real arXiv papers use
            // it (28 occurrences on 2605.22507 alone) to avoid the
            // overloading ambiguity of `\\` at the end of optional-arg
            // brackets.
            Some("\\\\") | Some("\\tabularnewline") => {
                if self.in_minipage {
                    // Intra-minipage line break — emit a Typst `#linebreak()`
                    // rather than the bare `\` the table row-splitter keys on,
                    // so a minipage used as a table cell isn't split into rows.
                    self.out.push_str("#linebreak()");
                } else {
                    if !self.out.ends_with(' ') && !self.out.ends_with('\n') {
                        self.out.push(' ');
                    }
                    self.out.push('\\');
                }
                // `\\[len]` carries an optional vertical-space argument. The
                // grammar does NOT attach the `[len]` to this node (it surfaces
                // as following raw text), so consume it from the source here —
                // otherwise it leaks as `\[len\]` into the next table cell and
                // breaks the surrounding Typst content block. The bracketed
                // length has no Typst analog in a row break and is dropped.
                let end = node.end_byte();
                if self.src.as_bytes().get(end) == Some(&b'[') {
                    if let Some(rel) = self.src[end..].find(']') {
                        let consumed = end + rel + 1;
                        self.skip_until = self.skip_until.max(consumed);
                        return consumed;
                    }
                }
                end
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
            // Text-mode symbol commands — emit the Unicode character directly.
            Some("\\texttimes") => {
                self.out.push('×');
                node.end_byte()
            }
            Some("\\textuparrow") => {
                self.out.push('↑');
                node.end_byte()
            }
            Some("\\textdownarrow") => {
                self.out.push('↓');
                node.end_byte()
            }
            Some("\\checkmark") => {
                self.out.push('✓');
                node.end_byte()
            }
            Some("\\AA") => {
                self.out.push('Å');
                node.end_byte()
            }
            Some("\\l") => {
                self.out.push('ł');
                node.end_byte()
            }
            // `\newline` — explicit line break outside a table.
            Some("\\newline") => {
                self.out.push_str("\\ \n");
                node.end_byte()
            }
            // More text-mode symbol commands.
            Some("\\textless") => {
                self.out.push('<');
                node.end_byte()
            }
            Some("\\textgreater") => {
                self.out.push('>');
                node.end_byte()
            }
            Some("\\ldots") | Some("\\dots") | Some("\\textellipsis") => {
                self.out.push('…');
                node.end_byte()
            }
            Some("\\slash") => {
                self.out.push('/');
                node.end_byte()
            }
            // `\today` — insert the current date at conversion time.
            Some("\\today") => {
                self.out.push_str(
                    "#datetime.today().display(\"[month repr:long] [day], [year]\")",
                );
                node.end_byte()
            }
            // `\phantom{X}` — reserves space equal to X but renders nothing.
            // No Typst equivalent; drop silently to preserve surrounding content.
            Some("\\phantom") | Some("\\hphantom") | Some("\\vphantom") => node.end_byte(),
            // `\relax` — TeX no-op primitive; silently consumed.
            Some("\\relax") => node.end_byte(),
            // `\par` — explicit paragraph break; emit Typst blank-line equivalent.
            Some("\\par") => {
                self.out.push_str("\n\n");
                node.end_byte()
            }
            // `\footnotemark` — superscript footnote counter reference. Without
            // coordinated `\footnotetext` tracking, emit a footnote placeholder.
            Some("\\footnotemark") => {
                self.out.push_str("#footnote[]");
                node.end_byte()
            }
            // `\mathtt{X}` in text mode — typewriter math font; render as code.
            Some("\\mathtt") => self.emit_inline_raw(node),
            // Deprecated font-switching commands (LaTeX 2.09 style). These change
            // the style of all following text until the group ends — Typst would
            // need a scope wrap, which requires end-of-group tracking we don't yet
            // have. Warn so the caller can see the loss.
            // Declarative font switches (TeX 2.09 + NFSS forms). The common
            // `{\bf x}` / `{\em y}` grouped form is wrapped in Typst markup at
            // the `curly_group` level (see emit_node, where both braces can be
            // dropped). Reaching one HERE means it's bare or mid-group, where
            // the scope can't be bounded cleanly — drop it silently (no
            // warning); the text still flows through.
            Some("\\bf") | Some("\\bfseries") | Some("\\em") | Some("\\it")
            | Some("\\itshape") | Some("\\sl") | Some("\\slshape") | Some("\\sf")
            | Some("\\rm") | Some("\\tt") | Some("\\sc") | Some("\\mdseries")
            | Some("\\upshape") | Some("\\scshape") | Some("\\rmfamily")
            | Some("\\sffamily") | Some("\\ttfamily") | Some("\\normalfont")
            | Some("\\boldmath") | Some("\\unboldmath") => node.end_byte(),
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
            // `\newblock` separates the blocks of a bibliography entry
            // (author / title / publication). It carries no content and has no
            // Typst equivalent — drop it silently, leaving the surrounding
            // whitespace so the reference parts stay separated.
            Some("\\newblock") => node.end_byte(),
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
                if !self.in_math =>
            {
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            // Forced page breaks — Typst's pagination is automatic; warn so the
            // user knows their explicit layout intent was not preserved.
            //
            // tree-sitter-latex sometimes attaches the following `{...}` group
            // as an argument to these argument-less commands (e.g.
            // `\newpage\n\n{\bibliography{refs}}`). We must emit that group's
            // content; only the command name itself is dropped.
            Some("\\pagebreak")
            | Some("\\nopagebreak")
            | Some("\\newpage")
            | Some("\\clearpage")
            | Some("\\cleardoublepage")
                if !self.in_math =>
            {
                self.warn_silently_dropped(node);
                // tree-sitter-latex sometimes attaches the following `{...}` group
                // as an argument to these argument-less commands (e.g.
                // `\newpage\n\n{\bibliography{refs}}`). Emit any `curly_group`
                // children so that content (bibliography, etc.) is preserved.
                let mut cursor = node.walk();
                let groups: Vec<_> = node
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "curly_group")
                    .collect();
                for g in &groups {
                    self.emit_node(*g);
                }
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            // Layout-only alignment directives — warn so the user knows their
            // alignment intent was not preserved.
            Some("\\centering")
            | Some("\\raggedright")
            | Some("\\raggedleft")
            | Some("\\justify")
            | Some("\\flushleft")
            | Some("\\flushright") => {
                self.warn_silently_dropped(node);
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            // Float/figure placement specifiers + page-style controls. These are
            // inert in Typst and carry no visible content — drop silently.
            Some("\\setcounter")
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
                node.end_byte()
            }
            // `\geometry{key=val,...}` — page margins / paper size. Parse the
            // raw argument into the layout (same keys as the geometry package
            // options); drop the command itself.
            Some("\\geometry") => {
                if let Some(arg) = first_curly_group(node) {
                    let raw = self.curly_group_inner_trimmed(arg);
                    self.layout.apply_geometry(raw);
                }
                node.end_byte()
            }
            // `\makeatletter ... \makeatother` brackets low-level TeX where `@`
            // is a letter: internal macro definitions, `\newcount`, `\csname`,
            // counter resets like `\rc@count=1`. tree-sitter-latex can't parse
            // these primitives, so their fragments (`=1`, `{}`, `rc@X@#1`) leak
            // verbatim through the default copy path. The region never produces
            // renderable body content, so skip it wholesale by jumping the
            // cursor past the matching `\makeatother` (via `skip_until`).
            //
            // But definitions inside the region (macros, `\def`, `\let`,
            // `\newtheorem`, `\newif` flags, tcolorbox/siam envs) must still be
            // registered before we stop walking it — the emit walk normally does
            // that node-by-node as it renders, and `\input` child emitters have
            // no prepass to fall back on (so e.g. a `\newcommand` inside a
            // `\makeatletter` block of an `\input`ed file would otherwise be
            // dropped). `harvest_definitions` re-parses the region and registers
            // them, parent-wins.
            Some("\\makeatletter") => {
                if let Some(end) = find_makeatother_end(self.src, node.end_byte()) {
                    let region = &self.src[node.end_byte()..end];
                    self.harvest_definitions(region);
                    self.skip_until = self.skip_until.max(end);
                    end
                } else if !self.saw_document_class {
                    // Unmatched `\makeatletter` in a fragment with no
                    // `\documentclass` (e.g. an `\input`ed macro helper relying
                    // on the at-letter catcode persisting to end of input). The
                    // remainder is internals — harvest its definitions, then skip
                    // to EOF so the low-level TeX doesn't leak.
                    let rest = &self.src[node.end_byte()..];
                    self.harvest_definitions(rest);
                    self.skip_until = self.src.len();
                    self.src.len()
                } else {
                    // Unmatched, but inside a full document — stay conservative
                    // and drop only the lone token; the body and `\title` /
                    // `\author` metadata that follow must still be processed.
                    node.end_byte()
                }
            }
            // Preamble plumbing with no visible rendered effect — drop silently.
            // • Debug/logging: \typeout writes to the .log; no output.
            // • Theorem styles: \theoremstyle{plain|definition|…} — Typst theorem
            //   environments don't have a style parameter in our emission layer.
            // • cleveref naming: \crefname / \Crefname configure label format only.
            // • hyperref setup: \hypersetup{key=val,…} — PDF metadata, not content.
            // • Paragraph layout hints: \enlargethispage, \looseness have no Typst
            //   equivalent; Typst auto-handles line/page breaking.
            // • TeX low-level: \endcsname, \expandafter, \makeatletter are TeX
            //   engine directives that tree-sitter surfaces as generic_command.
            // • Column formatting: \addlinespace is a booktabs spacing hint; Typst
            //   table auto-handles row spacing.
            // • Hook registration: \AddToHook is LaTeX3 machinery with no Typst
            //   equivalent.
            // • Float barriers: \FloatBarrier (placeins) forces floats before the
            //   current point; Typst places figures where #figure() is called.
            // • Color definitions: \colorlet defines a colour alias; without colortbl
            //   support the alias is never used, so the definition is inert.
            //   (`\definecolor` is a dedicated `color_definition` node — dropped
            //   earlier in emit_node, near the `color_reference` arm.)
            // • Conditionals: \ifthenelse/\fi/\else are xcolor/ifthen preamble
            //   control flow that tree-sitter surfaces as bare generic_commands
            //   (the contained body is processed separately by tree-sitter's normal
            //   child walk). Dropping these tokens is safe; the content nodes are
            //   emitted normally.
            Some("\\typeout")
            | Some("\\theoremstyle")
            | Some("\\crefname")
            | Some("\\Crefname")
            | Some("\\hypersetup")
            | Some("\\enlargethispage")
            | Some("\\looseness")
            | Some("\\endcsname")
            | Some("\\expandafter")
            | Some("\\makeatother")
            | Some("\\addlinespace")
            | Some("\\AddToHook")
            | Some("\\FloatBarrier")
            | Some("\\colorlet")
            | Some("\\ifthenelse")
            | Some("\\fi")
            | Some("\\else")
            // TeX conditionals surfaced as generic_command
            | Some("\\ifpdf")
            | Some("\\ifdefined")
            | Some("\\ifcsname")
            | Some("\\ifxetex")
            | Some("\\ifluatex")
            | Some("\\ifx")
            | Some("\\if")
            | Some("\\ifdim")
            | Some("\\ifnum")
            // PGF/pgfplots preamble setup — no visible rendered body.
            | Some("\\pgfplotsset")
            | Some("\\pgfplotscreateplotcyclelist")
            | Some("\\pgfplotscreatecyclelist")
            // titlesec / fancyhdr / titling formatting
            | Some("\\titlespacing")
            | Some("\\titlespacing*")
            | Some("\\titleformat")
            | Some("\\fancyhead")
            | Some("\\fancyfoot")
            | Some("\\fancypagestyle")
            | Some("\\renewpagestyle")
            // Index / nomenclature / glossary setup calls — no visible body output.
            // \printindex / \printnomenclature / \printglossary warn via DropOnly below.
            | Some("\\index")
            | Some("\\nomenclature")
            | Some("\\makenomenclature")
            | Some("\\makeglossaries")
            // Document structure helpers with no Typst equivalent
            | Some("\\numberwithin")
            | Some("\\makeindex")
            | Some("\\RequirePackage")
            | Some("\\DeclareGraphicsExtensions")
            | Some("\\DeclareGraphicsRule")
            | Some("\\DeclareRobustCommand")
            | Some("\\DeclareOption")
            | Some("\\ExecuteOptions")
            | Some("\\ProcessOptions")
            | Some("\\ProcessList")
            // TeX low-level primitives
            | Some("\\csname")
            | Some("\\global")
            | Some("\\long")
            | Some("\\outer")
            // Conference-specific conditionals
            | Some("\\ificmlshowauthors")
            | Some("\\ifanonymous")
            // titling / SIAM header commands
            | Some("\\headers")
            | Some("\\titrun")
            | Some("\\titlerunning")
            | Some("\\authorrunning")
            // tcolorbox theorem-env declarations — handled below so their
            // bodies are passed through rather than warned on.
            // Some("\\newtcolorbox") | Some("\\newmdenv") — see dedicated arms.
            // \newmdtheoremenv: theorem-like, but display name is in 2nd arg
            // (same shape as \newtheorem).
            | Some("\\newmdtheoremenv")
            // caption / subfigure setup
            | Some("\\captionsetup")
            | Some("\\DeclareCaptionFont")
            | Some("\\DeclareCaptionStyle")
            // misc no-effect preamble
            | Some("\\setlist")
            | Some("\\sisetup")
            | Some("\\lstset")
            | Some("\\tcbuselibrary")
            // TeX output / engine primitives
            | Some("\\pdfoutput")
            | Some("\\pdfcompresslevel")
            | Some("\\pdfobjcompresslevel")
            | Some("\\string")
            // Springer/LNCS running-head variants (content appears in full \title / \author)
            | Some("\\title*")
            | Some("\\author*")
            // Springer abstract variant
            | Some("\\abstract*")
            // TOC entry injection — Typst auto-generates ToC from headings so
            // manual \addcontentsline calls are unnecessary.
            | Some("\\addcontentsline")
            | Some("\\addtocontents")
            // setspace body commands — line spacing is controlled via
            // `set par(leading: ...)` in Typst; these switch commands are noops.
            | Some("\\doublespacing")
            | Some("\\singlespacing")
            | Some("\\onehalfspacing")
            | Some("\\setstretch")
            // colortbl / xcolor table-coloring commands — both packages are
            // in the noop allowlist; Typst has no direct row/cell fill API
            // at the command level so these are silently dropped.
            | Some("\\rowcolor")
            | Some("\\cellcolor")
            | Some("\\columncolor")
            | Some("\\arrayrulecolor")
            | Some("\\doublerulesepcolor")
            // Orphaned \begin{X} / \end{X}: tree-sitter-latex did not match
            // the open/close pair (e.g. the snippet ends before \end{document}
            // or starts after \begin{document}), so these tokens appear as
            // generic_command nodes.  Silently drop — they are structural
            // markers with no renderable content.
            | Some("\\begin")
            | Some("\\end")
            // Beamer presentation-layer styling — no Typst equivalent.
            // Silently drop: theme/color/font commands are presentation-only
            // and the underlying content (if any) is preserved elsewhere.
            | Some("\\usetheme")
            | Some("\\usecolortheme")
            | Some("\\useinnertheme")
            | Some("\\useoutertheme")
            | Some("\\usebeamertheme")
            | Some("\\usebeamercolor")
            | Some("\\usebeamerfont")
            | Some("\\setbeamertemplate")
            | Some("\\setbeamerfont")
            | Some("\\setbeamercolor")
            | Some("\\setbeamercovered")
            | Some("\\AtBeginSection")
            | Some("\\AtBeginSubsection")
            | Some("\\titlegraphic")
            // \subtitle is beamer's title-block subtitle; no Typst subtitle
            // slot is maintained, so silently drop the content.
            | Some("\\subtitle") => {
                node.end_byte()
            }

            // Standard LaTeX counter display commands — emit as Typst context
            // counter expressions.  These never take arguments so they are
            // a single token; the `#` prefix works in both markup and math mode.
            Some("\\thepage") => {
                self.out.push_str("#context counter(page).display()");
                node.end_byte()
            }
            Some("\\thesection") => {
                self.out.push_str("#context counter(heading.1).display()");
                node.end_byte()
            }
            Some("\\thesubsection") => {
                self.out.push_str("#context counter(heading.2).display()");
                node.end_byte()
            }
            Some("\\thesubsubsection") => {
                self.out.push_str("#context counter(heading.3).display()");
                node.end_byte()
            }
            Some("\\thechapter") => {
                // Chapters are top-level headings in Typst.
                self.out.push_str("#context counter(heading.1).display()");
                node.end_byte()
            }
            Some("\\thefigure") => {
                self.out.push_str("#context counter(figure).display()");
                node.end_byte()
            }
            Some("\\thetable") => {
                self.out.push_str("#context counter(figure.where(kind: table)).display()");
                node.end_byte()
            }
            Some("\\theequation") => {
                self.out.push_str("#context counter(math.equation).display()");
                node.end_byte()
            }

            // `\newsiamremark` / `\newsiamthm` (SIAM theorem declarations) —
            // harvest `{name}{Display}` into theorem_kinds so the env is routed
            // correctly when encountered in the body.
            Some("\\newsiamremark") | Some("\\newsiamthm") => {
                self.harvest_generic_theorem_cmd(node, self.src);
                node.end_byte()
            }
            // \newtcolorbox{name}{opts} / \newmdenv{name}{opts}: harvest the
            // env name only. The body of any `\begin{name}...\end{name}` is
            // then passed through transparently (empty display = transparent sentinel).
            Some("\\newtcolorbox") | Some("\\newmdenv") => {
                self.harvest_tcolorbox_decl(node, self.src);
                node.end_byte()
            }
            // `\newcommandx` (xargs/xargspec package) parses as a bare
            // generic_command with only its command_name child; the
            // `\name[N][K=def]{body}` definition lands as sibling
            // nodes. Bump `skip_until` past those siblings so we don't
            // leak the raw definition into the output.
            Some("\\newcommandx") => {
                if let Some((_n, def_end)) =
                    extract_newcommandx_and_end(node, self.src)
                {
                    self.skip_until = self.skip_until.max(def_end);
                    return def_end;
                }
                node.end_byte()
            }
            // Macro (re)definitions in text mode — warn because the user may
            // have redefined a command that the conversion depends on.
            Some("\\renewcommand") | Some("\\providecommand") => {
                self.warn_silently_dropped(node);
                node.end_byte()
            }
            // ACM publication-metadata (display-only administrative fields) —
            // no visible author content, drop silently.
            Some("\\setcopyright") | Some("\\copyrightyear") | Some("\\acmYear") => {
                node.end_byte()
            }
            // ACM fields that carry real visible content in the published paper —
            // warn so the user knows they were not rendered.
            Some("\\acmConference")
            | Some("\\acmBooktitle")
            | Some("\\acmDOI")
            | Some("\\acmISBN")
            | Some("\\acmPrice")
            | Some("\\acmSubmissionID") => {
                self.warn_silently_dropped(node);
                node.end_byte()
            }
            // Per-author sibling-scope attribution (elsearticle / authblk pattern).
            // Commands like `\author{Alice}\email{a@x} \author{Bob}\email{b@y}`
            // place per-author fields as siblings of `\author{}` rather than
            // nested inside it. Append them as raw LaTeX to the most recently
            // seen \author{} buffer so parse_one_author picks them up at
            // finish-time. When no \author{} has been seen yet, fall through to
            // class_metadata (orphan / global scope).
            Some("\\email")
            | Some("\\orcid")
            | Some("\\orcidID")
            | Some("\\affiliation")
            | Some("\\affil")
            | Some("\\address")
            | Some("\\institution")
            | Some("\\institute") => {
                if !self.raw_authors.is_empty() {
                    if let Some(arg) = first_curly_like(node) {
                        let cmd = command_name_text(node, self.src).unwrap_or_default();
                        let inner = self
                            .src
                            .get(arg.start_byte() + 1..arg.end_byte().saturating_sub(1))
                            .unwrap_or("")
                            .to_string();
                        if let Some(last) = self.raw_authors.last_mut() {
                            last.push(' ');
                            last.push_str(&cmd);
                            last.push('{');
                            last.push_str(&inner);
                            last.push('}');
                        }
                    }
                } else {
                    // No author context — fall back to class_metadata so
                    // external callers can still inspect the value.
                    if let Some(key) = command_name_text(node, self.src) {
                        let field = key.trim_start_matches('\\').to_string();
                        if let Some(arg) = first_curly_like(node) {
                            let content = self.render_curly_group_content(arg);
                            self.metadata.class_metadata.entry(field).or_insert(content);
                        }
                    }
                    self.warn_unsupported_command(node);
                }
                node.end_byte()
            }
            // ACM/authblk author-info fields that don't have a per-author
            // counterpart. Capture into class_metadata for external callers.
            Some("\\city")
            | Some("\\country")
            | Some("\\state")
            | Some("\\streetaddress")
            | Some("\\postcode")
            | Some("\\authornote")
            | Some("\\additionalaffiliation")
            | Some("\\ccsdesc")
            | Some("\\shortauthors")
            | Some("\\funding") => {
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
            // `\keywords{a, b, c}` and `\IEEEkeywords{...}` — always capture
            // into the title-block field. Template classes render them via the
            // show_call slot; the rich native renderer in flush_title_block
            // renders them for Unknown/Lncs/SvMult classes.
            Some("\\keywords") | Some("\\IEEEkeywords") => {
                if let Some(arg) = first_curly_like(node) {
                    let rendered = self.render_curly_group_content(arg);
                    self.metadata.keywords = rendered
                        .split(',')
                        .map(|k| k.trim().to_string())
                        .filter(|k| !k.is_empty())
                        .collect();
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
            // Tables-of-contents et al. — Typst equivalents not yet emitted; warn
            // so the user knows these structural sections were not preserved.
            Some("\\tableofcontents")
            | Some("\\listoffigures")
            | Some("\\listoftables")
            | Some("\\printbibliography")
            | Some("\\printindex")
            // Nomenclature / glossary output commands — generated list is lost.
            | Some("\\printnomenclature")
            | Some("\\printglossary")
            | Some("\\printglossaries")
            // Book-class structure dividers — affect page numbering / heading
            // numbering in ways Typst doesn't model; warn so users are aware.
            | Some("\\frontmatter")
            | Some("\\mainmatter")
            | Some("\\backmatter") => {
                self.warn_silently_dropped(node);
                consume_trailing_inline_space(self.src, node.end_byte())
            }
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
            // \hologo{Name} / \Hologo{Name}: hologo package function form.
            // The argument is the logo identifier; map known names to plain
            // text (same output as the existing dedicated logo commands above).
            Some("\\hologo") | Some("\\Hologo") => self.emit_hologo(node),
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
            // `\graphicspath{{dir1/}{dir2/}}` — register the search dirs so bare
            // `\includegraphics{name}` resolves against them (D7). Renders
            // nothing; the dirs feed `emit_graphics_include`'s probe list.
            Some("\\graphicspath") => {
                if let Some(arg) = first_curly_group(node) {
                    let raw = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte().saturating_sub(1))
                        .unwrap_or("");
                    for dir in parse_graphicspath_dirs(raw) {
                        if !self.graphics_paths.contains(&dir) {
                            self.graphics_paths.push(dir);
                        }
                    }
                }
                node.end_byte()
            }
            Some("\\author") => {
                // Raw-bytes capture — same rationale as `author_declaration` above.
                if let Some(arg) = first_curly_group(node) {
                    let inner = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte().saturating_sub(1))
                        .unwrap_or("")
                        .to_string();
                    self.raw_authors.push(inner);
                }
                node.end_byte()
            }
            // ── ICML author block (icml20XX.sty) ──────────────────────────────
            // These commands are full TeX (`\ifcsname`, `\csname`, counters,
            // `\@for`) that the text-substitution macro expander cannot evaluate.
            // Harvested from the .sty and expanded, they leak raw machinery
            // (`@icmlsymbolequal`, `\@affil\@anon`, `\stepcounter{...}`) into the
            // body — and the stray `@icmlsymbolequal` tripped `typst` with
            // `label <icmlsymbolequal> does not exist` (Bug B, paper 2605.22579).
            // Intercept them here, BEFORE the harvested-macro fallback below, to
            // capture the author names and drop the unparseable machinery.
            Some("\\icmltitle") => {
                if self.metadata.title.is_none() {
                    if let Some(arg) = first_curly_group(node) {
                        self.metadata.title =
                            Some(Content::Typst(self.render_curly_group_content(arg)));
                    }
                }
                node.end_byte()
            }
            // `\icmlauthor{Name}{affil-keys}` — keep the name (first arg) as a
            // raw author entry (parsed by class_map for DocClass::Icml); the
            // affiliation-key list (second arg) maps onto machinery we drop.
            Some("\\icmlauthor") => {
                if let Some(arg) = first_curly_group(node) {
                    let name = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte().saturating_sub(1))
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !name.is_empty() {
                        self.raw_authors.push(name);
                    }
                }
                node.end_byte()
            }
            // The remaining ICML author-block directives carry no body-renderable
            // content (affiliation tables, symbol definitions, the notice
            // footnote). Drop them — their arguments are children of the
            // generic_command node, so returning end_byte() consumes them whole.
            Some("\\icmlaffiliation")
            | Some("\\icmlsetsymbol")
            | Some("\\icmlcorrespondingauthor")
            | Some("\\icmlkeywords")
            | Some("\\icmlEqualContribution")
            | Some("\\printAffiliationsAndNotice") => node.end_byte(),
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
                    let url = self.curly_group_inner_trimmed(groups[0]);
                    let display = self.render_curly_group_content(groups[1]);
                    let _ = write!(self.out, "#link(\"{}\")[{}]", url, display);
                } else if let Some(arg) = first_curly_group(node) {
                    let url = self.curly_group_inner_trimmed(arg);
                    let _ = write!(self.out, "#link(\"{}\")", url);
                }
                node.end_byte()
            }
            // `\url{X}` → bare link in Typst.
            Some("\\url") => {
                if let Some(arg) = first_curly_group(node) {
                    let url = self.curly_group_inner_trimmed(arg);
                    let _ = write!(self.out, "#link(\"{}\")", url);
                }
                node.end_byte()
            }
            // `\nolinkurl{URL}` → monospace raw (no hyperlink; same as \texttt).
            Some("\\nolinkurl") => self.emit_inline_raw(node),
            // `\hyperlink{id}{text}` / `\hypertarget{id}{text}` — emit visible
            // text; drop the hyperlink id (Typst cross-references require @label
            // syntax which needs coordinated target/source changes).
            Some("\\hyperlink") | Some("\\hypertarget") => {
                let mut cursor = node.walk();
                let groups: Vec<Node<'_>> = node
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "curly_group")
                    .collect();
                if groups.len() >= 2 {
                    let content = self.render_curly_group_content(groups[1]);
                    self.out.push_str(&content);
                } else if let Some(arg) = first_curly_group(node) {
                    let content = self.render_curly_group_content(arg);
                    self.out.push_str(&content);
                }
                node.end_byte()
            }
            // Font-size directives — unscoped toggles. Typst's equivalent
            // would be a #text(size: …)[…] wrap but that needs end-of-group
            // tracking we don't yet have. Silently drop so papers don't accumulate
            // dozens of low-signal warnings (one per paragraph size switch).
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
                    let raw = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    let key = sanitize_label_key(raw);
                    if self.in_math {
                        // Inside math the bare label attaches to the equation,
                        // which is referenceable — keep the existing behaviour.
                        let _ = write!(self.out, " <{}>", key);
                    } else {
                        // In text / list / inline context a bare `<key>` would
                        // attach to non-referenceable paragraph text (Typst
                        // aborts with "cannot reference text"), or — when the
                        // enclosing env was dropped — never be emitted at all
                        // ("label does not exist"). Emit a hidden, self-numbered
                        // anchor figure instead: it IS referenceable and, thanks
                        // to the `kind: "anchor"` show rule in `finish()`, renders
                        // nothing. Its own per-kind counter leaves real figure/
                        // table numbering untouched.
                        self.used_text_label_anchor = true;
                        let _ = write!(
                            self.out,
                            " #box[#figure(kind: \"anchor\", supplement: none, numbering: \"1\", [])<{}>]",
                            key
                        );
                    }
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
        // Walk the call's children once, collecting brack_groups
        // (optional args) and curly_groups (mandatory args) in source
        // order. Both lists feed the per-position resolution below.
        let mut cursor = node.walk();
        let mut brack_args: Vec<String> = Vec::new();
        let mut curly_args: Vec<String> = Vec::new();
        for c in node.children(&mut cursor) {
            match c.kind() {
                "brack_group" => {
                    brack_args.push(
                        self.src
                            .get(c.start_byte() + 1..c.end_byte() - 1)
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                "curly_group" => {
                    curly_args.push(
                        self.src
                            .get(c.start_byte() + 1..c.end_byte() - 1)
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                _ => {}
            }
        }
        // Source-byte peek for an immediately-following `[optional]` —
        // tree-sitter sometimes attaches it as an AST sibling rather
        // than a child of the generic_command. Same pattern PR #27
        // proved out for `\xrightarrow[g]{f}`.
        let mut consumed_end = node.end_byte();
        if !macro_def.optional_defaults.is_empty() && brack_args.is_empty() {
            let bytes = self.src.as_bytes();
            let mut i = consumed_end;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'[' {
                let inner_start = i + 1;
                let mut j = inner_start;
                let mut depth = 0i32;
                while j < bytes.len() {
                    match bytes[j] {
                        b'\\' if j + 1 < bytes.len() => {
                            j += 2;
                            continue;
                        }
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b']' if depth == 0 => break,
                        _ => {}
                    }
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b']' {
                    brack_args.push(self.src[inner_start..j].to_string());
                    consumed_end = j + 1;
                }
            }
        }
        // Resolve each parameter position from defaults + call args.
        // Positions in `optional_defaults` consume from brack_args (in
        // 1-indexed sorted order); other positions consume from
        // curly_args (in order). Missing brack_args fall back to the
        // captured default.
        let mut args: Vec<String> = Vec::with_capacity(macro_def.params);
        if macro_def.optional_defaults.is_empty() {
            // Fast path — no optional args, behave exactly as before.
            args.extend(curly_args.iter().cloned());
        } else {
            let mut optional_positions: Vec<usize> =
                macro_def.optional_defaults.keys().copied().collect();
            optional_positions.sort();
            let mut brack_iter = brack_args.iter();
            let mut curly_iter = curly_args.iter();
            for pos in 1..=macro_def.params {
                if optional_positions.binary_search(&pos).is_ok() {
                    match brack_iter.next() {
                        Some(v) => args.push(v.clone()),
                        None => args.push(
                            macro_def
                                .optional_defaults
                                .get(&pos)
                                .cloned()
                                .unwrap_or_default(),
                        ),
                    }
                } else if let Some(v) = curly_iter.next() {
                    args.push(v.clone());
                }
            }
        }
        // If the call site has fewer curly_groups than the macro expects,
        // try LaTeX's brace-less calling convention: read the next N
        // tokens from the raw source (`\name`, `{group}`, or one char).
        // Real arXiv papers heavily rely on this — `$\mat X$`, `\vec a`,
        // `\rvec \alpha`. Without it every such call site is dropped with
        // a `custom_macro` warning.
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
        let mut expanded = substitute_macro_args(&macro_def.body, &args[..macro_def.params]);
        // If the call site provided MORE curly_group args than the
        // macro declares (`params`), append the excess to the body
        // before re-parsing. This handles macros whose body ends in a
        // dangling command — e.g. `\newcommand{\conj}{\overline}` then
        // called as `$\conj{z}$`. The substituted body alone is just
        // `\overline` (which would emit "missing argument"); the
        // caller's `{z}` is real LaTeX that LaTeX would flow into
        // `\overline`'s arg position. Splice it in so the sub-emitter
        // sees `\overline {z}` and renders correctly.
        if args.len() > macro_def.params {
            for extra in &args[macro_def.params..] {
                expanded.push('{');
                expanded.push_str(extra);
                expanded.push('}');
            }
        }
        // Re-parse and emit. Use a sub-emitter so we don't disturb
        // our `out` cursor management — its output is appended.
        // `increment_depth = true` so a self-referential macro
        // (e.g. `\newcommand{\foo}{\foo}`) hits MAX_MACRO_DEPTH and
        // warns instead of overflowing the stack.
        let body_out = self.render_in_sub_emitter(&expanded, self.in_math, true);
        // Trim the trailing newline the child may have added if the
        // body is a one-liner; otherwise math expansions get
        // unwanted line breaks.
        let body_out = body_out.trim_end_matches('\n');
        // Bug #25: when a user macro is invoked in math right after a
        // literal letter (e.g. `d\src` where `\src` expands to
        // `\nu_{...}`), the sub-emitter's `out` starts empty so its
        // own letter-boundary check sees no preceding letter. The
        // expansion's first character then fuses with our `d`,
        // producing `dnu_(...)` — Typst reads it as an unknown
        // identifier. Re-run the boundary check at the parent level
        // before appending.
        if self.in_math {
            self.ensure_math_letter_boundary(body_out);
        }
        self.out.push_str(body_out);
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
        // Walk the file's AST looking for `new_command_definition`,
        // `old_command_definition`, and `theorem_definition` nodes;
        // harvest each one into a fresh map, then merge.
        let tree = crate::parser::parse(&source);
        let mut harvested: HashMap<String, MacroDef> = HashMap::new();
        let mut harvested_theorems: HashMap<String, String> = HashMap::new();
        let mut harvested_env_argc: HashMap<String, usize> = HashMap::new();
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
                "theorem_definition" => {
                    if let Some((name, display)) = extract_theorem_def(n, &source) {
                        harvested_theorems.entry(name).or_insert(display);
                    }
                }
                "environment_definition" => {
                    // `\newenvironment{name}{...}{...}` in a local .sty/.cls or
                    // \input'd file — register as a transparent (empty-display)
                    // kind so its body passes through when used.
                    if let Some((name, nargs)) = extract_environment_def(n, &source) {
                        if nargs > 0 {
                            harvested_env_argc.entry(name.clone()).or_insert(nargs);
                        }
                        harvested_theorems.entry(name).or_default();
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
        // Merge into self.macros / self.theorem_kinds, parent-wins.
        for (k, v) in harvested {
            self.macros.entry(k).or_insert(v);
        }
        for (k, v) in harvested_theorems {
            self.theorem_kinds.entry(k).or_insert(v);
        }
        for (k, v) in harvested_env_argc {
            self.env_arg_counts.entry(k).or_insert(v);
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
        // Try base_dir first (current file's directory), then fall back to
        // root_dir (the project root). LaTeX resolves \input paths from the
        // project root, so a path like `appendix/d_lemmas` inside
        // `appendix/proofs.tex` should resolve to `<root>/appendix/d_lemmas.tex`.
        let resolved_opt = resolve_input_path(&base_dir, &raw_path).or_else(|| {
            self.root_dir
                .as_deref()
                .filter(|r| *r != base_dir.as_path())
                .and_then(|r| resolve_input_path(r, &raw_path))
        });
        let resolved = match resolved_opt {
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
        // Inherit the parent's macro table so `\input`ed files can use
        // macros defined in the parent (or pre-scanned by the project
        // layer). Without this, an arXiv paper with `\newcommand\src`
        // in `style/header.tex` and `$\src$` in `1-intro.tex` would
        // produce an `ambiguous_math` warning for every call site,
        // because the sub-emitter for `1-intro.tex` started with an
        // empty macro table.
        let macros = self.macros.clone();
        let mut sub = Emitter::with_includes_and_macros(
            &source,
            &source_name,
            Some(new_base),
            visited,
            macros,
        );
        // Propagate the project root so nested \input paths that are
        // relative to the root (LaTeX convention) resolve correctly.
        sub.root_dir = self.root_dir.clone();
        // Pass down any \graphicspath dirs seen so far (e.g. preamble loaded
        // before this include) so figures in the included file can use them.
        sub.graphics_paths = self.graphics_paths.clone();
        // Inherit parent's theorem-kind map so that environments defined in a
        // previously-processed \input file (e.g. macros.tex) are recognisable
        // when they appear in a later sibling include (e.g. sections/04_…tex).
        sub.theorem_kinds = self.theorem_kinds.clone();
        sub.env_arg_counts = self.env_arg_counts.clone();
        // Inherit project-wide referenced labels so a `\section` with multiple
        // `\label`s in this included file attaches the alias that some other
        // file `\ref`s (see pick_label_to_attach).
        sub.referenced_labels = self.referenced_labels.clone();
        sub.emit_root(tree.root_node());
        // Merge the child's body and state back into the parent.
        if !self.out.ends_with('\n') && !self.out.is_empty() {
            self.out.push('\n');
        }
        self.out.push_str(&sub.out);
        self.warnings.append(&mut sub.warnings);
        self.asset_refs.append(&mut sub.asset_refs);
        // Merge back any \graphicspath dirs the included file declared (e.g. a
        // preamble.tex pulled in via \input) so LATER figures in the parent
        // resolve against them too.
        for dir in sub.graphics_paths.drain(..) {
            if !self.graphics_paths.contains(&dir) {
                self.graphics_paths.push(dir);
            }
        }
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
        // Propagate any macros the include newly defined back to the
        // parent so subsequent calls at the parent level see them.
        // `or_insert` preserves parent-wins semantics (the parent's
        // pre-existing definitions, including those seeded by the
        // project-layer pre-scan, take precedence).
        for (k, v) in sub.macros.drain() {
            self.macros.entry(k).or_insert(v);
        }
        // Same for theorem-kind declarations (`\newtheorem` et al.).
        for (k, v) in sub.theorem_kinds.drain() {
            self.theorem_kinds.entry(k).or_insert(v);
        }
        for (k, v) in sub.env_arg_counts.drain() {
            self.env_arg_counts.entry(k).or_insert(v);
        }
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

    /// `\hologo{Name}` / `\Hologo{Name}` → plain text logo string.
    fn emit_hologo(&mut self, node: Node<'_>) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let id = self
                .src
                .get(arg.start_byte() + 1..arg.end_byte() - 1)
                .unwrap_or("")
                .trim();
            let text = match id {
                "TeX" => "TeX",
                "LaTeX" | "LaTeX2e" => "LaTeX",
                "LaTeX2" => "LaTeX2",
                "eTeX" => "eTeX",
                "pdfTeX" => "pdfTeX",
                "pdfLaTeX" => "pdfLaTeX",
                "XeTeX" => "XeTeX",
                "XeLaTeX" => "XeLaTeX",
                "LuaTeX" => "LuaTeX",
                "LuaLaTeX" => "LuaLaTeX",
                "BibTeX" => "BibTeX",
                "BibTeX8" => "BibTeX8",
                "biber" => "Biber",
                "ConTeXt" => "ConTeXt",
                "METAPOST" => "METAPOST",
                "METAFONT" => "METAFONT",
                other => other,
            };
            self.out.push_str(text);
            return node.end_byte();
        }
        self.warn_unsupported_command(node);
        node.end_byte()
    }

    /// Emit the rich native title block from captured \title/\author/\date/
    /// \begin{abstract}/\keywords. Used for Unknown/Lncs/SvMult classes (no
    /// Typst Universe template binding).
    fn flush_title_block(&mut self) {
        self.materialize_authors();
        if self.metadata.is_title_block_empty() {
            return;
        }
        self.ensure_paragraph_break();

        // ── Centred title + author block ──────────────────────────────────
        self.out.push_str("#align(center)[\n");
        if let Some(title) = self.metadata.title.take() {
            let _ = writeln!(
                self.out,
                "  #text(size: 1.5em, weight: \"bold\")[{}]",
                title.as_content()
            );
        }

        if !self.metadata.authors.is_empty() {
            self.out.push_str("  #v(0.6em)\n");
            // Clone (not take): `finish()` still needs `metadata.authors` to
            // emit `#set document(author: …)` for the PDF metadata field.
            let authors = self.metadata.authors.clone();

            // Collect per-author affiliation text, deduplicating to assign
            // superscript indices (1-based, in order of first appearance).
            let aff_texts: Vec<Option<String>> = authors
                .iter()
                .map(|a| aff_display_text(&a.affiliation))
                .collect();
            let mut deduped: Vec<String> = Vec::new();
            let aff_indices: Vec<Option<usize>> = aff_texts
                .iter()
                .map(|at| match at {
                    None => None,
                    Some(text) => {
                        if let Some(pos) = deduped.iter().position(|x| x == text) {
                            Some(pos)
                        } else {
                            deduped.push(text.clone());
                            Some(deduped.len() - 1)
                        }
                    }
                })
                .collect();
            let has_affiliations = !deduped.is_empty();

            // Author name line: "Alice#super[1], Bob#super[2,3]"
            self.out.push_str("  ");
            let name_parts: Vec<String> = authors
                .iter()
                .zip(aff_indices.iter())
                .map(|(author, aff_idx)| {
                    let mut part = escape_text_for_typst_content(author.name.as_content());
                    if let Some(idx) = aff_idx {
                        let _ = write!(part, "#super[{}]", idx + 1);
                    }
                    if let Some(orcid) = &author.orcid {
                        let _ = write!(
                            part,
                            " #link(\"https://orcid.org/{orcid}\")[#text(size: 0.75em)[{orcid}]]"
                        );
                    }
                    part
                })
                .collect();
            self.out.push_str(&name_parts.join(", "));
            self.out.push('\n');

            // Grouped affiliation footer
            if has_affiliations {
                self.out.push_str("  #v(0.3em)\n  #text(size: 0.9em)[\n");
                for (i, aff_text) in deduped.iter().enumerate() {
                    let aff_text = escape_text_for_typst_content(aff_text);
                    if i + 1 < deduped.len() {
                        let _ =
                            writeln!(self.out, "    #super[{}] {} #linebreak()", i + 1, aff_text);
                    } else {
                        let _ = writeln!(self.out, "    #super[{}] {}", i + 1, aff_text);
                    }
                }
                self.out.push_str("  ]\n");
            }

            // Email line (italic, all authors)
            let emails: Vec<&str> = authors.iter().filter_map(|a| a.email.as_deref()).collect();
            if !emails.is_empty() {
                let _ = writeln!(
                    self.out,
                    "  #v(0.3em)\n  #text(size: 0.85em, style: \"italic\")[{}]",
                    escape_text_for_typst_content(&emails.join(", "))
                );
            }
        }

        if let Some(date) = self.metadata.date.take() {
            let _ = write!(self.out, "  #v(0.4em)\n  {}\n", date);
        }
        self.out.push_str("]\n");

        // ── Abstract block ────────────────────────────────────────────────
        // LaTeX's `article` abstract: a centered bold "Abstract" heading above
        // a narrowed, justified text column — no fill / border / rounded box.
        if let Some(abstract_) = self.metadata.r#abstract.take() {
            if !abstract_.is_empty() {
                self.out.push_str(
                    "#v(1em)\n\
                     #align(center)[#text(weight: \"bold\")[Abstract]]\n\
                     #v(0.4em)\n\
                     #pad(x: 2em)[\n  ",
                );
                let _ = writeln!(self.out, "{}", abstract_.as_content());
                self.out.push_str("]\n#v(0.6em)\n");
            }
        }

        // ── Keywords line ─────────────────────────────────────────────────
        if !self.metadata.keywords.is_empty() {
            let kws = self
                .metadata
                .keywords
                .drain(..)
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                self.out,
                "#v(0.3em)\n#text(size: 0.9em)[*Keywords:* {}]",
                kws
            );
        }

        self.out.push('\n');
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
            // Move whitespace that sits just inside the group OUTSIDE the wrap
            // markers. Typst's `_`/`*` emphasis shorthands require a word
            // boundary at the closing marker, so `\textit{correct }word` must
            // become `_correct_ word`, not `_correct _word` (closing `_` after a
            // space → never closes → `unclosed delimiter`, corpus 2605.31567).
            // Harmless for the `#super[...]` / `#align(center)[...]` wraps too.
            let raw = self
                .src
                .get(arg.start_byte() + 1..arg.end_byte().saturating_sub(1))
                .unwrap_or("");
            let lead = &raw[..raw.len() - raw.trim_start().len()];
            let trail = &raw[raw.trim_end().len()..];
            let mid = content.trim();
            self.out.push_str(lead);
            if mid.is_empty() {
                // Whitespace-only (or empty) content — emit it once, no markers.
                if lead.is_empty() {
                    self.out.push_str(trail);
                }
            } else {
                self.out.push_str(left);
                self.out.push_str(mid);
                self.out.push_str(right);
                self.out.push_str(trail);
            }
        }
        node.end_byte()
    }

    /// A `{\bf ...}` / `{\em ...}` group: the first child is a declarative
    /// font switch that scopes to the rest of the group. Emit the remaining
    /// content wrapped in Typst markup, dropping the (pure-grouping) braces.
    /// Returns the group's end byte (the whole group is consumed).
    fn emit_font_switch_group(
        &mut self,
        node: Node<'_>,
        switch_end: usize,
        wrap: (&str, &str),
    ) -> usize {
        let content_end = node.end_byte().saturating_sub(1); // exclude the `}`
        if content_end > switch_end {
            let rest = self.src[switch_end..content_end].to_string();
            let rendered = self.render_in_sub_emitter(&rest, false, false);
            let rendered = rendered.trim();
            if !rendered.is_empty() {
                self.out.push_str(wrap.0);
                self.out.push_str(rendered);
                self.out.push_str(wrap.1);
            }
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

    /// `\begin{lstlisting}[options]...code...\end{lstlisting}` → `#raw("...", block: true)`.
    /// tree-sitter-latex gives lstlisting a `listing_environment` node kind with a
    /// `source_code` child containing the raw code (including any `[options]` prefix
    /// since the listing grammar declares no structured options field).
    fn emit_listing_environment(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let raw = match node
            .children(&mut cursor)
            .find(|c| c.kind() == "source_code")
        {
            Some(cn) => self.src[cn.start_byte()..cn.end_byte()].to_string(),
            None => return node.end_byte(),
        };

        // The source_code span may begin with [key=val,...] (the optional
        // lstlisting argument that the grammar does not parse as a field).
        // Strip it and extract `language=` if present.
        let rest = raw.trim_start_matches('\n');
        let (lang, code) = if rest.starts_with('[') {
            let end_bracket = rest.find(']').unwrap_or(rest.len().saturating_sub(1));
            let options = &rest[1..end_bracket];
            let lang = options.split(',').find_map(|kv| {
                let kv = kv.trim();
                kv.strip_prefix("language")
                    .map(|r| r.trim().strip_prefix('=').unwrap_or("").trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.trim_matches('{').trim_matches('}').to_lowercase())
            });
            let code = rest[end_bracket + 1..].trim_start_matches('\n');
            (lang, code.to_string())
        } else {
            (None, rest.to_string())
        };

        let escaped = code
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");

        let _ = match lang.as_deref() {
            Some(l) => write!(
                self.out,
                "\n#raw(\"{}\", block: true, lang: \"{}\")\n",
                escaped, l
            ),
            None => write!(self.out, "\n#raw(\"{}\", block: true)\n", escaped),
        };
        node.end_byte()
    }

    /// Emit just the body of `\textrm{X}` etc. — strips the command, keeps `X`.
    fn emit_inline_unwrap(&mut self, node: Node<'_>) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let content = self.render_curly_group_content(arg);
            self.out.push_str(&content);
            return node.end_byte();
        }
        // AST-sibling fallback: tree-sitter-latex places the required {content}
        // as a sibling of the generic_command node when an optional [...] arg is
        // present (e.g. \makecell[l]{content}). Walk past the command end, skip
        // any [...] group, then consume the first {content} found in the source.
        let bytes = self.src.as_bytes();
        let mut i = node.end_byte();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        // Skip optional [...]
        if i < bytes.len() && bytes[i] == b'[' {
            i += 1;
            let mut depth = 0i32;
            while i < bytes.len() {
                match bytes[i] {
                    b'[' => {
                        depth += 1;
                        i += 1;
                    }
                    b']' if depth == 0 => {
                        i += 1;
                        break;
                    }
                    b']' => {
                        depth -= 1;
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        }
        if i < bytes.len() && bytes[i] == b'{' {
            let content_start = i + 1;
            i += 1;
            let mut depth = 1i32;
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' if i + 1 < bytes.len() => {
                        i += 2;
                        continue;
                    }
                    b'{' => {
                        depth += 1;
                        i += 1;
                    }
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            let content_text = &self.src[content_start..i];
                            i += 1;
                            let rendered = self.render_in_sub_emitter(content_text, false, false);
                            self.out.push_str(rendered.trim());
                            self.skip_until = self.skip_until.max(i);
                            return i;
                        }
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
        }
        node.end_byte()
    }

    /// `\textcolor{color}{content}` / `\colorbox{color}{content}` →
    /// drops the first `{color}` argument and emits only `{content}`.
    /// `\textcolor{color}{content}` — tree-sitter-latex `color_reference` node.
    /// Drops the color arg; emits only the content arg.
    fn emit_textcolor(&mut self, node: Node<'_>) -> usize {
        // color_reference children: command token, curly_group_text (color),
        // curly_group (content).  Find the curly_group (second arg).
        let mut cursor = node.walk();
        let content_node = node
            .children(&mut cursor)
            .find(|c| c.kind() == "curly_group");
        if let Some(cnode) = content_node {
            let content = self.render_curly_group_content(cnode);
            self.out.push_str(&content);
        }
        node.end_byte()
    }

    /// `\textcolor{color}{content}` in math mode — drops color, renders content
    /// as math.
    fn emit_math_textcolor(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let content_node = node
            .children(&mut cursor)
            .find(|c| c.kind() == "curly_group");
        if let Some(cnode) = content_node {
            let inner = self.render_math_group(cnode);
            self.out.push_str(inner.trim());
        }
        node.end_byte()
    }

    // ─── Environment dispatch & lists ─────────────────────────────────────────

    fn emit_generic_environment(&mut self, node: Node<'_>) -> usize {
        let env = environment_name(node, self.src);
        match env.as_deref() {
            Some("itemize") => self.emit_simple_list(node, "-"),
            Some("enumerate") => self.emit_simple_list(node, "+"),
            Some("description") => self.emit_description(node),
            // Abstract: capture into the title-block field. The generated
            // title block renders it for every class now, so capture it
            // unconditionally (and consume the inline body so it isn't shown
            // twice).
            Some("abstract") => {
                if self.metadata.r#abstract.is_none() {
                    let body = self.render_env_body_to_string(node);
                    self.metadata.r#abstract = Some(Content::Typst(body.trim().to_string()));
                    node.end_byte()
                } else {
                    self.emit_environment_body(node)
                }
            }
            // IEEEtran's keywords env. Same capture-into-title-block as abstract.
            Some("IEEEkeywords") => {
                if self.metadata.keywords.is_empty() {
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
            // `subequations` wraps one or more math envs and provides
            // a single shared numbering. Bug #44: any `\label{...}`
            // calls that are direct children of `subequations` belong
            // to the equation group as a whole, not to the surrounding
            // text. Pre-stage them into `pending_math_labels` so the
            // inner math env's close-flush attaches them.
            Some("subequations") => self.emit_subequations_env(node),
            // Transparent wrappers: emit body, no markup. `\documentclass` etc.
            // already produced warnings as separate top-level commands.
            // `minipage` is a transparent wrapper too, but it takes a mandatory
            // `{width}` argument (plus optional `[pos]` groups) that must be
            // skipped — otherwise the width group leaks as a stray `{}` and the
            // `\linewidth`/`\textwidth` inside it warns as unsupported.
            Some("minipage") => self.emit_minipage(node),
            Some("document") | Some("center") | Some("flushleft")
            | Some("flushright") | Some("quote") | Some("quotation") | Some("verse")
            | Some("titlepage")
            // Acknowledgements, keyword-list, and conference-specific metadata
            // blocks that carry plain content with no Typst-renderable structure.
            | Some("ack") | Some("keywords") | Some("MSCcodes") | Some("icmlauthorlist")
            // Color-styled box environments (tcolorbox, framed). Styling is not
            // round-trippable to Typst without a full color-name map; pass the
            // body through so at least the content is preserved.
            | Some("tcolorbox") | Some("promptbox") | Some("framed") | Some("mdframed")
            // IEEE author biography blocks at the end of papers. The photo arg
            // and author name arg are in the environment arguments and will be
            // dropped; pass the bio text through.
            | Some("IEEEbiography") | Some("IEEEbiographynophoto")
                => self.emit_environment_body(node),
            // Matrix family — handled wherever we encounter them. If we're
            // not already in math mode, the surrounding container will wrap
            // us; pmatrix() etc. assume math context.
            Some("pmatrix") | Some("bmatrix") | Some("vmatrix") | Some("Vmatrix")
            | Some("Bmatrix") | Some("matrix")
            // `smallmatrix`: same rendering as `matrix` — Typst sizes math
            // contextually, so the "small" qualifier is dropped.
            | Some("smallmatrix")
                => self.emit_matrix_env(node, env.as_deref()),
            // `cases` env produces piecewise display.
            Some("cases") => self.emit_cases_env(node),
            // M4: tables and figure floats.
            // `array` is dispatched specially: when nested inside a math
            // container (`align*`, `gather`, `\left\{...\right\}`, etc.)
            // it should render as Typst `cases(...)`, not as a `#table(...)`
            // (which is text-mode-only and breaks the surrounding `$...$`).
            Some("array") if self.in_math => self.emit_array_in_math(node),
            Some("tabular") | Some("tabular*") | Some("array")
            // tabularx / tabulary: same layout shape as tabular, but take a
            // leading {width} argument before the column spec. emit_tabular
            // already skips that width group (see `needs_skip`); without this
            // dispatch arm the env fell through and its whole body was dropped.
            | Some("tabularx") | Some("tabulary")
            // tblr (tabularray): same layout shape as tabular; leading
            // key=value options group is ignored if emit_tabular trips on it.
            | Some("tblr")
                => self.emit_tabular(node),
            Some("figure") | Some("figure*") | Some("table") | Some("table*")
            // algorithm / algorithm*: float wrapper around \begin{algorithmic}.
            // The inner algorithmic steps pass through; only the float shell
            // needs handling here.
            | Some("algorithm") | Some("algorithm*") | Some("algorithm2e")
            // wrapfigure / wraptable: degrade to a standard float — the
            // text-wrap positioning is lost but the content (caption + graphic
            // or table) is preserved.
            | Some("wrapfigure") | Some("wraptable")
                => self.emit_figure(node),
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
            // User-defined environments harvested from `\newtheorem` (non-empty
            // display) or `\newtcolorbox`/`\newmdenv` (empty display sentinel →
            // transparent body pass-through).
            Some(other) if self.theorem_kinds.contains_key(other) => {
                let display = self.theorem_kinds[other].clone();
                if display.is_empty() {
                    self.emit_environment_body(node)
                } else {
                    self.emit_theorem_env(node, &display)
                }
            }
            // tikzpicture: TikZ drawing commands have no Typst equivalent.
            // tikz package is already nooped; silently drop the environment body.
            Some("tikzpicture") | Some("tikzpicture*") => node.end_byte(),
            // multicols: multi-column layout; content is meaningful text, so
            // pass it through. Column layout itself is lost (Typst handles this
            // separately via `set page(columns: N)`), but no warning is needed
            // since the multicols package is already in the noop allowlist.
            Some("multicols") | Some("multicols*") => self.emit_environment_body(node),
            // A bare `algorithmic` env (NOT wrapped in an `algorithm` float —
            // the float case routes to emit_figure above) carries the pseudocode
            // steps and, crucially, their `\State\label{...}` anchors that other
            // text `\cref`s. Dropping it whole loses those labels → dangling
            // `@alg:step:N` → compile failure (corpus 2605.31510). Pass the body
            // through: `\State`/`\Procedure`/… are unknown commands that degrade
            // to text, and the inner `\label`s reach the orphan-label anchor.
            Some("algorithmic") | Some("algorithmicx") | Some("algpseudocode")
            | Some("algpseudocodex") | Some("ALC@g") => self.emit_environment_body(node),
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

    /// Emit a `\begin{subequations}...\end{subequations}` env. Bug #44:
    /// a `\label{...}` that's a direct child of `subequations` (e.g.
    /// `\begin{subequations}\label{eqn:AMP}\begin{align}...\end{align}
    /// \end{subequations}`) targets the equation group as a whole. By
    /// the time we visit the inner math env, that text-mode label has
    /// already been emitted as a stray `<key>` and Typst attaches it
    /// to the wrong content. Pre-stage direct-child labels into
    /// `pending_math_labels` so the inner math env's close-flush
    /// attaches them all together.
    fn emit_subequations_env(&mut self, env: Node<'_>) -> usize {
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
    fn emit_minipage(&mut self, env: Node<'_>) -> usize {
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
    fn emit_environment_body(&mut self, env: Node<'_>) -> usize {
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

    /// True when a `\bibliography{...}` directive is paired with a `.bib` that
    /// resolved on disk — its `#bibliography(.bib)` is the complete, canonical
    /// reference list, so any manual `\bibitem`/`thebibliography` entries are
    /// redundant and would collide with it. (corpus 2605.31440)
    fn bib_file_is_authoritative(&self) -> bool {
        self.has_bibtex_include && self.had_bib_file
    }

    /// Hidden anchors for every `\ref`/`\cref`-referenced key that has no
    /// resolving target — neither a `<key>` label already in the output nor a
    /// bibliography entry. Without these a reference to an undefined label
    /// (commented-out `\label`, dropped environment) leaves a dangling `@key`
    /// that aborts the Typst compile. See the call site in `finish()`.
    fn dangling_ref_anchors(&self) -> String {
        if self.referenced_labels.is_empty() {
            return String::new();
        }
        // Defined labels = every `<key>` already emitted into the body. The `>`
        // terminator makes this scan unambiguous; keys match the sanitized form
        // used by both `<key>` and `@key`.
        let chars: Vec<char> = self.out.chars().collect();
        let mut defined: HashSet<String> = HashSet::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '<' {
                let mut j = i + 1;
                while j < chars.len() && is_typst_label_char(chars[j]) {
                    j += 1;
                }
                if j > i + 1 && j < chars.len() && chars[j] == '>' {
                    defined.insert(chars[i + 1..j].iter().collect());
                    i = j + 1;
                    continue;
                }
            }
            i += 1;
        }
        let mut anchors = String::new();
        // Deterministic order for stable output.
        let mut missing: Vec<&String> = self
            .referenced_labels
            .iter()
            .filter(|k| !defined.contains(*k) && !self.bibliography_keys.contains(*k))
            .collect();
        missing.sort();
        for key in missing {
            let _ = write!(anchors, "\n#hide[#figure([]) <{}>]", key);
        }
        anchors
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

    // ─── Theorem / proof / bibliography environments ──────────────────────────

    /// `\begin{theorem}[note]\label{X} body \end{theorem}` →
    /// `#figure(kind: "<name>", supplement: [Name], [body]) <X>`.
    fn emit_theorem_env(&mut self, env: Node<'_>, name: &str) -> usize {
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
        // When a `\bibliography{...}` with a resolvable .bib is present, its
        // `#bibliography(.bib)` is the authoritative, complete reference list.
        // This manual list is then redundant and its `<key>` labels collide
        // with the .bib entries — drop it entirely. (corpus 2605.31440)
        if self.bib_file_is_authoritative() {
            return env.end_byte();
        }
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

    // ─── Theorem & tcolorbox macro harvesting ─────────────────────────────────

    /// Harvest a `theorem_definition` node (`\newtheorem{name}{Display}` and
    /// variants) into `self.theorem_kinds` so that `emit_generic_environment`
    /// can route unknown environment names to `emit_theorem_env` instead of
    /// warning.
    fn harvest_theorem_definition(&mut self, node: Node<'_>) {
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
    fn harvest_definitions(&mut self, source: &str) {
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
    fn harvest_tcolorbox_decl(&mut self, node: Node<'_>, source: &str) {
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
    fn harvest_environment_definition(&mut self, node: Node<'_>) {
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
    fn harvest_generic_theorem_cmd(&mut self, node: Node<'_>, source: &str) {
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

    // ===== Math mode =====

    // ─── Math primitives & letter-boundary helpers ────────────────────────────

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
        if boundary::needs_trailing_sentinel(typst, true) {
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
        if boundary::starts_with_letter(next) && boundary::ends_with_letter(&self.out) {
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

    /// Escape `;` inside any `(...)` group in the in-progress math
    /// body. Typst math treats `f(a; b)` as a 2-row matrix call —
    /// `\pi(\cdot; V)` (conditional-probability notation) would
    /// otherwise render as `pi(dot.c; V)` and Typst aborts with
    /// `expected content, found array`. Replacing with `#";"` keeps
    /// the literal semicolon glyph without triggering the
    /// matrix-row interpretation.
    fn escape_math_semicolons(&mut self, body_start: usize) {
        if body_start > self.out.len() {
            return;
        }
        let escaped = escape_paren_semicolons(&self.out[body_start..]);
        if escaped.len() != self.out.len() - body_start {
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
                        // Find the byte index just past the last whitespace
                        // char. We must advance by the whitespace's UTF-8
                        // length, not by 1 byte — non-breaking space
                        // (`\u{a0}`) and other multi-byte whitespace would
                        // otherwise land in the middle of the char and
                        // panic on the slice.
                        let last_ws = s
                            .char_indices()
                            .rev()
                            .find(|(_, c)| c.is_whitespace())
                            .map(|(p, c)| p + c.len_utf8())
                            .unwrap_or(0);
                        s[last_ws..].contains('.')
                    };
                    // Only `(` can make Typst interpret the dotted symbol as a
                    // function call (e.g. `arrow.r(` → function call). `)`, `,`
                    // and other punct are fine without a separator.
                    let next_is_call_open = next == '(';
                    if boundary::is_word_char(next) || (prev_token_dotted && next_is_call_open) {
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

    // ─── Math environment containers ──────────────────────────────────────────

    fn emit_inline_math(&mut self, node: Node<'_>) -> usize {
        if self.in_math {
            // Already inside a math container (e.g. a \newcommand body with
            // `$...$` expanded in math context).  Adding another `$` would
            // close the outer math and produce "unclosed delimiter" errors.
            // Emit the body children directly — the outer container handles
            // post-processing.
            self.emit_math_children(node);
            return node.end_byte();
        }
        self.out.push('$');
        let body_start = self.out.len();
        let was = self.in_math;
        self.in_math = true;
        self.emit_math_children(node);
        self.in_math = was;
        self.collapse_math_spaces(body_start);
        self.balance_math_brackets(body_start);
        self.escape_math_semicolons(body_start);
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
        self.escape_math_semicolons(body_start);
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
            // Save the outer env's pending labels so a nested env's
            // body can collect its own `\label{...}` calls.
            let prev_labels = std::mem::take(&mut self.pending_math_labels);
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
            // Bug #30 / #44: don't flush labels inline (we're inside
            // an outer `$...$` — `<label>` inside math parses as `<`
            // op followed by identifier(s) and breaks compile).
            // Propagate the labels up so the outer env's post-`$`
            // flush attaches them. Concat outer-first, then any new
            // labels collected during the nested body, deduped.
            let inner_labels = std::mem::take(&mut self.pending_math_labels);
            self.pending_math_labels = prev_labels;
            for l in inner_labels {
                if !self.pending_math_labels.contains(&l) {
                    self.pending_math_labels.push(l);
                }
            }
            return node.end_byte();
        }
        self.ensure_paragraph_break();
        self.out.push_str("$ ");
        let body_start = self.out.len();
        let was = self.in_math;
        self.in_math = true;

        // Bug #44: INHERIT pre-staged labels (e.g. from
        // `subequations`'s top-level `\label{...}`). Don't take/restore
        // here — the body emission may push more labels, and the close
        // flush emits the full set.
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
        self.escape_math_semicolons(body_start);
        self.out.push_str(" $");
        // Emit ALL collected labels — first attached to this equation,
        // the rest as hidden equation-kind figures so each `\ref{...}`
        // still resolves. Typst only honours the LAST `<label>` next
        // to one equation; further `<label>`s on the same equation
        // are silently ignored, and `#hide[...]` of a raw `$..$`
        // produces a `hide` element that can't itself be referenced —
        // wrapping in `#figure(kind: "equation", ...)` makes the
        // hidden stub a valid `@key` target.
        let labels = std::mem::take(&mut self.pending_math_labels);
        if let Some((first, rest)) = labels.split_first() {
            let _ = write!(self.out, " <{}>", first);
            if !rest.is_empty() {
                self.needs_equation_numbering = true;
            }
            for extra in rest {
                let _ = write!(
                    self.out,
                    "\n#hide[#figure(kind: \"equation\", supplement: [Eq.], $ \"\" $) <{}>]",
                    extra
                );
            }
        }
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
        self.emit_math_node_slice(&body);
    }

    /// Emit a slice of math child nodes with end-of-group scope
    /// tracking for TeX font-style declarations (`\bf`, `\it`, `\rm`,
    /// etc.). On encountering a font declaration, we open the matching
    /// Typst wrapper (`bold(`, `italic(`, ...), recurse into the
    /// remaining slice inside the wrapper, then close `)`. The font
    /// declaration node itself is not emitted. Subsequent font
    /// declarations nest — `{\bf a \it b}` → `bold(a italic(b))` — a
    /// partial-fidelity render that keeps both intents visible (LaTeX
    /// would actually set `b` to bold-italic, not nested italic).
    ///
    /// `text` containers are transparent — tree-sitter-latex puts
    /// adjacent words and commands into a single `text` node, so
    /// `{a \bf b}` has the `\bf` *inside* a `text` sibling of the
    /// `{`/`}` braces. We flatten such containers before scanning so
    /// the declaration is visible at the slice level.
    fn emit_math_node_slice(&mut self, body: &[Node<'_>]) {
        if body.is_empty() {
            return;
        }
        let flat = flatten_text_children(body);
        if flat.is_empty() {
            return;
        }
        let mut last = flat[0].start_byte();
        for (i, child) in flat.iter().enumerate() {
            if let Some(wrap) = math_font_decl_wrapper(*child, self.src) {
                self.safe_copy(last, child.start_byte());
                self.out.push_str(wrap);
                self.out.push('(');
                // tree-sitter parses `\rm{d}` with the `{d}` group as a CHILD of
                // the `\rm` generic_command. Emit those absorbed argument
                // children first (else their content is dropped → empty
                // `upright()`, corpus 2605.31306), then the trailing siblings
                // the declaration scopes over.
                let mut dc = child.walk();
                let own: Vec<Node<'_>> = child
                    .children(&mut dc)
                    .filter(|c| c.kind() != "command_name")
                    .collect();
                for oc in &own {
                    // `\rm{d} {\mathbb Q}` parses with BOTH groups as children of
                    // `\rm`; emit them separated by a space so adjacent atoms
                    // don't fuse into one identifier (`upright(d bb(Q))`, not
                    // `dbb(Q)` → Typst `unknown variable: dbb`).
                    if !self.out.ends_with('(') && !self.out.ends_with(' ') {
                        self.out.push(' ');
                    }
                    // The absorbed arg is a LaTeX grouping `{...}`.
                    if oc.kind() == "curly_group" {
                        let raw = self
                            .src
                            .get(oc.start_byte() + 1..oc.end_byte() - 1)
                            .unwrap_or("")
                            .trim();
                        // A multi-character alphanumeric run is a function/text
                        // name (`\rm{db2mag}`); quote it so Typst keeps it as one
                        // token (`upright("db2mag")`) instead of splitting it into
                        // juxtaposed atoms (`d b 2mag` → `unknown variable: 2mag`,
                        // corpus 2605.31510). A single atom or any group with a
                        // command (`{\mathbb Q}`) renders as math, brace-stripped.
                        if raw.chars().count() > 1
                            && raw.chars().all(|c| c.is_ascii_alphanumeric())
                        {
                            let _ = write!(self.out, "\"{}\"", raw);
                        } else {
                            let inner = self.render_math_group(*oc);
                            self.out.push_str(inner.trim());
                        }
                    } else {
                        let _ = self.emit_node(*oc);
                    }
                }
                // Separate the absorbed arg from the scoped siblings too.
                if !own.is_empty()
                    && i + 1 < flat.len()
                    && !self.out.ends_with(' ')
                    && !self.out.ends_with('(')
                {
                    self.out.push(' ');
                }
                self.emit_math_node_slice(&flat[i + 1..]);
                self.out.push(')');
                return;
            }
            let cs = child.start_byte();
            self.safe_copy(last, cs);
            last = self.emit_node(*child);
        }
        let end = flat.last().unwrap().end_byte();
        self.safe_copy(last, end);
    }

    // ─── Math commands & operators ────────────────────────────────────────────

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
        // Single-arg wrapper commands: open(arg)close. `wrap_for_command_name`
        // is the single source of truth for the prefix/suffix pairs; both this
        // path and the bare-command_name path at the top of `emit_math_node`
        // delegate here so adding a new wrapper only requires one edit.
        if let Some((l, r)) = wrap_for_command_name(n) {
            return self.emit_math_wrap(node, l, r);
        }
        match n {
            "\\frac" | "\\tfrac" | "\\dfrac" | "\\cfrac" => self.emit_math_frac(node),
            "\\sqrt" => self.emit_math_sqrt(node),
            "\\binom" | "\\dbinom" | "\\tbinom" => self.emit_math_binom(node),
            // `\text{X}` and `\mathrm{X}` switch to upright text inside math.
            // Typst renders quoted strings as upright text in math context.
            // `\mbox{X}` and `\hbox{X}` are TeX-primitive boxes; in math
            // mode they switch to text mode like `\text` does.
            // `\textnormal`/`\texttt`/`\textbf`/`\textup`/`\textit`/
            // `\textsc`/`\textsl` are LaTeX2e text-style commands that
            // also occasionally appear inside math; we render them as
            // the same upright-quoted text (the style attribute is lost
            // — partial render).
            "\\text" | "\\mathrm" | "\\textrm" | "\\mathnormal" | "\\mbox" | "\\hbox"
            | "\\textnormal" | "\\texttt" | "\\textbf" | "\\textup" | "\\textit" | "\\textsc"
            | "\\textsl" => self.emit_math_text_call(node),
            // `\smash[t/b]{X}`, `\raisebox{offset}{X}`, `\scalebox{factor}{X}`
            // — layout/positioning primitives with no Typst equivalent for
            // the offset, but the inner content should still render. Drop
            // the positioning args and emit the last curly_group as math.
            "\\smash" => self.emit_math_layout_inner(node, 0),
            "\\raisebox" => self.emit_math_layout_inner(node, 1),
            "\\scalebox" => self.emit_math_layout_inner(node, 1),
            // `\mathgroup{N}{X}` — TeX font-group hint, two args; the
            // first is the group code (we drop it), the second is the
            // content (we emit). Same shape as the layout helpers.
            "\\mathgroup" => self.emit_math_layout_inner(node, 1),
            // `\ ` (backslash + space) — LaTeX forced thin/normal
            // space in math mode. Emit a plain space.
            "\\ " => {
                self.out.push(' ');
                node.end_byte()
            }
            // Math class modifiers (`\mathrel`, `\mathord`, `\mathbin`, etc.)
            // tell LaTeX "treat the argument as a relation/atom/binary-op for
            // spacing purposes". Typst auto-spaces math, so the class hint is
            // effectively a no-op for rendering — just unwrap and emit the
            // content as math.
            "\\mathrel" | "\\mathord" | "\\mathbin" | "\\mathopen" | "\\mathclose"
            | "\\mathpunct" | "\\mathinner" => {
                if let Some(arg) = first_curly_group(node) {
                    let inner = self.render_math_group(arg);
                    self.out.push_str(inner.trim());
                }
                node.end_byte()
            }
            // `\nicefrac{a}{b}` — slanted inline fraction. Typst renders
            // a slash-separated form well; render `(a) / (b)` like \frac
            // but with the understanding that the Typst output isn't
            // visually identical (no slanting).
            "\\nicefrac" => self.emit_math_frac(node),
            // `\raisetag{N}` — pure equation-tag positioning. No visible
            // rendering effect; the tag itself was handled by `\tag` (or
            // wasn't emitted at all in our model). Silent drop.
            "\\raisetag" => node.end_byte(),
            // TeX control-flow primitives. These leak into math when a
            // user macro body (e.g. \pdata, \traceD) uses \ifthenelse /
            // \ifstrempty / etc. and we expand it inline. We can't
            // actually evaluate the condition at conversion time, so
            // pick a sensible branch and partial-render. Honest about
            // the loss but compiles cleanly.
            //
            // `\ifthenelse{cond}{true}{false}` → emit `{true}`; the
            // condition tested at TeX time is usually "interesting
            // case" vs "fallback" and the true branch is the richer
            // form in the bodies we see in 2605.22765 and 2605.22159.
            "\\ifthenelse" => self.emit_math_then_branch(node, 1),
            // `\ifstrempty{x}{empty}{nonempty}` → emit `{nonempty}`;
            // most call sites pass a non-empty `x`. Same rationale.
            "\\ifstrempty" => self.emit_math_then_branch(node, 2),
            // `\notempty[default]{value}` (xargspec): when value is
            // non-empty, returns value; else default. Emit value.
            //
            // Bug #28: tree-sitter often parses `\notempty[X]{Y}` with
            // BOTH the brack and the curly as AST siblings of the
            // command_name (not children). The AST-child-only path
            // misses them and they leak as raw `[X]{Y}` tokens that
            // Typst then re-parses (the `^` from `\sscript` body etc.
            // breaks the surrounding math). Use source-byte fallback
            // to consume both — same shape as `\xrightarrow`/text
            // family fixes from PRs #27/#33.
            "\\notempty" => self.emit_math_notempty(node),
            // Bare conditionals / expansion primitives — drop silently.
            // Their arguments are AST siblings that get emitted by the
            // normal walker. Without this drop, every body that uses
            // TeX conditionals warns once per primitive token.
            "\\relax" | "\\expandafter" | "\\fi" | "\\else" | "\\ifx" | "\\if" | "\\ifdim"
            | "\\ifnum" | "\\ifdefined" | "\\ifcsname" | "\\ifpdf" | "\\ifxetex" | "\\ifluatex"
            | "\\detokenize" | "\\noexpand" | "\\unexpanded" | "\\csname" | "\\endcsname"
            | "\\protect" | "\\equal" => node.end_byte(),
            // `\xrightarrow[below]{above}` — extensible right arrow with
            // optional labels. Typst's `arrow.r` is the base symbol;
            // attaching labels needs `attach(arrow.r, t: ..., b: ...)`.
            // Render with labels when present; fall back to bare arrow
            // when not.
            "\\xrightarrow"
            | "\\xleftarrow"
            | "\\xLeftarrow"
            | "\\xRightarrow"
            | "\\xLeftrightarrow"
            | "\\xleftrightarrow"
            | "\\xmapsto"
            | "\\xhookleftarrow"
            | "\\xhookrightarrow"
            | "\\xtwoheadleftarrow"
            | "\\xtwoheadrightarrow"
            | "\\xleftharpoondown"
            | "\\xleftharpoonup"
            | "\\xrightharpoondown"
            | "\\xrightharpoonup" => self.emit_math_extensible_arrow(node, n),
            // `\substack{a\\b\\c}` — multi-line subscript content used
            // inside `\sum_{...}` / `\max_{...}` etc. Render the lines
            // joined with Typst paragraph separator (`#h(0pt)\` doesn't
            // work in math; the cleanest equivalent is just space-
            // separated). The actual multi-line layout would need
            // `attach` machinery; this is a partial render that keeps
            // the math compiling.
            // `\varinjlim` / `\varprojlim` / `\varliminf` / `\varlimsup` —
            // amsmath limit operators with a directional arrow or bar
            // underset on the `lim` base. Render via Typst `attach` or
            // `underline`/`overline` on `op("lim")`.
            "\\varinjlim" => {
                self.out.push_str("attach(op(\"lim\"), b: arrow.r)");
                node.end_byte()
            }
            "\\varprojlim" => {
                self.out.push_str("attach(op(\"lim\"), b: arrow.l)");
                node.end_byte()
            }
            "\\varliminf" => {
                self.out.push_str("underline(op(\"lim\"))");
                node.end_byte()
            }
            "\\varlimsup" => {
                self.out.push_str("overline(op(\"lim\"))");
                node.end_byte()
            }
            "\\substack" => {
                if let Some(arg) = first_curly_group(node) {
                    let inner = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte() - 1)
                        .unwrap_or("")
                        .trim();
                    // Replace `\\` (row break) with `,` so the result
                    // is a comma-separated list — readable but flat.
                    let flattened = inner.replace("\\\\", ", ");
                    // Re-render the flattened source through a math
                    // sub-emitter so symbols still translate.
                    let rendered = self.render_in_sub_emitter(&flattened, true, true);
                    self.out.push_str(rendered.trim());
                }
                node.end_byte()
            }
            // `\mathbf{X}` → bold math; `\mathbb{X}` → blackboard bold (`bb(X)`).
            "\\mathbf" | "\\bm" | "\\bs" | "\\bold" => self.emit_math_wrap(node, "bold(", ")"),
            // `\mathds` (dsfont) and `\mathbbold` (bbold) — visually
            // identical to `\mathbb` for the common single-letter use.
            "\\mathbb" | "\\mathbbm" | "\\Bbb" | "\\mathds" | "\\mathbbold" => {
                self.emit_math_wrap(node, "bb(", ")")
            }
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
            // `\phantom{X}` in Typst math needs `#hide[$X$]` — `hide` is a
            // content function, not a math operator, so it requires the `#`
            // escape and a math content block argument.
            "\\phantom" | "\\hphantom" | "\\vphantom" => self.emit_math_phantom(node),
            "\\emph" => self.emit_math_wrap(node, "italic(", ")"),
            "\\mathop" => self.emit_math_wrap(node, "op(", ")"),
            // `\operatorname{name}` → `op("name")` — upright math text.
            "\\operatorname" => self.emit_math_operatorname(node),
            // Math-mode spacing primitives. `\hspace` emits a thin space so
            // that content wrapping it (e.g. `\underbrace{\hspace{4cm}}`) does
            // not produce an empty body that Typst rejects. `\vspace` and the
            // zero-width commands are dropped silently.
            "\\hspace" => {
                // `thin` must not fuse with a preceding identifier letter
                // (e.g. `v\hspace{...}` → `vthin` = unknown variable).
                self.ensure_math_letter_boundary("thin");
                self.out.push_str("thin ");
                node.end_byte()
            }
            "\\vspace" | "\\!" | "\\linebreak" | "\\nobreak" => node.end_byte(),
            // `\tag{...}` adds LaTeX equation labels for presentation only;
            // Typst handles equation numbering itself. Warn so the user knows
            // their custom label text was not preserved.
            "\\tag" => {
                self.warn_silently_dropped(node);
                node.end_byte()
            }
            // `\not` is a prefix slash-overlay (e.g. `\not =` → `≠`).
            // Typst's cancel(...) takes an argument, so the bare prefix form
            // can't be mechanically translated. Warn rather than silently
            // dropping the negation, which would produce incorrect math.
            "\\not" => {
                self.warn_silently_dropped(node);
                node.end_byte()
            }
            // Math style switches (\displaystyle/\textstyle/\scriptstyle/
            // \scriptscriptstyle) are pure size declarations with no content
            // and no Typst equivalent — Typst sizes math contextually.
            // Silent-drop, same family as \small/\large/\normalsize in text mode.
            "\\displaystyle" | "\\textstyle" | "\\scriptstyle" | "\\scriptscriptstyle" => {
                node.end_byte()
            }
            // Row break inside math envs. Emit `\` followed by `\n` —
            // the newline guarantees the row-break is unambiguously
            // recognizable by downstream splitters (matrix/cases),
            // even when the source has `\\X` with no whitespace
            // before the next content (e.g. `\begin{smallmatrix}a&-a\\0&0`,
            // Bug #31). Also makes the output readable.
            //
            // Bug #20/#21 additionally consumes an optional `[length]`
            // bracket so it doesn't leak into the output and trip
            // Typst's matrix-delimiter parser.
            "\\\\" => {
                self.out.push('\\');
                // Only append our own `\n` when the source doesn't
                // already provide one. Many `align` bodies write
                // `\\\n` already; adding ours would yield a blank
                // line between rows.
                let bytes = self.src.as_bytes();
                let next_non_space = {
                    let mut k = node.end_byte();
                    while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                        k += 1;
                    }
                    bytes.get(k).copied()
                };
                if next_non_space != Some(b'\n') && next_non_space != Some(b'\r') {
                    self.out.push('\n');
                }
                let mut i = node.end_byte();
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'[' {
                    let mut j = i + 1;
                    let mut depth = 0i32;
                    while j < bytes.len() {
                        match bytes[j] {
                            b'\\' if j + 1 < bytes.len() => {
                                j += 2;
                                continue;
                            }
                            b'{' => depth += 1,
                            b'}' => depth -= 1,
                            b']' if depth == 0 => break,
                            _ => {}
                        }
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b']' {
                        let end = j + 1;
                        self.skip_until = self.skip_until.max(end);
                        return end;
                    }
                }
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

    /// Render one [`BracelessArg`] as a math-mode string. Used by every
    /// structural math command that supports both `\foo{x}` and `\foo x`
    /// argument forms.
    ///
    /// - `Command(\name)` — look up via `lookup_math_symbol`, fall back
    ///   to user-macro expansion, fall back to the raw command text.
    /// - `Group({...})` — render via a sub-emitter in math context.
    /// - `Char(c)` — pass through as-is.
    fn render_braceless_math_arg(&mut self, arg: BracelessArg) -> String {
        match arg {
            BracelessArg::Command(cmd) => {
                if let Some(typst) = lookup_math_symbol(&cmd) {
                    typst.to_string()
                } else if let Some(macro_def) = self.macros.get(&cmd).cloned() {
                    self.render_in_sub_emitter(&macro_def.body, true, true)
                        .trim()
                        .to_string()
                } else {
                    cmd
                }
            }
            BracelessArg::Group(inner_src) => self
                .render_in_sub_emitter(&inner_src, true, true)
                .trim()
                .to_string(),
            BracelessArg::Char(c) => c,
        }
    }

    /// `\frac{a}{b}` → `(a) / (b)`. Also accepts the brace-less form
    /// `\frac a b` (rare in arXiv but legal LaTeX) by consuming up to
    /// two trailing tokens via `consume_braceless_arg`. Mixed forms
    /// like `\frac{a} b` work too — the helper picks up whichever
    /// brace-less args remain after the curly_group children.
    fn emit_math_frac(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let groups: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "curly_group")
            .collect();
        let mut rendered: Vec<String> = groups.iter().map(|g| self.render_math_group(*g)).collect();
        let mut consumed_end = node.end_byte();
        while rendered.len() < 2 {
            match try_consume_math_arg(self.src, consumed_end) {
                Some((arg, end)) => {
                    rendered.push(self.render_braceless_math_arg(arg));
                    consumed_end = end;
                }
                None => break,
            }
        }
        if rendered.len() < 2 {
            self.warn_ambiguous_math(node, "\\frac (missing args)");
            return node.end_byte();
        }
        if consumed_end > node.end_byte() {
            self.skip_until = self.skip_until.max(consumed_end);
        }
        let _ = write!(
            self.out,
            "({}) / ({})",
            rendered[0].trim(),
            rendered[1].trim()
        );
        consumed_end
    }

    /// `\sqrt{x}` → `sqrt(x)`. Also accepts brace-less `\sqrt x` and
    /// `\sqrt\alpha`. The optional radical index `\sqrt[n]{x}` form
    /// keeps the existing curly-only path (handled by `first_curly_group`).
    fn emit_math_sqrt(&mut self, node: Node<'_>) -> usize {
        if let Some(g) = first_curly_group(node) {
            let inner = self.render_math_group(g);
            self.ensure_math_letter_boundary("sqrt(");
            let _ = write!(self.out, "sqrt({})", inner.trim());
            return node.end_byte();
        }
        // Brace-less: consume one token from raw source. `try_consume_math_arg`
        // refuses to gobble math delimiters (`$`, `\)`, `\]`, `}`) so we
        // don't accidentally eat a closing `$` when the source is malformed.
        match try_consume_math_arg(self.src, node.end_byte()) {
            // A structural command radicand (`\sqrt\frac{a}{b}`): `\frac` takes
            // its OWN brace args, but `consume_braceless_arg` returns just the
            // `\frac` token, leaving `{a}{b}` to spill out as `sqrt(\frac){a}{b}`
            // (corpus 2605.31596). When the command is followed by `{...}` arg
            // groups, consume the whole application and render it as math.
            Some((BracelessArg::Command(cmd), cmd_end))
                if consume_trailing_brace_groups(self.src, cmd_end) > cmd_end =>
            {
                let end = consume_trailing_brace_groups(self.src, cmd_end);
                let frag = self.src[node.end_byte()..end].trim();
                let inner = self.render_in_sub_emitter(frag, true, true);
                self.skip_until = self.skip_until.max(end);
                self.ensure_math_letter_boundary("sqrt(");
                let _ = write!(self.out, "sqrt({})", inner.trim());
                let _ = cmd;
                end
            }
            Some((arg, end)) => {
                let inner = self.render_braceless_math_arg(arg);
                if end > node.end_byte() {
                    self.skip_until = self.skip_until.max(end);
                }
                self.ensure_math_letter_boundary("sqrt(");
                let _ = write!(self.out, "sqrt({})", inner.trim());
                end
            }
            None => {
                self.warn_ambiguous_math(node, "\\sqrt (missing arg)");
                node.end_byte()
            }
        }
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

    /// Render an extensible arrow command (`\xrightarrow{above}`,
    /// `\xleftarrow[below]{above}`, etc.). Maps the command name to
    /// Typst's `arrow.r` / `arrow.l` / `arrow.r.long` / etc. and
    /// attaches the above/below labels via Typst's `attach` mechanism
    /// (`arrow.r^"above"_"below"`). When labels are missing, emits
    /// the bare arrow.
    /// Render a `\text{X}`-family call in math mode. Emits `"X"` (a
    /// Typst quoted string that renders as upright text inside math).
    /// Handles the case where tree-sitter attached the `{X}` as an
    /// AST sibling rather than a child of the generic_command —
    /// same source-byte fallback shape PR #27 used for `\xrightarrow`.
    fn emit_math_text_call(&mut self, node: Node<'_>) -> usize {
        // First: AST child path.
        if let Some(arg) = first_curly_group(node) {
            let inner = self
                .src
                .get(arg.start_byte() + 1..arg.end_byte() - 1)
                .unwrap_or("")
                .trim();
            let _ = write!(self.out, "\"{}\"", inner);
            return node.end_byte();
        }
        // Fallback: scan source bytes after node.end_byte() for `{...}`.
        let bytes = self.src.as_bytes();
        let mut i = node.end_byte();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'{' {
            let inner_start = i + 1;
            let mut j = inner_start;
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
            if j < bytes.len() && bytes[j] == b'}' {
                let inner = self.src[inner_start..j].trim();
                let _ = write!(self.out, "\"{}\"", inner);
                let end = j + 1;
                self.skip_until = self.skip_until.max(end);
                return end;
            }
        }
        // Truly no argument — emit nothing, no warning.
        node.end_byte()
    }

    /// `\notempty[default]{value}` (xargspec): emit `value` as math.
    /// Consumes any AST-sibling `[...]` and `{...}` via source-byte
    /// scanning so the brack arg doesn't leak as raw tokens. Same
    /// shape as `emit_math_layout_inner` but the brack is mandatory-
    /// to-consume and the curly is the rendered output (not skipped).
    fn emit_math_notempty(&mut self, node: Node<'_>) -> usize {
        let bytes = self.src.as_bytes();
        let mut i = node.end_byte();
        // Skip optional `[default]` if present (drop its bytes).
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'[' {
            let mut j = i + 1;
            let mut depth = 0i32;
            while j < bytes.len() {
                match bytes[j] {
                    b'\\' if j + 1 < bytes.len() => {
                        j += 2;
                        continue;
                    }
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b']' if depth == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b']' {
                i = j + 1;
            }
        }
        // Expect `{value}` and render its inner content as math.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut consumed = node.end_byte();
        if i < bytes.len() && bytes[i] == b'{' {
            let inner_start = i + 1;
            let mut j = inner_start;
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
            if j < bytes.len() && bytes[j] == b'}' {
                let inner_src = self.src[inner_start..j].to_string();
                let rendered = self.render_in_sub_emitter(&inner_src, true, true);
                self.out.push_str(rendered.trim());
                consumed = j + 1;
            }
        }
        // AST-children fallback: if no source-byte sibling found but a
        // child curly_group exists, emit its content.
        if consumed == node.end_byte() {
            let mut cursor = node.walk();
            let curlys: Vec<Node<'_>> = node
                .children(&mut cursor)
                .filter(|c| c.kind() == "curly_group")
                .collect();
            if let Some(arg) = curlys.first() {
                let inner = self.render_math_group(*arg);
                self.out.push_str(inner.trim());
            }
        }
        if consumed > node.end_byte() {
            self.skip_until = self.skip_until.max(consumed);
        }
        consumed
    }

    /// Emit a chosen `curly_group` branch of a TeX conditional like
    /// `\ifthenelse{cond}{true}{false}` or `\ifstrempty{x}{empty}{nonempty}`.
    /// `branch_idx` is the 0-based index into the command's
    /// curly_group children (1 for the "true" branch of \ifthenelse,
    /// 2 for the "nonempty" branch of \ifstrempty). Source-byte
    /// scanning picks up curly_groups that tree-sitter attached as
    /// AST siblings; `skip_until` is advanced past them.
    fn emit_math_then_branch(&mut self, node: Node<'_>, branch_idx: usize) -> usize {
        self.emit_chosen_curly_branch(node, branch_idx, /* skip_optional_brack = */ false)
    }

    /// `\smash{X}`, `\raisebox{offset}{X}`, `\scalebox{factor}{X}`,
    /// `\mathgroup{N}{X}` — render only the *content* curly_group,
    /// dropping the positioning args. `content_idx` is the 0-based
    /// index (0 for `\smash` which takes only the content, 1 for the
    /// two-arg helpers). `\smash` also has an optional `[t]`/`[b]`
    /// we silently drop.
    fn emit_math_layout_inner(&mut self, node: Node<'_>, content_idx: usize) -> usize {
        self.emit_chosen_curly_branch(node, content_idx, /* skip_optional_brack = */ true)
    }

    /// Common helper for `emit_math_then_branch` and
    /// `emit_math_layout_inner`. Collects AST-child curly_groups plus
    /// any source-byte sibling `{...}` groups, renders the
    /// `target_idx`-th one as math, and bumps `skip_until` past the
    /// rest. When `skip_optional_brack` is true, also skips a leading
    /// `[...]` (the `\smash[t]` shape) before the curly groups.
    fn emit_chosen_curly_branch(
        &mut self,
        node: Node<'_>,
        target_idx: usize,
        skip_optional_brack: bool,
    ) -> usize {
        // Collect AST-child curly_groups.
        let mut cursor = node.walk();
        let mut curlys: Vec<(usize, usize)> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "curly_group")
            .map(|c| (c.start_byte(), c.end_byte()))
            .collect();

        // Source-byte sibling scan: optional `[...]` then any number of `{...}`.
        let bytes = self.src.as_bytes();
        let mut i = node.end_byte();
        if skip_optional_brack {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'[' {
                let mut j = i + 1;
                let mut depth = 0i32;
                while j < bytes.len() {
                    match bytes[j] {
                        b'\\' if j + 1 < bytes.len() => {
                            j += 2;
                            continue;
                        }
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b']' if depth == 0 => break,
                        _ => {}
                    }
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b']' {
                    i = j + 1;
                }
            }
        }
        loop {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] != b'{' {
                break;
            }
            let inner_start = i + 1;
            let mut j = inner_start;
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
            if j >= bytes.len() {
                break;
            }
            curlys.push((i, j + 1));
            i = j + 1;
        }
        // Dedup by start_byte (AST + source-byte sets may overlap).
        curlys.sort_by_key(|c| c.0);
        curlys.dedup_by_key(|c| c.0);

        let mut consumed = node.end_byte();
        if let Some((start, end)) = curlys.get(target_idx).copied() {
            let inner_src = self
                .src
                .get(start + 1..end.saturating_sub(1))
                .unwrap_or("")
                .to_string();
            let rendered = self.render_in_sub_emitter(&inner_src, true, true);
            self.out.push_str(rendered.trim());
        }
        if let Some((_, last_end)) = curlys.last().copied() {
            consumed = consumed.max(last_end);
        }
        if consumed > node.end_byte() {
            self.skip_until = self.skip_until.max(consumed);
        }
        consumed
    }

    // ─── Math layout & structures ─────────────────────────────────────────────

    fn emit_math_extensible_arrow(&mut self, node: Node<'_>, name: &str) -> usize {
        // Map command name → Typst arrow base symbol. The `x` family
        // is the "extensible" form (auto-stretched in LaTeX); Typst's
        // base arrow already auto-stretches when annotated, so we
        // just emit the base.
        let arrow = match name {
            "\\xrightarrow" => "arrow.r",
            "\\xleftarrow" => "arrow.l",
            "\\xLeftarrow" => "arrow.l.double",
            "\\xRightarrow" => "arrow.r.double",
            "\\xLeftrightarrow" => "arrow.l.r.double",
            "\\xleftrightarrow" => "arrow.l.r",
            "\\xmapsto" => "arrow.r.bar",
            "\\xhookleftarrow" => "arrow.l.hook",
            "\\xhookrightarrow" => "arrow.r.hook",
            "\\xtwoheadleftarrow" => "arrow.l.twohead",
            "\\xtwoheadrightarrow" => "arrow.r.twohead",
            "\\xleftharpoondown" => "harpoon.lb",
            "\\xleftharpoonup" => "harpoon.lt",
            "\\xrightharpoondown" => "harpoon.rb",
            "\\xrightharpoonup" => "harpoon.rt",
            _ => "arrow.r",
        };
        // Collect optional [below] and the mandatory {above}. They can
        // be AST children OR siblings depending on tree-sitter's parse
        // — `\xrightarrow{f}` typically has the `{f}` as a child of
        // the generic_command, while `\xrightarrow[g]{f}` sometimes
        // ends up with both as siblings of a bare `command_name`. Try
        // children first, then peek raw source.
        let mut cursor = node.walk();
        let mut below: Option<String> = None;
        let mut above: Option<String> = None;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "brack_group" if below.is_none() => {
                    let inner_start = child.start_byte() + 1;
                    let inner_end = child.end_byte().saturating_sub(1);
                    below = Some(
                        self.src
                            .get(inner_start..inner_end)
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                "curly_group" if above.is_none() => {
                    let inner_start = child.start_byte() + 1;
                    let inner_end = child.end_byte().saturating_sub(1);
                    above = Some(
                        self.src
                            .get(inner_start..inner_end)
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                _ => {}
            }
        }
        // Source-byte fallback: scan after node.end_byte() for
        // `[below]` and `{above}` we missed as AST siblings.
        let mut consumed_end = node.end_byte();
        let bytes = self.src.as_bytes();
        let mut cursor_bytes = consumed_end;
        // Skip whitespace.
        while cursor_bytes < bytes.len() && bytes[cursor_bytes].is_ascii_whitespace() {
            cursor_bytes += 1;
        }
        // Optional `[below]`.
        if below.is_none() && cursor_bytes < bytes.len() && bytes[cursor_bytes] == b'[' {
            let inner_start = cursor_bytes + 1;
            let mut j = inner_start;
            let mut depth = 0i32;
            while j < bytes.len() {
                match bytes[j] {
                    b'\\' if j + 1 < bytes.len() => {
                        j += 2;
                        continue;
                    }
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b']' if depth == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b']' {
                below = Some(self.src[inner_start..j].to_string());
                cursor_bytes = j + 1;
                consumed_end = cursor_bytes;
                while cursor_bytes < bytes.len() && bytes[cursor_bytes].is_ascii_whitespace() {
                    cursor_bytes += 1;
                }
            }
        }
        // Mandatory `{above}`.
        if above.is_none() && cursor_bytes < bytes.len() && bytes[cursor_bytes] == b'{' {
            let inner_start = cursor_bytes + 1;
            let mut j = inner_start;
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
            if j < bytes.len() && bytes[j] == b'}' {
                above = Some(self.src[inner_start..j].to_string());
                consumed_end = j + 1;
            }
        }
        self.ensure_math_letter_boundary(arrow);
        self.out.push_str(arrow);
        // Render labels in math context so contained symbols translate.
        if let Some(a) = above {
            let rendered = self.render_in_sub_emitter(&a, true, true);
            let _ = write!(self.out, "^({})", rendered.trim());
        }
        if let Some(b) = below {
            let rendered = self.render_in_sub_emitter(&b, true, true);
            let _ = write!(self.out, "_({})", rendered.trim());
        }
        // Mark source-byte-consumed labels as already-emitted so the
        // AST walker doesn't re-emit them as raw text.
        if consumed_end > node.end_byte() {
            self.skip_until = self.skip_until.max(consumed_end);
        }
        consumed_end
    }

    /// `\phantom{X}` / `\hphantom{X}` / `\vphantom{X}` → `#hide[$X$]`.
    /// `hide` is a content function so it needs the `#` escape inside math,
    /// and the argument must be a math content block `[$...$]`.
    fn emit_math_phantom(&mut self, node: Node<'_>) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let inner = self.render_math_group(arg);
            let _ = write!(self.out, "#hide[${}$]", inner.trim());
            return node.end_byte();
        }
        node.end_byte()
    }

    /// Wrap the first curly_group argument in a Typst math function call:
    /// `\mathbf{X}` → `bold(X)`. Recursively renders the inner content in
    /// math mode so nested commands are translated.
    fn emit_math_wrap(&mut self, node: Node<'_>, left: &str, right: &str) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let inner = self.render_math_group(arg);
            let inner_trimmed = inner.trim();
            self.ensure_math_letter_boundary(left);
            // In Typst math `func(a,b)` passes two arguments — comma is an
            // arg separator. When the inner expression contains a comma and
            // the wrapper is a simple `funcname(` call (no named args already
            // in `left`), switch to content-block syntax `funcname[inner]`
            // where commas are inert content, not separators.
            let prefix = left.strip_suffix('(');
            if inner_trimmed.contains(',')
                && prefix.is_some_and(|p| !p.contains(','))
                && right == ")"
            {
                self.out.push_str(prefix.unwrap());
                self.out.push('[');
                self.out.push_str(inner_trimmed);
                self.out.push(']');
            } else {
                self.out.push_str(left);
                self.out.push_str(inner_trimmed);
                self.out.push_str(right);
            }
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
        let arg_render = self.render_braceless_math_arg(parsed_arg);
        self.ensure_math_letter_boundary(left);
        self.out.push_str(left);
        self.out.push_str(arg_render.trim());
        self.out.push_str(right);
        // Mark the consumed argument range as already-emitted.
        self.skip_until = self.skip_until.max(arg_end);
        arg_end
    }

    /// `\binom{n}{k}` → `binom(n, k)`. Also accepts brace-less
    /// `\binom n k` by consuming up to two trailing tokens.
    fn emit_math_binom(&mut self, node: Node<'_>) -> usize {
        let mut cursor = node.walk();
        let groups: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "curly_group")
            .collect();
        let mut rendered: Vec<String> = groups.iter().map(|g| self.render_math_group(*g)).collect();
        let mut consumed_end = node.end_byte();
        while rendered.len() < 2 {
            match try_consume_math_arg(self.src, consumed_end) {
                Some((arg, end)) => {
                    rendered.push(self.render_braceless_math_arg(arg));
                    consumed_end = end;
                }
                None => break,
            }
        }
        if rendered.len() < 2 {
            self.warn_ambiguous_math(node, "\\binom (missing args)");
            return node.end_byte();
        }
        if consumed_end > node.end_byte() {
            self.skip_until = self.skip_until.max(consumed_end);
        }
        self.ensure_math_letter_boundary("binom(");
        let _ = write!(
            self.out,
            "binom({}, {})",
            rendered[0].trim(),
            rendered[1].trim()
        );
        consumed_end
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
                    // Bug #33: a bare-letter subscript (`_h`) followed
                    // by a letter token (`j` in `\{g_hj\}`) fuses into
                    // `hj` because Typst greedily consumes alphanumeric
                    // chars after `_`. Drop a MATH_WORD_BOUNDARY
                    // sentinel so `collapse_math_spaces` inserts a
                    // separator when the next token is letter/digit.
                    if boundary::needs_trailing_sentinel(trimmed, false) {
                        self.out.push(MATH_WORD_BOUNDARY);
                    }
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
            emitter.emit_math_node_slice(inner);
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

        // Split on the `\` token that our math-mode `\\` emitter writes.
        // Pre-Bug #20 it was always ` \` (leading space); the Bug #20
        // fix appends `\n` so the format is `\\n`. Sources with no
        // space before `\\` (common in `\begin{smallmatrix}...\\...`)
        // are not caught by the pre-fix splitter. Use a manual scan
        // that finds the row-break char unambiguously.
        let rows: Vec<&str> = split_math_rows(&body_str);
        let rendered: Vec<String> = rows
            .into_iter()
            .map(|row| {
                row.split('&')
                    .map(|cell| cell.trim().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect();
        // Bug #26: a preceding identifier letter (e.g. `Q\begin{pmatrix}`)
        // fuses with `mat(` into the undefined identifier `Qmat`. Insert
        // a space when the previous output ends in a letter — same shape
        // as `push_math_symbol` / `emit_math_wrap` guards.
        self.ensure_math_letter_boundary("mat(");
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
        // Row break in LaTeX cases is `\\`. The render walker emitted
        // that as `\` (with optional leading space / trailing newline).
        // Use the same scan helper as the matrix emitter so the row
        // break is found regardless of whether the source had a space
        // before the `\\` (Bug #31 driver).
        let rows: Vec<String> = split_math_rows(&body_str)
            .into_iter()
            .map(|r| {
                let r = r.trim();
                // Inside a row, `&` separates value from condition.
                // Replace with ` quad ` (an em of horizontal space) and
                // wrap the row in `[...]` so internal commas are
                // preserved as content, not parsed as cases separators.
                let row = r.replace('&', " quad ");
                // Pre-escape any unbalanced parens INSIDE this row before
                // wrapping it in `[...]`. Without this, an extra `)` from a
                // malformed LaTeX source (e.g. stray `)` inside `\frac{}{}`)
                // leaks into the global math body and causes the outer
                // `cases(...)` closing paren to be incorrectly identified
                // as unbalanced by `escape_unbalanced_math_brackets`.
                let row = escape_unbalanced_math_brackets(&row);
                format!("[{}]", row)
            })
            .filter(|r| r != "[]")
            .collect();
        // Letter-boundary guard so a preceding identifier doesn't fuse
        // with the leading `c` of `cases(` (same shape as Bug #26 for
        // `mat(`).
        self.ensure_math_letter_boundary("cases(");
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
        // Rows are split on the rendered row-break (`\` from the
        // emit_math_command `\\` handler) — same as emit_cases_env.
        // Use the manual scan via `split_math_rows`.
        let rows: Vec<String> = split_math_rows(&body_str)
            .into_iter()
            .map(|r| {
                let r = r.trim();
                // Cells: `&` separator gets collapsed to `quad`. Wrap
                // the whole row in `[content]` so internal commas
                // don't get read as cases() argument separators.
                // Pre-escape unbalanced parens as in emit_cases_env.
                let row = r.replace('&', " quad ");
                let row = escape_unbalanced_math_brackets(&row);
                format!("[{}]", row)
            })
            .filter(|r| r != "[]")
            .collect();
        let _ = write!(self.out, "cases({})", rows.join(", "));
        node.end_byte()
    }

    // ─── Cross-references & bibliography ──────────────────────────────────────

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
        // PR-3: validate each key against the harvested set of
        // available bibliography entries. Keys that aren't defined
        // would crash Typst with `label <key> does not exist` —
        // emit a plain-text placeholder instead and warn once.
        //
        // Skip validation entirely when the set is empty (legacy
        // bare-string convert calls with no base_dir to scan, or
        // papers with no bibliography at all). In that mode we
        // assume the user provided the right keys and preserve the
        // old behaviour.
        let mut missing: Vec<&str> = Vec::new();
        let mut typst_parts: Vec<String> = Vec::new();
        for raw_key in &keys {
            let sanitized = sanitize_label_key(raw_key);
            if !self.bibliography_keys.is_empty() && !self.bibliography_keys.contains(&sanitized) {
                missing.push(raw_key.as_str());
                typst_parts.push(format!("[cite: missing key `{}`]", raw_key));
            } else {
                typst_parts.push(format!("@{}", sanitized));
            }
        }
        for miss in &missing {
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::NeedsManualReview {
                    reason: format!("\\cite{{{}}}: key not found in any bibliography", miss),
                },
                severity: Severity::Warning,
                message: format!(
                    "cite key `{}` is not defined in any `.bib`/`.bbl` — emitting a plain-text placeholder",
                    miss
                ),
                snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                suggested_skill: None,
            });
        }
        let typst = typst_parts.join(" ");
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
        // curly group with the label(s).
        let mut cursor = node.walk();
        let first_kind = node
            .children(&mut cursor)
            .next()
            .map(|c| c.kind().to_string());
        let (raw_keys, end_after_brace) = match extract_label_ref_keys_and_end(node, self.src) {
            Some(x) => x,
            None => {
                self.warn_unsupported_command(node);
                return node.end_byte();
            }
        };
        // Sanitize each key independently — `sanitize_label_key` maps
        // non-[A-Za-z0-9_\-:.] chars (incl. commas) to `-`. By splitting
        // first and sanitizing per-key, `\cref{a,b}` → ["a", "b"] rather
        // than the old single-string "a-b" (Bug #45).
        let keys: Vec<String> = raw_keys
            .iter()
            .map(|k| sanitize_label_key(k))
            .filter(|k| !k.is_empty())
            .collect();
        if keys.is_empty() {
            self.warn_unsupported_command(node);
            return node.end_byte();
        }
        // Cover the truncated-grammar tail (`_objective}`) when a key
        // contains underscores — same as `\label{...}` handling above.
        self.skip_until = self.skip_until.max(end_after_brace);
        // Bug #24: inside math mode, a bare `@key` is parsed by Typst as
        // an identifier. Wrap in `#ref(<key>)` to escape math context.
        let in_math = self.in_math;
        match first_kind.as_deref() {
            Some("\\eqref") => {
                self.needs_equation_numbering = true;
                // Wrap the full comma-separated list in one pair of parens —
                // `\eqref{a,b}` → `(@a, @b)`, matching LaTeX convention.
                let parts: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        if in_math {
                            format!("#ref(<{}>)", k)
                        } else {
                            format!("@{}", k)
                        }
                    })
                    .collect();
                let _ = write!(self.out, "({})", parts.join(", "));
            }
            Some("\\pageref") => {
                // Typst doesn't have a direct equivalent; warn once and emit refs.
                let parts: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        if in_math {
                            format!("#ref(<{}>)", k)
                        } else {
                            format!("@{}", k)
                        }
                    })
                    .collect();
                self.out.push_str(&parts.join(", "));
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
                // Heuristic: prefix tells us what the ref targets. Apply
                // over all keys (any key matching triggers the flag).
                for k in &keys {
                    if k.starts_with("eq:") || k.starts_with("eqn:") {
                        self.needs_equation_numbering = true;
                    } else if !k.starts_with("fig:")
                        && !k.starts_with("tab:")
                        && !k.starts_with("thm:")
                        && !k.starts_with("lem:")
                        && !k.starts_with("cor:")
                        && !k.starts_with("def:")
                        && !k.starts_with("prop:")
                    {
                        self.needs_heading_numbering = true;
                    }
                }
                let parts: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        if in_math {
                            format!("#ref(<{}>)", k)
                        } else {
                            format!("@{}", k)
                        }
                    })
                    .collect();
                self.out.push_str(&parts.join(", "));
            }
        }
        // Typst labels include `-`, `.`, `:`, etc. If the source has
        // `\ref{A}--\ref{B}` with no space between, Typst will glue the dashes
        // onto the label. Append an explicit space when the next source byte
        // would form an identifier-continuation character.
        let end = node.end_byte();
        if let Some(&b) = self.src.as_bytes().get(end) {
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
        // Bug #27: Typst's `#bibliography` aborts when ANY listed path
        // doesn't resolve on disk. arXiv preprints frequently bundle
        // only a subset of the `.bib` files the LaTeX `\bibliography`
        // call lists (other entries may be from local TeX-distribution
        // paths). Filter to just the paths that resolve, emit a
        // `needs_manual_review` warning for the missing ones, and
        // skip the call entirely if NONE resolve.
        let mut kept: Vec<(String, String)> = Vec::new();
        let mut missing: Vec<String> = Vec::new();
        if let Some(ref base) = self.base_dir.clone() {
            for (raw, with_ext) in paths.iter().zip(paths_with_ext.iter()) {
                if let Some(source_path) = probe_bib_on_disk(base, raw) {
                    self.asset_refs.push(crate::AssetRef {
                        kind: crate::AssetKind::Bibliography,
                        typst_path: with_ext.clone(),
                        source_path,
                    });
                    kept.push((raw.clone(), with_ext.clone()));
                } else {
                    missing.push(with_ext.clone());
                }
            }
        } else {
            // No base_dir to probe — keep all paths as-is (legacy
            // bare-string convert call; the user is responsible for
            // file resolution).
            for (raw, with_ext) in paths.iter().zip(paths_with_ext.iter()) {
                kept.push((raw.clone(), with_ext.clone()));
            }
        }
        for miss in &missing {
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::NeedsManualReview {
                    reason: format!("\\bibliography references missing file: {}", miss),
                },
                severity: Severity::Warning,
                message: format!(
                    "bibliography file `{}` not found in source tree — \
                     omitted from #bibliography(...) so the rest still compiles",
                    miss
                ),
                snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                suggested_skill: None,
            });
        }
        if kept.is_empty() {
            // PR-2: before giving up, look for a pre-rendered `.bbl`
            // file in base_dir. arXiv preprints whose authors only
            // ship the BibTeX-output `.bbl` (no `.bib` source) are
            // common — e.g. 2605.22159 ships only `GS4AGBEM.bbl`.
            // The `.bbl` is LaTeX text containing
            // `\begin{thebibliography}{...}\bibitem{k}...\end{thebibliography}`
            // which our existing `emit_thebibliography` already
            // handles. Inline its content as fresh LaTeX source.
            if let Some(ref base) = self.base_dir.clone() {
                if let Some(bbl_content) = probe_any_bbl(base) {
                    self.warnings.push(Warning {
                        range: range_of(node),
                        category: Category::NeedsManualReview {
                            reason: "\\bibliography{...}: no `.bib` found, rendered `.bbl` inlined as fallback".to_string(),
                        },
                        severity: Severity::Info,
                        message: "the LaTeX `.bib` source was not bundled; inlined the pre-rendered `.bbl` instead — entries should still resolve for `\\cite{}` lookups, but the bibliography style is whatever the original BibTeX produced".to_string(),
                        snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                        suggested_skill: None,
                    });
                    let rendered = self.render_in_sub_emitter(&bbl_content, false, false);
                    self.ensure_paragraph_break();
                    self.out.push_str(rendered.trim_end());
                    return node.end_byte();
                }
            }
            // No `.bib` AND no `.bbl` — drop the call entirely.
            self.warnings.push(Warning {
                range: range_of(node),
                category: Category::NeedsManualReview {
                    reason: "\\bibliography{...}: no listed file resolved on disk".to_string(),
                },
                severity: Severity::Warning,
                message: "all bibliography paths missing; #bibliography call dropped".to_string(),
                snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                suggested_skill: None,
            });
            return node.end_byte();
        }
        self.ensure_paragraph_break();
        let mapped = style.as_deref().and_then(map_bibliography_style);
        // Typst's `#bibliography` takes either a single path string or a
        // tuple of paths. Emit the tuple form when we have multiple.
        let path_arg = if kept.len() == 1 {
            format!("\"{}\"", kept[0].1)
        } else {
            let joined = kept
                .iter()
                .map(|(_raw, with_ext)| format!("\"{}\"", with_ext))
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

    // ─── Figures, graphics & tabular ──────────────────────────────────────────

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
        // Bug #37: resolve the image path with extension. LaTeX
        // `\includegraphics{foo}` omits the extension; Typst's
        // `image()` requires it. When we find `foo.png` on disk,
        // emit `image("foo.png")` rather than the bare `image("foo")`
        // which Typst rejects with `file not found`.
        let mut resolved_path = path.clone();
        // Probe the image relative to the current file's dir first, then the
        // project root. LaTeX resolves `\includegraphics` paths from the MAIN
        // document's directory, so a figure `figures/x.png` referenced inside an
        // `\input`-ed `appendix/foo.tex` lives at `<root>/figures/x.png`, not
        // `<root>/appendix/figures/x.png`. Without the root_dir fallback every
        // figure in an `\input`-ed file resolves as "missing" (Bug D6).
        let mut probed_source: Option<PathBuf> = None;
        let probe_dirs: Vec<PathBuf> = {
            let mut v = Vec::new();
            if let Some(ref b) = self.base_dir {
                v.push(b.clone());
            }
            if let Some(ref r) = self.root_dir {
                if !v.contains(r) {
                    v.push(r.clone());
                }
            }
            // `\graphicspath` search dirs, resolved relative to the project root
            // (then base_dir as a fallback). LaTeX searches these for a bare
            // `\includegraphics{name}` whose file isn't directly under the
            // current/root dir (D7).
            for gp in &self.graphics_paths {
                if let Some(ref r) = self.root_dir {
                    v.push(r.join(gp));
                }
                if let Some(ref b) = self.base_dir {
                    let cand = b.join(gp);
                    if !v.contains(&cand) {
                        v.push(cand);
                    }
                }
            }
            v
        };
        if let Some(source_path) =
            probe_dirs.iter().find_map(|d| probe_image_on_disk(d, &path))
        {
            if std::path::Path::new(&path).extension().is_none() {
                if let Some(name) = source_path.file_name().and_then(|n| n.to_str()) {
                    let dir = std::path::Path::new(&path)
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or("");
                    resolved_path = if dir.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}/{}", dir, name)
                    };
                }
            }
            probed_source = Some(source_path);
        }
        let mut args = format!("\"{}\"", resolved_path);
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
            match probed_source {
                Some(source_path) => {
                    self.asset_refs.push(crate::AssetRef {
                        kind: crate::AssetKind::Image,
                        typst_path: resolved_path.clone(),
                        source_path,
                    });
                }
                None => {
                    // Bug #37b: when probe fails (file not found in
                    // the source tree — common when LaTeX uses
                    // `\graphicspath{{./fig/}}` or when the arXiv
                    // bundle omits a figure), emit a compileable
                    // placeholder rect instead of `image("...")`
                    // which would abort typst compile.
                    self.warnings.push(Warning {
                        range: range_of(node),
                        category: Category::NeedsManualReview {
                            reason: format!("image not found relative to base: {}", path),
                        },
                        severity: Severity::Warning,
                        message: format!(
                            "could not resolve `\\includegraphics{{{}}}` against `{}` — emitting a placeholder. The original asset is missing from the source tree.",
                            path,
                            base.display()
                        ),
                        snippet: self.src[node.start_byte()..node.end_byte()].to_string(),
                        suggested_skill: None,
                    });
                    let _ = write!(
                        self.out,
                        "rect(width: 60%, height: 4em, stroke: 0.5pt, fill: luma(240))[#align(center + horizon)[`{}` (missing)]]",
                        path
                    );
                    return node.end_byte();
                }
            }
        }
        let _ = write!(self.out, "image({})", args);
        node.end_byte()
    }

    /// `\begin{figure}...\caption{X}...\label{fig:y}...\end{figure}` →
    /// `#figure(image(...), caption: [X]) <fig:y>`.
    /// Render one `subfigure` environment as a Typst figure panel:
    /// `figure(image("..."), caption: [sub-caption])`. Returns `None` when the
    /// subfigure has no `\includegraphics` (nothing to show). The panel is bare
    /// (no `#`) so it can sit inside a `grid(...)` argument. Subfigure `\label`s
    /// are NOT attached here — `emit_figure` collects every subfigure label
    /// into its outer set and anchors the referenced ones (so a `\ref` to a
    /// dropped/image-less panel still resolves).
    fn render_subfigure_panel(&mut self, node: Node<'_>) -> Option<String> {
        let mut graphics: Option<Node<'_>> = None;
        let mut caption: Option<Node<'_>> = None;
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                match child.kind() {
                    "graphics_include" if graphics.is_none() => graphics = Some(child),
                    "caption" if caption.is_none() => caption = Some(child),
                    _ => stack.push(child),
                }
            }
        }
        let g = graphics?;
        let img = self.with_sub_buffer(|e| {
            e.emit_graphics_include(g);
        });
        let mut panel = format!("figure({}", img.trim());
        if let Some(c) = caption {
            if let Some(arg) = first_curly_group(c) {
                let text = self.render_curly_group_content(arg);
                let _ = write!(panel, ", caption: [{}]", text);
            }
        }
        panel.push(')');
        Some(panel)
    }

    fn emit_figure(&mut self, node: Node<'_>) -> usize {
        let mut graphics: Option<Node<'_>> = None;
        let mut caption: Option<Node<'_>> = None;
        // `\captionof{type}{cap}` fallback, used only when no real `\caption`
        // is present (a real \caption always wins regardless of walk order).
        let mut captionof: Option<Node<'_>> = None;
        // All `\label`s in the float (a main label plus subfigure labels, or
        // two `\captionof` blocks). Typst keeps one per element, so we attach
        // the referenced alias and anchor the other referenced ones.
        let mut labels: Vec<String> = Vec::new();
        let mut nested_tabular: Option<Node<'_>> = None;
        // `\input{file}` nodes inside the float — the tabular often lives in a
        // separate file (`\begin{table}{\input{results}}...`), so when no inline
        // tabular is found we resolve these to recover the table body.
        let mut includes: Vec<Node<'_>> = Vec::new();
        // `subfigure` environments — each holds its own `\includegraphics` and
        // sub-`\caption`. A figure with N subfigures must emit ALL N images, not
        // just one (Bug D5); collected here and rendered as a grid of panels.
        let mut subfigures: Vec<Node<'_>> = Vec::new();

        // Walk the entire subtree because IEEE-style templates often wrap
        // `\includegraphics` in `\centerline{...}` or `\centering{...}`.
        let mut stack: Vec<Node<'_>> = vec![node];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                match child.kind() {
                    "graphics_include" if graphics.is_none() => graphics = Some(child),
                    "latex_include" => includes.push(child),
                    "caption" if caption.is_none() => caption = Some(child),
                    // `\captionof{type}{cap}` (caption package) — a caption
                    // source too. Captured only if no real `\caption` won yet;
                    // its 2nd arg is the caption, its 1st arg the kind.
                    "generic_command"
                        if captionof.is_none()
                            && command_name_text(child, self.src).as_deref()
                                == Some("\\captionof") =>
                    {
                        captionof = Some(child);
                    }
                    "label_definition" => {
                        if let Some(k) = extract_label_name(child, self.src) {
                            if !labels.contains(&k) {
                                labels.push(k);
                            }
                        }
                    }
                    "generic_environment" => {
                        let env = environment_name(child, self.src);
                        if matches!(
                            env.as_deref(),
                            Some("subfigure") | Some("subcaptionblock") | Some("subfloat")
                        ) {
                            // Capture the whole subfigure as a panel; do NOT
                            // descend for graphics/caption (those belong to the
                            // panel). BUT still collect its `\label`s into the
                            // outer set: a subfigure may be `\ref`'d, and if it
                            // has no image its panel is dropped — its label must
                            // still be anchored by the outer figure or the
                            // reference dangles ("label does not exist").
                            let mut sc = child.walk();
                            let mut sub_stack: Vec<Node<'_>> = child.children(&mut sc).collect();
                            while let Some(sn) = sub_stack.pop() {
                                if sn.kind() == "label_definition" {
                                    if let Some(k) = extract_label_name(sn, self.src) {
                                        if !labels.contains(&k) {
                                            labels.push(k);
                                        }
                                    }
                                }
                                let mut c2 = sn.walk();
                                for gc in sn.children(&mut c2) {
                                    sub_stack.push(gc);
                                }
                            }
                            subfigures.push(child);
                            continue;
                        }
                        if matches!(
                            env.as_deref(),
                            Some("tabular")
                                | Some("tabular*")
                                | Some("tabularx")
                                | Some("tabulary")
                                | Some("array")
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

        // Harvest `\label`s from any `\input`-ed float body (e.g. an `algorithm`
        // float whose `algorithmic` + `\State\label{alg:step:N}` live in a
        // separate file: `\begin{algorithm}\input{Alg/iDANSE}\caption{}\end{...}`,
        // corpus 2605.31510). Those labels aren't AST children of this node, so
        // the walk above can't see them; without this the `\cref{alg:step:N}`
        // references dangle → compile failure. Merge them into the label set so
        // the anchor loop below emits the referenced ones.
        for inc in &includes {
            for k in self.labels_from_include(*inc) {
                if !labels.contains(&k) {
                    labels.push(k);
                }
            }
        }

        // Whether the float's body is a tabular (vs an image): when it is, the
        // emitted `#figure` must carry `kind: table` so Typst captions/refs read
        // "Table N" rather than the default "Figure N".
        // Render subfigure panels up front (Bug D5): each becomes its own
        // `figure(image(...), caption: [..])`. Empty when there are no
        // subfigures or none yields an image — in which case we fall back to
        // the single-graphic / tabular / placeholder chain below.
        let panels: Vec<String> = subfigures
            .iter()
            .filter_map(|sf| self.render_subfigure_panel(*sf))
            .collect();

        let mut body_is_table = false;
        let body_str = if !panels.is_empty() {
            // Multi-panel figure: lay the panels out in a grid so every image
            // survives. A single surviving panel collapses to just that panel.
            if panels.len() == 1 {
                panels.into_iter().next().unwrap()
            } else {
                format!(
                    "grid(\n  columns: 2,\n  gutter: 0.5em,\n  {}\n)",
                    panels.join(",\n  ")
                )
            }
        } else if let Some(g) = graphics {
            self.with_sub_buffer(|emitter| {
                emitter.emit_graphics_include(g);
            })
        } else if let Some(t) = nested_tabular {
            // `\begin{table}` wrapping a `tabular` (common IEEE pattern).
            // emit_tabular writes `#table(...)`; strip the leading `#` since
            // inside a `#figure(...)` argument the function call must be bare.
            body_is_table = true;
            let s = self
                .with_sub_buffer(|emitter| {
                    emitter.emit_tabular(t);
                })
                .trim()
                .to_string();
            s.strip_prefix('#').map(|s| s.to_string()).unwrap_or(s)
        } else if let Some(tbl) = includes
            .iter()
            .find_map(|inc| self.tabular_from_include(*inc))
        {
            // `\begin{table}{\input{results}}` — the tabular lives in an
            // `\input`-ed file. Render it (already a bare `table(...)`).
            body_is_table = true;
            tbl
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
        // Decide the figure `kind`. An explicit `\captionof{type}` wins (it names
        // the type directly); otherwise a tabular body implies `kind: table` so
        // refs read "Table N". An image body uses Typst's default (no `kind:`).
        let mut kind: Option<&str> = None;
        if caption.is_none() {
            if let Some(c) = captionof {
                if let Some(type_arg) = nth_curly_group(c, 0) {
                    let ty = self.render_curly_group_content(type_arg);
                    kind = match ty.trim() {
                        "table" => Some("table"),
                        "figure" => Some("image"),
                        _ => None,
                    };
                }
            }
        }
        if kind.is_none() && body_is_table {
            kind = Some("table");
        }
        if let Some(k) = kind {
            let _ = write!(self.out, ",\n  kind: {}", k);
        }
        let caption_node = caption.or(captionof);
        if let Some(c) = caption_node {
            // `\caption{cap}` → 1st group; `\captionof{type}{cap}` → 2nd group.
            let arg = if c.kind() == "generic_command" {
                nth_curly_group(c, 1)
            } else {
                first_curly_group(c)
            };
            if let Some(arg) = arg {
                let text = self.render_curly_group_content(arg);
                let _ = write!(self.out, ",\n  caption: [{}]", text);
            }
        }
        self.out.push_str(",\n)");
        // Attach the referenced alias (or the first label); then give every
        // OTHER referenced label its own hidden, referenceable anchor — a
        // single float (subfigures, or two `\captionof`s) can be `\ref`'d
        // under several labels, but Typst allows only one label per element.
        let primary = self.pick_label_to_attach(&labels);
        if let Some(l) = &primary {
            let _ = write!(self.out, " <{}>", l);
        }
        for l in &labels {
            if Some(l) != primary.as_ref()
                && self.referenced_labels.contains(&sanitize_label_key(l))
            {
                let _ = write!(self.out, "\n#hide[#figure([]) <{}>]", l);
            }
        }
        node.end_byte()
    }

    /// If `inc` is a `\input{file}` whose resolved file contains a `tabular`
    /// (or array family) environment, render that file in a sub-emitter and
    /// return the bare `table(...)` body (the leading `#` stripped) so it can be
    /// spliced into a `#figure(...)`. Returns `None` when there's no base dir,
    /// Resolve an `\input`-ed file referenced by `inc` and return the keys of
    /// every `\label{...}` it defines. Used by `emit_figure` to recover labels
    /// from a float body that lives in a separate file (e.g. an `algorithm`
    /// float `\input`-ing its `algorithmic` steps). Best-effort: returns empty
    /// when there's no base dir, the path doesn't resolve, or the file can't be
    /// read. A regex suffices — `\label{key}` is unambiguous.
    fn labels_from_include(&self, inc: Node<'_>) -> Vec<String> {
        let Some(base_dir) = self.base_dir.clone() else {
            return Vec::new();
        };
        let Some(raw_path) = extract_latex_include_path(inc, self.src) else {
            return Vec::new();
        };
        let resolved = resolve_input_path(&base_dir, &raw_path).or_else(|| {
            self.root_dir
                .as_deref()
                .filter(|r| *r != base_dir.as_path())
                .and_then(|r| resolve_input_path(r, &raw_path))
        });
        let Some(resolved) = resolved else {
            return Vec::new();
        };
        let Ok(source) = std::fs::read_to_string(&resolved) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        // Scan for `\label{...}` (honoring balanced braces in the key is
        // unnecessary — label keys don't contain braces). Skip `%`-commented.
        for line in source.lines() {
            let line = match line.find('%') {
                Some(p) if p == 0 || line.as_bytes()[p - 1] != b'\\' => &line[..p],
                _ => line,
            };
            let mut rest = line;
            while let Some(pos) = rest.find("\\label{") {
                let after = &rest[pos + "\\label{".len()..];
                if let Some(end) = after.find('}') {
                    let key = after[..end].trim();
                    if !key.is_empty() {
                        out.push(key.to_string());
                    }
                    rest = &after[end + 1..];
                } else {
                    break;
                }
            }
        }
        out
    }

    /// the path doesn't resolve, the file can't be read, or it has no tabular.
    /// Best-effort: emits no warnings (the caller falls back to its own path).
    fn tabular_from_include(&mut self, inc: Node<'_>) -> Option<String> {
        let base_dir = self.base_dir.clone()?;
        let raw_path = extract_latex_include_path(inc, self.src)?;
        let resolved = resolve_input_path(&base_dir, &raw_path).or_else(|| {
            self.root_dir
                .as_deref()
                .filter(|r| *r != base_dir.as_path())
                .and_then(|r| resolve_input_path(r, &raw_path))
        })?;
        let source = std::fs::read_to_string(&resolved).ok()?;
        // Cheap pre-check: only parse when a tabular-family env is present.
        if !source.contains("\\begin{tabular")
            && !source.contains("\\begin{array")
            && !source.contains("\\begin{tabulary")
            && !source.contains("\\begin{tabularx")
        {
            return None;
        }
        let tree = crate::parser::parse(&source);
        // Find the first tabular-family environment in the included file.
        let mut stack = vec![tree.root_node()];
        let mut tabular: Option<Node<'_>> = None;
        while let Some(n) = stack.pop() {
            if n.kind() == "generic_environment"
                && matches!(
                    environment_name(n, &source).as_deref(),
                    Some("tabular") | Some("tabular*") | Some("tabularx")
                        | Some("tabulary") | Some("array")
                )
            {
                tabular = Some(n);
                break;
            }
            let mut cursor = n.walk();
            for ch in n.children(&mut cursor) {
                stack.push(ch);
            }
        }
        let tabular = tabular?;
        // Render the tabular through a sub-emitter bound to the INCLUDED file's
        // source (the node borrows `source`, not `self.src`), then strip the
        // leading `#` so it sits inside the `#figure(...)` call.
        let visited = std::mem::take(&mut self.visited_includes);
        let macros = self.macros.clone();
        let mut sub = Emitter::with_includes(
            &source,
            self.source_name,
            self.base_dir.clone(),
            visited,
        );
        sub.macros = macros;
        sub.referenced_labels = self.referenced_labels.clone();
        let rendered = sub.with_sub_buffer(|e| {
            e.emit_tabular(tabular);
        });
        // Merge side-effects back (warnings/assets discovered while rendering).
        self.visited_includes = std::mem::take(&mut sub.visited_includes);
        self.warnings.append(&mut sub.warnings);
        self.asset_refs.append(&mut sub.asset_refs);
        let s = rendered.trim().to_string();
        if s.is_empty() {
            return None;
        }
        Some(s.strip_prefix('#').map(str::to_string).unwrap_or(s))
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

        // Collect body children (everything between begin and end). Skip only
        // the LEADING column-spec curly_group (and the preceding `{width}` group
        // for tabular*/tabularx/tabulary) — NOT every curly_group: a cell can be
        // brace-wrapped (`{$\Braket{…}$}`, `{\small …}`), and dropping it here
        // made the parent gap-copy spill its raw LaTeX (corpus 2605.31203;
        // 22507 `\small`/`\textpm` leak).
        let leading_groups_to_skip = if needs_skip { 2 } else { 1 };
        let mut cursor = node.walk();
        let mut cg_seen = 0usize;
        let body: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| {
                if matches!(c.kind(), "begin" | "end") {
                    return false;
                }
                if c.kind() == "curly_group" {
                    cg_seen += 1;
                    return cg_seen > leading_groups_to_skip;
                }
                true
            })
            .collect();

        // Render body to a string, then parse rows + cells. Clear `in_minipage`
        // around the body: this table's own row-break `\\` must stay the bare
        // `\` that `split_math_rows` keys on, even when the table is itself
        // nested inside a minipage (otherwise the inner rows would collapse into
        // `#linebreak()`s and cells would be dropped).
        let saved_in_minipage = self.in_minipage;
        self.in_minipage = false;
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
        self.in_minipage = saved_in_minipage;

        // Strip \hline (already emitted as raw text by the default emitter).
        let cleaned = body_str.replace("\\hline", "");
        // Rows are separated by `\` followed by whitespace (the LaTeX
        // `\\` row break, which our `\\` emitter writes as a single
        // backslash). Use `split_math_rows` (Bug #31's helper) so we
        // don't accidentally split inside escape sequences like
        // `\$`, `\_`, `\*` that legitimately appear in cell content
        // — e.g. `\multicolumn{2}{c}{\textbf{\$10.23}}` which used
        // to fragment at every `\$`/`\*` and corrupt the table.
        let rows: Vec<&str> = split_math_rows(&cleaned)
            .into_iter()
            .filter(|r| !r.trim().is_empty())
            .collect();
        // Build per-row cell lists so we can track rowspan/colspan occupancy.
        // (`split_math_rows` already consumed any `\\[len]` vertical-space arg.)
        let rows_2d: Vec<Vec<String>> = rows
            .iter()
            .map(|row| row.split('&').map(|c| c.trim().to_string()).collect())
            .collect();

        // Booktabs styling. Typst's default table draws a full grid (a line
        // around every cell); academic papers (≈75% of the corpus) instead use
        // booktabs — no vertical lines, three horizontal rules (top / after the
        // header / bottom). And a LaTeX tabular with NO rule commands draws no
        // lines at all. So: `stroke: none` always (kills the spurious grid),
        // and add booktabs rules only when the source actually ruled the table.
        let raw_env = &self.src[node.start_byte()..node.end_byte()];
        let has_rules = raw_env.contains("\\toprule")
            || raw_env.contains("\\midrule")
            || raw_env.contains("\\bottomrule")
            || raw_env.contains("\\hline")
            || raw_env.contains("\\cmidrule");

        self.ensure_paragraph_break();
        let _ = write!(
            self.out,
            "#table(\n  columns: {},\n  align: ({}),\n  stroke: none,\n",
            count,
            aligns.join(", ")
        );
        if has_rules {
            // Top rule (heavier), then the header rule is injected after the
            // first emitted row below.
            self.out.push_str("  table.hline(stroke: 0.08em),\n");
        }

        // rowspan_cols[c] = number of additional rows for which column c is
        // already occupied by a rowspan cell from a previous row.  When we
        // encounter a rowspan=N cell at column c we set rowspan_cols[c] = N-1.
        // Each subsequent visit to that column decrements the counter.
        let mut rowspan_cols = vec![0usize; count];

        let mut emitted_rows = 0usize;
        for row_cells in &rows_2d {
            let mut row_output: Vec<String> = Vec::new();
            let mut src = row_cells.iter();
            let mut col = 0usize;

            while col < count {
                if rowspan_cols[col] > 0 {
                    // Column is covered by an active rowspan — skip the LaTeX
                    // placeholder cell (always an empty & in well-formed LaTeX).
                    src.next();
                    rowspan_cols[col] -= 1;
                    col += 1;
                } else if let Some(cell) = src.next() {
                    let (cs, rs) = table_cell_span(cell);
                    if rs > 1 {
                        // Mark every column this rowspan covers.
                        for slot in rowspan_cols
                            .iter_mut()
                            .take((col + cs).min(count))
                            .skip(col)
                        {
                            *slot = rs - 1;
                        }
                    }
                    row_output.push(cell.clone());
                    col += cs;
                } else {
                    break;
                }
            }

            if row_output.is_empty() {
                continue;
            }
            self.out.push_str("  ");
            for (i, cell) in row_output.iter().enumerate() {
                if i > 0 {
                    self.out.push_str(", ");
                }
                // Cells produced by `\multicolumn` / `\multirow` are already
                // `table.cell(...)` calls and must not be wrapped again.
                if cell.starts_with("table.cell(") {
                    self.out.push_str(cell);
                } else {
                    let _ = write!(self.out, "[{}]", escape_text_cell(cell));
                }
            }
            self.out.push_str(",\n");
            // Header rule: after the first emitted row (the common single-row
            // header). Booktabs' `\midrule` sits here in the vast majority of
            // academic tables.
            if has_rules && emitted_rows == 0 {
                self.out.push_str("  table.hline(stroke: 0.05em),\n");
            }
            emitted_rows += 1;
        }
        if has_rules {
            self.out.push_str("  table.hline(stroke: 0.08em),\n");
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

    // ─── Sectioning ───────────────────────────────────────────────────────────

    /// Of several `\label` aliases on one element, choose the one to attach
    /// (Typst keeps a single label per element): the first alias that is
    /// referenced anywhere by a `\ref`-family command, else the first label.
    /// Matching is on the sanitized key, since that's what both `<label>` and
    /// `@ref` use.
    fn pick_label_to_attach(&self, labels: &[String]) -> Option<String> {
        labels
            .iter()
            .find(|l| self.referenced_labels.contains(&sanitize_label_key(l)))
            .or_else(|| labels.first())
            .cloned()
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

    /// Emit one `UnsupportedCommand` warning per unknown package, with the
    /// warning `name` set to `usepackage:<pkg>` so callers can distinguish
    /// and rank individual packages rather than seeing a generic `\usepackage`.
    fn warn_unsupported_package(&mut self, node: Node<'_>, pkg: &str, opts: Option<&str>) {
        let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
        let message = match opts {
            Some(o) => format!("package `{pkg}` (options: `{o}`) not supported by ByeTex; dropped"),
            None => format!("package `{pkg}` not supported by ByeTex; dropped"),
        };
        self.warnings.push(Warning {
            range: range_of(node),
            category: Category::UnsupportedCommand {
                name: format!("usepackage:{pkg}"),
            },
            severity: Severity::Warning,
            message,
            snippet,
            suggested_skill: None,
        });
    }

    fn warn_silently_dropped(&mut self, node: Node<'_>) {
        let snippet = self.src[node.start_byte()..node.end_byte()].to_string();
        let name = command_name_of(&snippet);
        self.warnings.push(Warning {
            range: range_of(node),
            category: Category::DropOnly { name: name.clone() },
            severity: Severity::Warning,
            message: format!(
                "`{name}` has no Typst equivalent and was dropped; \
                 the rendered output may differ from the LaTeX original"
            ),
            snippet,
            suggested_skill: None,
        });
    }
}

// ─── Node classification helpers ──────────────────────────────────────────────

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

// ─── Node span & text utilities ───────────────────────────────────────────────

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

/// If `node` is a `curly_group` whose first named child is a declarative
/// bold/italic font switch (`{\bf ..}`, `{\em ..}`), return the Typst wrap
/// markup and the byte just after the switch command. Other family switches
/// (`\sc`, `\tt`, ...) have no clean inline equivalent and are not wrapped.
fn leading_font_switch(node: Node<'_>, src: &str) -> Option<((&'static str, &'static str), usize)> {
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
fn first_curly_group(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .find(|child| child.kind() == "curly_group");
    result
}

/// The `n`-th (0-based) `curly_group`-family child of `node`. Used to read
/// `\captionof{type}{caption}`: arg 0 is the type, arg 1 the caption.
fn nth_curly_group(node: Node<'_>, n: usize) -> Option<Node<'_>> {
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
fn flatten_text_children<'a>(body: &[Node<'a>]) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    for child in body {
        push_flat(*child, &mut out);
    }
    out
}

fn push_flat<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
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
fn math_font_decl_wrapper(node: Node<'_>, src: &str) -> Option<&'static str> {
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
fn first_curly_like(node: Node<'_>) -> Option<Node<'_>> {
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
fn needs_empty_base(out: &str) -> bool {
    let trimmed = out.trim_end_matches([' ', '\t']);
    trimmed.ends_with('$')
        || trimmed.ends_with("$ ")
        || trimmed.ends_with('(')
        || trimmed.ends_with('[')
        || trimmed.ends_with('{')
}

/// Map a LaTeX text accent + base letter to the precomposed Unicode codepoint.
///
/// `accent` is the accent character: `'\''` acute, '`' grave, `'"'` diaeresis,
/// `'^'` circumflex, `'~'` tilde. Returns a `String` so the combining-mark
/// fallback path (two code points) is representable.
pub(crate) fn apply_text_accent(accent: char, letter: char) -> String {
    let precomposed: Option<char> = match (accent, letter) {
        // Acute (')
        ('\'', 'a') => Some('á'),
        ('\'', 'A') => Some('Á'),
        ('\'', 'e') => Some('é'),
        ('\'', 'E') => Some('É'),
        ('\'', 'i') => Some('í'),
        ('\'', 'I') => Some('Í'),
        ('\'', 'o') => Some('ó'),
        ('\'', 'O') => Some('Ó'),
        ('\'', 'u') => Some('ú'),
        ('\'', 'U') => Some('Ú'),
        ('\'', 'y') => Some('ý'),
        ('\'', 'Y') => Some('Ý'),
        ('\'', 'n') => Some('ń'),
        ('\'', 'N') => Some('Ń'),
        ('\'', 'c') => Some('ć'),
        ('\'', 'C') => Some('Ć'),
        ('\'', 's') => Some('ś'),
        ('\'', 'S') => Some('Ś'),
        ('\'', 'z') => Some('ź'),
        ('\'', 'Z') => Some('Ź'),
        ('\'', 'l') => Some('ĺ'),
        ('\'', 'L') => Some('Ĺ'),
        ('\'', 'r') => Some('ŕ'),
        ('\'', 'R') => Some('Ŕ'),
        // Grave (`)
        ('`', 'a') => Some('à'),
        ('`', 'A') => Some('À'),
        ('`', 'e') => Some('è'),
        ('`', 'E') => Some('È'),
        ('`', 'i') => Some('ì'),
        ('`', 'I') => Some('Ì'),
        ('`', 'o') => Some('ò'),
        ('`', 'O') => Some('Ò'),
        ('`', 'u') => Some('ù'),
        ('`', 'U') => Some('Ù'),
        ('`', 'n') => Some('ǹ'),
        ('`', 'N') => Some('Ǹ'),
        // Diaeresis (")
        ('"', 'a') => Some('ä'),
        ('"', 'A') => Some('Ä'),
        ('"', 'e') => Some('ë'),
        ('"', 'E') => Some('Ë'),
        ('"', 'i') => Some('ï'),
        ('"', 'I') => Some('Ï'),
        ('"', 'o') => Some('ö'),
        ('"', 'O') => Some('Ö'),
        ('"', 'u') => Some('ü'),
        ('"', 'U') => Some('Ü'),
        ('"', 'y') => Some('ÿ'),
        ('"', 'Y') => Some('Ÿ'),
        // Circumflex (^)
        ('^', 'a') => Some('â'),
        ('^', 'A') => Some('Â'),
        ('^', 'e') => Some('ê'),
        ('^', 'E') => Some('Ê'),
        ('^', 'i') => Some('î'),
        ('^', 'I') => Some('Î'),
        ('^', 'o') => Some('ô'),
        ('^', 'O') => Some('Ô'),
        ('^', 'u') => Some('û'),
        ('^', 'U') => Some('Û'),
        ('^', 'c') => Some('ĉ'),
        ('^', 'C') => Some('Ĉ'),
        ('^', 'g') => Some('ĝ'),
        ('^', 'G') => Some('Ĝ'),
        ('^', 'h') => Some('ĥ'),
        ('^', 'H') => Some('Ĥ'),
        ('^', 'j') => Some('ĵ'),
        ('^', 'J') => Some('Ĵ'),
        ('^', 's') => Some('ŝ'),
        ('^', 'S') => Some('Ŝ'),
        ('^', 'w') => Some('ŵ'),
        ('^', 'W') => Some('Ŵ'),
        ('^', 'y') => Some('ŷ'),
        ('^', 'Y') => Some('Ŷ'),
        // Tilde (~)
        ('~', 'a') => Some('ã'),
        ('~', 'A') => Some('Ã'),
        ('~', 'e') => Some('ẽ'),
        ('~', 'E') => Some('Ẽ'),
        ('~', 'i') => Some('ĩ'),
        ('~', 'I') => Some('Ĩ'),
        ('~', 'n') => Some('ñ'),
        ('~', 'N') => Some('Ñ'),
        ('~', 'o') => Some('õ'),
        ('~', 'O') => Some('Õ'),
        ('~', 'u') => Some('ũ'),
        ('~', 'U') => Some('Ũ'),
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

// ─── Label, citation & graphics extraction ────────────────────────────────────

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
fn label_ref_splits_on_comma(node: Node<'_>) -> bool {
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
fn extract_label_ref_keys_and_end(node: Node<'_>, src: &str) -> Option<(Vec<String>, usize)> {
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
                    let raw = &src[grandchild.start_byte()..grandchild.end_byte()];
                    // A multi-line `\bibliography{\n a,\n b\n % c,\n}` (corpus
                    // 2605.31443) yields path tokens carrying newlines, spaces
                    // and even commented-out paths, and tree-sitter may fold the
                    // whole comma list into one token. Split on commas, drop any
                    // trailing `%`-comment, and trim — so the live paths resolve
                    // against base_dir and the commented ones are ignored.
                    for piece in raw.split(',') {
                        let p = piece.split('%').next().unwrap_or("").trim();
                        if !p.is_empty() {
                            out.push(p.to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

/// Extract the path argument from a `graphics_include` (`\includegraphics{X}`).
/// Parse the inner argument of `\graphicspath{{dir1/}{dir2/}}` — i.e. the text
/// between the OUTER braces — into a list of search directories. Each dir is a
/// `{...}`-wrapped group; brace nesting is honored. Trailing slashes are kept
/// as written (joined with the image name later). A malformed/empty arg yields
/// no dirs. Example: `{figures/main/}{figures/tasks/}` → ["figures/main/",
/// "figures/tasks/"].
fn parse_graphicspath_dirs(inner: &str) -> Vec<String> {
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut depth = 1;
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
                j += 1;
            }
            let dir = inner[start..j].trim();
            if !dir.is_empty() {
                out.push(dir.to_string());
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

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

// ─── Tabular, math rows & math sanitization ───────────────────────────────────

/// Parse a LaTeX tabular column spec like `lcr` or `|l|c|r|` into a count and
/// a vector of Typst alignment names (`"left"`, `"center"`, `"right"`).
fn parse_column_spec(spec: &str) -> (usize, Vec<String>) {
    let mut aligns = Vec::new();
    let bytes = spec.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] as char {
            'l' | 'L' => {
                aligns.push("left".to_string());
                i += 1;
            }
            'c' | 'C' => {
                aligns.push("center".to_string());
                i += 1;
            }
            'r' | 'R' => {
                aligns.push("right".to_string());
                i += 1;
            }
            // Paragraph/width columns (p, m, b) take {width} argument — skip
            // the argument but count the column as left-aligned.
            'p' | 'm' | 'b' | 'w' | 'W' => {
                aligns.push("left".to_string());
                i += 1;
                if bytes.get(i) == Some(&b'{') {
                    i = skip_balanced_braces(spec, i);
                }
            }
            // tabularx X column — count as left-aligned.
            'X' => {
                aligns.push("left".to_string());
                i += 1;
            }
            // array-package repeat: `*{N}{cols}` expands `cols` N times.
            // Without this the inner spec was counted once (or mis-counted),
            // undercounting columns — so `\multicolumn` header rows summed to
            // more than `columns:` and Typst aborted with "colspan exceeds
            // available columns" (arXiv:2605.22724).
            '*' => {
                i += 1;
                let count = if bytes.get(i) == Some(&b'{') {
                    let close = skip_balanced_braces(spec, i);
                    let n = spec[i + 1..close.saturating_sub(1)]
                        .trim()
                        .parse()
                        .unwrap_or(0);
                    i = close;
                    n
                } else {
                    0
                };
                if bytes.get(i) == Some(&b'{') {
                    let close = skip_balanced_braces(spec, i);
                    let inner = &spec[i + 1..close.saturating_sub(1)];
                    i = close;
                    let (_, inner_aligns) = parse_column_spec(inner);
                    for _ in 0..count {
                        aligns.extend(inner_aligns.iter().cloned());
                    }
                }
            }
            // @{...} and !{...}: inter-column material, not data columns.
            // >{...} and <{...}: column format decorators (array package).
            '@' | '!' | '>' | '<' => {
                i += 1;
                if bytes.get(i) == Some(&b'{') {
                    i = skip_balanced_braces(spec, i);
                }
            }
            // Vertical rules and whitespace — ignore.
            _ => {
                i += 1;
            }
        }
    }
    (aligns.len(), aligns)
}

/// Skip a `{...}` balanced-brace group starting at `start` (where `src[start] == '{'`).
/// Returns the index one past the closing `}`.
fn skip_balanced_braces(src: &str, start: usize) -> usize {
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

/// Escape unbalanced paired delimiters (`[`, `]`, `(`, `)`) in a Typst math
/// body. LaTeX half-open intervals such as `(0, s_*]` or `[a, b)` mix
/// delimiter kinds: Typst pairs `[..]` and `(..)` independently, so when one
/// kind doesn't balance, both the orphan close (`]`) AND the partner of the
/// other kind that no longer has a matching close (`(`) need escaping —
/// otherwise Typst complains about an unclosed delimiter on the *other* one.
/// Balanced pairs are left untouched. Pre-existing backslash escapes are
/// skipped so we never double-escape.
/// Collapse runs of whitespace (spaces, tabs, newlines) in `s` to a
/// single space and trim leading/trailing whitespace. Used to keep
/// content like heading titles on a single line so Typst's
/// reference-target detection works.
fn collapse_inline_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = true; // skip leading whitespace
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(c);
            last_was_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
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
fn split_math_rows(body: &str) -> Vec<&str> {
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

// ─── Math symbol table ────────────────────────────────────────────────────────

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
        // `\vcentcolon` (mathtools): a vertically-centered colon used
        // in combinations like `\vcentcolon=` to form `:=`. Map to the
        // plain colon character; users who wanted the full `:=` typed
        // `\coloneqq` directly.
        "\\vcentcolon" => "colon",
        // `\lbrace`/`\rbrace` — alternate names for `\{`/`\}`,
        // frequent in arXiv math. Emit the escaped brace glyph (Typst
        // would parse a bare `{` as a group-start syntax). The plain
        // `\{`/`\}` aliases are handled elsewhere with the same shape.
        "\\lbrace" => "\\{",
        "\\rbrace" => "\\}",
        // `\llbracket` / `\rrbracket` (stmaryrd, mathbb-related):
        // Iverson-style double square brackets. Typst has dedicated
        // glyphs.
        "\\llbracket" => "bracket.l.double",
        "\\rrbracket" => "bracket.r.double",
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
        // `\not` is handled by an explicit arm in emit_math_command that
        // emits a DropOnly warning; it must not appear here or push_math_symbol
        // would silently swallow it via the empty-string early-return.
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
        "\\coth" => "coth",
        // Standard math operators — Typst renders these upright by name.
        "\\det" => "det",
        "\\dim" => "dim",
        "\\ker" => "ker",
        "\\arg" => "arg",
        "\\deg" => "deg",
        "\\hom" => "hom",
        "\\Pr" => "Pr",
        "\\lg" => "lg",
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
        "\\limits" | "\\nolimits" => "",
        // `\displaystyle` / `\textstyle` / `\scriptstyle` /
        // `\scriptscriptstyle` are handled by explicit warning arms in
        // emit_math_command; they must not appear here.
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

// ─── Braceless-arg & macro machinery ──────────────────────────────────────────

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
/// Math-context wrapper around [`consume_braceless_arg`] that refuses
/// to consume a math-terminating delimiter (`$`, `\)`, `\]`, or `}`
/// at the outer level). Used by structural math commands (`\frac`,
/// `\sqrt`, `\binom`) when filling missing brace-less args: without
/// this guard, `$\frac{a}$` would greedily eat the closing `$` as the
/// second argument and break the surrounding math container.
pub(crate) fn try_consume_math_arg(src: &str, start: usize) -> Option<(BracelessArg, usize)> {
    let bytes = src.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    if bytes[i] == b'$' || bytes[i] == b'}' {
        // Math closer (`$`, `$$`) or surrounding-group closer. Bail.
        return None;
    }
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
        match bytes[i + 1] {
            b')' | b']' => return None, // `\)` / `\]` math closers
            _ => {}
        }
        // `\end{...}` — math environment closer.
        if src[i..].starts_with("\\end{") {
            return None;
        }
    }
    consume_braceless_arg(src, start)
}

/// Starting at `start`, skip leading whitespace then consume zero or more
/// consecutive balanced `{...}` argument groups, returning the byte index past
/// the last one (or `start` if none follow). Used to gather the brace args of a
/// structural command that was consumed brace-less, e.g. the `{a}{b}` of
/// `\sqrt\frac{a}{b}`.
fn consume_trailing_brace_groups(src: &str, start: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = start;
    loop {
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'{' {
            return i;
        }
        let mut depth = 1i32;
        let mut k = j + 1;
        while k < bytes.len() {
            match bytes[k] {
                b'\\' if k + 1 < bytes.len() => {
                    k += 2;
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
            k += 1;
        }
        if k >= bytes.len() {
            // Unbalanced — stop at what we had.
            return i;
        }
        i = k + 1;
    }
}

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
        return Some((BracelessArg::Group(src[inner_start..j].to_string()), j + 1));
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

/// Extract `(name, nargs)` from an `environment_definition` node
/// (`\newenvironment{name}[nargs][default]{begindef}{enddef}` /
/// `\renewenvironment`). The grammar exposes the name as a `name:`-field
/// `curly_group_text` (fall back to the first `curly_group_text`/
/// `curly_group_word` child) and the argument count as an `argc:`-field
/// `brack_group_argc`. `nargs` is 0 when the env takes no arguments.
fn extract_environment_def(node: Node<'_>, src: &str) -> Option<(String, usize)> {
    let name_node = match node.child_by_field_name("name") {
        Some(n) => n,
        None => {
            let mut cursor = node.walk();
            let found = node
                .children(&mut cursor)
                .find(|c| matches!(c.kind(), "curly_group_text" | "curly_group_word"));
            found?
        }
    };
    let name = src[name_node.start_byte()..name_node.end_byte()]
        .trim_matches(|c: char| c == '{' || c == '}')
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }
    let nargs = node
        .child_by_field_name("argc")
        .and_then(|argc| {
            src[argc.start_byte()..argc.end_byte()]
                .trim_matches(|c: char| c == '[' || c == ']')
                .trim()
                .parse::<usize>()
                .ok()
        })
        .unwrap_or(0);
    Some((name, nargs))
}

/// Extract `(env_name, display_name)` from a `theorem_definition` node.
/// Handles all four variant patterns:
///
/// - `\newtheorem{name}{Display}`
/// - `\newtheorem{name}[counter]{Display}`
/// - `\newtheorem{name}{Display}[parent]`
/// - `\newtheorem*{name}{Display}`
///
/// Falls back to capitalizing `name` when no display curly group is found
/// (e.g. `\declaretheorem[name=Foo]{foo}` whose title is in options).
fn extract_theorem_def(node: Node<'_>, src: &str) -> Option<(String, String)> {
    let mut cursor = node.walk();
    let mut name_bytes: Option<(usize, usize)> = None;
    let mut title_bytes: Option<(usize, usize)> = None;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "curly_group_text_list" | "curly_group_text" if name_bytes.is_none() => {
                name_bytes = Some((child.start_byte(), child.end_byte()));
            }
            "curly_group" if name_bytes.is_some() && title_bytes.is_none() => {
                title_bytes = Some((child.start_byte(), child.end_byte()));
            }
            _ => {}
        }
    }
    let (ns, ne) = name_bytes?;
    let name = src[ns..ne]
        .trim_matches(|c: char| c == '{' || c == '}')
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }
    let display = title_bytes
        .map(|(ts, te)| {
            src[ts..te]
                .trim_matches(|c: char| c == '{' || c == '}')
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let mut s = name.clone();
            if let Some(first) = s.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            s
        });
    Some((name, display))
}

/// Find the byte index one past the `}` that closes the `{` at `start`.
/// Returns `None` if `bytes[start]` is not `{` or braces are unbalanced.
/// Skips `\{` and `\}` so escaped braces don't affect the depth count.
fn brace_balanced_end(bytes: &[u8], start: usize) -> Option<usize> {
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

/// Extract a `\newcommand{\name}[N]{body}` definition from a `new_command_definition` node.
///
/// Returns `None` when the node cannot be parsed or has an optional-default argument.
///
/// Accepts both name forms tree-sitter-latex produces:
///
/// - `\newcommand{\name}{body}` — canonical curly-wrapped name (`curly_group_command_name`).
/// - `\newcommand\name{body}` — brace-less name form, common in arXiv preamble files.
fn extract_newcommand(node: Node<'_>, src: &str) -> Option<(String, MacroDef)> {
    let mut cursor = node.walk();
    let mut name: Option<String> = None;
    let mut params: usize = 0;
    let mut body_group: Option<Node<'_>> = None;
    let mut optional_default: Option<String> = None;
    // Track whether we've seen the declaration child yet. The
    // brace-less form has a `command_name` as the declaration field,
    // but the body of the macro is also a curly group, and the AST
    // may include the macro `\newcommand` token itself as a separate
    // `command_name` sibling. We only treat the FIRST `command_name`
    // (the one before any `curly_group`) as the declaration name.
    let mut saw_declaration = false;
    for child in node.children(&mut cursor) {
        match child.kind() {
            "curly_group_command_name" => {
                let mut sub = child.walk();
                for gc in child.children(&mut sub) {
                    if gc.kind() == "command_name" {
                        name = Some(src[gc.start_byte()..gc.end_byte()].to_string());
                    }
                }
                saw_declaration = true;
            }
            "command_name" if !saw_declaration && name.is_none() => {
                // Brace-less name form: `\newcommand\name{body}`.
                // tree-sitter-latex parses the name as a direct
                // `command_name` child of `new_command_definition`.
                name = Some(src[child.start_byte()..child.end_byte()].to_string());
                saw_declaration = true;
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
            "brack_group" if optional_default.is_none() && params > 0 => {
                // LaTeX2e `\newcommand\foo[N][default]{body}` form:
                // position 1 is optional with this default. Capture
                // the raw bytes between `[` and `]`, including an
                // empty default (`[]` — common, e.g. `\traceD[1][]`
                // means "1 arg, defaults to empty string").
                let start = child.start_byte() + 1;
                let end = child.end_byte().saturating_sub(1);
                optional_default = Some(src.get(start..end).unwrap_or("").to_string());
            }
            "curly_group" if body_group.is_none() => {
                body_group = Some(child);
            }
            _ => {}
        }
    }
    let name = name?;
    let body_node = body_group?;
    // Use brace-counting to find the true end of the body group.
    // tree-sitter-latex sometimes truncates curly_group end_byte when the
    // body contains a nested \newcommand (wrapper-macro pattern), so we
    // cannot trust end_byte() alone. Brace-counting is always correct.
    let body_start = body_node.start_byte();
    let body_end = brace_balanced_end(src.as_bytes(), body_start).unwrap_or(body_node.end_byte());
    let body = src
        .get(body_start + 1..body_end - 1)
        .unwrap_or("")
        .to_string();
    let mut optional_defaults = HashMap::new();
    if let Some(default) = optional_default {
        // LaTeX2e: position 1 is the optional position when a default
        // is given. Positions 2..=N remain mandatory.
        optional_defaults.insert(1, default);
    }
    Some((
        name,
        MacroDef {
            params,
            body,
            optional_defaults,
        },
    ))
}

/// Extract a `\newcommandx\name[N][K=default, ...]{body}` definition.
/// `\newcommandx` is from the `xparse`/`xargspec` LaTeX packages and
/// extends `\newcommand` with positionally-keyed optional defaults:
/// `[K=default]` makes position K optional with the given default.
/// Multiple positions can be specified, comma-separated.
///
/// tree-sitter-latex parses `\newcommandx` as a *bare* generic_command
/// containing just the `\newcommandx` command_name token — the new
/// macro name, the brackets, and the body all end up as *sibling*
/// nodes of the generic_command, not children. So we can't walk the
/// AST: we scan the raw source bytes forward from `node.end_byte()`
/// to find the pieces.
///
/// Returns `None` if the source doesn't parse cleanly as a
/// `\newcommandx` definition.
fn extract_newcommandx(node: Node<'_>, src: &str) -> Option<(String, MacroDef)> {
    extract_newcommandx_and_end(node, src).map(|(def, _end)| def)
}

/// Variant that also returns the source byte position immediately
/// after the closing `}` of the body. The emit-time dispatcher uses
/// this to bump `skip_until` so the sibling AST nodes carrying the
/// definition's bracket/body fragments don't leak into the output.
fn extract_newcommandx_and_end(node: Node<'_>, src: &str) -> Option<((String, MacroDef), usize)> {
    let bytes = src.as_bytes();
    let mut i = node.end_byte();

    // Skip whitespace, then expect `\name`.
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'\\' {
        return None;
    }
    let name_start = i;
    i += 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'@') {
        i += 1;
    }
    let name = src.get(name_start..i)?.to_string();
    if name.len() < 2 {
        return None;
    }

    // Helper: skip whitespace and read a `[...]` bracket group with
    // brace-aware nesting. Returns `(inner, end_after_closing_bracket)`
    // when found, `None` otherwise.
    fn read_brack(bytes: &[u8], src: &str, mut i: usize) -> Option<(String, usize)> {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'[' {
            return None;
        }
        let inner_start = i + 1;
        let mut j = inner_start;
        let mut depth = 0i32;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' if j + 1 < bytes.len() => {
                    j += 2;
                    continue;
                }
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b']' if depth == 0 => break,
                _ => {}
            }
            j += 1;
        }
        if j >= bytes.len() {
            return None;
        }
        Some((src[inner_start..j].to_string(), j + 1))
    }

    // Helper: skip whitespace and read a `{...}` curly group. Returns
    // `(inner, end_after_closing_brace)`.
    fn read_curly(bytes: &[u8], src: &str, mut i: usize) -> Option<(String, usize)> {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'{' {
            return None;
        }
        let inner_start = i + 1;
        let mut j = inner_start;
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
        if j >= bytes.len() {
            return None;
        }
        Some((src[inner_start..j].to_string(), j + 1))
    }

    // Optional `[N]` for arity.
    let mut params = 0usize;
    let mut defaults_src: Option<String> = None;
    if let Some((inner, after)) = read_brack(bytes, src, i) {
        if let Ok(n) = inner.trim().parse::<usize>() {
            params = n;
            i = after;
            // A second optional `[K=def, ...]` for default values.
            if let Some((defs, after2)) = read_brack(bytes, src, i) {
                defaults_src = Some(defs);
                i = after2;
            }
        }
    }

    // Mandatory `{body}`.
    let (body, end_after_body) = read_curly(bytes, src, i)?;

    // Parse the K=default entries (brace-aware split on top-level
    // commas, then split each entry on the first `=`).
    let mut optional_defaults: HashMap<usize, String> = HashMap::new();
    if let Some(defs) = defaults_src {
        for entry in split_xargspec_defaults(&defs) {
            if let Some((k, v)) = entry.split_once('=') {
                if let Ok(pos) = k.trim().parse::<usize>() {
                    optional_defaults.insert(pos, v.trim().to_string());
                }
            }
        }
    }

    Some((
        (
            name,
            MacroDef {
                params,
                body,
                optional_defaults,
            },
        ),
        end_after_body,
    ))
}

/// Brace-aware split of an xargspec defaults string like
/// `1=, 3={a, b}, 4=foo` into entries `["1=", "3={a, b}", "4=foo"]`.
/// Top-level commas separate entries; commas inside `{...}` are kept.
fn split_xargspec_defaults(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b',' if depth == 0 => {
                out.push(s[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start < bytes.len() {
        let tail = s[start..].trim();
        if !tail.is_empty() {
            out.push(tail.to_string());
        }
    }
    out
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
            "curly_group" if display.is_none() => {
                // The display text group (e.g. "{sinc}")
                let body_src = &src[child.start_byte()..child.end_byte()];
                display = Some(if body_src.starts_with('{') && body_src.ends_with('}') {
                    body_src[1..body_src.len() - 1].to_string()
                } else {
                    body_src.to_string()
                });
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
    Some((
        name,
        MacroDef {
            params: 0,
            body,
            optional_defaults: HashMap::new(),
        },
    ))
}

// ─── Command dispatch helpers ──────────────────────────────────────────────────

/// Parse the `colspan` and `rowspan` from a Typst `table.cell(...)` string.
/// Returns `(colspan, rowspan)` — both default to 1 for plain cells.
fn table_cell_span(cell: &str) -> (usize, usize) {
    if !cell.starts_with("table.cell(") {
        return (1, 1);
    }
    let after = &cell["table.cell(".len()..];
    let close = match after.find(')') {
        Some(i) => i,
        None => return (1, 1),
    };
    let mut colspan = 1usize;
    let mut rowspan = 1usize;
    for kv in after[..close].split(',') {
        let kv = kv.trim();
        if let Some(v) = kv.strip_prefix("colspan:") {
            colspan = v.trim().parse().unwrap_or(1);
        } else if let Some(v) = kv.strip_prefix("rowspan:") {
            rowspan = v.trim().parse().unwrap_or(1);
        }
    }
    (colspan, rowspan)
}

/// Decide whether a Typst math-mode subscript/superscript argument
/// needs an explicit `(...)` wrapper. A single token (one letter or
/// digit, optionally with one trailing `prime`-style suffix) parses
/// correctly as `_x` / `^x`; anything more compound (function call,
/// space-separated tokens, multi-char identifier) needs `_(...)`
/// because `_cal(T)` reads as `_c · al(T)`.
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
                    macros.insert(
                        name,
                        MacroDef {
                            params,
                            body,
                            optional_defaults: HashMap::new(),
                        },
                    );
                    return Some(j + 1);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

// ─── Document class, path & package resolution ────────────────────────────────

/// Maps the well-known math wrap commands to their Typst `(left, right)`
/// delimiter pair. Used by the bare `command_name` branch of
/// `emit_node` to recover the brace-less form (e.g. `_\mathcal{T}` —
/// tree-sitter parses the `{T}` as a sibling of the enclosing
/// subscript, so the command_name itself reaches us without a child).
pub(crate) fn wrap_for_command_name(name: &str) -> Option<(&'static str, &'static str)> {
    Some(match name {
        // `\mathds` (dsfont) and `\mathbbold` (bbold) — visually
        // identical to `\mathbb` for the common single-letter case.
        "\\mathbb" | "\\mathbbm" | "\\Bbb" | "\\mathds" | "\\mathbbold" => ("bb(", ")"),
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
        "\\overbrace" => ("overbrace(", ")"),
        "\\underbrace" => ("underbrace(", ")"),
        "\\cancel" => ("cancel(", ")"),
        "\\bcancel" => ("cancel(inverted: true, ", ")"),
        "\\xcancel" => ("cancel(cross: true, ", ")"),
        "\\sout" => ("strike(", ")"),
        "\\emph" => ("italic(", ")"),
        "\\mathop" => ("op(", ")"),
        _ => return None,
    })
}

/// Byte offset just past the first `\makeatother` *control word* at or after
/// `from`, or `None` if there is no closing `\makeatother`. Used to skip a
/// `\makeatletter` region wholesale.
///
/// Scans like [`find_conditional_bounds`]: `%` line comments are skipped (so a
/// `\makeatother` mentioned in a comment doesn't end the region early), and the
/// match is on the whole control word (so `\makeatotherwise` is not mistaken
/// for the closer).
fn find_makeatother_end(src: &str, from: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'\\' => {
                let cs_start = i;
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == i + 1 {
                    // Control symbol (`\\`, `\{`, `\%`, ...): consume both bytes.
                    i += 2;
                    continue;
                }
                if &src[cs_start..j] == "\\makeatother" {
                    return Some(j);
                }
                i = j;
            }
            _ => i += 1,
        }
    }
    None
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

// ─── Asset & bibliography filesystem probing ──────────────────────────────────

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
/// Walk the immediate `base_dir` (non-recursive) for `.bib` and
/// `.bbl` files, parse their entry / `\bibitem` keys, and insert
/// the sanitized forms into `out`. Errors (unreadable file,
/// unparseable content) are silently skipped — the worst case is a
/// citation that should have resolved gets dropped, which is the
/// same end state as before this validation existed.
fn harvest_bib_keys_from_dir(base: &Path, out: &mut std::collections::HashSet<String>) {
    let entries = match std::fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some(e) if e.eq_ignore_ascii_case("bib") => {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for key in extract_bib_entry_keys(&content) {
                        out.insert(sanitize_label_key(&key));
                    }
                }
            }
            Some(e) if e.eq_ignore_ascii_case("bbl") => {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for key in extract_bbl_bibitem_keys(&content) {
                        out.insert(sanitize_label_key(&key));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Scan a `.bib` file for entry keys (the identifier right after
/// `@type{`). Permissive — picks up any `@<word>{<key>,` pattern,
/// ignores @string/@preamble/@comment, doesn't validate the rest of
/// the entry.
fn extract_bib_entry_keys(content: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Find next `@`.
        let at = match content[i..].find('@') {
            Some(p) => i + p,
            None => break,
        };
        // Read type identifier.
        let type_start = at + 1;
        let type_end = bytes[type_start..]
            .iter()
            .position(|&b| !b.is_ascii_alphabetic())
            .map(|p| type_start + p)
            .unwrap_or(bytes.len());
        let entry_type = content[type_start..type_end].to_ascii_lowercase();
        i = at + 1;
        if matches!(entry_type.as_str(), "string" | "preamble" | "comment") {
            continue;
        }
        // Skip whitespace, expect `{`.
        let mut j = type_end;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'{' {
            continue;
        }
        j += 1;
        // Skip whitespace inside.
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        // Read key up to the first `,`.
        let key_start = j;
        while j < bytes.len() && bytes[j] != b',' && bytes[j] != b'}' {
            j += 1;
        }
        let key = content[key_start..j].trim();
        if !key.is_empty() {
            keys.push(key.to_string());
        }
        i = j;
    }
    keys
}

/// Scan a `.bbl` file for `\bibitem{key}` and `\bibitem[label]{key}`
/// occurrences and return the keys.
fn extract_bbl_bibitem_keys(content: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    let needle = b"\\bibitem";
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let mut j = i + needle.len();
            // Skip optional `[label]`.
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'[' {
                let mut depth = 0i32;
                let mut k = j + 1;
                while k < bytes.len() {
                    match bytes[k] {
                        b'\\' if k + 1 < bytes.len() => {
                            k += 2;
                            continue;
                        }
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b']' if depth == 0 => break,
                        _ => {}
                    }
                    k += 1;
                }
                if k < bytes.len() {
                    j = k + 1;
                }
            }
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'{' {
                let key_start = j + 1;
                let mut k = key_start;
                let mut depth = 1i32;
                while k < bytes.len() {
                    match bytes[k] {
                        b'\\' if k + 1 < bytes.len() => {
                            k += 2;
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
                    k += 1;
                }
                if k < bytes.len() {
                    let key = content[key_start..k].trim();
                    if !key.is_empty() {
                        keys.push(key.to_string());
                    }
                    i = k + 1;
                    continue;
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    keys
}

/// Read the contents of any `.bbl` file in `base` if exactly one
/// exists. Returns `None` when no `.bbl` is present or when multiple
/// candidates are ambiguous (better to drop the call than guess wrong).
///
/// Used by `emit_bibliography` as a fallback when the LaTeX
/// `\bibliography{Foo}` references a `.bib` file that isn't bundled
/// but a pre-rendered `.bbl` is (common in arXiv preprints — the
/// author only shipped what BibTeX wrote, not the source `.bib`).
fn probe_any_bbl(base: &Path) -> Option<String> {
    let entries = std::fs::read_dir(base).ok()?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|d| d.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("bbl"))
        })
        .collect();
    found.sort();
    if found.is_empty() {
        return None;
    }
    // Prefer the first hit (sorted) — when an arXiv source bundles
    // multiple `.bbl` files they're usually variants of the same
    // bibliography, so picking one consistently is acceptable.
    std::fs::read_to_string(&found[0]).ok()
}

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

/// Extract all package names from `\usepackage[opts]{name}` or
/// `\usepackage{pkg1,pkg2,...}`. Returns every `path` child found in the
/// curly group, so a comma-separated list yields multiple entries.
fn extract_package_names(node: Node<'_>, src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "curly_group_path" | "curly_group_path_list") {
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "path" {
                    let pkg = src[grandchild.start_byte()..grandchild.end_byte()].trim();
                    if !pkg.is_empty() {
                        out.push(pkg.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Extract the bracket-group option text from `\usepackage[opts]{...}`,
/// returning the inner content without the `[` / `]` delimiters.
/// tree-sitter-latex uses `brack_group_key_value` for this optional argument.
fn extract_package_options(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "brack_group_key_value" {
            let text = &src[child.start_byte()..child.end_byte()];
            let inner = text.trim_start_matches('[').trim_end_matches(']').trim();
            if !inner.is_empty() {
                return Some(inner.to_string());
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
        // Theorem / proof tools
        | "thmtools" | "thm-restate" | "ntheorem"
        // List styling
        | "enumerate" | "paralist" | "mdwlist"
        // Paragraph / spacing
        | "parskip" | "parskip2"
        // Hyperlinks / DOI
        | "doi"
        // Math symbols
        | "gensymb" | "esint" | "mathdots" | "yhmath" | "extarrows" | "extpfeil"
        | "dutchcal" | "cancel"
        // Table extensions
        | "tabulary" | "tabularray" | "diagbox" | "cellspace"
        // Font / encoding
        | "cmap" | "fontawesome5" | "pdfrender"
        // Conditional
        | "ifxetex" | "ifluatex"
        // Misc layout/utility
        | "standalone" | "titletoc" | "etoc" | "todonotes" | "overpic"
        | "numprint" | "totcount"
        // Conference/journal style files commonly preloaded by templates.
        | "neurips_2022" | "neurips_2023" | "neurips_2024" | "neurips_2025"
        | "neurips_2026" | "iclr2024_conference" | "iclr2025_conference"
        | "iclr_conference" | "icml2024" | "icml2025" | "icml2026"
        | "acmart" | "IEEEtran" | "spconf"
        // Indexing / nomenclature / cross-reference plumbing.
        // The package load itself is inert; body calls (\index, \nomenclature)
        // warn separately on their own merits.
        | "imakeidx" | "nomencl" | "tocbibind"
        // Hyphenation / line-break control; stylistic only.
        | "hyphenat"
        // Layout / debug / sample-content helpers.
        | "emptypage" | "subfiles" | "import" | "layout" | "mwe"
        // pict2e extends kernel `picture` primitives; no new body commands.
        | "pict2e"
        // Logo macros (\TeX, \LaTeX family) — handled at command level.
        | "hologo"
        // Lua-based rendering backends; pure rendering.
        | "luacolor" | "lua-ul"
        // Margin notes — package load is silent; \marginnote calls warn.
        | "marginnote"
        // KOMA-Script page headers; Typst `set page(header:)` covers this.
        | "scrlayer-scrpage"
        // Language / script packages: the load is silently dropped because
        // visible effects surface through body commands that warn separately
        // (\foreignlanguage, \gls, etc.).  Rendering of non-Latin scripts will
        // diverge unless the user selects an appropriate Typst font.
        | "polyglossia" | "xeCJK" | "luatexja" | "arabtex"
        | "glossaries" | "markdown"
        // Font-family selection (cosmetic; same pattern as times/helvet above).
        | "luaotfload" | "noto" | "bookman" | "tgbonum"
        // Greek-letter text-mode access; symbol table already covers math mode.
        | "alphabeta"
        // OpenType math fonts; load is inert (\setmathfont etc. warn separately).
        | "unicode-math"
        // Page-count label (\pageref{LastPage} handling is a separate question).
        | "lastpage"
        // Body-command packages; load is inert, body commands warn on their own.
        | "emoji" | "epigraph" | "shellesc"
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

// ─── Math word recognition & post-processing ──────────────────────────────────

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

/// LaTeX math operators that Typst does NOT provide as built-in upright
/// identifiers (sin/cos/… exist; these don't), so a bare `cov` parses as an
/// unknown variable. Emitted via `op("…")` (upright, like `\operatorname`).
/// They are also "function names" for the no-split rule.
fn is_operatorname_only_function(s: &str) -> bool {
    matches!(s, "cov" | "var" | "argmax" | "argmin")
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
/// Return the display string for an affiliation record, or `None` if the
/// record carries no renderable text. Prefers structured fields
/// (department → institution → city → country) and falls back to the raw
/// blob when no structured fields are populated.
fn aff_display_text(aff: &Option<crate::document::Affiliation>) -> Option<String> {
    let aff = aff.as_ref()?;
    let mut parts: Vec<&str> = Vec::new();
    if let Some(dept) = &aff.department {
        let s = dept.as_content();
        if !s.is_empty() {
            parts.push(s);
        }
    }
    if let Some(inst) = &aff.institution {
        let s = inst.as_content();
        if !s.is_empty() {
            parts.push(s);
        }
    }
    if let Some(city) = &aff.city {
        if !city.is_empty() {
            parts.push(city.as_str());
        }
    }
    if let Some(country) = &aff.country {
        if !country.is_empty() {
            parts.push(country.as_str());
        }
    }
    if !parts.is_empty() {
        return Some(parts.join(", "));
    }
    // Fall back to the raw unstructured blob (e.g. from \IEEEauthorblockA or
    // a plain \affiliation{...} without per-field markers).
    aff.raw
        .as_ref()
        .map(|r| r.as_content().to_string())
        .filter(|s| !s.is_empty())
}

/// be preserved, but in our text-mode output the only `\`` we emit comes from
/// `\texttt{...}` — those wrappers are short and don't typically contain
/// `--`/`---`/``''. We accept the small risk in v0.2 and revisit if a real
/// template triggers it.
fn post_process_typography(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut prev: Option<char> = None;
    // Track whether we're inside a `$...$` math span. Inside math, the
    // typographic transformations (especially `''` → `"`) must NOT
    // apply: `''` in math is a double-prime derivative (`f''(x)`),
    // not a closing double-quote. Treating it as `"` opens a Typst
    // string literal that runs through the rest of the expression
    // and corrupts parsing (Bug #29 / 2605.22820: `B_i''(u_i)`
    // started a string that consumed everything until the next
    // unescaped `"` deep into the line, breaking the math).
    let mut in_math = false;
    while let Some(c) = chars.next() {
        // Toggle math state on unescaped `$`. We never receive `\$` here
        // because the upstream emitter converts those to text-mode
        // escapes (`\$` is a `text_command` that becomes a literal `$`
        // rendered outside math context, so it's already handled).
        if c == '$' && prev != Some('\\') {
            in_math = !in_math;
            out.push('$');
            prev = Some('$');
            // LaTeX math `/` is a plain character; Typst math `/` is a binary
            // operator that requires a left operand. When the very first char
            // of a math span is `/`, wrap it in a string literal so Typst
            // renders it as a glyph rather than an operator: `$/` → `$"/"`.
            if in_math && chars.peek() == Some(&'/') {
                chars.next();
                out.push_str("\"/\"");
                prev = Some('"');
            }
            continue;
        }
        if in_math {
            // Inside math, pass everything through unchanged. Typst
            // math has its own typography rules (prime `'`, en-dash
            // via `dash`, etc.) — don't mangle.
            out.push(c);
            prev = Some(c);
            continue;
        }
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
            // `@` is Typst's reference operator. byetex emits a REAL `@ref`
            // only after whitespace (` @key`, start-of-content) OR after `(`
            // (`\eqref` wraps the ref in parens → `(@eqn:a)`). An email `@` is
            // instead glued to the end of a local part: a word char (`cli@uta`)
            // or a `}` from an escaped brace group (`\{a, b\}@mavs.uta.edu`,
            // corpus 2605.31564). Escape ONLY those gluing chars to `\@` so the
            // address isn't parsed as `@label` (→ dangling `<mavs.uta.edu>`),
            // while leaving `(@eqn:a)` and ` @key` as live references.
            '@' if prev.is_some_and(|p| p.is_ascii_alphanumeric() || p == '}') => {
                out.push_str("\\@");
                prev = Some('@');
            }
            // `<key>` is Typst's label syntax. Only emit a raw `<` when
            // the span up to `>` consists entirely of valid Typst label
            // chars (`[a-zA-Z0-9_:.-]`). Otherwise escape as `\<` to
            // prevent Typst from misidentifying it as a label (e.g.
            // `<email@host>`, `<http://url>`).
            '<' => {
                let mut lookahead = chars.clone();
                let mut key_len: usize = 0;
                let mut found_close = false;
                'scan: loop {
                    match lookahead.next() {
                        Some('>') => {
                            found_close = true;
                            break 'scan;
                        }
                        // Must match `sanitize_label_key` exactly (Unicode
                        // alphanumerics included) so a label emitted with e.g.
                        // `ö` is recognised here as a label, not escaped.
                        Some(c) if is_typst_label_char(c) => {
                            key_len += 1;
                        }
                        _ => break 'scan,
                    }
                }
                if found_close && key_len > 0 {
                    out.push('<');
                } else {
                    out.push_str("\\<");
                }
                prev = Some('<');
            }
            other => {
                out.push(other);
                prev = Some(other);
            }
        }
    }
    out
}

/// Insert a space between `#raw("…")` and an immediately following `(` so
/// that Typst does not greedily parse the `(…)` as function-call arguments
/// on the content value returned by `raw(…)`.
///
/// In Typst markup mode `#raw("X")(Y)` is parsed as "call the result of
/// raw("X") with Y as argument", which fails when Y contains characters that
/// are not valid in code (e.g. `↓`).  A plain space breaks the chain: Typst
/// only applies function-call syntax to `#expr(` with no intervening space.
/// Break `*/` and `/*` token pairs that occur INSIDE math (`$…$`). They are
/// Typst's block-comment delimiters; adjacent in math (e.g. a superscript star
/// before a division, `h^*/x` — corpus 2605.31549) they make the lexer abort
/// with "unexpected end of block comment". Neither pair is meaningful adjacent
/// in math, so a separating space is safe. `\$` does not toggle math.
fn break_math_comment_tokens(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 8);
    let mut in_math = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            out.push(c);
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '$' {
            in_math = !in_math;
            out.push(c);
            i += 1;
            continue;
        }
        if in_math && i + 1 < chars.len() {
            let n = chars[i + 1];
            if (c == '*' && n == '/') || (c == '/' && n == '*') {
                out.push(c);
                out.push(' ');
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn break_raw_paren_chains(s: &str) -> String {
    let needle = "#raw(\"";
    if !s.contains(needle) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Fast-path: find the next occurrence of `#raw("`.
        if bytes[i..].starts_with(needle.as_bytes()) {
            // Scan forward for the closing `")` of the #raw(...) call.
            let start = i + needle.len();
            let mut j = start;
            let mut found_close = false;
            while j + 1 < bytes.len() {
                if bytes[j] == b'"' && bytes[j + 1] == b')' {
                    // Found `")` — the call ends at j+2.
                    j += 2;
                    found_close = true;
                    break;
                }
                j += 1;
            }
            if found_close && bytes.get(j) == Some(&b'(') {
                // Emit `#raw("…")` then insert a space before the `(`.
                out.push_str(&s[i..j]);
                out.push(' ');
                i = j; // the `(` is emitted in the normal path below
            } else {
                out.push(s[i..].chars().next().unwrap());
                i += s[i..].chars().next().unwrap().len_utf8();
            }
        } else {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
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

// ─── Label extraction & normalization ─────────────────────────────────────────

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
