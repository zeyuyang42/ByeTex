//! Figure / graphics emission, extracted from emit.rs (pure code motion).

use std::fmt::Write;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::{
    command_name_text, environment_name, extract_label_name, first_curly_group, nth_curly_group,
    range_of, sanitize_label_key, Emitter,
};
use crate::warnings::{Category, Severity, Warning};

impl<'a> Emitter<'a> {
    // ─── Figures, graphics & tabular ──────────────────────────────────────────

    pub(in crate::emit) fn emit_graphics_include(&mut self, node: Node<'_>) -> usize {
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

    /// Render one captioned block as a Typst `figure(...)` string — no leading
    /// `#`, no trailing `<label>`. Used both for the single-figure path and for
    /// each panel of a `subpar.grid`. `kind` is `Some("table")` / `Some("image")`
    /// or `None` (image default); `caption_text` is the already-rendered caption
    /// body (without brackets) or `None`.
    fn emit_figure_inner(
        &self,
        body_str: &str,
        kind: Option<&str>,
        caption_text: Option<&str>,
    ) -> String {
        let mut s = String::new();
        s.push_str("figure(\n  ");
        s.push_str(body_str);
        if let Some(k) = kind {
            let _ = write!(s, ",\n  kind: {}", k);
        }
        if let Some(text) = caption_text {
            let _ = write!(s, ",\n  caption: [{}]", text);
        }
        s.push_str(",\n)");
        s
    }

    pub(in crate::emit) fn emit_figure(&mut self, node: Node<'_>) -> usize {
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

        // Resolve kind: an explicit `\captionof{type}` wins; else a tabular
        // body implies `kind: table`; else image default.
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
        // Resolve caption text: `\caption{cap}` → 1st group; `\captionof{t}{cap}` → 2nd.
        let caption_node = caption.or(captionof);
        let caption_text = caption_node.and_then(|c| {
            let arg = if c.kind() == "generic_command" {
                nth_curly_group(c, 1)
            } else {
                first_curly_group(c)
            };
            arg.map(|a| self.render_curly_group_content(a))
        });
        let inner = self.emit_figure_inner(&body_str, kind, caption_text.as_deref());
        self.ensure_paragraph_break();
        self.out.push('#');
        self.out.push_str(&inner);
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
}

/// Extract the path argument from a `graphics_include` (`\includegraphics{X}`).
/// Parse the inner argument of `\graphicspath{{dir1/}{dir2/}}` — i.e. the text
/// between the OUTER braces — into a list of search directories. Each dir is a
/// `{...}`-wrapped group; brace nesting is honored. Trailing slashes are kept
/// as written (joined with the image name later). A malformed/empty arg yields
/// no dirs. Example: `{figures/main/}{figures/tasks/}` → ["figures/main/",
/// "figures/tasks/"].
pub(in crate::emit) fn parse_graphicspath_dirs(inner: &str) -> Vec<String> {
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

/// Greedily pack sub-block width fractions into rows whose cumulative width is
/// <= ~1.05; return the column count = the widest row's block count. A block
/// with no width (`None`) counts as a full-width row break unless every block
/// is `None`, in which case the answer is 1 (stacked). Never returns 0.
pub fn columns_for_widths(widths: &[Option<f32>]) -> usize {
    if widths.is_empty() {
        return 1;
    }
    if widths.iter().all(|w| w.is_none()) {
        return 1;
    }
    let mut max_row = 1usize;
    let mut row_count = 0usize;
    let mut row_width = 0.0f32;
    for w in widths {
        match w {
            Some(frac) => {
                if row_count > 0 && row_width + frac > 1.05 {
                    row_count = 0;
                    row_width = 0.0;
                }
                row_count += 1;
                row_width += frac;
                max_row = max_row.max(row_count);
            }
            None => {
                row_count = 0;
                row_width = 0.0;
            }
        }
    }
    max_row.max(1)
}

/// Extract a width fraction (e.g. `0.41` from `{0.41\textwidth}` /
/// `{0.5\linewidth}` / `{0.5\columnwidth}`) from a `minipage` / `subfigure` /
/// `subtable` environment node. Returns `None` when no fraction-of-text-width
/// argument is present.
pub(in crate::emit) fn width_fraction_of(node: Node<'_>, src: &str) -> Option<f32> {
    let text = &src[node.start_byte()..node.end_byte()];
    for unit in ["\\textwidth", "\\linewidth", "\\columnwidth"] {
        if let Some(pos) = text.find(unit) {
            let bytes = text.as_bytes();
            let mut start = pos;
            while start > 0 {
                let c = bytes[start - 1];
                if c.is_ascii_digit() || c == b'.' {
                    start -= 1;
                } else {
                    break;
                }
            }
            if start < pos {
                if let Ok(v) = text[start..pos].parse::<f32>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

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
