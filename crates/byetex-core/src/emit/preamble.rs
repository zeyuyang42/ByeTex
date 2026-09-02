//! Preamble building, title-block flush, author materialization & package/class extraction, extracted from emit.rs (pure code motion).

use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::ir::Node;

use super::{escape_text_for_typst_content, Emitter};

impl<'a> Emitter<'a> {
    /// Emit the rich native title block from captured \title/\author/\date/
    /// \begin{abstract}/\keywords. Used for Unknown/Lncs/SvMult classes (no
    /// Typst Universe template binding).
    pub(in crate::emit) fn flush_title_block(&mut self) {
        self.materialize_authors();
        // A `\makecover` cover page already rendered the title/subtitle/author
        // into its banner; don't also draw a centered title block (the thesis
        // gets its full title page from `\input{frontmatter/title-…}`). Authors
        // are still materialized above so the PDF document metadata is set.
        if self.cover_emitted {
            return;
        }
        if self.metadata.is_title_block_empty() {
            return;
        }

        // Beamer → touying: the title slide is built by touying from the
        // `config-info(…)` block (emitted in the preamble). `\titlepage` /
        // `\maketitle` just triggers `#title-slide()`. No hand-rolled centered
        // title/author block.
        if matches!(self.detected_class, crate::class_map::DocClass::Beamer) {
            self.ensure_paragraph_break();
            self.out.push_str("#title-slide()\n\n");
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
            // Class-faithful title casing (amsart's \MakeUppercase).
            let body = if profile.title_uppercase {
                format!("#upper[{body}]")
            } else {
                body
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

        // `\subtitle{…}`: a smaller line just below the title (any class).
        if let Some(subtitle) = self.metadata.subtitle.take() {
            let _ = writeln!(
                self.out,
                "  #v(0.2em)\n  #text(size: 1.1em)[{}]",
                subtitle.as_content()
            );
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
        let tail_open =
            !with_rules || !self.metadata.authors.is_empty() || self.metadata.date.is_some();

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
            // Beamer title slides show author + `\institute` as plain centered lines,
            // not academic-paper superscript-numbered affiliations.
            let beamer = matches!(self.detected_class, crate::class_map::DocClass::Beamer);

            // Collect per-author affiliation text, deduplicating to assign
            // superscript indices (1-based, in order of first appearance).
            // Two index spaces have to coexist: `\affil[n]` states its own
            // number, while `\thanks`/`\affiliation` entries are numbered by
            // order of appearance. Letting the numbered table REPLACE the
            // text-derived one dropped every text affiliation while its author
            // still printed a superscript — attributing them to someone else's
            // institution, which is worse than dropping one.
            let numbered = self.numbered_affils.clone();
            let text_base = numbered.keys().copied().max().unwrap_or(0) as usize + 1;
            let aff_texts: Vec<Option<String>> = authors
                .iter()
                .map(|a| {
                    // With authblk in play, an author who declared `[n]` already
                    // has their affiliation from the numbered table; whatever
                    // `\thanks` left behind is a footnote (an email, a
                    // corresponding-author marker), not a second institution.
                    // Test the SANITIZED ref, the same value the superscript is
                    // emitted from. Keying on the raw one dropped the author's
                    // text affiliation while giving them no superscript either,
                    // so the content vanished entirely.
                    if !numbered.is_empty()
                        && a.affil_ref
                            .as_deref()
                            .and_then(crate::emit::sanitize_affil_ref)
                            .is_some()
                    {
                        return None;
                    }
                    aff_display_text(&a.affiliation)
                })
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
                    // The author's own declared ref wins; the text-dedup index
                    // is the fallback for authors that declared none.
                    // With no numbered table the declared ref points at nothing
                    // the footer prints, so fall back to the text-dedup index.
                    let declared = if numbered.is_empty() {
                        None
                    } else {
                        author
                            .affil_ref
                            .as_deref()
                            .and_then(crate::emit::sanitize_affil_ref)
                    };
                    match (declared, aff_idx) {
                        (Some(refs), _) if !beamer => {
                            let _ = write!(part, "#super[{refs}]");
                        }
                        (None, Some(idx)) if !beamer => {
                            let _ = write!(part, "#super[{}]", idx + text_base);
                        }
                        _ => {}
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
            if has_affiliations || !numbered.is_empty() {
                self.out.push_str("  #v(0.3em)\n  #text(size: 0.9em)[\n");
                let total = numbered.len() + deduped.len();
                for (i, (n, aff_text)) in numbered.iter().enumerate() {
                    // Already rendered Typst — escaping again would turn an
                    // emitted `#v(-2.5mm)` into a literal `\#v(-2.5mm)`.
                    let marker = if beamer {
                        String::new()
                    } else {
                        format!("#super[{n}] ")
                    };
                    let brk = if i + 1 < total { " #linebreak()" } else { "" };
                    let _ = writeln!(self.out, "    {marker}{}{brk}", aff_text.trim());
                }
                for (i, aff_text) in deduped.iter().enumerate() {
                    let aff_text = escape_text_for_typst_content(aff_text);
                    // Beamer: plain centered line, no superscript ref number.
                    let marker = if beamer {
                        String::new()
                    } else {
                        format!("#super[{}] ", i + text_base)
                    };
                    if numbered.len() + i + 1 < numbered.len() + deduped.len() {
                        let _ = writeln!(self.out, "    {marker}{aff_text} #linebreak()");
                    } else {
                        let _ = writeln!(self.out, "    {marker}{aff_text}");
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
                    escape_text_for_typst_content(&strip_latex_font_wrappers(&emails.join(", ")))
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

    /// Build the touying (metropolis-theme) preamble for a beamer deck:
    /// the `@preview/touying` import, the theme import, and the
    /// `#show: metropolis-theme.with(aspect-ratio:, config-info(…))` line
    /// populated from `self.metadata`. touying owns the page geometry,
    /// header/footer chrome, and slide numbering — so this REPLACES the
    /// plain `#set page(paper: "presentation-…")` neutral preamble and the
    /// `#set heading(numbering)` line for beamer.
    ///
    /// Must run after `materialize_authors()` so author names are populated.
    pub(in crate::emit) fn build_beamer_touying_preamble(&self) -> String {
        // beamer's own slide geometry, from `[aspectratio=…]` or its 4:3 default.
        // The `config-page` width/height are what actually pin the page: touying's
        // `aspect-ratio` only selects a Typst presentation *preset*, and those are
        // 1.9-2.2x larger in each dimension than the slide beamer produces.
        let slide = self
            .layout
            .beamer_slide
            .clone()
            .unwrap_or_else(crate::class_map::beamer_default_slide);

        let mut info = String::new();
        if let Some(title) = &self.metadata.title {
            let _ = writeln!(info, "    title: [{}],", title.as_content());
        }
        if let Some(subtitle) = &self.metadata.subtitle {
            let _ = writeln!(info, "    subtitle: [{}],", subtitle.as_content());
        }
        if !self.metadata.authors.is_empty() {
            // touying renders the author field as a single content block; join
            // multiple authors with a spacer (beamer's `\and` puts them side by
            // side on the title slide).
            let names: Vec<String> = self
                .metadata
                .authors
                .iter()
                .map(|a| escape_text_for_typst_content(a.name.as_content()))
                .collect();
            let _ = writeln!(info, "    author: [{}],", names.join(" #h(1em) "));
        }
        // `\institute{…}` is appended onto the last author's raw buffer at parse
        // time (so it lands in that author's affiliation). Surface the first
        // non-empty affiliation as touying's `institution`.
        if let Some(inst) = self
            .metadata
            .authors
            .iter()
            .find_map(|a| aff_display_text(&a.affiliation))
        {
            let _ = writeln!(
                info,
                "    institution: [{}],",
                escape_text_for_typst_content(&inst)
            );
        }
        if let Some(date) = &self.metadata.date {
            let _ = writeln!(info, "    date: [{date}],");
        }

        // Theme color → touying `config-colors`. Beamer's "structure" color is the
        // accent used for title rules / section frames; metropolis's `primary` is the
        // accent line + progress bar, so map structure→primary. An explicit
        // `\setbeamercolor{frametitle}{fg=…}` is the most specific; else the structure
        // color (`\setbeamercolor{structure}` / `\usecolortheme{name}`). Detect-don't-
        // hardcode: only emit the override when a color was actually parsed from the
        // source — otherwise leave the metropolis default untouched (no `config-colors`).
        let colors = self
            .beamer_frametitle_color
            .clone()
            .or_else(|| self.beamer_structure_color.clone())
            .or_else(|| self.beamer_alert_color.clone())
            .map(|c| format!("  config-colors(primary: {c}),\n"))
            .unwrap_or_default();

        // The metropolis theme hard-codes `set text(size: 20pt)`, sized for its
        // 297mm-wide preset; on a real 160mm slide that is ~2x too large. Restating
        // beamer's own body size rescales the deck as a whole, because every other
        // length in the theme — page margins, header, footer, the `0.8em` chrome —
        // is em-derived and follows it.
        let body_size = self.layout.beamer_font_size.unwrap_or("11pt");
        let (w, h, aspect) = (&slide.width, &slide.height, &slide.aspect);

        format!(
            "#import \"@preview/touying:0.7.3\": *\n\
             #import themes.metropolis: *\n\n\
             #show: metropolis-theme.with(\n\
             \x20 aspect-ratio: \"{aspect}\",\n\
             \x20 config-page(width: {w}, height: {h}),\n{colors}\
             \x20 config-info(\n{info}  ),\n\
             )\n\
             #set text(size: {body_size})\n\n"
        )
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
        let refs = std::mem::take(&mut self.author_affil_refs);
        let mut parsed =
            crate::class_map::parse_authors_with_refs(&raw, &refs, &self.detected_class);
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
/// The caption font size requested by `\captionsetup`, as a Typst `em` ratio.
///
/// LaTeX classes almost always set captions smaller than body text. The emitter
/// set caption POSITION but never SIZE, so captions rendered at full body size —
/// a large share of the corpus-wide small-text deficit
/// (`layout_small_tier_share_delta` negative on 46 of 64 papers, mean -0.106).
///
/// Read from the source rather than baked in per class: `\captionsetup{font=...}`
/// is what the author actually wrote. A `[float-type]` optional argument is
/// accepted — scoping the rule per float kind is not worth the divergence, and
/// ignoring the declaration outright is strictly worse than applying it.
/// Run `probe` over the document, then the files it `\input`s, then any bundled
/// `.cls`/`.sty`, returning the first hit.
///
/// The order encodes precedence: the document's own statement beats a preamble it
/// includes, which beats the class it ships with. Scanning only `src` reaches
/// almost nobody — of 67 corpus papers just 3 put `\captionsetup` in the entry
/// file; the rest carry these declarations in an `\input`ed preamble or the
/// class/style bundled beside it.
fn scan_project_sources<T>(
    src: &str,
    base_dir: Option<&std::path::Path>,
    probe: impl Fn(&str) -> Option<T>,
) -> Option<T> {
    if let Some(hit) = probe(src) {
        return Some(hit);
    }
    let dir = base_dir?;
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for raw in crate::emit::extract_include_paths_from_source(src) {
        if let Some(p) = crate::emit::resolve_include_path(dir, &raw) {
            files.push(p);
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut local: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "sty" || e == "cls"))
            .collect();
        local.sort();
        files.extend(local);
    }
    for f in files {
        if let Ok(text) = std::fs::read_to_string(&f) {
            if let Some(hit) = probe(&text) {
                return Some(hit);
            }
        }
    }
    None
}

/// The bibliography font size the document asks for, as a Typst `em` ratio.
///
/// Reference lists are set smaller than body text by most venue classes, and the
/// section is a large dense block: on 2605.31604 the last three pages of the
/// LaTeX truth are 100% small-tier while ours were 0%. Comparing the small-tier
/// share of the LAST pages of truth vs ours across the corpus, 19 papers show a
/// large gap, several at 1.00 vs 0.00 — the single biggest remaining contributor
/// to `layout_small_tier_share_delta`.
pub(in crate::emit) fn bibliography_font_size(
    src: &str,
    base_dir: Option<&std::path::Path>,
    base_pt: f64,
) -> Option<String> {
    // natbib's documented hook is the more specific statement, so it is probed
    // across ALL sources before falling back to a `thebibliography` definition.
    let ratio = scan_project_sources(src, base_dir, bibfont_hook_in)
        .or_else(|| scan_project_sources(src, base_dir, thebibliography_size_in))?;
    absolute(ratio, base_pt)
}

/// `\def\bibfont{\small}` / `\renewcommand{\bibfont}{\small}`.
fn bibfont_hook_in(src: &str) -> Option<f64> {
    let mut from = 0;
    while let Some(rel) = src.get(from..)?.find("bibfont") {
        let at = from + rel;
        from = at + "bibfont".len();
        if crate::emit::is_commented_out(src, at) {
            continue;
        }
        // The size lives in the brace group that follows the name.
        let bytes = src.as_bytes();
        let mut i = from;
        while bytes.get(i).is_some_and(|b| b.is_ascii_whitespace() || *b == b'}') {
            i += 1;
        }
        if bytes.get(i) != Some(&b'{') {
            continue;
        }
        let Some(end) = crate::emit::node_utils::brace_balanced_end(bytes, i) else {
            continue;
        };
        let body = src.get(i + 1..end.saturating_sub(1)).unwrap_or("");
        if let Some(em) = crate::emit::size_declaration_ratio(body.trim()) {
            return Some(em);
        }
    }
    None
}

/// A size switch inside a class's `thebibliography` definition.
fn thebibliography_size_in(src: &str) -> Option<f64> {
    let mut from = 0;
    while let Some(rel) = src.get(from..)?.find("thebibliography") {
        let at = from + rel;
        from = at + "thebibliography".len();
        if crate::emit::is_commented_out(src, at) {
            continue;
        }
        // Only a DEFINITION carries the styling; a `\begin{thebibliography}` in a
        // document body (or a `.bbl`) is a use, and scanning that would pick up
        // whatever size happened to be in force nearby.
        let head = &src[..at];
        if !head.trim_end().ends_with("newenvironment{")
            && !head.trim_end().ends_with("renewenvironment{")
            && !head.trim_end().ends_with("\\def\\")
        {
            continue;
        }
        let bytes = src.as_bytes();
        // Skip to the first brace group of the definition body.
        let mut i = from;
        while i < bytes.len() && bytes[i] != b'{' {
            // `[1]` arity, `}` closing the env name, whitespace — but stop at a
            // newline-newline gap so a runaway scan cannot leave the definition.
            if bytes[i] == b'\n' && bytes.get(i + 1) == Some(&b'\n') {
                break;
            }
            i += 1;
        }
        let Some(end) = crate::emit::node_utils::brace_balanced_end(bytes, i) else {
            continue;
        };
        let body = src.get(i..end).unwrap_or("");
        for name in [
            "\\tiny",
            "\\scriptsize",
            "\\footnotesize",
            "\\small",
            "\\normalsize",
        ] {
            if let Some(p) = body.find(name) {
                let after = body.as_bytes().get(p + name.len());
                if after.is_some_and(|c| c.is_ascii_alphabetic()) {
                    continue;
                }
                return crate::emit::size_declaration_ratio(name);
            }
        }
    }
    None
}

pub(in crate::emit) fn caption_font_size(
    src: &str,
    base_dir: Option<&std::path::Path>,
    base_pt: f64,
) -> Option<String> {
    // `\captionsetup` is the document's explicit statement and wins everywhere it
    // appears. Classes predating the `caption` package instead redefine
    // `\@makecaption`; 8 corpus papers carry a size ONLY there, nearly as many
    // again as the 9 the package form reaches.
    let ratio = scan_project_sources(src, base_dir, caption_font_size_in)
        .or_else(|| scan_project_sources(src, base_dir, makecaption_size_in))?;
    absolute(ratio, base_pt)
}

/// A ratio against the document base as an absolute Typst length; `None` when it
/// resolves to body size, where a rule would be a no-op.
fn absolute(ratio: f64, base_pt: f64) -> Option<String> {
    if (ratio - 1.0).abs() < f64::EPSILON {
        return None;
    }
    let pt = (ratio * base_pt * 10.0).round() / 10.0;
    Some(if (pt - pt.round()).abs() < f64::EPSILON {
        format!("{}pt", pt.round() as i64)
    } else {
        format!("{pt}pt")
    })
}

/// A size switch inside a `\@makecaption` redefinition.
fn makecaption_size_in(src: &str) -> Option<f64> {
    let mut from = 0;
    while let Some(rel) = src.get(from..)?.find("@makecaption") {
        let at = from + rel;
        from = at + "@makecaption".len();
        if crate::emit::is_commented_out(src, at) {
            continue;
        }
        let bytes = src.as_bytes();
        // Step over the `#1#2` parameter text to the definition body.
        let mut i = from;
        while i < bytes.len() && bytes[i] != b'{' {
            i += 1;
        }
        let Some(end) = crate::emit::node_utils::brace_balanced_end(bytes, i) else {
            continue;
        };
        let body = src.get(i..end).unwrap_or("");
        for name in [
            "\\tiny",
            "\\scriptsize",
            "\\footnotesize",
            "\\small",
            "\\normalsize",
        ] {
            let mut k = 0;
            while let Some(r) = body[k..].find(name) {
                let p = k + r;
                k = p + name.len();
                // `\small` must not match inside `\smallskip`.
                if body.as_bytes().get(p + name.len()).is_some_and(|c| c.is_ascii_alphabetic()) {
                    continue;
                }
                return crate::emit::size_declaration_ratio(name);
            }
        }
    }
    None
}

fn caption_font_size_in(src: &str) -> Option<f64> {
    let mut found: Option<f64> = None;
    let mut from = 0;
    while let Some(rel) = src.get(from..)?.find("\\captionsetup") {
        let at = from + rel;
        from = at + "\\captionsetup".len();
        if crate::emit::is_commented_out(src, at) {
            continue;
        }
        let bytes = src.as_bytes();
        let mut i = from;
        // Skip an optional `[table]` / `[figure]` float-type selector.
        while bytes.get(i).is_some_and(|b| b.is_ascii_whitespace()) {
            i += 1;
        }
        if bytes.get(i) == Some(&b'[') {
            match src[i..].find(']') {
                Some(off) => i += off + 1,
                None => continue,
            }
        }
        while bytes.get(i).is_some_and(|b| b.is_ascii_whitespace()) {
            i += 1;
        }
        let Some(end) = crate::emit::node_utils::brace_balanced_end(bytes, i) else {
            continue;
        };
        let opts = src.get(i + 1..end.saturating_sub(1)).unwrap_or("");
        // `font=small`, `font={small,it}`, `font={it,small}` — only the `font`
        // key sets the whole caption; `labelfont`/`textfont` style only a part,
        // so a `font`-suffixed key must not match.
        //
        // Read ONLY the value belonging to this `font=`, so a size appearing in a
        // neighbouring key cannot be mistaken for the caption size — `labelfont`
        // styles the "Figure 1:" label alone, not the caption text.
        for key in ["font=", "font ="] {
            let mut k = 0;
            while let Some(r) = opts[k..].find(key) {
                let at = k + r;
                k = at + key.len();
                let before = opts[..at].chars().last();
                if before.is_some_and(|c| c.is_ascii_alphabetic()) {
                    continue; // `labelfont=` / `textfont=`
                }
                let rest = opts[k..].trim_start();
                let value = match rest.strip_prefix('{') {
                    // Braced list: every piece inside THIS group, nothing beyond.
                    Some(inner) => &inner[..inner.find('}').unwrap_or(inner.len())],
                    // Bare: a single token, ending at the next key separator.
                    None => &rest[..rest.find(',').unwrap_or(rest.len())],
                };
                for piece in value.split(',') {
                    if let Some(em) = crate::emit::size_declaration_ratio(piece.trim()) {
                        found = Some(em);
                    }
                }
            }
        }
    }
    // An explicit reset to body size is a real declaration, but emitting
    // `size: 1em` is a no-op that only adds noise.
    found
}

pub(in crate::emit) fn build_neutral_preamble(
    layout: &crate::class_map::Layout,
    class: &crate::class_map::DocClass,
    caption_size: Option<&str>,
    bibliography_size: Option<&str>,
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
    // No beamer branch here: a beamer deck never reaches this function. `finish()`
    // routes `DocClass::Beamer` to `build_beamer_touying_preamble`, which owns the
    // slide geometry, and calls this only in the `else`.
    // Document-level two-column: a PAGE-level `columns: 2` (the body flows in two
    // balanced columns across pages, with figures/floats handled natively). The
    // title block spans both columns via a `#place(scope: "parent", float: true)`
    // float in `finish()`. Page columns replace the old `#columns(2)[body]`
    // content-block, which blew up on figure-heavy docs (corpus 2605.31586: 81pp).
    let columns = if layout.is_two_column(class) {
        ", columns: 2"
    } else {
        ""
    };
    // IEEE captions abbreviate the figure label ("Fig. 1") and use an all-caps,
    // roman-numbered table label ("TABLE I"). Other classes keep Typst's defaults
    // ("Figure 1" / "Table 1").
    let figure_supplement = if matches!(class, crate::class_map::DocClass::IeeeTran { .. }) {
        "#set figure(supplement: [Fig.])\n\
         #show figure.where(kind: table): set figure(supplement: [TABLE], numbering: \"I\")\n"
    } else {
        ""
    };
    // Class-faithful level-1 heading alignment/casing (amsart centers; REVTeX
    // centers + uppercases) — driven by the StyleProfile, not per-class matches.
    let heading_align = match (profile.heading_centered, profile.heading_uppercase) {
        (true, true) => "#show heading.where(level: 1): it => align(center, upper(it))\n",
        (true, false) => "#show heading.where(level: 1): it => align(center, it)\n",
        (false, true) => "#show heading.where(level: 1): it => upper(it)\n",
        (false, false) => "",
    };
    // Captions are smaller than body text in essentially every LaTeX class; the
    // size is taken from the document's own `\captionsetup`, not assumed.
    let caption_size = match caption_size {
        Some(em) => format!("#show figure.caption: set text(size: {em})\n"),
        None => String::new(),
    };
    // Reference lists are smaller than body text in most venue classes, and the
    // section is a large block — the biggest single small-tier contributor.
    let bibliography_size = match bibliography_size {
        Some(em) => format!("#show bibliography: set text(size: {em})\n"),
        None => String::new(),
    };
    format!(
        "#set page(paper: \"{paper}\", margin: {margin}{columns}, numbering: \"1\")\n\
         #set text(font: \"{body_font}\", size: {font_size})\n\
         #set par(justify: true, leading: 0.65em, spacing: 0.65em, first-line-indent: 1.2em)\n\
         #show heading.where(level: 1): set text(size: {h1}, weight: \"bold\")\n\
         #show heading.where(level: 2): set text(size: {h2}, weight: \"bold\")\n\
         #show heading.where(level: 3): set text(size: {h3}, weight: \"bold\")\n\
         #show heading: it => block(above: if it.level == 1 {{ 1.5em }} else {{ 1.4em }}, below: if it.level == 1 {{ 1.0em }} else {{ 0.65em }}, it)\n\
         #show figure.where(kind: table): set figure.caption(position: top)\n\
         {caption_size}{bibliography_size}{figure_supplement}{heading_align}\n"
    )
}

pub(in crate::emit) fn extract_class_and_options(
    node: Node<'_>,
    src: &str,
) -> (Option<String>, Vec<String>) {
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
                        // Push the whole `key=value` (or a bare flag) with whitespace
                        // removed, so value-bearing options like `aspectratio=169`
                        // survive — not just the key. Bare flags (`12pt`, `a4paper`)
                        // come through unchanged.
                        let token: String = src[gc.start_byte()..gc.end_byte()]
                            .split_whitespace()
                            .collect();
                        if !token.is_empty() {
                            opts.push(token);
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
pub(in crate::emit) fn aff_display_text(
    aff: &Option<crate::document::Affiliation>,
) -> Option<String> {
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
        return Some(strip_latex_font_wrappers(&parts.join(", ")));
    }
    // Fall back to the raw unstructured blob (e.g. from \IEEEauthorblockA or
    // a plain \affiliation{...} without per-field markers).
    aff.raw
        .as_ref()
        .map(|r| strip_latex_font_wrappers(r.as_content()))
        .filter(|s| !s.is_empty())
}

/// Strip `\texttt{…}`/`\textbf{…}`/… font wrappers from affiliation / `\email{}`
/// text, keeping the inner content. These fields are captured as raw LaTeX, so a
/// `\textsc{Dept}` would otherwise leak its macro name. Emails reaching here are
/// already unwrapped at capture (see `extract_email_token`); this only handles
/// the proper backslash form that survives in affiliations and `\email{\…{}}`.
fn strip_latex_font_wrappers(s: &str) -> String {
    // Fast path: no group, nothing to unwrap (the common case for clean text).
    if !s.contains('{') {
        return s.to_string();
    }
    const CMDS: [&str; 8] = [
        "\\texttt{",
        "\\textbf{",
        "\\textit{",
        "\\emph{",
        "\\textrm{",
        "\\textsc{",
        "\\textnormal{",
        "\\mbox{",
    ];
    let mut out = s.to_string();
    loop {
        let Some((pos, cmd_len)) = CMDS
            .iter()
            .filter_map(|c| out.find(c).map(|p| (p, c.len())))
            .min_by_key(|(p, _)| *p)
        else {
            break;
        };
        let inner_start = pos + cmd_len;
        let bytes = out.as_bytes();
        let mut depth = 1i32;
        let mut i = inner_start;
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            break; // unbalanced — leave the rest as-is rather than corrupt
        }
        out = format!("{}{}{}", &out[..pos], &out[inner_start..i - 1], &out[i..]);
    }
    out
}
