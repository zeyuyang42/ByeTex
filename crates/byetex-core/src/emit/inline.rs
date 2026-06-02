//! Inline wrapping, font-switch groups, raw/listing & textcolor emission, extracted from emit.rs (pure code motion).

use std::fmt::Write;

use tree_sitter::Node;

use super::{consume_trailing_inline_space, first_curly_group, Emitter};

impl<'a> Emitter<'a> {
    /// Emit a typographic-logo command (`\LaTeX`, `\TeX`, etc.) as plain text.
    /// LaTeX users normally write `\LaTeX{}` so the empty group blocks LaTeX
    /// from swallowing the following space. tree-sitter parses that `{}` as a
    /// `curly_group` child of the command — when we see it, the caller's
    /// intent was to preserve the following space, so we do.
    pub(in crate::emit) fn emit_logo(&mut self, node: Node<'_>, name: &str) -> usize {
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
    pub(in crate::emit) fn emit_hologo(&mut self, node: Node<'_>) -> usize {
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

    /// Find the first `curly_group` child of `node` and render its inner
    /// content wrapped between `left` and `right`. Falls back to dropping
    /// the command if no argument is present.
    pub(in crate::emit) fn emit_inline_wrap(&mut self, node: Node<'_>, left: &str, right: &str) -> usize {
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
    pub(in crate::emit) fn emit_font_switch_group(
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
    pub(in crate::emit) fn emit_inline_raw(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_listing_environment(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_inline_unwrap(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_textcolor(&mut self, node: Node<'_>) -> usize {
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
    pub(in crate::emit) fn emit_math_textcolor(&mut self, node: Node<'_>) -> usize {
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
}
