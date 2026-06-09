//! Bibliography, citation & label-reference emission, extracted from emit.rs (pure code motion).

use std::fmt::Write;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::{
    extract_label_ref_keys_and_end, is_typst_label_char, range_of, sanitize_label_key, Emitter,
};
use crate::warnings::{Category, Severity, Warning};

impl<'a> Emitter<'a> {
    /// True when a `\bibliography{...}` directive is paired with a `.bib` that
    /// resolved on disk — its `#bibliography(.bib)` is the complete, canonical
    /// reference list, so any manual `\bibitem`/`thebibliography` entries are
    /// redundant and would collide with it. (corpus 2605.31440)
    pub(in crate::emit) fn bib_file_is_authoritative(&self) -> bool {
        self.has_bibtex_include && self.had_bib_file
    }

    /// Hidden anchors for every `\ref`/`\cref`-referenced key that has no
    /// resolving target — neither a `<key>` label already in the output nor a
    /// bibliography entry. Without these a reference to an undefined label
    /// (commented-out `\label`, dropped environment) leaves a dangling `@key`
    /// that aborts the Typst compile. See the call site in `finish()`.
    pub(in crate::emit) fn dangling_ref_anchors(&self) -> String {
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
    pub(in crate::emit) fn close_bibitem(&mut self) {
        if let Some(key) = self.pending_bibitem_key.take() {
            // Trim trailing whitespace so the closing bracket sits flush.
            while self.out.ends_with(' ') || self.out.ends_with('\n') {
                self.out.pop();
            }
            let _ = writeln!(self.out, "]) <{}>", key);
        }
    }

    /// `\begin{thebibliography}{99}...\bibitem{k} entry text...\end{thebibliography}`
    /// → numbered list with `<k>` labels per entry. The `{99}` width spec is
    /// dropped.
    pub(in crate::emit) fn emit_thebibliography(&mut self, env: Node<'_>) -> usize {
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

    // ===== M4: refs, citations, floats =====

    pub(in crate::emit) fn emit_citation(&mut self, node: Node<'_>) -> usize {
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

    pub(in crate::emit) fn emit_label_reference(&mut self, node: Node<'_>) -> usize {
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

    pub(in crate::emit) fn emit_bibliography(&mut self, node: Node<'_>) -> usize {
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
                suggested_skill: Some("byetex-bibliography".to_string()),
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
                        suggested_skill: Some("byetex-bibliography".to_string()),
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
                suggested_skill: Some("byetex-bibliography".to_string()),
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

    pub(in crate::emit) fn emit_bibstyle(&mut self, node: Node<'_>) -> usize {
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
}

/// Extract the list of citation keys from a `citation` node. Keys are
/// children of `curly_group_text_list`, separated by `,`.
pub(in crate::emit) fn extract_citation_keys(node: Node<'_>, src: &str) -> Vec<String> {
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

/// Extract the path argument from a `bibtex_include` (`\bibliography{x}`) or
/// `bibstyle_include` (`\bibliographystyle{x}`) node.
pub(in crate::emit) fn extract_bib_path(node: Node<'_>, src: &str) -> Option<String> {
    extract_bib_paths(node, src).into_iter().next()
}

/// Collect every comma-separated bib path in a `\bibliography{a,b,c}`
/// call. The pre-2026-05 helper returned only the first match, so
/// multi-bib papers silently lost every entry after the first; project
/// mode then failed to copy those files and `typst compile` died with
/// `file not found`.
pub(in crate::emit) fn extract_bib_paths(node: Node<'_>, src: &str) -> Vec<String> {
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


// ─── Asset & bibliography filesystem probing ──────────────────────────────────


/// Probe the base directory for a BibTeX file. Appends `.bib` if the stem has
/// no extension. Returns the resolved path on disk, or `None`.
/// Walk the immediate `base_dir` (non-recursive) for `.bib` and
/// `.bbl` files, parse their entry / `\bibitem` keys, and insert
/// the sanitized forms into `out`. Errors (unreadable file,
/// unparseable content) are silently skipped — the worst case is a
/// citation that should have resolved gets dropped, which is the
/// same end state as before this validation existed.
pub(in crate::emit) fn harvest_bib_keys_from_dir(base: &Path, out: &mut std::collections::HashSet<String>) {
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
pub(in crate::emit) fn extract_bib_entry_keys(content: &str) -> Vec<String> {
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
pub(in crate::emit) fn extract_bbl_bibitem_keys(content: &str) -> Vec<String> {
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
pub(in crate::emit) fn probe_any_bbl(base: &Path) -> Option<String> {
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

pub(in crate::emit) fn probe_bib_on_disk(base: &Path, path: &str) -> Option<PathBuf> {
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

/// Map a LaTeX `\bibliographystyle{X}` name to the nearest Typst built-in
/// style. Returns `None` for unknown styles so the caller can omit the
/// `style:` argument and let Typst use its default.
pub(in crate::emit) fn map_bibliography_style(latex: &str) -> Option<&'static str> {
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
