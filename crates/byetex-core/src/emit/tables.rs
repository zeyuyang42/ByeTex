//! Tabular emission + column-spec parsing, extracted from emit.rs (pure code motion).

use std::fmt::Write;

use tree_sitter::Node;

use super::{
    environment_name, escape_text_cell, extract_latex_include_path, resolve_input_path,
    skip_balanced_braces, split_math_rows, Emitter,
};

impl<'a> Emitter<'a> {
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
    pub(in crate::emit) fn labels_from_include(&self, inc: Node<'_>) -> Vec<String> {
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
    pub(in crate::emit) fn tabular_from_include(&mut self, inc: Node<'_>) -> Option<String> {
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
    pub(in crate::emit) fn emit_tabular(&mut self, node: Node<'_>) -> usize {
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
            .map(|row| {
                row.split('&')
                    .map(|c| strip_cell_braces(c.trim()))
                    .collect()
            })
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
}

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

/// Strip one layer of matched outer braces from a rendered table cell. A LaTeX
/// cell wrapped in `{...}` is just grouping (the braces are invisible); without
/// this the literal `{`/`}` leak into the Typst cell — e.g.
/// `{0.131\small{\textpm 0.034}}` rendered `[{0.131± 0.034}]` (corpus
/// 2605.22507). Only strips when the FIRST `{` matches the LAST char `}` with
/// no earlier return to depth 0 (so `{a}{b}` and `{a} {b}` are left intact).
/// `\{`/`\}` escapes don't change depth.
fn strip_cell_braces(cell: &str) -> String {
    let bytes = cell.as_bytes();
    if bytes.first() != Some(&b'{') || bytes.last() != Some(&b'}') {
        return cell.to_string();
    }
    let mut depth = 0i32;
    let mut chars = cell.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '\\' => {
                chars.next(); // skip the escaped char
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    // The opening brace's match must be the final char to strip.
                    return if i + ch.len_utf8() == cell.len() {
                        cell[1..i].trim().to_string()
                    } else {
                        cell.to_string()
                    };
                }
            }
            _ => {}
        }
    }
    cell.to_string()
}
