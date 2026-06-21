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
use std::path::PathBuf;

use tree_sitter::Node;

use crate::class_map::DocClass;
use crate::document::{Content, DocumentMetadata};
use crate::warnings::{Category, Severity, Warning};

mod bibliography;
mod boundary;
mod braceless;
mod environments;
mod escape;
pub(crate) mod figures;
mod inline;
mod macros;
mod math;
mod math_symbols;
mod node_utils;
mod preamble;
mod sections;
mod tables;
mod typography;
pub(in crate::emit) use bibliography::harvest_bib_keys_from_dir;
pub(crate) use braceless::{consume_braceless_arg, try_consume_math_arg, BracelessArg};
pub(in crate::emit) use braceless::{
    consume_trailing_arg_groups, consume_trailing_brace_groups, substitute_macro_args,
};
pub(crate) use escape::{
    escape_paren_semicolons, escape_text_cell, escape_text_for_typst_content,
    escape_unbalanced_math_brackets, is_typst_label_char, needs_text_escape, sanitize_label_key,
    strip_trailing_typst_label,
};
pub(in crate::emit) use figures::parse_graphicspath_dirs;
pub(in crate::emit) use macros::{
    extract_declare_math_operator_from_newcmd, extract_def_and_record, extract_environment_def,
    extract_let, extract_newcommand, extract_newcommandx, extract_newcommandx_and_end,
    extract_theorem_def, find_explsyntaxoff_end, find_makeatother_end, let_alias_def,
    new_command_token_kind, read_newif_flag,
};
pub(crate) use macros::{
    harvest_macros_from_source, harvest_referenced_labels_from_source, MacroDef,
};
pub(crate) use math_symbols::lookup_math_symbol;
pub(in crate::emit) use node_utils::{
    brace_balanced_end, color_command_parts, color_from_model_spec, command_name_of,
    command_name_text, environment_name, extract_label_name, extract_label_name_and_end,
    extract_label_ref_keys_and_end, first_curly_group, first_curly_like, flatten_text_children,
    is_command, is_comment, is_section_kind, leading_font_switch, math_font_decl_wrapper,
    named_color, needs_empty_base, needs_subscript_parens, nth_curly_group, parse_definecolor,
    peek_tex_assignment_end, range_of, section_level, skip_balanced_braces, split_math_rows,
};
pub(in crate::emit) use preamble::{
    build_neutral_preamble, extract_class_and_options, extract_latex_include_path,
    extract_package_names, extract_package_options, is_known_noop_package, resolve_input_path,
    resolve_package_path,
};
pub(crate) use typography::apply_text_accent;
use typography::{is_operatorname_only_function, should_split_math_word};

/// Sentinel character emitted by `push_math_symbol` immediately after a
/// multi-character math identifier so that `collapse_math_spaces` can
/// later decide whether to insert a real separator (when the next char
/// would fuse — letter or digit) or drop it (when Typst already breaks
/// — `_`, `^`, `,`, `(`, `)`). Chosen as U+0017 ETB which has no
/// legitimate use in either LaTeX source or rendered Typst.
const MATH_WORD_BOUNDARY: char = '\u{17}';

/// Beamer's default theme "structure" color — `rgb(0.2, 0.2, 0.7)` ≈ `#3333b3`.
/// Used for frame titles so slides match beamer's blue title style.
pub(in crate::emit) const BEAMER_TITLE_BLUE: &str = "#3333b3";

/// Make a plain-text run safe inside a Typst `"…"` string literal: convert the common
/// LaTeX char-escapes (`\$ \% \& \# \_ \{ \}`) to their literal characters, then escape
/// any remaining `\` and `"` so the string is well-formed (no dangling backslash escaping
/// the closing quote).
fn typst_string_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < s.len() {
        let c = bytes[i];
        if c == b'\\' && i + 1 < s.len() {
            let n = bytes[i + 1];
            if matches!(n, b'$' | b'%' | b'&' | b'#' | b'_' | b'{' | b'}') {
                out.push(n as char); // LaTeX literal-char escape → the char
                i += 2;
                continue;
            }
            out.push_str("\\\\"); // a real backslash → escaped for the Typst string
            i += 1;
            continue;
        }
        if c == b'"' {
            out.push_str("\\\"");
            i += 1;
            continue;
        }
        // Preserve multi-byte UTF-8 as a unit.
        let ch = s[i..].chars().next().unwrap_or('?');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Advance past ASCII whitespace from `i`.
fn skip_ascii_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// Given `bytes[open] == b'{'`, return the index of the matching `}` (depth-aware,
/// skipping `\{`/`\}` escapes), or `None` if unbalanced.
fn match_brace_from(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 2; // skip the escaped char
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Approximate Typst color expression for a beamer `\usecolortheme{<name>}`'s
/// structure color. Exact `\setbeamercolor`/`\definecolor` values are resolved
/// separately; this only covers the named built-in color themes. `None` (unknown
/// theme) leaves the default blue.
fn colortheme_structure_color(name: &str) -> Option<String> {
    let expr = match name.trim() {
        // Blue family (beamer's default-ish structure).
        "default" | "whale" | "dolphin" | "orchid" => "rgb(\"#3333b3\")",
        "beaver" => "rgb(\"#a62640\")",  // dark red
        "crane" => "rgb(\"#d4900a\")",   // orange
        "rose" | "lily" => "rgb(\"#b03060\")",
        "seahorse" => "rgb(\"#7a6da8\")", // muted purple
        "dove" => "black",                // grayscale theme
        "seagull" | "beetle" | "fly" => "rgb(\"#4d4d4d\")", // gray
        _ => return None,
    };
    Some(expr.to_string())
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
    /// biblatex `\addbibresource{file.bib}` paths collected in the prepass. A
    /// following `\printbibliography` renders `#bibliography(...)` from these
    /// (the bibtex `bibtex_include` path does not fire for biblatex docs).
    addbibresource_paths: Vec<String>,
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
    /// `\definecolor`-harvested colours: LaTeX name → Typst colour expression
    /// (e.g. `brand` → `rgb("#FF8800")`). Populated in the prepass so a
    /// `\textcolor{brand}{…}` in the body resolves regardless of definition order.
    colors: HashMap<String, String>,
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
    /// Sanitized label keys already emitted as a `<key>` definition. LaTeX
    /// tolerates the same `\label` twice (warning); Typst hard-errors "label
    /// occurs multiple times". We emit each key only once (first-def-wins;
    /// refs still resolve). Corpus 2605.31345 duplicated `\label{ssec:comparison}`.
    emitted_labels: HashSet<String>,
    /// True once a `\documentclass` is seen — i.e. this is a full document
    /// (not a bare fragment). Gates the self-generated neutral preamble so
    /// fragment conversions stay preamble-free.
    saw_document_class: bool,
    /// True for a chapter-bearing class (`book`/`report`/memoir/thesis), where
    /// `\chapter` is the top level so `\section` sits at level 2 (not level 1 as in
    /// article). Detected from the `\documentclass` name (round-5 dogfood T2).
    chapter_based: bool,
    /// True once we've entered the `document` environment, i.e. we're past the
    /// preamble. Gates preamble-only commands like `\abstract{…}` (the command form)
    /// so an incidental `{…}` group in the BODY isn't mistaken for the abstract.
    in_document_body: bool,
    /// Beamer frame-title color resolved from `\setbeamercolor{frametitle}{fg=…}`
    /// (most specific) — a Typst color expression. Wins over `beamer_structure_color`.
    beamer_frametitle_color: Option<String>,
    /// Beamer structure color from `\setbeamercolor{structure}{fg=…}` or
    /// `\usecolortheme{name}` — used for the frame title when no explicit
    /// `frametitle` color is set. Both `None` → beamer's default structure blue.
    beamer_structure_color: Option<String>,
    /// `\newtheorem{name}{Display}` declarations harvested as we walk the
    /// source. When `emit_generic_environment` encounters an unknown env name
    /// that matches a key here, it routes to `emit_theorem_env_dyn` instead of
    /// `warn_unsupported_environment`.
    theorem_kinds: HashMap<String, String>,
    /// Theorem `kind:` strings actually emitted, so `finish()` can add one head
    /// show rule per kind (`#show figure.where(kind: …): "Theorem N (Note). body"`).
    used_theorem_kinds: std::collections::HashSet<String>,
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
    /// Set when a `#subpar.grid(...)` is emitted; triggers the conditional
    /// `#import "@preview/subpar:0.2.2"` at the top of the document in `finish()`.
    used_subpar: bool,
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
    /// True when a `\bibliography{...}` names a path that resolves to a `.bib`
    /// on disk — i.e. `emit_bibliography` will emit a real `#bibliography(.bib)`.
    /// This is the precondition for emitting `#cite(<key>, form: …)` citation
    /// forms: against an inlined `.bbl`/`thebibliography` (no `#bibliography`)
    /// `#cite` aborts the compile. Stricter than `had_bib_file`, which is also
    /// set for `.bbl`-only papers (the key harvest reads `.bbl` files too).
    bib_will_render: bool,
    /// Citation mode forced by an explicit natbib/biblatex package option
    /// (`\usepackage[numbers]{natbib}` → Numeric, `[authoryear]` → AuthorYear).
    /// `None` when no relevant option was given — then the `\bibliographystyle`
    /// bst name or the document-class default decides. Consumed by
    /// `resolve_bib_style` to pick the `#bibliography(..., style: ...)` arg.
    natbib_mode: Option<crate::style_profile::CiteMode>,
    /// When true, `emit_node` records each node's output text + source span
    /// into `source_map`. Off by default (zero-overhead normal conversion).
    pub(crate) record_source_map: bool,
    /// Content-anchored provenance entries (see source_map.rs). Empty unless
    /// `record_source_map` is set. When capture is enabled, total cloned output
    /// is O(document size × node depth) — fine for one-shot `byetex diagnose`,
    /// not for bulk corpus processing.
    pub(crate) source_map: Vec<crate::source_map::NodeOutput>,
}

/// Maximum allowed `\newcommand` expansion depth (see `Emitter::macro_depth`).
/// Each level allocates a fresh sub-Emitter and re-parses the body, so the
/// per-level stack usage is high; values much above 24 can overflow test
/// threads' default 2 MB stack. Real papers rarely nest macros more than
/// 4-5 levels.
const MAX_MACRO_DEPTH: u32 = 24;

/// Everything [`Emitter::finish`] produces. A named struct (vs a tuple) keeps
/// the signature readable and avoids positional destructuring as fields grow.
pub(crate) struct FinishOutput {
    pub typst: String,
    pub warnings: Vec<Warning>,
    pub asset_refs: Vec<crate::AssetRef>,
    pub class_metadata: std::collections::HashMap<String, String>,
    pub source_map: Vec<crate::source_map::NodeOutput>,
}

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
            addbibresource_paths: Vec::new(),
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
            colors: HashMap::new(),
            beamer_frametitle_color: None,
            beamer_structure_color: None,
            base_dir,
            visited_includes: visited,
            macros: preseeded_macros,
            newif_flags: HashMap::new(),
            referenced_labels: HashSet::new(),
            emitted_labels: HashSet::new(),
            saw_document_class: false,
            chapter_based: false,
            in_document_body: false,
            theorem_kinds: HashMap::new(),
            used_theorem_kinds: std::collections::HashSet::new(),
            env_arg_counts: HashMap::new(),
            bibliography_keys: std::collections::HashSet::new(),
            asset_refs: Vec::new(),
            macro_depth: 0,
            in_minipage: false,
            used_text_label_anchor: false,
            used_subpar: false,
            has_bibtex_include: false,
            had_bib_file: false,
            bib_will_render: false,
            natbib_mode: None,
            record_source_map: false,
            source_map: Vec::new(),
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
                // A real `#bibliography(.bib)` is emitted only when one of the
                // listed paths resolves to a `.bib` on disk (mirrors the `kept`
                // logic in `emit_bibliography`). When it does NOT — a `.bbl` is
                // inlined as `#figure ... <key>` labels, or the call is dropped —
                // `#cite(<key>, …)` would abort, so citation forms must stay
                // `@key`. (`had_bib_file`/`bib_file_is_authoritative` can't be
                // used for this: the key harvest also reads `.bbl` files, so it
                // is true for `.bbl`-only papers where no `#bibliography` renders.)
                if let Some(ref base) = self.base_dir {
                    if bibliography::extract_bib_paths(n, self.src)
                        .iter()
                        .any(|p| bibliography::probe_bib_on_disk(base, p).is_some())
                    {
                        self.bib_will_render = true;
                    }
                }
            }
            // biblatex `\addbibresource{file.bib}` (tree-sitter `biblatex_include`)
            // — record the resource so a following `\printbibliography` renders a
            // real `#bibliography(.bib)` (the bibtex `bibtex_include` branch above
            // never fires for biblatex docs; corpus 2605.31009/30843). Mirror the
            // resolve-on-disk → `bib_will_render` logic so cites resolve.
            if n.kind() == "biblatex_include" {
                if let Some(g) = n.child_by_field_name("glob") {
                    let inner = self.src[g.start_byte() + 1..g.end_byte() - 1].to_string();
                    for raw in inner.split(',') {
                        let p = raw.trim().to_string();
                        if p.is_empty() {
                            continue;
                        }
                        if let Some(ref base) = self.base_dir {
                            let stem = p.strip_suffix(".bib").unwrap_or(p.as_str());
                            if bibliography::probe_bib_on_disk(base, stem).is_some() {
                                self.bib_will_render = true;
                                self.has_bibtex_include = true;
                            }
                        }
                        self.addbibresource_paths.push(p);
                    }
                }
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
                "color_definition" => {
                    // `\definecolor{name}{model}{spec}` — harvest into the colour
                    // table so a later `\textcolor{name}{…}` resolves. The node
                    // itself is dropped at emit time (xcolor styling is inert).
                    if let Some((name, typst)) = parse_definecolor(n, self.src) {
                        self.colors.entry(name).or_insert(typst);
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

    /// One head show rule per emitted theorem kind: renders the figure as
    /// `*Supplement N (Note).* body` (LaTeX shows a theorem head + number; a
    /// bare `#figure` shows neither). Kinds are sorted for deterministic output.
    fn theorem_show_rules(&self) -> String {
        let mut kinds: Vec<&String> = self.used_theorem_kinds.iter().collect();
        kinds.sort();
        let mut s = String::new();
        for k in kinds {
            let _ = writeln!(
                s,
                "#show figure.where(kind: \"{k}\"): it => block(width: 100%, above: 1.1em, below: 1.1em)[#strong[#it.supplement #context it.counter.display(it.numbering)#if it.caption != none [ (#it.caption.body)].] #it.body]"
            );
        }
        s
    }

    pub(crate) fn finish(mut self) -> FinishOutput {
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
            if self.used_subpar {
                self.out.push_str("#import \"@preview/subpar:0.2.2\"\n");
                // Emitted here; clear so the fragment-preamble block below
                // (which runs unconditionally) doesn't prepend it a second time.
                self.used_subpar = false;
            }
            // Venue styles (e.g. ACL's acl.sty) hard-code page geometry over the
            // class options; apply now that the class, packages and user geometry
            // are all resolved.
            self.layout.apply_venue_style(&self.detected_class);
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
            if !self.used_theorem_kinds.is_empty() {
                let rules = self.theorem_show_rules();
                self.out.push_str(&rules);
                self.used_theorem_kinds.clear();
            }
            if self.needs_equation_numbering {
                self.out
                    .push_str("#set math.equation(numbering: \"(1)\")\n");
            }
            self.out.push('\n');
            if self.layout.is_two_column(&self.detected_class) {
                // Document-level two-column (page `columns: 2` set in the
                // preamble). The title block spans BOTH columns as a full-width
                // float at the top of page 1; the body then flows in two columns
                // below it (mirrors LaTeX `\twocolumn[\maketitle …]`).
                self.out
                    .push_str("#place(top + center, scope: \"parent\", float: true)[\n");
                self.out.push_str(title_block.trim_end());
                self.out.push_str("\n]\n");
                // In-column conference classes (ICML/IEEE/ACM/ACL) defer their
                // abstract (and IEEE keywords): they flow as the first in-column
                // content (column 1), matching the LaTeX layout.
                let profile = crate::style_profile::StyleProfile::for_class(&self.detected_class);
                if profile.abstract_in_columns {
                    if let Some(a) = self.metadata.r#abstract.take() {
                        if !a.is_empty() {
                            let block =
                                self.render_abstract_block(profile.abstract_style, a.as_content());
                            self.out.push_str(&block);
                        }
                    }
                    if !self.metadata.keywords.is_empty() {
                        let kws = self
                            .metadata
                            .keywords
                            .drain(..)
                            .collect::<Vec<_>>()
                            .join(", ");
                        let _ =
                            writeln!(self.out, "#v(0.3em)\n#text(size: 0.9em)[*Keywords:* {kws}]");
                    }
                }
                self.out.push_str(body.trim_start_matches('\n'));
            } else {
                // Single-column: full-width title block, then body.
                self.out.push_str(&title_block);
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
        if self.used_subpar {
            preamble.push_str("#import \"@preview/subpar:0.2.2\"\n");
        }
        if self.used_text_label_anchor {
            preamble.push_str("#show figure.where(kind: \"anchor\"): it => none\n");
        }
        if !self.used_theorem_kinds.is_empty() {
            preamble.push_str(&self.theorem_show_rules());
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

        // Fill each warning's suggested_skill from its category when an emit site
        // didn't set one explicitly, so every warning points at a repair guide.
        for w in &mut self.warnings {
            if w.suggested_skill.is_none() {
                w.suggested_skill =
                    crate::skill_map::default_skill_for(&w.category).map(str::to_string);
            }
        }

        let class_metadata = self.metadata.class_metadata;
        let source_map = std::mem::take(&mut self.source_map);
        FinishOutput {
            typst: self.out,
            warnings: self.warnings,
            asset_refs: self.asset_refs,
            class_metadata,
            source_map,
        }
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
    /// Emit a `\text{…}` body that appears inside math. Plain runs become quoted
    /// upright strings; an embedded `$…$` is re-entered as math (round-4 A5:
    /// `\text{if $x_t = y$}` must render the inner math, not a literal `$x_t = y$`).
    /// No `$` → a single quoted string (the historical behavior).
    fn emit_math_text_mode(&mut self, inner: &str) {
        // Split on UNESCAPED `$` only (a literal `\$` stays in the text run).
        let has_math = {
            let b = inner.as_bytes();
            (0..b.len()).any(|i| b[i] == b'$' && (i == 0 || b[i - 1] != b'\\'))
        };
        if !has_math {
            let _ = write!(self.out, "\"{}\"", typst_string_escape(inner));
            return;
        }
        let mut pieces: Vec<String> = Vec::new();
        let bytes = inner.as_bytes();
        let mut i = 0;
        while i < inner.len() {
            // Plain-text run up to the next UNESCAPED `$` (skip `\x` escapes).
            let text_start = i;
            while i < inner.len() {
                if bytes[i] == b'\\' && i + 1 < inner.len() {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'$' {
                    break;
                }
                i += 1;
            }
            let text = inner[text_start..i].trim();
            if !text.is_empty() {
                pieces.push(format!("\"{}\"", typst_string_escape(text)));
            }
            if i >= inner.len() {
                break;
            }
            // `$ … $` math run (closing `$` need not be escape-aware — math ends it).
            let dollar = i;
            i += 1; // opening `$`
            let math_start = i;
            while i < inner.len() && bytes[i] != b'$' {
                i += 1;
            }
            if i >= inner.len() {
                // Unbalanced `$` (a literal dollar with no closer) — re-emit the `$` and
                // the remainder as literal text rather than swallowing it as math.
                let tail = inner[dollar..].trim();
                if !tail.is_empty() {
                    pieces.push(format!("\"{}\"", typst_string_escape(tail)));
                }
                break;
            }
            let math = &inner[math_start..i];
            i += 1; // closing `$`
            let rendered = self.render_in_sub_emitter(math, true, true);
            let rendered = rendered.trim();
            if !rendered.is_empty() {
                pieces.push(rendered.to_string());
            }
        }
        self.out.push_str(&pieces.join(" "));
    }

    fn render_in_sub_emitter(&mut self, src: &str, in_math: bool, increment_depth: bool) -> String {
        // Source-map note: the fresh sub-emitter does NOT inherit
        // `record_source_map` and its `source_map` is not merged back, so content
        // routed through here (e.g. inlined `.bbl` bibliography) yields no
        // fine-grained provenance entries — such error lines resolve only to the
        // coarse enclosing node's span. Threading capture through here is the
        // deferred "fine-grained sub-buffer mapping" follow-up.
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
        // Citation forms are safe in the child iff the root will emit a real
        // `#bibliography(.bib)`; inherit that gate so `\citet` etc. in
        // expanded/included content also resolve as `#cite(form: …)`.
        sub.bib_will_render = self.bib_will_render;
        // The bibliography-style mode forced by a natbib option (parsed from
        // the preamble) must reach sub-emitted content too (mirrors
        // `bib_will_render`).
        sub.natbib_mode = self.natbib_mode;
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
    /// When `record_source_map` is set, records each node's output text and
    /// source span into `self.source_map` (content-anchored provenance).
    fn emit_node(&mut self, node: Node<'_>) -> usize {
        if !self.record_source_map {
            return self.emit_node_inner(node);
        }
        let out_start = self.out.len();
        let src = (node.start_byte(), node.end_byte());
        let r = self.emit_node_inner(node);
        if self.out.len() > out_start {
            self.source_map.push(crate::source_map::NodeOutput {
                src,
                output: self.out[out_start..].to_string(),
            });
        }
        r
    }

    fn emit_node_inner(&mut self, node: Node<'_>) -> usize {
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
            // A counter setter with a value tree-sitter can't parse — most often a
            // NEGATIVE step, `\addtocounter{footnote}{-1}` — comes through as an ERROR
            // node that GREEDILY spans the call AND following content, which the walker
            // would copy verbatim (dogfood A1). Skip only the command + its `{c}{n}`
            // arg groups (paragraph-safe) and let the rest of the node emit normally,
            // so a following paragraph's `{…}` isn't dropped with it.
            "ERROR"
                if {
                    let trimmed = self.src[node.start_byte()..node.end_byte()].trim_start();
                    ["\\setcounter", "\\addtocounter", "\\stepcounter", "\\refstepcounter"]
                        .iter()
                        .any(|c| trimmed.starts_with(c))
                } =>
            {
                // `args_start` = byte just after the leading `\command` token (skip the
                // node's leading whitespace, then the `\` + ASCII-letter command name).
                let after_ws = node.start_byte()
                    + (self.src[node.start_byte()..].len()
                        - self.src[node.start_byte()..].trim_start().len());
                let args_start = after_ws
                    + 1
                    + self.src[after_ws + 1..]
                        .find(|c: char| !c.is_ascii_alphabetic())
                        .unwrap_or(0);
                // Skip the command + its `{c}{n}` arg groups (paragraph-safe), then fall
                // through to the default child-walk so any post-arg content still emits.
                self.skip_until = self
                    .skip_until
                    .max(consume_trailing_arg_groups(self.src, args_start, false));
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
                    self.emit_math_text_mode(inner);
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
            // Words with a leading NON-alphabetic prefix: digit-prefix forms
            // like "2JX"/"2kg", but also a delimiter glued onto the following
            // identifier — tree-sitter parses `|arrival` (and `(arrival`) as a
            // single `word` node, so `alpha_end==0` and the alpha-split branch
            // above never fires; the run would otherwise leak verbatim and
            // Typst reads `arrival` as an unknown variable (corpus 2605.31072).
            // Strip the leading non-alpha run, then split the alpha run after it.
            if alpha.is_empty() {
                let prefix_end = text
                    .find(|c: char| c.is_ascii_alphabetic())
                    .unwrap_or(text.len());
                let prefix = &text[..prefix_end];
                let rest = &text[prefix_end..];
                let rest_alpha_end = rest
                    .find(|c: char| !c.is_ascii_alphabetic())
                    .unwrap_or(rest.len());
                let rest_alpha = &rest[..rest_alpha_end];
                let rest_tail = &rest[rest_alpha_end..];
                if should_split_math_word(rest_alpha) {
                    self.out.push_str(prefix);
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

        // `\begin{comment}...\end{comment}` (comment package) — tree-sitter
        // gives this its own `comment_environment` node whose body is a
        // `comment` child (already dropped). The `begin`/`end` markers, though,
        // leaked verbatim through the default walker (corpus 2605.22779 spilled
        // `\begin{comment}`/`\end{comment}` next to the body). Drop it whole.
        if node.kind() == "comment_environment" {
            return node.end_byte();
        }

        // Counter-manipulation commands (`\setcounter`, `\addtocounter`,
        // `\stepcounter`, `\refstepcounter`, `\newcounter`) get dedicated
        // tree-sitter node kinds, so they never reach `emit_generic_command` and
        // the default walker copied them verbatim into the body (dogfood F5). They
        // produce no visible output — drop the whole node (args are children).
        // NOT `counter_value`/`counter_typesetting` (`\value`/`\arabic`/`\the…`),
        // which DO render and must fall through.
        if matches!(
            node.kind(),
            "counter_definition"
                | "counter_addition"
                | "counter_increment"
                | "counter_declaration"
                | "counter_within_declaration"
                | "counter_without_declaration"
        ) {
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
            // biblatex `\addbibresource{...}` — emits nothing; the path was
            // collected in the prepass and rendered by `\printbibliography`.
            "biblatex_include" => return node.end_byte(),
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
                //
                // tree-sitter-latex mis-bounds `curly_group_author_list` on a
                // bare comma list (`\author{A, B, C}`): it ends the group at the
                // first comma and emits a zero-width close brace, so the node's
                // `end_byte` truncates the last name and leaks the rest into the
                // body. Re-derive the true `{...}` extent by brace-matching from
                // the group's opening `{` in source, and resume past that close.
                if let Some(arg) = first_curly_like(node) {
                    let open = arg.start_byte();
                    if let Some(end) = brace_balanced_end(self.src.as_bytes(), open) {
                        let inner = self.src.get(open + 1..end - 1).unwrap_or("").to_string();
                        self.raw_authors.push(inner);
                        // Skip every node within the author-block extent — the
                        // return value only governs raw-text gap copy; a sibling
                        // node inside `[node.end, end]` (e.g. the affiliation
                        // line after a tree-sitter mis-bound `\\`) would still be
                        // re-emitted into the body otherwise (corpus 2605.31063).
                        self.skip_until = self.skip_until.max(end);
                        return node.end_byte().max(end);
                    }
                    let inner = self
                        .src
                        .get(open + 1..arg.end_byte().saturating_sub(1))
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
                    // natbib/biblatex options pick the citation mode that drives
                    // the resolved bibliography style. `[numbers]`/`[super]` →
                    // Numeric; `[authoryear]` → AuthorYear. Bracket-shape options
                    // (round/square/sort/compress/comma/colon) carry no mode and
                    // are ignored. Bare `\usepackage{natbib}` (no relevant option)
                    // leaves the mode None so the bst/class still decides — this
                    // avoids contradicting a numeric bst paired with bare natbib.
                    if pkg == "natbib" || pkg == "biblatex" {
                        if let Some(o) = opts.as_deref() {
                            for tok in o.split(',') {
                                match tok.trim() {
                                    "numbers" | "super" | "superscript" => {
                                        self.natbib_mode =
                                            Some(crate::style_profile::CiteMode::Numeric);
                                    }
                                    "authoryear" => {
                                        self.natbib_mode =
                                            Some(crate::style_profile::CiteMode::AuthorYear);
                                    }
                                    _ => {}
                                }
                            }
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
                // Chapter-bearing class → `\section` is level 2 under `\chapter`. Detect
                // from the class name (covers book/report/memoir KOMA variants and custom
                // thesis classes like `tudelft-report`); article-family stays level-1.
                let cl = c.to_ascii_lowercase();
                self.chapter_based = matches!(
                    cl.as_str(),
                    // KOMA-Script chapter classes (scrreprt's name lacks a full "report").
                    "book" | "report" | "memoir" | "scrbook" | "scrreprt"
                ) || cl.contains("thesis")
                    || cl.contains("dissertation")
                    || ((cl.contains("book") || cl.contains("report")) && !cl.contains("article"));
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
        // braces.
        if node.kind() == "curly_group" {
            if let Some((wrap, switch_end)) = leading_font_switch(node, self.src) {
                return self.emit_font_switch_group(node, switch_end, wrap);
            }
            // Any other bare `{...}` in TEXT mode is pure LaTeX grouping (local
            // scope), not literal braces. Emit the inner content WITHOUT the
            // braces — Typst reads `{...}` as a CODE block, which breaks after a
            // set rule (`#set heading(...){ ... }`) and lets nested groups leak
            // (`\textbf{{X}}` → `*{X}*`); corpus 2605.31603. (Math `{...}` is
            // handled earlier in this function and keeps its braces.)
            let inner_end = node.end_byte().saturating_sub(1);
            let mut cursor = node.walk();
            let mut last = node.start_byte() + 1; // skip the opening `{`
            for child in node.children(&mut cursor) {
                // Skip the boundary brace tokens themselves.
                if child.start_byte() == node.start_byte() || child.end_byte() == node.end_byte() {
                    continue;
                }
                let cs = child.start_byte();
                self.safe_copy(last, cs);
                last = self.emit_node(child);
            }
            self.safe_copy(last, inner_end);
            return node.end_byte();
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

        // TeX register/penalty/dimen/skip assignments (`\clubpenalty=300`,
        // `\interfootnotelinepenalty=10000`, `\parindent=2em`,
        // `\parskip=0pt plus 1pt minus 1pt`) are preamble-tuning primitives with no
        // visual content. The grammar parses the command alone and leaves the
        // `=<value>` tail as sibling tokens that leaked into the body as text
        // (corpus 2605.31586: `=10000` rendered as a stray heading). Drop the
        // command (still warned) AND consume the assignment tail.
        if let Some(end) = peek_tex_assignment_end(self.src, node.end_byte()) {
            self.warn_unsupported_command(node);
            self.skip_until = self.skip_until.max(end);
            return end;
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
                    // `#linebreak()` directly followed by `(` is parsed as a call
                    // chain (`#linebreak()(…)` → "expected function, found
                    // content"); a zero-width space ends the expression so the
                    // `(` stays literal markup (corpus 2605.31063
                    // `\makecell{...\\($\Delta F$)}`).
                    if self.src[node.end_byte()..].starts_with('(') {
                        self.out.push('\u{200B}');
                    }
                } else {
                    if !self.out.ends_with(' ') && !self.out.ends_with('\n') {
                        self.out.push(' ');
                    }
                    self.out.push('\\');
                }
                // `\\*` (the no-page-break variant) and `\\[len]` (an optional
                // vertical skip) carry trailing modifiers the grammar does NOT
                // attach to this node — they surface as following source, so
                // consume them here (LaTeX order: `\\*[len]`). Otherwise the `*`
                // is emitted as an escaped `\*` glued onto the linebreak `\`,
                // which Typst reads as a live strong toggle; a run of verse `\\*`
                // lines then unbalances `*` → `unclosed delimiter` reported far
                // downstream (corpus ctan-memoir, the Villon ballade). The `[len]`
                // would otherwise leak as `\[len\]` into the next table cell.
                // Neither modifier has a Typst row-break analog → drop both.
                let mut end = node.end_byte();
                if self.src.as_bytes().get(end) == Some(&b'*') {
                    end += 1;
                }
                if self.src.as_bytes().get(end) == Some(&b'[') {
                    if let Some(rel) = self.src[end..].find(']') {
                        end += rel + 1;
                    }
                }
                if end != node.end_byte() {
                    self.skip_until = self.skip_until.max(end);
                }
                end
            }
            // Layout-only commands: drop silently and eat the trailing space
            // that LaTeX would consume after a command-without-args.
            Some("\\noindent") | Some("\\indent") => {
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            // Table rule commands have no Typst equivalent in our default
            // table emission (the table emitter reconstructs rules). Drop.
            Some("\\hline") | Some("\\toprule") | Some("\\midrule") | Some("\\bottomrule") => {
                node.end_byte()
            }
            // `\cmidrule[width](trim){a-b}`: drop the command but CONSUME its
            // trailing `(trim)`/`[width]`/`{range}` args — tree-sitter leaves
            // them as following text, so without this they leak into the next
            // table cell (corpus: corrupts a cell in every \cmidrule table). The
            // partial rule itself is reconstructed in emit_tabular from source.
            Some("\\cmidrule") => {
                let bytes = self.src.as_bytes();
                let mut i = node.end_byte();
                loop {
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    match bytes.get(i) {
                        Some(b'(') => {
                            while i < bytes.len() && bytes[i] != b')' {
                                i += 1;
                            }
                            i += usize::from(i < bytes.len());
                        }
                        Some(b'[') => {
                            while i < bytes.len() && bytes[i] != b']' {
                                i += 1;
                            }
                            i += usize::from(i < bytes.len());
                        }
                        Some(b'{') => {
                            while i < bytes.len() && bytes[i] != b'}' {
                                i += 1;
                            }
                            i += usize::from(i < bytes.len());
                            break;
                        }
                        _ => break,
                    }
                }
                self.skip_until = self.skip_until.max(i);
                i
            }
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
            // `\textpm` / `\textmp` (textcomp) — ± / ∓. Common in results tables
            // for uncertainty values (corpus 2605.22507).
            Some("\\textpm") => {
                self.out.push('±');
                node.end_byte()
            }
            Some("\\textmp") => {
                self.out.push('∓');
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
            // `\footnotemark[N]` prints footnote mark N *without* creating a
            // footnote (the body is supplied separately by `\footnotetext`).
            // tree-sitter parses the optional `[N]` as an AST sibling, so peek the
            // source bytes to consume it — otherwise it leaked as literal `\[N\]`
            // (corpus 2606.12397 author block). Render the mark as a superscript.
            // A bare `\footnotemark` keeps the placeholder behaviour.
            Some("\\footnotemark") => {
                let bytes = self.src.as_bytes();
                let mut i = node.end_byte();
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                let mut end = node.end_byte();
                let mut mark: Option<String> = None;
                if i < bytes.len() && bytes[i] == b'[' {
                    let mut j = i + 1;
                    let mut depth = 0i32;
                    let mut closed = false;
                    while j < bytes.len() {
                        match bytes[j] {
                            b'\\' if j + 1 < bytes.len() => {
                                j += 2;
                                continue;
                            }
                            b'{' => depth += 1,
                            b'}' => depth -= 1,
                            b']' if depth == 0 => {
                                closed = true;
                                break;
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    if closed {
                        mark = Some(self.src.get(i + 1..j).unwrap_or("").trim().to_string());
                        end = j + 1;
                    }
                }
                match mark.as_deref() {
                    Some(m) if !m.is_empty() => {
                        let _ = write!(self.out, "#super[{m}]");
                    }
                    _ => self.out.push_str("#footnote[]"),
                }
                // Mark the consumed `[N]` so its AST-sibling node isn't re-emitted.
                self.skip_until = self.skip_until.max(end);
                end
            }
            // `\mathtt{X}` in text mode — typewriter math font; render as code.
            Some("\\mathtt") => self.emit_inline_raw(node),
            // Deprecated font-switching commands (LaTeX 2.09 style). These change
            // the style of all following text until the group ends — Typst would
            // need a scope wrap, which requires end-of-group tracking we don't yet
            // have. Warn so the caller can see the loss.
            // Declarative font switches (TeX 2.09 + NFSS forms). The common
            // `{\bf x}` / `{\em y}` grouped form is wrapped in Typst markup at
            // the `curly_group` level (see emit_node_inner, where both braces can be
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
            // `\vspace{len}` / `\hspace{len}` — emit `#v(len)` / `#h(len)` when
            // the length is a plain Typst dimension (em/cm/mm/in/pt/ex, signed);
            // LaTeX length macros (`\baselineskip`, `\dimexpr…`) → drop.
            Some("\\vspace") | Some("\\vspace*") | Some("\\hspace") | Some("\\hspace*")
                if !self.in_math =>
            {
                let func = if name.as_deref().is_some_and(|n| n.starts_with("\\hspace")) {
                    "h"
                } else {
                    "v"
                };
                if let Some(arg) = first_curly_group(node) {
                    let raw = self
                        .src
                        .get(arg.start_byte() + 1..arg.end_byte().saturating_sub(1))
                        .unwrap_or("")
                        .trim();
                    if let Some(len) = convertible_length(raw) {
                        let _ = write!(self.out, "#{func}({len})");
                        // `#h(..)`/`#v(..)` glued to following punctuation (`(`,
                        // `[`, `@`, `.`, …) is parsed by Typst as a call / content
                        // arg / field-access / ref chain on the function result →
                        // code context → error (corpus 2605.22728 `\hspace{-0.1mm}(`
                        // and `\hspace{-0.1mm}@cite`). A zero-width space ends the
                        // expression as markup. Letters/digits/space are safe.
                        let glued_punct = self.src[node.end_byte()..]
                            .chars()
                            .next()
                            .is_some_and(|c| !c.is_alphanumeric() && !c.is_whitespace());
                        if glued_punct {
                            self.out.push('\u{200B}');
                        }
                    }
                }
                node.end_byte()
            }
            Some("\\kern")
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
            // minted config — takes an OPTIONAL `[lang]` plus a `{opts}` brace
            // group. No visible output; drop it with its args (the options leaked
            // into the body, dogfood F5). Only these take a `[opt]`, so they're the
            // only commands allowed to consume a leading `[...]`.
            Some("\\setminted") | Some("\\setmintedinline") | Some("\\usemintedstyle") => {
                let end = consume_trailing_arg_groups(self.src, node.end_byte(), true);
                self.skip_until = self.skip_until.max(end);
                end
            }
            // Inert no-output commands: counter/length setters + page-style controls.
            // When these parse normally their `{arg}` groups are CHILDREN (covered by
            // `node.end_byte()`), so we must NOT greedily consume following groups — that
            // would eat real body content (`\pagestyle{x} {bold}`). The leak case is a
            // counter setter whose VALUE breaks the parse (e.g. `\addtocounter{c}{-1}`);
            // that surfaces as an ERROR node and is handled there (dogfood A1).
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
            | Some("\\bibliographystyle") => node.end_byte(),
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
            // `\ExplSyntaxOn ... \ExplSyntaxOff` brackets expl3 code (`\cs_new:Npn`,
            // `\seq_new:N`, `\tl_set:Nn`, …) whose `:`/`_` argument-spec catcodes
            // tree-sitter can't parse, so the region leaks verbatim through the copy
            // path as garbage (corpus 2605.22821 leaked ~294 such lines). It is
            // preamble-only with no renderable body, so skip it wholesale to the
            // matching `\ExplSyntaxOff`, mirroring the `\makeatletter` region-skip.
            Some("\\ExplSyntaxOn") => {
                if let Some(end) = find_explsyntaxoff_end(self.src, node.end_byte()) {
                    self.skip_until = self.skip_until.max(end);
                    end
                } else if !self.saw_document_class {
                    // Unmatched in a preamble fragment — the remainder is all expl3.
                    self.skip_until = self.src.len();
                    self.src.len()
                } else {
                    node.end_byte()
                }
            }
            // A stray `\ExplSyntaxOff` (no matching `\ExplSyntaxOn` on this walk —
            // e.g. the `\ExplSyntaxOn` lived in an already-skipped region) is a
            // no-op switch; drop it so it doesn't leak.
            Some("\\ExplSyntaxOff") => node.end_byte(),
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
                    // it so the low-level TeX doesn't leak.
                    //
                    // BUT a BibTeX `.bbl` (apsrev4-1.bst / natbib) opens its
                    // `thebibliography` with `\makeatletter` + `\providecommand
                    // \@…` internals and never writes `\makeatother`. Skipping to
                    // EOF there would swallow every `\bibitem`, so the inlined
                    // bibliography would emit no `<key>` anchors and each
                    // `\cite{…}` would dangle (corpus 2605.31203). Cap the skip at
                    // the first `\bibitem` / `\end{thebibliography}`: the macro
                    // preamble is still harvested and dropped, but the entries
                    // (and their anchors) render.
                    let rest_from = node.end_byte();
                    let rest = &self.src[rest_from..];
                    let cap = ["\\bibitem", "\\end{thebibliography}"]
                        .iter()
                        .filter_map(|m| rest.find(m))
                        .min()
                        .map(|rel| rest_from + rel)
                        .unwrap_or(self.src.len());
                    self.harvest_definitions(&self.src[rest_from..cap]);
                    self.skip_until = self.skip_until.max(cap);
                    cap
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
            //   earlier in emit_node_inner, near the `color_reference` arm.)
            // • Conditionals: \ifthenelse/\fi/\else are xcolor/ifthen preamble
            //   control flow that tree-sitter surfaces as bare generic_commands
            //   (the contained body is processed separately by tree-sitter's normal
            //   child walk). Dropping these tokens is safe; the content nodes are
            //   emitted normally.
            // Beamer color-theme commands (beamer only) — harvest the frame-title /
            // structure color BEFORE the presentation-layer drop below silently
            // consumes them, so titles match the deck's theme not a hard-coded blue.
            Some(cmd @ "\\setbeamercolor") | Some(cmd @ "\\usecolortheme")
                if self.detected_class == DocClass::Beamer =>
            {
                self.harvest_beamer_color(node, cmd);
                node.end_byte()
            }
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
            | Some("\\titlegraphic") => {
                node.end_byte()
            }
            // `\subtitle{…}`: rendered under the title on a beamer title slide. Papers
            // have no subtitle slot, so for them it stays dropped.
            // `\subtitle{…}` — a title-block element (beamer, but also report/book/thesis
            // and the `subtitle` package on article). Capture + render it under the title
            // for any class (`flush_title_block`); was previously captured for beamer only
            // and dropped elsewhere, losing content (round-5 dogfood T1).
            Some("\\subtitle") => {
                if self.metadata.subtitle.is_none() {
                    if let Some(arg) = first_curly_group(node) {
                        self.metadata.subtitle =
                            Some(Content::Typst(self.render_curly_group_content(arg)));
                    }
                }
                node.end_byte()
            }

            // Standard LaTeX counter display commands — emit as Typst context
            // counter expressions.  These never take arguments so they are
            // a single token; the `#` prefix works in both markup and math mode.
            Some("\\thepage") => {
                self.out.push_str("#context counter(page).display()");
                node.end_byte()
            }
            // `counter(heading.N)` is INVALID Typst (`.N` is field access on the
            // `heading` element function → "expected comma"). The heading counter
            // is `counter(heading)`; `.display()` formats it per the document's
            // own numbering and reflects the current position. We deliberately
            // display the full current heading number rather than slicing to the
            // command's level — simpler, always valid, and works in markup AND
            // math mode (corpus ctan-memoir, gh-sikatikenmogne-report).
            Some("\\thesection")
            | Some("\\thesubsection")
            | Some("\\thesubsubsection")
            | Some("\\thechapter") => {
                self.out.push_str("#context counter(heading).display()");
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
            // `\newtcbtheorem[init]{name}{title}{options}{prefix}` (tcolorbox
            // theorem def). Unhandled, its 4 mandatory groups — especially the
            // big `{options}` with nested braces / `\rule` / `\bfseries` — leak
            // into the body as siblings and can cascade into following preamble
            // content like `\author` (corpus 2605.31063). Consume the optional
            // `[..]` plus the 4 mandatory `{..}` groups.
            Some("\\newtcbtheorem") | Some("\\newtcbtheorem*") => {
                let bytes = self.src.as_bytes();
                let mut i = node.end_byte();
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if bytes.get(i) == Some(&b'[') {
                    while i < bytes.len() && bytes[i] != b']' {
                        i += 1;
                    }
                    i += usize::from(i < bytes.len());
                }
                for _ in 0..4 {
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if bytes.get(i) == Some(&b'{') {
                        match brace_balanced_end(bytes, i) {
                            Some(end) => i = end,
                            None => break,
                        }
                    } else {
                        break;
                    }
                }
                self.skip_until = self.skip_until.max(i);
                i
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
            // biblatex `\printbibliography[opts]` — render `#bibliography(.bib)`
            // from the collected `\addbibresource` paths, then consume the
            // trailing optional `[title={...}]` so it doesn't leak (the bracket
            // group is a tree-sitter sibling of the command).
            Some("\\printbibliography") => {
                let paths = self.addbibresource_paths.clone();
                let end = if paths.is_empty() {
                    self.warn_silently_dropped(node);
                    node.end_byte()
                } else {
                    self.render_bibliography_from_paths(paths, node)
                };
                let bytes = self.src.as_bytes();
                let mut i = end;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if bytes.get(i) == Some(&b'[') {
                    while i < bytes.len() && bytes[i] != b']' {
                        i += 1;
                    }
                    i += usize::from(i < bytes.len());
                    self.skip_until = self.skip_until.max(i);
                    i
                } else {
                    end
                }
            }
            // Beamer `\tableofcontents` → a section outline. Beamer `\section`s convert
            // to level-1/2 headings, so `#outline` lists them; the frame already supplies
            // the "Outline" title, so the outline itself carries none. Slides only show
            // sections/subsections, so cap the depth at 2.
            Some("\\tableofcontents") if self.detected_class == DocClass::Beamer => {
                self.ensure_paragraph_break();
                self.out.push_str("#outline(title: none, depth: 2)\n");
                node.end_byte()
            }
            // Book/report/thesis `\tableofcontents` → a titled `#outline` of the
            // chapters/sections (they convert to Typst headings). Was dropped (B-toc
            // covered only beamer); the thesis dogfood had to add this by hand.
            // depth 3 = chapter/section/subsection. Article-family keeps the drop below.
            Some("\\tableofcontents") if self.chapter_based => {
                self.ensure_paragraph_break();
                self.out.push_str("#outline(depth: 3)\n\n");
                node.end_byte()
            }
            // Book/report front/main-matter page numbering: `\frontmatter` → roman page
            // numbers, `\mainmatter` → arabic reset to 1. Were dropped (numbering lost; the
            // thesis dogfood added them by hand). Only in chapter-bearing classes; else they
            // fall through to the drop arm below.
            Some("\\frontmatter") if self.chapter_based => {
                self.ensure_paragraph_break();
                self.out.push_str("#set page(numbering: \"i\")\n\n");
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            Some("\\mainmatter") if self.chapter_based => {
                self.ensure_paragraph_break();
                self.out
                    .push_str("#set page(numbering: \"1\")\n#counter(page).update(1)\n\n");
                consume_trailing_inline_space(self.src, node.end_byte())
            }
            // Tables-of-contents et al. — Typst equivalents not yet emitted; warn
            // so the user knows these structural sections were not preserved.
            Some("\\tableofcontents")
            | Some("\\listoffigures")
            | Some("\\listoftables")
            | Some("\\printindex")
            // Nomenclature / glossary output commands — generated list is lost.
            | Some("\\printnomenclature")
            | Some("\\printglossary")
            | Some("\\printglossaries")
            // Book-class structure dividers — affect page numbering / heading
            // numbering in ways Typst doesn't model; warn so users are aware.
            // (`\frontmatter`/`\mainmatter` are handled above for chapter classes.)
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
                // Keep the Typst escape `\&` (renders a literal `&`). A bare `&`
                // is indistinguishable from a tabular column separator and gets
                // split as one — corrupting cells (corpus 2605.31604:
                // `\multicolumn{3}{c}{Document \& Diagram}`). The table cell
                // splitter only breaks on UNescaped `&`.
                self.out.push_str("\\&");
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
            // `\frametitle{X}` (beamer): the slide title, when not given as the
            // `\begin{frame}{X}` argument. Render it as a bold heading line.
            Some("\\frametitle") if self.detected_class == DocClass::Beamer => {
                if let Some(arg) = first_curly_group(node) {
                    let title = self.render_curly_group_content(arg).trim().to_string();
                    if !title.is_empty() {
                        self.ensure_paragraph_break();
                        let fill = self.beamer_title_fill();
                        let _ = write!(
                            self.out,
                            "#text(size: 1.2em, weight: \"bold\", fill: {fill})[{title}]\n\n"
                        );
                    }
                }
                node.end_byte()
            }
            // `\titlepage` (beamer): the title block is auto-emitted at the document
            // head (like `\maketitle`), so this is a no-op — it just must not leak.
            Some("\\titlepage") if self.detected_class == DocClass::Beamer => node.end_byte(),
            // `\frame{X}` (beamer): the COMMAND form of a slide. Render X as a slide
            // with a leading weak pagebreak. `\frame{\titlepage}` renders to nothing
            // (titlepage is a no-op + the title is at the top), so emit no blank slide.
            Some("\\frame") if self.detected_class == DocClass::Beamer => {
                if let Some(arg) = first_curly_group(node) {
                    let content = self.render_curly_group_content(arg);
                    if !content.trim().is_empty() {
                        self.ensure_paragraph_break();
                        let _ = write!(self.out, "#pagebreak(weak: true)\n\n{}\n", content.trim());
                    }
                }
                node.end_byte()
            }
            // Beamer overlay commands. A static PDF can't animate, so show content
            // unconditionally: skip the `<overlay-spec>` (so it doesn't leak) and let
            // the following `{content}` group render normally as a sibling. `\pause`
            // carries no content — drop it.
            Some("\\pause") if self.detected_class == DocClass::Beamer => node.end_byte(),
            // `\alt<spec>{default}{alternative}` — show the default (first) arg, drop
            // the spec and the alternative (a static PDF can't switch overlays).
            Some("\\alt") if self.detected_class == DocClass::Beamer => {
                self.emit_beamer_alt(node)
            }
            // Single-content overlay commands.
            Some("\\only") | Some("\\onslide") | Some("\\uncover") | Some("\\visible")
            | Some("\\action") | Some("\\alert")
                if self.detected_class == DocClass::Beamer =>
            {
                let before = self.skip_until;
                self.skip_overlay_spec(node.end_byte());
                if self.skip_until == before {
                    // No `<spec>` → tree-sitter attaches the `{content}` group as a
                    // CHILD of this command (returning end_byte would consume it
                    // unrendered). Render it explicitly so content isn't dropped.
                    if let Some(arg) = first_curly_group(node) {
                        let content = self.render_curly_group_content(arg);
                        self.out.push_str(&content);
                    }
                }
                // With a spec, the spec breaks the attachment so `{content}` is a
                // sibling and renders normally after the skipped spec.
                node.end_byte()
            }
            // `\abstract{text}` COMMAND form — some classes `\renewcommand{\abstract}`
            // to take the abstract as an argument (e.g. bytedance_seed.cls, corpus
            // 2605.31604) instead of the `abstract` environment. Like `\title`, it's a
            // PREAMBLE title-block command; capture it into the abstract field instead
            // of dropping the content. Restricted to the preamble so an incidental
            // body `{…}` group after a (rare) no-arg `\abstract` isn't mis-captured;
            // an earlier preamble command also can't clobber a real `abstract` env,
            // which is always in the body.
            Some("\\abstract") if !self.in_document_body && self.metadata.r#abstract.is_none() => {
                if let Some(arg) = first_curly_group(node) {
                    self.metadata.r#abstract = Some(Content::Typst(
                        self.render_curly_group_content(arg).trim().to_string(),
                    ));
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
                // Raw-bytes capture — same rationale as `author_declaration` above,
                // including the comma-list brace-boundary fix.
                if let Some(arg) = first_curly_group(node) {
                    let open = arg.start_byte();
                    if let Some(end) = brace_balanced_end(self.src.as_bytes(), open) {
                        let inner = self.src.get(open + 1..end - 1).unwrap_or("").to_string();
                        self.raw_authors.push(inner);
                        // See `author_declaration` above: skip the whole extent so
                        // sibling nodes inside it don't re-emit into the body.
                        self.skip_until = self.skip_until.max(end);
                        return node.end_byte().max(end);
                    }
                    let inner = self
                        .src
                        .get(open + 1..arg.end_byte().saturating_sub(1))
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
            // `\fcolorbox{frame}{bg}{X}` (tree-sitter parses this as a generic
            // command, not a `color_reference`, so without a handler the whole
            // node — content included — was warned-and-dropped). Emit a filled,
            // stroked box; drop just the frame/bg if a colour can't be resolved.
            Some("\\fcolorbox") => {
                let mut cursor = node.walk();
                let content = node
                    .children(&mut cursor)
                    .filter(|c| c.kind().starts_with("curly_group"))
                    .last()
                    .map(|c| self.render_curly_group_content(c))
                    .unwrap_or_default();
                let text = self.src.get(node.start_byte()..node.end_byte()).unwrap_or("");
                let (model, groups) = color_command_parts(text, "\\fcolorbox");
                let frame = groups
                    .first()
                    .and_then(|a| self.resolve_color_arg(model.as_deref(), a));
                let bg = groups
                    .get(1)
                    .and_then(|a| self.resolve_color_arg(model.as_deref(), a));
                match (bg, frame) {
                    (Some(b), Some(f)) => {
                        let _ = write!(
                            self.out,
                            "#box(fill: {b}, stroke: {f}, inset: 2pt)[{content}]"
                        );
                    }
                    (Some(b), None) => {
                        let _ = write!(self.out, "#box(fill: {b}, inset: 2pt)[{content}]");
                    }
                    _ => self.out.push_str(&content),
                }
                node.end_byte()
            }
            // `\resizebox{w}{h}{X}` / `\scalebox{f}{X}` / `\rotatebox{a}{X}` /
            // `\reflectbox{X}` (graphicx) scale/transform their LAST argument.
            // ByeTex can't reproduce the scaling, but the wrapped content —
            // frequently a wide `tabular` — MUST survive rather than be dropped
            // with the size args (corpus: 21 papers use
            // `\resizebox{\textwidth}{!}{…}` to fit a table to the text width).
            // Emit the last group's content; drop the size/transform args.
            Some("\\resizebox") | Some("\\scalebox") | Some("\\rotatebox")
            | Some("\\reflectbox") => {
                let mut cursor = node.walk();
                let content = node
                    .children(&mut cursor)
                    .filter(|c| c.kind().starts_with("curly_group"))
                    .last();
                if let Some(c) = content {
                    let rendered = self.render_curly_group_content(c);
                    self.out.push_str(&rendered);
                }
                node.end_byte()
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
                let n_raw = self
                    .src
                    .get(groups[0].start_byte() + 1..groups[0].end_byte() - 1)
                    .unwrap_or("1")
                    .trim();
                // LaTeX `\multirow{-N}` spans N rows UPWARD; Typst has no upward
                // span and rejects non-positive rowspans ("number must be
                // positive", corpus 2605.31563). Use the magnitude, floored at 1.
                let n = n_raw.parse::<i64>().unwrap_or(1).unsigned_abs().max(1);
                let content = self.render_curly_group_content(groups[2]);
                let _ = write!(self.out, "table.cell(rowspan: {})[{}]", n, content);
                node.end_byte()
            }
            // `\makecell[opts]{X}` — render the content; the [opts] are layout
            // hints we ignore.
            Some("\\makecell") => {
                // A makecell is a multi-line box: its internal `\\` is an
                // intra-cell line break. Render with `in_minipage` set so it
                // emits `#linebreak()` rather than the bare `\` the table
                // row-splitter keys on (a bare `\` glued to the next `*`/`_`
                // becomes an escaped literal `\*`, leaving the bold unclosed —
                // corpus 2606.12406 `\makecell{\textbf{Up-sampling}\\\textbf{Strategy}}`).
                let saved = self.in_minipage;
                self.in_minipage = true;
                let end = self.emit_inline_unwrap(node);
                self.in_minipage = saved;
                end
            }
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
            | Some("\\tiny") => {
                // These are unscoped switches with no argument. In the common
                // `{\small text}` form `\small` has no curly_group child and the
                // following text renders as siblings — just drop the directive.
                // But tree-sitter sometimes ABSORBS a following `{...}` as the
                // command's argument (e.g. `\small{\textpm 0.034}` in a table
                // cell, corpus 2605.22507); dropping the whole node then loses
                // that content. Render the absorbed group's content so it
                // survives, then drop the directive itself.
                if let Some(arg) = first_curly_group(node) {
                    let inner = self.render_curly_group_content(arg);
                    self.out.push_str(&inner);
                }
                node.end_byte()
            }
            // `\appendix` toggles section-number style to letters AND resets the
            // section/chapter counter so the first appendix is A (not a continuation of
            // the body count — round-6 dogfood: appendices showed D/E after 3 chapters).
            Some("\\appendix") => {
                self.out
                    .push_str("\n#set heading(numbering: \"A.1\")\n#counter(heading).update(0)\n");
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
                    // Emit each label key at most once (Typst rejects dups).
                    if !self.label_first_use(&key) {
                        // already defined elsewhere — refs resolve to the first.
                    } else if self.in_math {
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

    // ─── Environment dispatch & lists ─────────────────────────────────────────

    fn emit_generic_environment(&mut self, node: Node<'_>) -> usize {
        let env = environment_name(node, self.src);
        match env.as_deref() {
            // Beamer slide. Each `frame` becomes its own page with a bold title.
            // Gated on the beamer class so a non-beamer `frame` env (custom package,
            // poster) keeps its previous handling.
            Some("frame") if self.detected_class == DocClass::Beamer => {
                self.emit_beamer_frame(node)
            }
            // Beamer side-by-side columns → a Typst `#grid`. A bare `column`
            // (shouldn't appear outside `columns`) skips its `{width}` like minipage.
            Some("columns") if self.detected_class == DocClass::Beamer => {
                self.emit_beamer_columns(node)
            }
            Some("column") if self.detected_class == DocClass::Beamer => {
                self.emit_minipage(node)
            }
            // Beamer titled callout boxes → a titled `#block` with a theme-ish accent.
            Some("block") if self.detected_class == DocClass::Beamer => {
                self.emit_beamer_block(node, "#2f5fb3")
            }
            Some("alertblock") if self.detected_class == DocClass::Beamer => {
                self.emit_beamer_block(node, "#b33a3a")
            }
            Some("exampleblock") if self.detected_class == DocClass::Beamer => {
                self.emit_beamer_block(node, "#2e7d4f")
            }
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
            // Entering `document` marks the end of the preamble.
            Some("document") => {
                self.in_document_body = true;
                self.emit_environment_body(node)
            }
            // `titlepage` is its own page in LaTeX — isolate the body with pagebreaks so
            // a thesis's inner title page doesn't flow into the following frontmatter
            // (round-6 dogfood A6). Typst suppresses the leading weak break at the head.
            Some("titlepage") => {
                self.ensure_paragraph_break();
                self.out.push_str("#pagebreak(weak: true)\n\n");
                self.emit_environment_body(node);
                self.ensure_paragraph_break();
                self.out.push_str("#pagebreak(weak: true)\n\n");
                node.end_byte()
            }
            Some("center") | Some("flushleft")
            | Some("flushright") | Some("quote") | Some("quotation") | Some("verse")
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
            // longtable: a multi-page table with the same `{colspec}` shape as tabular.
            // Its page-break header/footer markers (`\endhead`, `\endfoot`, …) are
            // dropped no-ops; the body rows render as a single Typst table. Was dropped
            // wholesale (round-5 dogfood).
            | Some("longtable") | Some("longtable*") | Some("xltabular")
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

    // ===== Math mode =====

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
            // The overset family — `\cmd{script}{base}` places `script` above (or
            // below, for `\underset`) `base`. Typst: `attach(base, t|b: script)`.
            // Without this they dropped both args and leaked the bare name as a
            // string (`"accentset"`, …). `\stackrel`/`\accentset` are top-set.
            "\\overset" | "\\stackrel" | "\\accentset" => self.emit_math_attach(node, false),
            "\\underset" | "\\underaccent" => self.emit_math_attach(node, true),
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

    // ─── Cross-references & bibliography ──────────────────────────────────────

    /// Returns true the first time `key` is seen as a `<key>` label definition,
    /// false afterwards. Callers skip emitting the `<key>` token on `false` —
    /// Typst rejects duplicate labels (corpus 2605.31345). First-def-wins; refs
    /// still resolve to the surviving definition.
    fn label_first_use(&mut self, key: &str) -> bool {
        self.emitted_labels.insert(key.to_string())
    }

    /// The Typst color expression for beamer frame titles, resolved from the deck's
    /// theme: an explicit `\setbeamercolor{frametitle}{fg=…}` wins, else the structure
    /// color (`\setbeamercolor{structure}` / `\usecolortheme`), else beamer's default
    /// structure blue.
    fn beamer_title_fill(&self) -> String {
        self.beamer_frametitle_color
            .clone()
            .or_else(|| self.beamer_structure_color.clone())
            .unwrap_or_else(|| format!("rgb(\"{BEAMER_TITLE_BLUE}\")"))
    }

    /// Record a beamer color command's effect on the frame-title color.
    /// `\setbeamercolor{frametitle|titlelike}{fg=C}` → frametitle color;
    /// `\setbeamercolor{structure}{fg=C}` / `\usecolortheme{name}` → structure color.
    fn harvest_beamer_color(&mut self, node: Node<'_>, kind: &str) {
        match kind {
            "\\usecolortheme" => {
                if let Some(name) = first_curly_group(node)
                    .map(|g| self.curly_group_inner_trimmed(g).trim().to_string())
                {
                    if let Some(c) = colortheme_structure_color(&name) {
                        self.beamer_structure_color = Some(c);
                    }
                }
            }
            "\\setbeamercolor" => {
                // `{element}{fg=color, bg=…}` — resolve `fg` for title-bearing elements.
                let element = first_curly_group(node)
                    .map(|g| self.curly_group_inner_trimmed(g).trim().to_string())
                    .unwrap_or_default();
                if !matches!(element.as_str(), "frametitle" | "titlelike" | "structure") {
                    return;
                }
                let Some(opts) = nth_curly_group(node, 1)
                    .map(|g| self.curly_group_inner_trimmed(g).to_string())
                else {
                    return;
                };
                let fg = opts.split(',').find_map(|kv| {
                    let (k, v) = kv.split_once('=')?;
                    (k.trim() == "fg").then(|| v.trim().to_string())
                });
                if let Some(color) = fg.and_then(|c| self.resolve_color_arg(None, &c)) {
                    if element == "structure" {
                        self.beamer_structure_color = Some(color);
                    } else {
                        self.beamer_frametitle_color = Some(color);
                    }
                }
            }
            _ => {}
        }
    }

    /// Emit a beamer `\alt<spec>{default}{alternative}`: render the DEFAULT (first)
    /// arg through the converter, and skip the `<spec>` + both brace groups (so the
    /// alternative and the spec don't leak). Returns `node.end_byte()`; the sibling
    /// groups are then skipped via `skip_until`.
    fn emit_beamer_alt(&mut self, node: Node<'_>) -> usize {
        let bytes = self.src.as_bytes();
        // Scan from just after the `\alt` token, NOT `node.end_byte()`: when there's no
        // `<spec>` the `{default}{alt}` groups attach as CHILDREN, so `end_byte()` would
        // already be past them and the scan would find nothing (content lost).
        // Skip an optional `<overlay-spec>` — validated + bounded (same rule as the
        // `\item<…>` / `\only<…>` skippers), so a stray unclosed `<` or a `>` elsewhere
        // in the body can't be mis-consumed.
        let after_cmd = node.start_byte() + "\\alt".len();
        let i = skip_ascii_ws(
            bytes,
            environments::skip_leading_overlay_spec(self.src, after_cmd),
        );
        // First group `{default}`.
        if bytes.get(i) != Some(&b'{') {
            return node.end_byte();
        }
        let Some(close1) = match_brace_from(bytes, i) else {
            return node.end_byte();
        };
        let default_inner = self.src[i + 1..close1].to_string();
        // Optional second group `{alternative}` — skip it.
        let j = skip_ascii_ws(bytes, close1 + 1);
        let end = if bytes.get(j) == Some(&b'{') {
            match_brace_from(bytes, j).map_or(close1 + 1, |c| c + 1)
        } else {
            close1 + 1
        };
        let rendered = self.render_in_sub_emitter(&default_inner, false, false);
        self.out.push_str(rendered.trim());
        self.skip_until = self.skip_until.max(end);
        node.end_byte()
    }

    /// Advance `skip_until` past a beamer `<overlay-spec>` that begins at (or just
    /// after whitespace from) byte `pos`, so the spec doesn't leak as text. An overlay
    /// spec contains only `0-9 + - . , | <space>` between `<` and `>`
    /// (e.g. `<1->`, `<2-3>`, `<+->`); a `<` followed by anything else is left alone.
    fn skip_overlay_spec(&mut self, pos: usize) {
        let bytes = self.src.as_bytes();
        let mut i = pos;
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if bytes.get(i) != Some(&b'<') {
            return;
        }
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != b'>' {
            if !matches!(bytes[j], b'0'..=b'9' | b'+' | b'-' | b'.' | b',' | b'|' | b' ') {
                return; // not an overlay spec — leave it
            }
            j += 1;
        }
        if bytes.get(j) == Some(&b'>') {
            self.skip_until = self.skip_until.max(j + 1);
        }
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

// ─── Document class, path & package resolution ────────────────────────────────

/// Maps the well-known math wrap commands to their Typst `(left, right)`
/// delimiter pair. Used by the bare `command_name` branch of
/// `emit_node_inner` to recover the brace-less form (e.g. `_\mathcal{T}` —
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
/// A `\vspace`/`\hspace` length that maps 1:1 to a Typst dimension: an
/// optionally-signed decimal followed by a Typst-valid unit (cm/mm/in/pt/em/ex).
/// `None` for LaTeX length macros / `\dimexpr` / fills, which have no analog.
fn convertible_length(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s.contains('\\') {
        return None;
    }
    for u in ["cm", "mm", "in", "pt", "em"] {
        if let Some(num) = s.strip_suffix(u) {
            if num.trim().parse::<f64>().is_ok() {
                return Some(s.to_string());
            }
        }
    }
    // `ex` is NOT a Typst unit — emitting `1ex` makes Typst read `1e` as broken
    // scientific notation ("invalid floating point number"; corpus 2605.31603
    // `\vspace{1ex}`). Approximate 1ex ≈ 0.5em.
    if let Some(num) = s.strip_suffix("ex") {
        if let Ok(n) = num.trim().parse::<f64>() {
            return Some(format!("{}em", n * 0.5));
        }
    }
    None
}

fn consume_trailing_inline_space(src: &str, mut pos: usize) -> usize {
    let bytes = src.as_bytes();
    while bytes.get(pos) == Some(&b' ') || bytes.get(pos) == Some(&b'\t') {
        pos += 1;
    }
    pos
}
