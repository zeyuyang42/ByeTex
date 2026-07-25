//! Tabular emission + column-spec parsing, extracted from emit.rs (pure code motion).

use std::fmt::Write;

use crate::ir::Node;

use super::{
    environment_name, escape_text_cell, extract_latex_include_path, resolve_input_path,
    skip_balanced_braces, split_math_rows, Emitter,
};

/// Sentinels the rule commands drop into the RENDERED tabular body so
/// `emit_tabular` can place each rule at the row the emitter's own split puts it
/// in. Control characters that cannot occur in real LaTeX content, and that
/// `split_math_rows` ignores (it keys on `\` + whitespace).
///
/// Why sentinels rather than a second scan of the source: the row a rule
/// follows has to agree with the row list the emission loop iterates, and that
/// list comes from splitting the *rendered* body. Any independent `\\` count
/// over the raw source disagrees with it — `\tabularnewline` is a `\\` synonym
/// the count misses, `\\` inside `\makecell{a \\ b}` is a line break within one
/// cell rather than a row separator, and `\newline` in a cell makes emitted rows
/// outnumber source breaks. 97 tabulars across 24 corpus papers are miscounted
/// that way. Carrying the position through the render is exact by construction.
pub(in crate::emit) const RULE_HEAVY: char = '\u{11}'; // \toprule / \bottomrule
pub(in crate::emit) const RULE_LIGHT: char = '\u{12}'; // \midrule
pub(in crate::emit) const RULE_HLINE: char = '\u{13}'; // \hline
pub(in crate::emit) const RULE_PARTIAL: char = '\u{14}'; // \cmidrule{a-b}, wraps the range

/// The rules declared at one boundary between rows (or at the table's edges).
#[derive(Default, Clone)]
struct RuleEdge {
    /// Weight of the full-width rule here, if any. Repeats collapse — Typst has
    /// no stacked-rule primitive and `\hline\hline` is visually negligible.
    full: Option<RuleWeight>,
    /// `\cmidrule` spans, as Typst `(start, end)` column boundaries.
    partials: Vec<(usize, usize)>,
}

impl RuleEdge {
    fn merge(&mut self, other: RuleEdge) {
        if let Some(w) = other.full {
            // A heavier rule at the same boundary wins (`\hline\bottomrule`).
            self.full = Some(match self.full {
                Some(cur) if cur >= w => cur,
                _ => w,
            });
        }
        self.partials.extend(other.partials);
    }

    fn is_empty(&self) -> bool {
        self.full.is_none() && self.partials.is_empty()
    }
}

/// Stroke weight of a full-width rule. Ordered so `merge` can keep the heavier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RuleWeight {
    /// `\hline` — no weight of its own; resolved against its position later.
    Plain,
    /// `\midrule`.
    Light,
    /// `\toprule` / `\bottomrule`.
    Heavy,
}

/// Write one boundary's rules: the full-width rule first, then any `\cmidrule`
/// spans (booktabs draws them in that order).
fn emit_rule_edge(
    out: &mut String,
    edge: &RuleEdge,
    idx: usize,
    cols: usize,
    stroke_of: &dyn Fn(usize, RuleWeight) -> &'static str,
) {
    if edge.is_empty() {
        return;
    }
    if let Some(w) = edge.full {
        let _ = writeln!(out, "  table.hline(stroke: {}),", stroke_of(idx, w));
    }
    for &(start, end) in &edge.partials {
        let end = end.min(cols);
        let start = start.min(end.saturating_sub(1));
        let _ = writeln!(
            out,
            "  table.hline(start: {start}, end: {end}, stroke: 0.05em),"
        );
    }
}

/// Split `seg` into its text content and the rules declared in it.
fn strip_rule_sentinels(seg: &str) -> (String, RuleEdge) {
    let mut text = String::with_capacity(seg.len());
    let mut edge = RuleEdge::default();
    let mut chars = seg.chars();
    while let Some(c) = chars.next() {
        match c {
            RULE_HEAVY => edge.merge_full(RuleWeight::Heavy),
            RULE_LIGHT => edge.merge_full(RuleWeight::Light),
            RULE_HLINE => edge.merge_full(RuleWeight::Plain),
            RULE_PARTIAL => {
                // `\u{14}a-b\u{14}` — take up to the closing sentinel.
                let range: String = chars.by_ref().take_while(|c| *c != RULE_PARTIAL).collect();
                if let Some((a, b)) = range.split_once('-') {
                    if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
                        // `{a-b}` is 1-indexed inclusive; Typst wants
                        // end-exclusive column boundaries.
                        edge.partials.push((a.saturating_sub(1), b));
                    }
                }
            }
            _ => text.push(c),
        }
    }
    (text, edge)
}

impl RuleEdge {
    fn merge_full(&mut self, w: RuleWeight) {
        self.full = Some(match self.full {
            Some(cur) if cur >= w => cur,
            _ => w,
        });
    }
}

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
        let tree = crate::ir::parse_and_lower(&source);
        // Find the first tabular-family environment in the included file.
        let mut stack = vec![tree.root_node()];
        let mut tabular: Option<Node<'_>> = None;
        while let Some(n) = stack.pop() {
            if n.kind() == "generic_environment"
                && matches!(
                    environment_name(n, &source).as_deref(),
                    Some("tabular")
                        | Some("tabular*")
                        | Some("tabularx")
                        | Some("tabulary")
                        | Some("array")
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
        let mut sub =
            Emitter::with_includes(&source, self.source_name, self.base_dir.clone(), visited);
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
        let (count, aligns, widths) = parse_column_spec(&col_spec);

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
        let saved_in_table_cell = self.in_table_cell;
        self.in_minipage = false;
        // Cell content is escaped per-cell afterwards; emit `\textbf`/`\emph` in
        // function form so the markers survive (see `in_table_cell`).
        self.in_table_cell = true;
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
        self.in_table_cell = saved_in_table_cell;

        // Strip any literal `\hline` left in the buffer. It is NOT turned into a
        // sentinel: this runs over the RENDERED body, where a `\hline` can also
        // be verbatim cell content (`\verb|\hline|` renders as `#raw("\hline")`),
        // and synthesizing a rule from that draws a line LaTeX never does. The
        // parsed command emits its own sentinel while rendering, which is the
        // only occurrence that carries a real position.
        let cleaned = body_str.replace("\\hline", "");
        // Rows are separated by `\` followed by whitespace (the LaTeX
        // `\\` row break, which our `\\` emitter writes as a single
        // backslash). Use `split_math_rows` (Bug #31's helper) so we
        // don't accidentally split inside escape sequences like
        // `\$`, `\_`, `\*` that legitimately appear in cell content
        // — e.g. `\multicolumn{2}{c}{\textbf{\$10.23}}` which used
        // to fragment at every `\$`/`\*` and corrupt the table.
        //
        // THIS split is what positions the rules: the rule commands dropped
        // sentinels into `body_str` as they were emitted, so a sentinel's row is
        // whatever row this split puts it in — no second scan of the source, and
        // therefore no way for the two to disagree about `\tabularnewline`, a
        // `\\` inside `\makecell{…}`, or a `\newline` that adds a row.
        // `edge[i]` holds the rules sitting at the boundary BEFORE row `i`;
        // `edge[rows.len()]` is the bottom edge. LaTeX always writes a rule at a
        // boundary (`… \\ \midrule next-row`), so attributing every sentinel to
        // the boundary preceding the row it was found in is exact.
        let mut rows: Vec<String> = Vec::new();
        let mut edges: Vec<RuleEdge> = vec![RuleEdge::default()];
        for seg in split_math_rows(&cleaned) {
            let (text, edge) = strip_rule_sentinels(seg);
            edges.last_mut().expect("edge always present").merge(edge);
            if text.trim().is_empty() {
                continue; // a rule-only (or empty) segment adds no row
            }
            rows.push(text);
            edges.push(RuleEdge::default());
        }
        // Build per-row cell lists so we can track rowspan/colspan occupancy.
        // (`split_math_rows` already consumed any `\\[len]` vertical-space arg.)
        let rows_2d: Vec<Vec<String>> = rows
            .iter()
            .map(|row| split_cells_on_unescaped_amp(row))
            .collect();

        // Booktabs styling. Typst's default table draws a full grid (a line
        // around every cell); academic papers (≈75% of the corpus) instead use
        // booktabs — no vertical lines, three horizontal rules (top / after the
        // header / bottom). And a LaTeX tabular with NO rule commands draws no
        // lines at all. So: `stroke: none` always (kills the spurious grid),
        // and add booktabs rules only when the source actually ruled the table.
        // `\hline` carries no weight of its own; booktabs' do. Resolve `Plain`
        // against position — heavy at the table's outer edges, light inside —
        // which is the shape an `\hline` table produced before rules were
        // placed from source.
        let last_edge = edges.len().saturating_sub(1);
        let first_ruled = edges.iter().position(|e| e.full.is_some());
        let last_ruled = edges.iter().rposition(|e| e.full.is_some());
        let stroke_of = |idx: usize, w: RuleWeight| -> &'static str {
            match w {
                RuleWeight::Heavy => "0.08em",
                RuleWeight::Light => "0.05em",
                RuleWeight::Plain => {
                    if Some(idx) == first_ruled || Some(idx) == last_ruled || idx == last_edge {
                        "0.08em"
                    } else {
                        "0.05em"
                    }
                }
            }
        };

        // The column SPEC is only an upper bound: LaTeX papers commonly
        // over-declare it (`{llrrrrrrrrrrrrrr}` = 16) while the rows — via
        // `\multicolumn{N}` groups — only occupy fewer (11). Typst rejects a
        // colspan/rowspan layout that doesn't fill the declared `columns:`
        // ("cell's colspan would cause it to exceed the available column(s)",
        // corpus 2605.31561). Clamp to the actual max row occupancy.
        let cols = effective_column_count(&rows_2d, count);
        let align_str = aligns
            .iter()
            .take(cols)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        // `columns:` is a bare count when every column is auto-width; a tuple of
        // widths (`(3cm, auto, …)`) when any `p{…}`/`m`/`b`/`w` column set one.
        let columns_expr = if widths.iter().take(cols).any(|w| w != "auto") {
            let mut ws: Vec<String> = widths.iter().take(cols).cloned().collect();
            while ws.len() < cols {
                ws.push("auto".to_string());
            }
            if ws.len() == 1 {
                // `(5cm)` is just grouping in Typst; a 1-track array needs `(5cm,)`.
                format!("({},)", ws[0])
            } else {
                format!("({})", ws.join(", "))
            }
        } else {
            cols.to_string()
        };

        self.ensure_paragraph_break();
        let _ = write!(
            self.out,
            "#table(\n  columns: {},\n  align: ({}),\n  stroke: none,\n",
            columns_expr, align_str
        );
        // Rules at the top edge (before any row).
        if let Some(e) = edges.first() {
            emit_rule_edge(&mut self.out, e, 0, cols, &stroke_of);
        }

        // rowspan_cols[c] = number of additional rows for which column c is
        // already occupied by a rowspan cell from a previous row.  When we
        // encounter a rowspan=N cell at column c we set rowspan_cols[c] = N-1.
        // Each subsequent visit to that column decrements the counter.
        let mut rowspan_cols = vec![0usize; cols];

        for (row_idx, row_cells) in rows_2d.iter().enumerate() {
            let mut row_output: Vec<String> = Vec::new();
            let mut src = row_cells.iter();
            let mut col = 0usize;

            while col < cols {
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
                        for slot in rowspan_cols.iter_mut().take((col + cs).min(cols)).skip(col) {
                            *slot = rs - 1;
                        }
                    }
                    row_output.push(cell.clone());
                    col += cs;
                } else {
                    break;
                }
            }

            // NOTE: a row that collapses to nothing (every column covered by an
            // active rowspan — the standard `\multirow` placeholder row) still
            // OWNS the boundary after it, so the rule emission below must run
            // even when no cells are written. Skipping the whole iteration
            // dropped the `\bottomrule` of any table whose last row is a
            // placeholder.
            let has_cells = !row_output.is_empty();
            // LaTeX pads a short row (fewer cells than the column count) to the
            // full width implicitly; Typst's `table()` auto-placement does NOT —
            // it flows cells continuously, so an un-padded short row shifts every
            // following cell left and can push a later `\multicolumn{N}` past the
            // grid edge ("colspan would exceed available columns", corpus
            // 2605.31203). Fill the remainder with empty cells, skipping columns
            // still held by an active rowspan (mirrors the placement loop above).
            if has_cells {
                while col < cols {
                    if rowspan_cols[col] > 0 {
                        rowspan_cols[col] -= 1;
                    } else {
                        row_output.push(String::new());
                    }
                    col += 1;
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
            }
            // Rules declared at the boundary AFTER this row — emitted whether or
            // not the row itself produced cells.
            if let Some(e) = edges.get(row_idx + 1) {
                emit_rule_edge(&mut self.out, e, row_idx + 1, cols, &stroke_of);
            }
        }
        self.out.push(')');
        node.end_byte()
    }
}

/// The actual number of columns the rows occupy — the max, over all rows, of
/// the columns a row fills (summing `\multicolumn` colspans and counting
/// `\multirow` carry-over). Used to clamp an over-declared column spec to what
/// the content really uses (corpus 2605.31561). `spec_count` is the upper bound
/// (rows are never placed past it); the placement here mirrors `emit_tabular`'s
/// emission loop so the clamped count matches what is emitted. Never returns 0.
/// Split a rendered table row into cells on the column separator `&`, skipping
/// an escaped `\&` (a literal ampersand rendered by the `\&` text command, e.g.
/// `\multicolumn{3}{c}{Document \& Diagram}` — corpus 2605.31604). Cells are
/// trimmed. A `\\` escape (literal backslash) before `&` still separates.
fn split_cells_on_unescaped_amp(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut prev_backslash = false;
    for ch in row.chars() {
        if ch == '&' && !prev_backslash {
            cells.push(cur.trim().to_string());
            cur.clear();
            prev_backslash = false;
            continue;
        }
        cur.push(ch);
        prev_backslash = ch == '\\' && !prev_backslash;
    }
    cells.push(cur.trim().to_string());
    cells
}

fn effective_column_count(rows_2d: &[Vec<String>], spec_count: usize) -> usize {
    if spec_count == 0 {
        return 0;
    }
    let mut max_occ = 0usize;
    let mut rowspan = vec![0usize; spec_count];
    for row_cells in rows_2d {
        let mut src = row_cells.iter();
        let mut col = 0usize;
        while col < spec_count {
            if rowspan[col] > 0 {
                src.next();
                rowspan[col] -= 1;
                col += 1;
            } else if let Some(cell) = src.next() {
                let (cs, rs) = table_cell_span(cell);
                if rs > 1 {
                    for slot in rowspan
                        .iter_mut()
                        .take((col + cs).min(spec_count))
                        .skip(col)
                    {
                        *slot = rs - 1;
                    }
                }
                col += cs;
            } else {
                break;
            }
        }
        max_occ = max_occ.max(col);
    }
    max_occ.max(1)
}

/// Convert a LaTeX column width (`p{…}` arg) to a Typst column width, or `auto`
/// when it can't be represented: a fraction of `\textwidth`/`\linewidth`/… →
/// percent; a bare `<n><unit>` (cm/mm/in/pt/em/ex) → itself; anything else (e.g.
/// `\dimexpr`) → `auto`.
fn normalize_table_width(raw: &str) -> String {
    let s = raw.trim();
    for kw in ["\\textwidth", "\\linewidth", "\\columnwidth", "\\hsize"] {
        if let Some(pos) = s.find(kw) {
            let factor = s[..pos].trim().trim_end_matches('*').trim();
            let f: f64 = if factor.is_empty() {
                1.0
            } else {
                match factor.parse() {
                    Ok(v) => v,
                    Err(_) => return "auto".to_string(),
                }
            };
            return format!("{:.0}%", f * 100.0);
        }
    }
    if ["cm", "mm", "in", "pt", "em", "ex"]
        .iter()
        .any(|u| s.ends_with(u))
        && s[..s.len() - 2].trim().parse::<f64>().is_ok()
    {
        return s.to_string();
    }
    "auto".to_string()
}

/// Parse a LaTeX tabular column spec like `lcr` or `p{3cm}c` into a count, a
/// vector of Typst alignment names (`"left"`/`"center"`/`"right"`), and a
/// parallel vector of Typst column widths (`"auto"` or e.g. `"3cm"`/`"30%"`).
fn parse_column_spec(spec: &str) -> (usize, Vec<String>, Vec<String>) {
    let mut aligns: Vec<String> = Vec::new();
    let mut widths: Vec<String> = Vec::new();
    // An array-package `>{…}` decorator applies to the column that FOLLOWS it; an
    // alignment verb in it (`\centering` / `\raggedleft` / `\raggedright`)
    // overrides that column's default alignment. Held here until the next column.
    let mut pending_align: Option<String> = None;
    let bytes = spec.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] as char {
            'l' | 'L' => {
                aligns.push(resolve_col_align("left", &mut pending_align));
                widths.push("auto".to_string());
                i += 1;
            }
            'c' | 'C' => {
                aligns.push(resolve_col_align("center", &mut pending_align));
                widths.push("auto".to_string());
                i += 1;
            }
            'r' | 'R' => {
                aligns.push(resolve_col_align("right", &mut pending_align));
                widths.push("auto".to_string());
                i += 1;
            }
            // Paragraph/width columns carry a fixed `{width}` (p/m/b → width
            // first; w/W → `{align}{width}`, width second).
            'p' | 'm' | 'b' | 'w' | 'W' => {
                let is_w = matches!(bytes[i], b'w' | b'W');
                aligns.push(resolve_col_align("left", &mut pending_align));
                i += 1;
                let mut width = "auto".to_string();
                if bytes.get(i) == Some(&b'{') {
                    let close = skip_balanced_braces(spec, i);
                    let first = spec[i + 1..close.saturating_sub(1)].to_string();
                    i = close;
                    if is_w && bytes.get(i) == Some(&b'{') {
                        let close2 = skip_balanced_braces(spec, i);
                        width = normalize_table_width(&spec[i + 1..close2.saturating_sub(1)]);
                        i = close2;
                    } else {
                        width = normalize_table_width(&first);
                    }
                }
                widths.push(width);
            }
            // tabularx X column — count as left-aligned, width auto.
            'X' => {
                aligns.push(resolve_col_align("left", &mut pending_align));
                widths.push("auto".to_string());
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
                    let (_, inner_aligns, inner_widths) = parse_column_spec(inner);
                    let first_idx = aligns.len();
                    for _ in 0..count {
                        aligns.extend(inner_aligns.iter().cloned());
                        widths.extend(inner_widths.iter().cloned());
                    }
                    // A `>{…}` decorator just before `*{N}{…}` applies to the
                    // first expanded column only.
                    if let Some(a) = pending_align.take() {
                        if let Some(slot) = aligns.get_mut(first_idx) {
                            *slot = a;
                        }
                    }
                }
            }
            // >{...}: column format decorator (array package) — applies to the
            // NEXT column. Recover an alignment verb from it (the dominant corpus
            // use is `>{\centering\arraybackslash}` / `>{\raggedleft…}` on a
            // p-column); other decorator material is still dropped.
            '>' => {
                i += 1;
                if bytes.get(i) == Some(&b'{') {
                    let close = skip_balanced_braces(spec, i);
                    let content = &spec[i + 1..close.saturating_sub(1)];
                    if let Some(a) = decorator_align(content) {
                        pending_align = Some(a);
                    }
                    i = close;
                }
            }
            // @{...} and !{...}: inter-column material, not data columns.
            // <{...}: trailing column decorator (no alignment effect).
            '@' | '!' | '<' => {
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
    (aligns.len(), aligns, widths)
}

/// A column's alignment is its spec default unless a preceding `>{…}` decorator
/// supplied an override (consumed here so it applies to one column only).
fn resolve_col_align(default: &str, pending: &mut Option<String>) -> String {
    pending.take().unwrap_or_else(|| default.to_string())
}

/// Map an array `>{…}` decorator body to a Typst column alignment, if it carries
/// an alignment verb. Handles plain LaTeX (`\centering`, `\raggedright`,
/// `\raggedleft`) and ragged2e's capitalised variants; case-insensitive.
fn decorator_align(content: &str) -> Option<String> {
    let lc = content.to_lowercase();
    if lc.contains("centering") {
        Some("center".to_string())
    } else if lc.contains("raggedleft") {
        Some("right".to_string())
    } else if lc.contains("raggedright") {
        Some("left".to_string())
    } else {
        None
    }
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
