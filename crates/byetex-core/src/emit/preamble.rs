//! Preamble building, title-block flush, author materialization & package/class extraction, extracted from emit.rs (pure code motion).

use std::fmt::Write;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::{escape_text_for_typst_content, Emitter};

impl<'a> Emitter<'a> {
    /// Emit the rich native title block from captured \title/\author/\date/
    /// \begin{abstract}/\keywords. Used for Unknown/Lncs/SvMult classes (no
    /// Typst Universe template binding).
    pub(in crate::emit) fn flush_title_block(&mut self) {
        self.materialize_authors();
        if self.metadata.is_title_block_empty() {
            return;
        }
        self.ensure_paragraph_break();

        let profile = crate::style_profile::StyleProfile::for_class(&self.detected_class);
        let title = self.metadata.title.take();
        // Horizontal title rules (NeurIPS/ICML) wrap ONLY an actually-emitted
        // title — a title-less block must not draw orphan rules.
        let with_rules = title.is_some()
            && (profile.title_rule_above.is_some() || profile.title_rule_below.is_some());

        if with_rules {
            if let Some((stroke, gap_below)) = profile.title_rule_above {
                let _ = writeln!(self.out, "#line(length: 100%, stroke: {stroke})");
                let _ = writeln!(self.out, "#v({gap_below})");
            }
        }

        // ── Centred title + author block ──────────────────────────────────
        self.out.push_str("#align(center)[\n");
        if let Some(title) = title {
            let content = title.as_content();
            let body = if profile.title_smallcaps {
                format!("#smallcaps[{content}]")
            } else {
                content.to_string()
            };
            if profile.title_bold {
                let _ = writeln!(
                    self.out,
                    "  #text(size: {}, weight: \"bold\")[{}]",
                    profile.title_size, body
                );
            } else {
                let _ = writeln!(self.out, "  #text(size: {})[{}]", profile.title_size, body);
            }
        }

        if with_rules {
            // Close the title-only block and draw the bottom rule; authors
            // land in a second centred block below it (matching the LaTeX
            // \@maketitle order: the rules wrap only the title).
            self.out.push_str("]\n");
            if let Some((gap_above, stroke, gap_below)) = profile.title_rule_below {
                let _ = writeln!(self.out, "#v({gap_above})");
                let _ = writeln!(self.out, "#line(length: 100%, stroke: {stroke})");
                let _ = writeln!(self.out, "#v({gap_below})");
            }
            if !self.metadata.authors.is_empty() || self.metadata.date.is_some() {
                self.out.push_str("#align(center)[\n");
            }
        }
        let tail_open = !with_rules
            || !self.metadata.authors.is_empty()
            || self.metadata.date.is_some();

        if !self.metadata.authors.is_empty() {
            // The title→author gap. In the rules layout (NeurIPS/ICML) the
            // bottom rule already emitted its own gap (e.g. NeurIPS's 0.09in),
            // and authors open a fresh centred block — adding 0.6em on top
            // would double the space, so suppress it there.
            if !with_rules {
                self.out.push_str("  #v(0.6em)\n");
            }
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
        if tail_open {
            self.out.push_str("]\n");
        }

        // ── Abstract block ────────────────────────────────────────────────
        // Two-column conference classes (ICML/IEEE/ACM) place the abstract
        // INSIDE the `#columns(2)[…]` body — `finish()` emits it there. Every
        // other case renders it here, full-width, in the class-faithful style.
        let defer_abstract =
            profile.abstract_in_columns && self.layout.is_two_column(&self.detected_class);
        if !defer_abstract {
            if let Some(abstract_) = self.metadata.r#abstract.take() {
                if !abstract_.is_empty() {
                    let block =
                        self.render_abstract_block(profile.abstract_style, abstract_.as_content());
                    self.out.push_str(&block);
                }
            }
        }

        // ── Keywords line ─────────────────────────────────────────────────
        // When the abstract is deferred into the columns, the keywords
        // (e.g. IEEE "Index Terms") follow it there — leave them in metadata.
        if !defer_abstract && !self.metadata.keywords.is_empty() {
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

    /// Render the class-faithful abstract block. `content` is the
    /// already-converted Typst abstract body. Each shape mirrors the heading
    /// size/weight/small-caps + run-in-vs-centered structure of the source
    /// class file (see [`crate::style_profile::AbstractStyle`]).
    pub(in crate::emit) fn render_abstract_block(
        &self,
        style: crate::style_profile::AbstractStyle,
        content: &str,
    ) -> String {
        use crate::style_profile::AbstractStyle;
        match style {
            // Byte-identical to the historical hardcoded block.
            AbstractStyle::Neutral => format!(
                "#v(1em)\n\
                 #align(center)[#text(weight: \"bold\")[Abstract]]\n\
                 #v(0.4em)\n\
                 #pad(x: 2em)[\n  {content}]\n\
                 #v(0.6em)\n"
            ),
            // article.cls wraps the whole abstract env in \small (0.9em).
            AbstractStyle::ArticleCentered => format!(
                "#v(1em)\n\
                 #text(size: 0.9em)[\n\
                 #align(center)[#text(weight: \"bold\")[Abstract]]\n\
                 #v(0.4em)\n\
                 #pad(x: 2.5em)[\n  {content}]\n\
                 ]\n\
                 #v(0.6em)\n"
            ),
            // NeurIPS/ICML/ACM: \large\bf centered heading + quote body.
            AbstractStyle::ConferenceHeading { smallcaps: false } => format!(
                "#v(0.075in)\n\
                 #align(center)[#text(size: 1.2em, weight: \"bold\")[Abstract]]\n\
                 #v(0.5em)\n\
                 #pad(x: 1em)[\n  {content}]\n\
                 #v(1em)\n"
            ),
            // ICLR: \large\sc — small caps, regular weight.
            AbstractStyle::ConferenceHeading { smallcaps: true } => format!(
                "#v(0.075in)\n\
                 #align(center)[#text(size: 1.2em)[#smallcaps[Abstract]]]\n\
                 #v(0.5em)\n\
                 #pad(x: 1em)[\n  {content}]\n\
                 #v(1em)\n"
            ),
            // IEEE conference: \small\bfseries\textit{Abstract}--- run-in.
            AbstractStyle::RunInBoldItalic => format!(
                "#text(size: 0.9em, weight: \"bold\")[#emph[Abstract]---{content}]\n\
                 #v(0.5em)\n"
            ),
            // LNCS: \small body, bold run-in "Abstract." with 1cm margins.
            AbstractStyle::RunInBold => format!(
                "#pad(x: 1cm)[#text(size: 0.9em)[*Abstract.* {content}]]\n\
                 #v(0.5em)\n"
            ),
        }
    }

    /// Convert the raw `\author{...}` strings collected during the AST
    /// walk into structured `Author` records by running the per-class
    /// parser from `class_map.rs`. Idempotent — calling it twice is a
    /// no-op.
    pub(in crate::emit) fn materialize_authors(&mut self) {
        if self.raw_authors.is_empty() {
            return;
        }
        let raw = std::mem::take(&mut self.raw_authors);
        let mut parsed = crate::class_map::parse_authors(&raw, &self.detected_class);
        self.metadata.authors.append(&mut parsed);
    }
}

/// Self-contained "clean neutral article" preamble (Task 1). Emits only native
/// Typst set/show rules — no `@preview` imports, compiles on stock Typst with
/// no packages or `typst.toml`. Scalar layout (paper size, font size) is taken
/// from `layout` when the source's `\documentclass` requested it (Task 2),
/// otherwise the neutral defaults (us-letter, 11pt) are kept. Heading
/// *numbering* is set by `finish()`, not here, so there is a single
/// `#set heading(numbering)` site.
pub(in crate::emit) fn build_neutral_preamble(
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
    // Body font + heading sizes are per-class profile knobs (e.g. acmart →
    // Libertinus Serif; compact conference sectioning vs article's). Unprofiled
    // classes keep the neutral "New Computer Modern" + 1.44/1.2/1em hierarchy.
    let profile = crate::style_profile::StyleProfile::for_class(class);
    let body_font = profile.body_font;
    let [h1, h2, h3] = profile.heading_sizes;
    // Document-level two-column: a PAGE-level `columns: 2` (the body flows in two
    // balanced columns across pages, with figures/floats handled natively). The
    // title block spans both columns via a `#place(scope: "parent", float: true)`
    // float in `finish()`. Page columns replace the old `#columns(2)[body]`
    // content-block, which blew up on figure-heavy docs (corpus 2605.31586: 81pp).
    let columns = if layout.is_two_column(class) { ", columns: 2" } else { "" };
    format!(
        "#set page(paper: \"{paper}\", margin: {margin}{columns})\n\
         #set text(font: \"{body_font}\", size: {font_size})\n\
         #set par(justify: true, leading: 0.65em, spacing: 0.65em, first-line-indent: 1.2em)\n\
         #show heading.where(level: 1): set text(size: {h1}, weight: \"bold\")\n\
         #show heading.where(level: 2): set text(size: {h2}, weight: \"bold\")\n\
         #show heading.where(level: 3): set text(size: {h3}, weight: \"bold\")\n\
         #show heading: it => block(above: if it.level == 1 {{ 1.5em }} else {{ 1.4em }}, below: if it.level == 1 {{ 1.0em }} else {{ 0.65em }}, it)\n\n"
    )
}

pub(in crate::emit) fn extract_class_and_options(node: Node<'_>, src: &str) -> (Option<String>, Vec<String>) {
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
pub(in crate::emit) fn extract_latex_include_path(node: Node<'_>, src: &str) -> Option<String> {
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

/// Resolve an `\input{rel}` style path against `base`. LaTeX accepts both
/// `\input{foo}` (no extension; the `.tex` is implicit) and `\input{foo.tex}`
/// — try the literal first, then the `.tex`-appended form.
pub(in crate::emit) fn resolve_input_path(base: &Path, raw: &str) -> Option<PathBuf> {
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
pub(in crate::emit) fn resolve_package_path(base: &Path, pkg: &str) -> Option<PathBuf> {
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
pub(in crate::emit) fn extract_package_names(node: Node<'_>, src: &str) -> Vec<String> {
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
pub(in crate::emit) fn extract_package_options(node: Node<'_>, src: &str) -> Option<String> {
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
pub(in crate::emit) fn is_known_noop_package(name: &str) -> bool {
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
        | "acmart" | "IEEEtran" | "spconf" | "acl" | "acl_natbib"
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


// ─── Math word recognition & post-processing ──────────────────────────────────


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
pub(in crate::emit) fn aff_display_text(aff: &Option<crate::document::Affiliation>) -> Option<String> {
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
