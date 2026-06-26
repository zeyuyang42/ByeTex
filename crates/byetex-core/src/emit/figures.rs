//! Figure / graphics emission, extracted from emit.rs (pure code motion).

use std::fmt::Write;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::{
    command_name_text, environment_name, extract_label_name, first_curly_group, nth_curly_group,
    range_of, sanitize_label_key, Emitter,
};
use crate::warnings::{Category, Severity, Warning};

/// One captioned sub-block discovered in a Pattern-B float.
struct CaptionBlock {
    inner: String,         // rendered `figure(...)` (no `#`, no `<label>`)
    label: Option<String>, // picked referenced label, if any
    width: Option<f32>,    // width fraction for column packing
}

impl<'a> Emitter<'a> {
    // ─── Figures, graphics & tabular ──────────────────────────────────────────

    /// Resolve an `\includegraphics`-style image path against the source tree
    /// and, if found, register it as a copyable asset. Returns the Typst-side
    /// path string to feed `image("…")`, or `None` when the file can't be
    /// located (caller degrades gracefully). Shared by the cover page so cover
    /// images get the same probe-then-copy plumbing as body figures.
    pub(in crate::emit) fn resolve_image_asset(&mut self, path: &str) -> Option<String> {
        let mut resolved_path = path.to_string();
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
        let source_path = probe_dirs
            .iter()
            .find_map(|d| probe_image_on_disk(d, path))?;
        // Fill in the extension if the LaTeX path omitted it.
        if Path::new(path).extension().is_none() {
            if let Some(name) = source_path.file_name().and_then(|n| n.to_str()) {
                let dir = Path::new(path)
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
        // Only register the asset when there's a base_dir for the project layer
        // to relocate into (mirrors `emit_graphics_include`).
        if self.base_dir.is_some() {
            self.asset_refs.push(crate::AssetRef {
                kind: crate::AssetKind::Image,
                typst_path: resolved_path.clone(),
                source_path,
            });
        }
        Some(resolved_path)
    }

    /// Emit a GENERIC thesis/report cover page from `\coverimage` + the title
    /// metadata: a near-full-bleed cover image with a title banner (title /
    /// subtitle / subject / author) overlaid at the top. Approximate — the
    /// bespoke per-class logo and exact banner colours/fonts are not replicated.
    /// Degrades gracefully: a missing/absent image yields the banner alone.
    pub(in crate::emit) fn emit_cover_page(&mut self) {
        // Materialize authors now so the banner can name them (the same
        // raw_authors → metadata.authors step flush_title_block runs later).
        self.materialize_authors();

        let image_path = self
            .metadata
            .cover_image
            .clone()
            .and_then(|p| self.resolve_image_asset(&p));

        let title = self.metadata.title.take();
        let subtitle = self.metadata.subtitle.take();
        let subject = self.metadata.subject.take();
        let author_line = if self.metadata.authors.is_empty() {
            None
        } else {
            Some(
                self.metadata
                    .authors
                    .iter()
                    .map(|a| {
                        super::escape_text_for_typst_content(a.name.as_content())
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        };

        // Nothing to show at all → no cover (don't emit an empty page).
        if image_path.is_none()
            && title.is_none()
            && subtitle.is_none()
            && subject.is_none()
            && author_line.is_none()
        {
            return;
        }

        self.ensure_paragraph_break();

        // Build the banner body (title / subtitle / subject / author).
        let mut banner = String::new();
        if let Some(title) = &title {
            let _ = write!(
                banner,
                "    #text(size: 2.2em, fill: rgb(\"#4884d6\"))[{}]\\\n",
                title.as_content()
            );
        }
        if let Some(subtitle) = &subtitle {
            let _ = write!(
                banner,
                "    #text(size: 1.2em, fill: white)[{}]\\\n",
                subtitle.as_content()
            );
        }
        if let Some(subject) = &subject {
            let _ = write!(
                banner,
                "    #text(size: 1em, fill: white)[{}]\\\n",
                subject.as_content()
            );
        }
        if let Some(author) = &author_line {
            let _ = write!(
                banner,
                "    #text(size: 1em, fill: white)[{}]\n",
                author
            );
        }

        // Full-page cover: a `#page` with no margins. When a cover image is
        // present it fills the page; the banner sits in a dark block placed
        // near the top via a `#place`. Without the image the banner alone
        // renders on the page.
        self.out.push_str("#page(margin: 0pt)[\n");
        if let Some(path) = &image_path {
            let _ = write!(
                self.out,
                "  #image(\"{}\", width: 100%, height: 100%, fit: \"cover\")\n",
                path
            );
            self.out.push_str("  #place(top + left, dx: 0pt, dy: 18%)[\n");
            self.out.push_str("    #block(width: 100%, inset: (x: 8%, y: 18pt), fill: rgb(0, 0, 0, 200))[\n");
            self.out.push_str(&banner);
            self.out.push_str("    ]\n  ]\n");
        } else {
            // No image: just the banner block on the page (graceful fallback).
            self.out.push_str("  #v(18%)\n");
            self.out.push_str("  #block(width: 100%, inset: (x: 8%, y: 18pt), fill: rgb(0, 0, 0, 200))[\n");
            self.out.push_str(&banner);
            self.out.push_str("  ]\n");
        }
        self.out.push_str("]\n\n");

        self.cover_emitted = true;
    }

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
        if let Some(source_path) = probe_dirs
            .iter()
            .find_map(|d| probe_image_on_disk(d, &path))
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
    /// One subfigure/subtable panel → (figure_string, picked_label, width_fraction).
    /// The figure string has no leading `#` and no trailing `<label>`.
    fn render_subfigure_panel(
        &mut self,
        node: Node<'_>,
    ) -> Option<(String, Option<String>, Option<f32>)> {
        let mut graphics: Vec<Node<'_>> = Vec::new();
        let mut caption: Option<Node<'_>> = None;
        let mut nested_tabular: Option<Node<'_>> = None;
        let mut labels: Vec<String> = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                match child.kind() {
                    // A subfigure can stack SEVERAL `\includegraphics` (the paper
                    // puts multiple image rows in one panel) — collect them all,
                    // not just the first, else real panels are silently dropped.
                    "graphics_include" => graphics.push(child),
                    "caption" if caption.is_none() => caption = Some(child),
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
        // Body: image(s) win, else nested tabular, else nothing → drop the panel.
        let (body, is_table) = if !graphics.is_empty() {
            // The DFS above collects children in reverse — restore document order
            // so stacked panels read top-to-bottom as written.
            graphics.sort_by_key(|g| g.start_byte());
            let imgs: Vec<String> = graphics
                .iter()
                .map(|g| {
                    self.with_sub_buffer(|e| {
                        e.emit_graphics_include(*g);
                    })
                    .trim()
                    .to_string()
                })
                .filter(|s| !s.is_empty())
                .collect();
            // One image stays a bare `image(...)`; several stack vertically (a
            // single Typst expr — bare `image(a) image(b)` is a parse error in a
            // `figure(...)` positional slot).
            let body = if imgs.len() == 1 {
                imgs.into_iter().next().unwrap()
            } else {
                format!("stack(dir: ttb, spacing: 0.5em, {})", imgs.join(", "))
            };
            (body, false)
        } else if let Some(t) = nested_tabular {
            let s = self
                .with_sub_buffer(|e| {
                    e.emit_tabular(t);
                })
                .trim()
                .to_string();
            (
                s.strip_prefix('#').map(|s| s.to_string()).unwrap_or(s),
                true,
            )
        } else {
            return None;
        };
        let kind = if is_table { Some("table") } else { None };
        let caption_text =
            caption.and_then(|c| first_curly_group(c).map(|a| self.render_curly_group_content(a)));
        let inner = self.emit_figure_inner(body.trim(), kind, caption_text.as_deref());
        let label = self.pick_label_to_attach(&labels);
        let width = width_fraction_of(node, self.src);
        Some((inner, label, width))
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
            if k == "algorithm" {
                // A custom kind needs an explicit supplement; this also gives the
                // float its own "Algorithm N" counter.
                s.push_str(",\n  kind: \"algorithm\", supplement: [Algorithm]");
            } else {
                let _ = write!(s, ",\n  kind: {}", k);
            }
        }
        if let Some(text) = caption_text {
            let _ = write!(s, ",\n  caption: [{}]", text);
        }
        s.push_str(",\n)");
        s
    }

    /// Render an `algorithmic`/`algorithmicx`/`algpseudocode` body as structured
    /// pseudocode: one numbered line per statement, control keywords bold, nesting
    /// shown by indentation, framed by top/bottom rules. Replaces the old behavior
    /// that collapsed every `\State`/`\For`/… into one `align(left)[…]` prose run
    /// with the keywords dropped.
    fn render_algorithmic_body(&mut self, env: Node<'_>) -> String {
        struct LineSpec<'n> {
            depth: i32,
            display: &'static str,
            suffix: &'static str,
            nodes: Vec<Node<'n>>,
        }
        // Pass 1 — parse into line specs (no rendering, so no &mut self borrow).
        let mut specs: Vec<LineSpec<'_>> = Vec::new();
        let mut depth: i32 = 0;
        let mut cur: Option<LineSpec<'_>> = None;
        let mut seen_opt = false;
        let mut cursor = env.walk();
        for child in env.children(&mut cursor) {
            match child.kind() {
                "begin" | "end" => continue,
                // The `[1]` line-numbering option directly after `\begin`.
                "brack_group" if !seen_opt && cur.is_none() => {
                    seen_opt = true;
                    continue;
                }
                "generic_command" => {
                    let name = command_name_text(child, self.src).unwrap_or_default();
                    let key = name.trim_start_matches('\\').to_ascii_lowercase();
                    if let Some(kw) = classify_algo_keyword(&key) {
                        if let Some(c) = cur.take() {
                            specs.push(c);
                        }
                        let line_depth = (depth + kw.line_delta).max(0);
                        depth = (depth + kw.next_delta).max(0);
                        let mut nodes = Vec::new();
                        // `\For{cond}` / `\If{cond}` carry the condition as a curly
                        // group child; its inner content is the line content.
                        if let Some(cg) = first_curly_group(child) {
                            let mut cc = cg.walk();
                            for gc in cg.children(&mut cc) {
                                if !matches!(gc.kind(), "{" | "}") {
                                    nodes.push(gc);
                                }
                            }
                        }
                        cur = Some(LineSpec {
                            depth: line_depth,
                            display: kw.display,
                            suffix: kw.suffix,
                            nodes,
                        });
                        continue;
                    }
                    if let Some(c) = cur.as_mut() {
                        c.nodes.push(child);
                    }
                }
                _ => {
                    if let Some(c) = cur.as_mut() {
                        c.nodes.push(child);
                    }
                }
            }
        }
        if let Some(c) = cur.take() {
            specs.push(c);
        }

        // Pass 2 — render each line.
        let mut body = String::new();
        for (i, spec) in specs.iter().enumerate() {
            let content = self
                .with_sub_buffer(|e| {
                    if let Some(first) = spec.nodes.first() {
                        let mut last = first.start_byte();
                        for n in &spec.nodes {
                            e.safe_copy(last, n.start_byte());
                            last = e.emit_node(*n);
                        }
                    }
                })
                .trim()
                .to_string();
            if i > 0 {
                body.push_str(" \\\n  ");
            }
            let _ = write!(body, "#box(width: 1.7em)[#text(size: 0.85em)[{}.]]", i + 1);
            for _ in 0..spec.depth {
                body.push_str("#h(1.2em)");
            }
            if !spec.display.is_empty() {
                let _ = write!(body, "#strong[{}] ", spec.display);
            }
            body.push_str(&content);
            if !spec.suffix.is_empty() {
                let _ = write!(body, " #strong[{}]", spec.suffix);
            }
        }
        // Frame with top/bottom rules (the booktabs-style algorithm box).
        format!(
            "block(width: 100%, stroke: (top: 0.8pt, bottom: 0.8pt), inset: (y: 4pt))[#align(left)[{body}]]"
        )
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
        // An `algorithm` float wraps one or more `\begin{algorithmic}` blocks; they
        // are the float's BODY. Captured here so they render instead of the empty
        // `(figure)` placeholder (dogfood F7). A Vec so a float with several blocks
        // keeps them all (the bare-algorithmic path emits every env).
        let mut algorithmic_bodies: Vec<Node<'_>> = Vec::new();

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
                            Some("subfigure")
                                | Some("subcaptionblock")
                                | Some("subfloat")
                                | Some("subtable")
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
                        // The pseudocode body of an `algorithm` float — render each
                        // whole (don't descend, or the steps scatter as loose text).
                        if matches!(
                            env.as_deref(),
                            Some("algorithmic")
                                | Some("algorithmicx")
                                | Some("algpseudocode")
                                | Some("algpseudocodex")
                        ) {
                            algorithmic_bodies.push(child);
                            continue;
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

        // Pattern A: explicit subfigure/subtable panels → subpar.grid.
        if subfigures.len() >= 2
            || (subfigures.len() == 1 && (caption.is_some() || captionof.is_some()))
        {
            let panels: Vec<(String, Option<String>, Option<f32>)> = subfigures
                .iter()
                .filter_map(|sf| self.render_subfigure_panel(*sf))
                .collect();
            if panels.len() >= 2 {
                // Sub-labels belong to the panels now; remove them from the
                // outer `labels` set so they are not also hidden-anchored.
                let panel_labels: std::collections::HashSet<String> =
                    panels.iter().filter_map(|(_, l, _)| l.clone()).collect();
                let parent_labels: Vec<String> = labels
                    .iter()
                    .filter(|l| !panel_labels.contains(*l))
                    .cloned()
                    .collect();
                let widths: Vec<Option<f32>> = panels.iter().map(|(_, _, w)| *w).collect();
                let cols = columns_for_widths(&widths);
                let parent_kind = if environment_name(node, self.src).as_deref() == Some("table") {
                    Some("table")
                } else {
                    None
                };
                let parent_caption = caption.or(captionof).and_then(|c| {
                    let arg = if c.kind() == "generic_command" {
                        nth_curly_group(c, 1)
                    } else {
                        first_curly_group(c)
                    };
                    arg.map(|a| self.render_curly_group_content(a))
                });
                self.emit_subpar_grid(
                    &panels,
                    cols,
                    parent_kind,
                    parent_caption.as_deref(),
                    &parent_labels,
                );
                return node.end_byte();
            }
        }

        // Pattern B: no subfigure envs, but >=2 top-level caption sources →
        // segment into captioned sub-blocks and emit a subpar.grid.
        {
            let blocks = self.collect_caption_blocks(node);
            if blocks.len() >= 2 {
                let widths: Vec<Option<f32>> = blocks.iter().map(|b| b.width).collect();
                let cols = columns_for_widths(&widths);
                let parent_kind = if environment_name(node, self.src).as_deref() == Some("table") {
                    Some("table")
                } else {
                    None
                };
                let panels: Vec<(String, Option<String>, Option<f32>)> = blocks
                    .into_iter()
                    .map(|b| (b.inner, b.label, b.width))
                    .collect();
                // No parent caption/label in Pattern B (every caption belongs to
                // a sub-block); pass an empty parent label set.
                self.emit_subpar_grid(&panels, cols, parent_kind, None, &[]);
                return node.end_byte();
            }
        }

        // Whether the float's body is a tabular (vs an image): when it is, the
        // emitted `#figure` must carry `kind: table` so Typst captions/refs read
        // "Table N" rather than the default "Figure N".
        // A lone subfigure (no main caption) collapses to just its panel as the
        // figure body; the >=2 case is handled by the Pattern-A subpar.grid
        // branch above (which returns early).
        let lone_panel: Option<String> = subfigures
            .iter()
            .filter_map(|sf| self.render_subfigure_panel(*sf))
            .map(|(inner, _label, _w)| inner)
            .next();

        let mut body_is_table = false;
        let mut body_is_algorithm = false;
        let body_str = if let Some(panel) = lone_panel {
            panel
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
        } else if !algorithmic_bodies.is_empty() {
            // An `algorithm` float's body is its `algorithmic` pseudocode —
            // structured numbered lines with bold control keywords and nesting.
            body_is_algorithm = true;
            let mut steps = String::new();
            for alg in &algorithmic_bodies {
                let block = self.render_algorithmic_body(*alg);
                if !steps.is_empty() {
                    steps.push('\n');
                }
                steps.push_str(&block);
            }
            steps
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
        if kind.is_none() && body_is_algorithm {
            kind = Some("algorithm");
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
        // A starred float (`figure*` / `table*`) spans BOTH columns in a
        // two-column layout; Typst needs an explicit parent-scope floating place
        // for that (a plain `#figure` stays inside one column). In one-column mode
        // the star is a no-op, so only wrap when the document is two-column.
        let spanning = environment_name(node, self.src)
            .as_deref()
            .map(|n| n.ends_with('*'))
            .unwrap_or(false)
            && self.layout.is_two_column(&self.detected_class);
        if spanning {
            self.out
                .push_str("#place(top, scope: \"parent\", float: true)[\n  ");
        }
        self.out.push('#');
        self.out.push_str(&inner);
        // Attach the referenced alias (or the first label); then give every
        // OTHER referenced label its own hidden, referenceable anchor — a
        // single float (subfigures, or two `\captionof`s) can be `\ref`'d
        // under several labels, but Typst allows only one label per element.
        let primary = self.pick_label_to_attach(&labels);
        if let Some(l) = &primary {
            if self.label_first_use(l) {
                let _ = write!(self.out, " <{}>", l);
            }
        }
        if spanning {
            self.out.push_str("\n]");
        }
        for l in &labels {
            if Some(l) != primary.as_ref()
                && self.referenced_labels.contains(&sanitize_label_key(l))
                && self.label_first_use(l)
            {
                let _ = write!(self.out, "\n#hide[#figure([]) <{}>]", l);
            }
        }
        node.end_byte()
    }

    /// Emit `#subpar.grid(...)` from rendered panels `(inner_figure, label, _)`.
    fn emit_subpar_grid(
        &mut self,
        panels: &[(String, Option<String>, Option<f32>)],
        cols: usize,
        parent_kind: Option<&str>,
        parent_caption: Option<&str>,
        parent_labels: &[String],
    ) {
        self.used_subpar = true;
        self.ensure_paragraph_break();
        self.out.push_str("#subpar.grid(\n");
        for (inner, label, _w) in panels {
            self.out.push_str("  ");
            self.out.push_str(inner);
            if let Some(l) = label {
                if self.label_first_use(l) {
                    let _ = write!(self.out, ", <{}>", l);
                }
            }
            self.out.push_str(",\n");
        }
        let cols_str = std::iter::repeat_n("1fr", cols)
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(self.out, "  columns: ({}),", cols_str);
        if let Some(k) = parent_kind {
            let _ = writeln!(self.out, "  kind: {},", k);
        }
        if let Some(c) = parent_caption {
            let _ = writeln!(self.out, "  caption: [{}],", c);
        }
        let primary = self.pick_label_to_attach(parent_labels);
        if let Some(l) = &primary {
            if self.label_first_use(l) {
                let _ = writeln!(self.out, "  label: <{}>,", l);
            }
        }
        self.out.push(')');
        // Any extra referenced parent labels get hidden anchors (existing pattern).
        for l in parent_labels {
            if Some(l) != primary.as_ref()
                && self.referenced_labels.contains(&sanitize_label_key(l))
                && self.label_first_use(l)
            {
                let _ = write!(self.out, "\n#hide[#figure([]) <{}>]", l);
            }
        }
    }

    /// Collect captioned sub-blocks of a Pattern-B float. Prefers `minipage`
    /// grouping (each minipage that contains a caption is one block); falls back
    /// to linear segmentation where each `\caption`/`\captionof` closes the run of
    /// preceding sibling content. Returns empty / single when the float is not a
    /// multi-caption float (caller then uses the single-figure path).
    fn collect_caption_blocks(&mut self, node: Node<'_>) -> Vec<CaptionBlock> {
        // Gather the float's direct children in source order.
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();

        // Path 1: minipage grouping.
        let minipages: Vec<Node<'_>> = children
            .iter()
            .copied()
            .filter(|c| {
                c.kind() == "generic_environment"
                    && environment_name(*c, self.src).as_deref() == Some("minipage")
            })
            .collect();
        let captioned_minipages: Vec<Node<'_>> = minipages
            .iter()
            .copied()
            .filter(|mp| self.subtree_has_caption(*mp))
            .collect();
        if captioned_minipages.len() >= 2 {
            return captioned_minipages
                .iter()
                .filter_map(|mp| self.render_caption_block(*mp))
                .collect();
        }

        // Path 2: linear segmentation by caption command.
        // A caption is a `caption` node or a `\captionof` generic_command.
        let is_caption = |c: &Node<'_>| -> bool {
            c.kind() == "caption"
                || (c.kind() == "generic_command"
                    && command_name_text(*c, self.src).as_deref() == Some("\\captionof"))
        };
        let caption_count = children.iter().filter(|c| is_caption(c)).count();
        if caption_count < 2 {
            return Vec::new();
        }
        let mut blocks: Vec<CaptionBlock> = Vec::new();
        let mut run: Vec<Node<'_>> = Vec::new();
        let mut run_labels: Vec<String> = Vec::new();
        for c in &children {
            // The environment's own `\begin{…}` / `\end{…}` markers are AST
            // children of the float node; never treat them as block content
            // (else the linear body leaks a raw `\begin{figure}`).
            if matches!(c.kind(), "begin" | "end") {
                continue;
            }
            if c.kind() == "label_definition" {
                if let Some(k) = extract_label_name(*c, self.src) {
                    run_labels.push(k);
                }
                continue;
            }
            if is_caption(c) {
                // Close the current run as a block captioned by `c`.
                if let Some(b) = self.render_linear_block(&run, *c, &run_labels) {
                    blocks.push(b);
                }
                run.clear();
                run_labels.clear();
            } else {
                run.push(*c);
            }
        }
        blocks
    }

    /// True if `node`'s subtree contains a `caption` node or a `\captionof`.
    fn subtree_has_caption(&self, node: Node<'_>) -> bool {
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                if child.kind() == "caption"
                    || (child.kind() == "generic_command"
                        && command_name_text(child, self.src).as_deref() == Some("\\captionof"))
                {
                    return true;
                }
                stack.push(child);
            }
        }
        false
    }

    /// Render a minipage (or any captioned container) as one CaptionBlock: its body
    /// (image or tabular), its own caption + label, and its width fraction.
    fn render_caption_block(&mut self, node: Node<'_>) -> Option<CaptionBlock> {
        let mut graphics: Option<Node<'_>> = None;
        let mut caption: Option<Node<'_>> = None;
        let mut captionof: Option<Node<'_>> = None;
        let mut nested_tabular: Option<Node<'_>> = None;
        let mut labels: Vec<String> = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                match child.kind() {
                    "graphics_include" if graphics.is_none() => graphics = Some(child),
                    "caption" if caption.is_none() => caption = Some(child),
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
        let (body, is_table) = if let Some(g) = graphics {
            (
                self.with_sub_buffer(|e| {
                    e.emit_graphics_include(g);
                }),
                false,
            )
        } else if let Some(t) = nested_tabular {
            let s = self
                .with_sub_buffer(|e| {
                    e.emit_tabular(t);
                })
                .trim()
                .to_string();
            (
                s.strip_prefix('#').map(|s| s.to_string()).unwrap_or(s),
                true,
            )
        } else {
            return None;
        };
        let cap_node = caption.or(captionof);
        let kind = self
            .captionof_kind(captionof)
            .or(if is_table { Some("table") } else { None });
        let caption_text = cap_node.and_then(|c| {
            let arg = if c.kind() == "generic_command" {
                nth_curly_group(c, 1)
            } else {
                first_curly_group(c)
            };
            arg.map(|a| self.render_curly_group_content(a))
        });
        let inner = self.emit_figure_inner(body.trim(), kind, caption_text.as_deref());
        Some(CaptionBlock {
            inner,
            label: self.pick_label_to_attach(&labels),
            width: width_fraction_of(node, self.src),
        })
    }

    /// Render a linear run of content nodes + a closing caption node as a block.
    fn render_linear_block(
        &mut self,
        run: &[Node<'_>],
        caption: Node<'_>,
        labels: &[String],
    ) -> Option<CaptionBlock> {
        // Render the run's content into a sub-buffer (images, tabulars, text).
        let body = self.with_sub_buffer(|e| {
            for n in run {
                e.emit_node(*n);
            }
        });
        let body = body.trim().to_string();
        let body = body
            .strip_prefix('#')
            .map(|s| s.to_string())
            .unwrap_or(body);
        if body.is_empty() {
            return None;
        }
        let is_table = body.starts_with("table(");
        let kind = self
            .captionof_kind(if caption.kind() == "generic_command" {
                Some(caption)
            } else {
                None
            })
            .or(if is_table { Some("table") } else { None });
        let arg = if caption.kind() == "generic_command" {
            nth_curly_group(caption, 1)
        } else {
            first_curly_group(caption)
        };
        let caption_text = arg.map(|a| self.render_curly_group_content(a));
        let inner = self.emit_figure_inner(&body, kind, caption_text.as_deref());
        Some(CaptionBlock {
            inner,
            label: self.pick_label_to_attach(labels),
            width: None, // linear/stacked → no width → single column
        })
    }

    /// `\captionof{type}{...}` → `Some("table")` / `Some("image")` / `None`.
    fn captionof_kind(&mut self, captionof: Option<Node<'_>>) -> Option<&'static str> {
        let c = captionof?;
        let type_arg = nth_curly_group(c, 0)?;
        let ty = self.render_curly_group_content(type_arg);
        match ty.trim() {
            "table" => Some("table"),
            "figure" => Some("image"),
            _ => None,
        }
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

/// A classified algorithmic keyword: its bold display text, an optional trailing
/// keyword (`do`/`then`, rendered after the condition), and how it shifts the
/// indentation of its own line (`line_delta`) and of the lines that follow
/// (`next_delta`).
struct AlgoKw {
    display: &'static str,
    suffix: &'static str,
    line_delta: i32,
    next_delta: i32,
}

/// Classify an algorithmic command (lowercased, no backslash) for both
/// algpseudocode (`\State`, `\For`) and the older algorithmic (`\STATE`, `\FOR`)
/// packages. Returns `None` for non-control commands (rendered as line content).
fn classify_algo_keyword(key: &str) -> Option<AlgoKw> {
    let kw = |display, suffix, line_delta, next_delta| {
        Some(AlgoKw {
            display,
            suffix,
            line_delta,
            next_delta,
        })
    };
    match key {
        "state" | "statex" => kw("", "", 0, 0),
        "require" => kw("Require:", "", 0, 0),
        "ensure" => kw("Ensure:", "", 0, 0),
        "input" => kw("Input:", "", 0, 0),
        "output" => kw("Output:", "", 0, 0),
        "return" => kw("return", "", 0, 0),
        "print" => kw("print", "", 0, 0),
        "for" | "forall" => kw("for", "do", 0, 1),
        "endfor" => kw("end for", "", -1, -1),
        "while" => kw("while", "do", 0, 1),
        "endwhile" => kw("end while", "", -1, -1),
        "if" => kw("if", "then", 0, 1),
        "elsif" | "elseif" => kw("else if", "then", -1, 0),
        "else" => kw("else", "", -1, 0),
        "endif" => kw("end if", "", -1, -1),
        "loop" => kw("loop", "", 0, 1),
        "endloop" => kw("end loop", "", -1, -1),
        "repeat" => kw("repeat", "", 0, 1),
        "until" => kw("until", "", -1, -1),
        "procedure" => kw("procedure", "", 0, 1),
        "endprocedure" => kw("end procedure", "", -1, -1),
        "function" => kw("function", "", 0, 1),
        "endfunction" => kw("end function", "", -1, -1),
        "comment" => kw("▷", "", 0, 0),
        _ => None,
    }
}

fn extract_graphics_path(node: Node<'_>, src: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "curly_group_path" {
            let mut sub = child.walk();
            for grandchild in child.children(&mut sub) {
                if grandchild.kind() == "path" {
                    // The `path` node glues on any whitespace/newline between `{`
                    // and the filename when `\includegraphics[…]{` puts the path
                    // on the next line — trim it, else the asset resolver looks
                    // for a file named "\n img/…" and drops a real figure
                    // (2605.22507 appendix lost several MNIST grids).
                    return Some(
                        src[grandchild.start_byte()..grandchild.end_byte()]
                            .trim()
                            .to_string(),
                    );
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
    // Container-relative LaTeX lengths → a percentage. Height keywords
    // (`\textheight` etc.) are mapped the same way as the width ones: Typst
    // reads the `%` against the containing block, an acceptable approximation
    // (corpus 2605.31597: `height=0.4\textheight` previously leaked the `\`).
    for kw in [
        "\\textwidth",
        "\\linewidth",
        "\\columnwidth",
        "\\textheight",
        "\\paperheight",
        "\\paperwidth",
        "\\columnheight",
    ] {
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
    // Any remaining LaTeX macro can't be expressed as a Typst length and would
    // leak a `\` into code context — drop the dimension (Typst `auto`) instead.
    if v.contains('\\') {
        return "auto".to_string();
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
