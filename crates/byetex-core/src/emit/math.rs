//! Math-mode emission (primitives, environments, commands, layout/structures), extracted from emit.rs (pure code motion).

use std::fmt::Write;

use crate::ir::Node;

use super::{
    apply_text_accent, boundary, consume_braceless_arg, consume_trailing_brace_groups,
    environment_name, escape_paren_semicolons, escape_unbalanced_math_brackets, first_curly_group,
    flatten_text_children, lookup_math_symbol, math_font_decl_wrapper, needs_empty_base,
    needs_subscript_parens, split_math_rows, try_consume_math_arg, BracelessArg, Emitter,
    MATH_WORD_BOUNDARY,
};

impl<'a> Emitter<'a> {
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
    pub(in crate::emit) fn push_math_symbol(&mut self, typst: &str) {
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
    pub(in crate::emit) fn ensure_math_letter_boundary(&mut self, next: &str) {
        if boundary::starts_with_letter(next) && boundary::ends_with_letter(&self.out) {
            self.out.push(' ');
        }
    }

    /// Replace the in-progress math body (output bytes from `body_start` to
    /// the current end of `self.out`) with a copy where unbalanced `[` / `]`
    /// have been escaped as `\[` / `\]`. Balanced pairs are left as-is. See
    /// [`escape_unbalanced_math_brackets`] for the rationale.
    pub(in crate::emit) fn balance_math_brackets(&mut self, body_start: usize) {
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
    pub(in crate::emit) fn escape_math_semicolons(&mut self, body_start: usize) {
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
    pub(in crate::emit) fn collapse_math_spaces(&mut self, body_start: usize) {
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

    pub(in crate::emit) fn emit_inline_math(&mut self, node: Node<'_>) -> usize {
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

    pub(in crate::emit) fn emit_display_math(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_math_environment(&mut self, node: Node<'_>) -> usize {
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
            if self.label_first_use(first) {
                let _ = write!(self.out, " <{}>", first);
            }
            // A referenced equation MUST be numbered, or `@key` errors with
            // "cannot reference equation without numbering" (corpus 2605.31603:
            // a single-label `\begin{equation}` referenced by `\ref`). Multi-
            // label equations always need numbering too.
            if !rest.is_empty()
                || self
                    .referenced_labels
                    .contains(&crate::emit::escape::sanitize_label_key(first))
            {
                self.needs_equation_numbering = true;
            }
            for extra in rest {
                if self.label_first_use(extra) {
                    let _ = write!(
                        self.out,
                        "\n#hide[#figure(kind: \"equation\", supplement: [Eq.], $ \"\" $) <{}>]",
                        extra
                    );
                }
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
    pub(in crate::emit) fn emit_math_delimiter(&mut self, node: Node<'_>) -> usize {
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

    pub(in crate::emit) fn emit_math_children(&mut self, node: Node<'_>) {
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
    pub(in crate::emit) fn emit_math_node_slice(&mut self, body: &[Node<'_>]) {
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
                        if raw.chars().count() > 1 && raw.chars().all(|c| c.is_ascii_alphanumeric())
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
    pub(in crate::emit) fn emit_text_accent(&mut self, node: Node<'_>, accent: char) -> usize {
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

    pub(in crate::emit) fn emit_unknown_math_command(
        &mut self,
        node: Node<'_>,
        name: &str,
    ) -> usize {
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
    pub(in crate::emit) fn render_braceless_math_arg(&mut self, arg: BracelessArg) -> String {
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
    pub(in crate::emit) fn emit_math_frac(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_math_sqrt(&mut self, node: Node<'_>) -> usize {
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
    /// The starred `\operatorname*{X}` (limits form: `\operatorname*{argmin}_x`
    /// places the subscript *under* the operator) emits `op("X", limits: #true)`.
    pub(in crate::emit) fn emit_math_operatorname(&mut self, node: Node<'_>, starred: bool) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let raw = self
                .src
                .get(arg.start_byte() + 1..arg.end_byte() - 1)
                .unwrap_or("")
                .trim();
            // `op(...)` already renders its argument upright, so a redundant
            // `\mathrm{…}` / `\text{…}` wrapper would otherwise be quoted
            // verbatim and render as the literal text `\mathrm{argmin}`. Unwrap
            // it to its content (`\DeclareMathOperator*{\argmin}{\mathrm{argmin}}`
            // expands to exactly this shape).
            let inner = unwrap_upright_wrapper(raw);
            self.ensure_math_letter_boundary("op(");
            if starred {
                let _ = write!(self.out, "op(\"{}\", limits: #true)", inner);
            } else {
                let _ = write!(self.out, "op(\"{}\")", inner);
            }
        } else {
            self.warn_ambiguous_math(node, "\\operatorname (missing arg)");
        }
        node.end_byte()
    }

    // helper below is a free fn so it can be unit-reasoned without `self`.

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
    pub(in crate::emit) fn emit_math_text_call(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_math_notempty(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_math_then_branch(
        &mut self,
        node: Node<'_>,
        branch_idx: usize,
    ) -> usize {
        self.emit_chosen_curly_branch(node, branch_idx, /* skip_optional_brack = */ false)
    }

    /// `\smash{X}`, `\raisebox{offset}{X}`, `\scalebox{factor}{X}`,
    /// `\mathgroup{N}{X}` — render only the *content* curly_group,
    /// dropping the positioning args. `content_idx` is the 0-based
    /// index (0 for `\smash` which takes only the content, 1 for the
    /// two-arg helpers). `\smash` also has an optional `[t]`/`[b]`
    /// we silently drop.
    pub(in crate::emit) fn emit_math_layout_inner(
        &mut self,
        node: Node<'_>,
        content_idx: usize,
    ) -> usize {
        self.emit_chosen_curly_branch(node, content_idx, /* skip_optional_brack = */ true)
    }

    /// Common helper for `emit_math_then_branch` and
    /// `emit_math_layout_inner`. Collects AST-child curly_groups plus
    /// any source-byte sibling `{...}` groups, renders the
    /// `target_idx`-th one as math, and bumps `skip_until` past the
    /// rest. When `skip_optional_brack` is true, also skips a leading
    /// `[...]` (the `\smash[t]` shape) before the curly groups.
    pub(in crate::emit) fn emit_chosen_curly_branch(
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

    pub(in crate::emit) fn emit_math_extensible_arrow(
        &mut self,
        node: Node<'_>,
        name: &str,
    ) -> usize {
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

    /// `\phantom{X}` / `\hphantom{X}` / `\vphantom{X}` → `(#hide[$X$])`.
    /// `hide` is a content function so it needs the `#` escape inside math, and
    /// the argument must be a math content block `[$...$]`. The whole call is
    /// wrapped in parens to make it self-delimiting: a literal `[...]` that
    /// immediately follows the phantom (e.g. `\phantom{0}[\text{3.3}]` in a
    /// subscript, corpus 2605.31561) would otherwise be parsed by Typst as a
    /// chained second content argument to `hide` → "unexpected argument".
    /// `(group)[...]` is plain juxtaposition in math and parses cleanly.
    pub(in crate::emit) fn emit_math_phantom(&mut self, node: Node<'_>) -> usize {
        if let Some(arg) = first_curly_group(node) {
            let inner = self.render_math_group(arg);
            let _ = write!(self.out, "(#hide[${}$])", inner.trim());
            return node.end_byte();
        }
        node.end_byte()
    }

    /// Wrap the first curly_group argument in a Typst math function call:
    /// `\mathbf{X}` → `bold(X)`. Recursively renders the inner content in
    /// math mode so nested commands are translated.
    pub(in crate::emit) fn emit_math_wrap(
        &mut self,
        node: Node<'_>,
        left: &str,
        right: &str,
    ) -> usize {
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
    pub(in crate::emit) fn emit_math_binom(&mut self, node: Node<'_>) -> usize {
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

    /// The overset family: `\overset{script}{base}` / `\stackrel{script}{rel}` /
    /// `\accentset{accent}{base}` (top-set, `bottom=false`) and
    /// `\underset{script}{base}` (`bottom=true`). LaTeX puts the FIRST arg above
    /// (or below) the SECOND. Typst: `attach(base, t|b: script)`. Same two-arg
    /// extraction as `\binom` (braced or braceless).
    pub(in crate::emit) fn emit_math_attach(&mut self, node: Node<'_>, bottom: bool) -> usize {
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
            self.warn_ambiguous_math(node, "\\overset-family (missing args)");
            return node.end_byte();
        }
        if consumed_end > node.end_byte() {
            self.skip_until = self.skip_until.max(consumed_end);
        }
        let script = rendered[0].trim();
        let base = rendered[1].trim();
        let mark = if bottom { "b" } else { "t" };
        self.ensure_math_letter_boundary("attach(");
        // Typst `attach(base, t|b: script)` takes the script as ONE argument; a
        // top-level comma in the over-text (e.g. `\overset{x_0, x_1}{=}`,
        // corpus 2605.31063) would be read as a stray SECOND positional argument
        // → `error: unexpected argument`. A top-level `;` is the same failure
        // class (also an arg-list separator). Wrap such a script in `#box[$ … $]`,
        // which contains the breaking token yet adds NO visible delimiters and
        // renders the over-text as proper inline math. Scripts with neither token
        // keep the bare form so the common `\overset{x}{=}` case is byte-identical.
        if script.contains(',') || script.contains(';') {
            let _ = write!(self.out, "attach({base}, {mark}: #box[${script}$])");
        } else {
            let _ = write!(self.out, "attach({base}, {mark}: {script})");
        }
        consumed_end
    }

    /// Subscript/superscript: emit the marker, then the argument. Single-char
    /// args go through unwrapped; multi-char args wrap in parens.
    pub(in crate::emit) fn emit_subscript(&mut self, node: Node<'_>, marker: &str) -> usize {
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
    pub(in crate::emit) fn render_math_group(&mut self, group: Node<'_>) -> String {
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
    pub(in crate::emit) fn emit_matrix_env(&mut self, node: Node<'_>, _env: Option<&str>) -> usize {
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
    pub(in crate::emit) fn emit_cases_env(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_array_in_math(&mut self, node: Node<'_>) -> usize {
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
}

/// Unwrap a redundant upright-text wrapper around an `\operatorname` argument.
/// `\mathrm{argmin}` / `\text{argmin}` / `\mbox{argmin}` → `argmin`. `op(...)`
/// already renders upright, so the wrapper is redundant and, quoted verbatim,
/// would render as the literal `\mathrm{argmin}`. Only unwraps when the *entire*
/// argument is one wrapper (so `\mathrm{a}+b` is left untouched); loops to peel a
/// nested wrapper (`\mathrm{\text{x}}`). Anything else is returned unchanged.
fn unwrap_upright_wrapper(mut s: &str) -> &str {
    const WRAPPERS: &[&str] = &[
        "\\mathrm{",
        "\\text{",
        "\\mbox{",
        "\\textrm{",
        "\\textnormal{",
        "\\mathnormal{",
    ];
    'outer: loop {
        for w in WRAPPERS {
            if let Some(rest) = s.strip_prefix(w) {
                let bytes = rest.as_bytes();
                let mut depth = 1usize;
                let mut i = 0;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' => i += 1, // skip the escaped byte
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                // The wrapper covers the whole argument iff its closing brace is
                // the final character.
                if depth == 0 && i == bytes.len() - 1 {
                    s = rest[..i].trim();
                    continue 'outer;
                }
            }
        }
        return s;
    }
}
