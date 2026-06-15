//! Macro / \newcommand / \def / \newif harvesting + expansion + \input inclusion, extracted from emit.rs (pure code motion).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tree_sitter::Node;

use super::{
    brace_balanced_end, consume_braceless_arg, extract_label_ref_keys_and_end,
    extract_latex_include_path, lookup_math_symbol, range_of, resolve_input_path,
    resolve_package_path, sanitize_label_key, substitute_macro_args, Emitter, MAX_MACRO_DEPTH,
};
use crate::class_map::DocClass;
use crate::warnings::{Category, Severity, Warning};

impl<'a> Emitter<'a> {
    /// Handle `\newif` flag machinery: the `\newif\ifX` definition, the
    /// `\Xtrue`/`\Xfalse` setters, and `\ifX ... [\else ...] \fi` conditionals
    /// for flags defined via `\newif`. Returns `Some(resume_byte)` when `name`
    /// is newif machinery (emitting the taken branch and/or updating state),
    /// or `None` to fall through to normal command dispatch. TeX's builtin
    /// `\if`-family (`\ifx`, `\ifnum`, `\iftrue`, ...) is left untouched.
    pub(in crate::emit) fn try_newif_command(
        &mut self,
        node: Node<'_>,
        name: Option<&str>,
    ) -> Option<usize> {
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
    pub(in crate::emit) fn expand_user_macro(&mut self, node: Node<'_>, name: &str) -> usize {
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
    pub(in crate::emit) fn expand_local_package(&mut self, pkg: &str) {
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

    pub(in crate::emit) fn expand_latex_include(&mut self, node: Node<'_>) -> bool {
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
        // Forward the citation mode set by a natbib/biblatex option in the main
        // preamble so a `\bibliography{}` that lives in THIS \input'ed file
        // resolves the right style (the `.or()` merge-back below handles the
        // reverse case — the option in the include, `\bibliography` at top).
        // NOTE: `bib_will_render` is deliberately NOT forwarded here — this
        // sub-emitter does not clone `bibliography_keys`, so enabling `#cite`
        // forms without that validation set could emit `#cite(<undefined>)`
        // against the real bib and abort the compile. \input'ed citations stay
        // `@key` (Unit 3's deliberate, conservative design).
        sub.natbib_mode = self.natbib_mode;
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
        // A `#subpar.grid(...)` emitted inside the included file needs the
        // parent's `finish()` to add the `@preview/subpar` import, so flow the
        // flag back (mirrors the numbering flags above; corpus 2605.31063).
        self.used_subpar |= sub.used_subpar;
        // Merge the included file's metadata into the parent, parent
        // taking priority for fields it already owns.
        self.metadata.merge_from(&mut sub.metadata);
        if self.raw_authors.is_empty() {
            self.raw_authors.append(&mut sub.raw_authors);
        }
        if matches!(self.detected_class, DocClass::Unknown) {
            self.detected_class = std::mem::replace(&mut sub.detected_class, DocClass::Unknown);
        }
        // `\usepackage[numbers]{natbib}` can live in an `\input`ed preamble
        // file; flow the resolved citation mode back so the parent's
        // `\bibliography` (at document end) picks the right style. Parent wins
        // if it already set a mode.
        self.natbib_mode = self.natbib_mode.or(sub.natbib_mode);
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
}

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
pub(in crate::emit) fn harvest_wrapper_newcommands(
    root: Node<'_>,
    src: &str,
    out: &mut HashMap<String, MacroDef>,
) {
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
pub(in crate::emit) fn collect_curly_args_static(node: Node<'_>, src: &str) -> Vec<String> {
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
pub(in crate::emit) fn command_name_text_static(node: Node<'_>, src: &str) -> Option<String> {
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
pub(in crate::emit) fn extract_let(node: Node<'_>, src: &str) -> Option<(String, String)> {
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
pub(in crate::emit) fn let_alias_def(
    old_name: &str,
    table: &HashMap<String, MacroDef>,
) -> MacroDef {
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
pub(in crate::emit) fn read_newif_flag(src: &str, after_newif: usize) -> Option<(String, usize)> {
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

/// Extract `(name, nargs)` from an `environment_definition` node
/// (`\newenvironment{name}[nargs][default]{begindef}{enddef}` /
/// `\renewenvironment`). The grammar exposes the name as a `name:`-field
/// `curly_group_text` (fall back to the first `curly_group_text`/
/// `curly_group_word` child) and the argument count as an `argc:`-field
/// `brack_group_argc`. `nargs` is 0 when the env takes no arguments.
pub(in crate::emit) fn extract_environment_def(
    node: Node<'_>,
    src: &str,
) -> Option<(String, usize)> {
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
pub(in crate::emit) fn extract_theorem_def(node: Node<'_>, src: &str) -> Option<(String, String)> {
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

/// Extract a `\newcommand{\name}[N]{body}` definition from a `new_command_definition` node.
///
/// Returns `None` when the node cannot be parsed or has an optional-default argument.
///
/// Accepts both name forms tree-sitter-latex produces:
///
/// - `\newcommand{\name}{body}` — canonical curly-wrapped name (`curly_group_command_name`).
/// - `\newcommand\name{body}` — brace-less name form, common in arXiv preamble files.
pub(in crate::emit) fn extract_newcommand(node: Node<'_>, src: &str) -> Option<(String, MacroDef)> {
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
pub(in crate::emit) fn extract_newcommandx(
    node: Node<'_>,
    src: &str,
) -> Option<(String, MacroDef)> {
    extract_newcommandx_and_end(node, src).map(|(def, _end)| def)
}

/// Variant that also returns the source byte position immediately
/// after the closing `}` of the body. The emit-time dispatcher uses
/// this to bump `skip_until` so the sibling AST nodes carrying the
/// definition's bracket/body fragments don't leak into the output.
pub(in crate::emit) fn extract_newcommandx_and_end(
    node: Node<'_>,
    src: &str,
) -> Option<((String, MacroDef), usize)> {
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
pub(in crate::emit) fn split_xargspec_defaults(s: &str) -> Vec<String> {
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
pub(in crate::emit) fn new_command_token_kind(node: Node<'_>) -> Option<String> {
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
pub(in crate::emit) fn extract_declare_math_operator_from_newcmd(
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
pub(in crate::emit) fn extract_def_and_record(
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

/// Byte offset just past the first `\makeatother` *control word* at or after
/// `from`, or `None` if there is no closing `\makeatother`. Used to skip a
/// `\makeatletter` region wholesale.
///
/// Scans like [`find_conditional_bounds`]: `%` line comments are skipped (so a
/// `\makeatother` mentioned in a comment doesn't end the region early), and the
/// match is on the whole control word (so `\makeatotherwise` is not mistaken
/// for the closer).
pub(in crate::emit) fn find_makeatother_end(src: &str, from: usize) -> Option<usize> {
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
