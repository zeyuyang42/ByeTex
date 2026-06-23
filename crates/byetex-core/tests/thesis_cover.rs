//! Generic thesis/report cover page: `\coverimage{...}` + `\makecover`.
//!
//! Thesis/report classes (e.g. `tudelft-report`) define a designed cover page
//! — a near-full-bleed cover image plus a banner carrying the title / subtitle /
//! subject / author. ByeTex used to drop the whole thing (`\coverimage`,
//! `\makecover`, `\subject` were all unhandled), so the converted thesis had no
//! cover. These tests assert ByeTex now emits a GENERIC cover page (image +
//! title banner) for chapter-bearing classes, gates it OFF for articles, and
//! degrades gracefully when the image is missing.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

fn run_with_base(main: &str, base: PathBuf) -> byetex_core::ConvertOutput {
    convert(
        main,
        &ConvertOptions {
            source_name: Some("report.tex".into()),
            base_dir: Some(base),
        },
    )
}

/// Drop a tiny valid-ish JPEG placeholder so the asset probe finds it on disk.
fn write_cover(dir: &std::path::Path) -> PathBuf {
    let figdir = dir.join("figures");
    fs::create_dir_all(&figdir).unwrap();
    let cover = figdir.join("cover.jpg");
    // Minimal JFIF header bytes — enough for the on-disk probe (we never
    // actually decode it in these unit tests).
    fs::write(&cover, b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00").unwrap();
    cover
}

/// A thesis-style doc (chapter-bearing class) with a cover image directive.
const THESIS: &str = r"\documentclass{report}
\begin{document}
\chapter{Intro}
\title{A Title to the Report}
\subtitle{A Catchy Optional Subtitle}
\subject{AB1234: Optional Course Name}
\author{Jane Author}
\coverimage{figures/cover.jpg}
\makecover
Body text.
\end{document}";

#[test]
fn thesis_makecover_emits_cover_image_and_banner() {
    let tmp = TempDir::new().unwrap();
    write_cover(tmp.path());
    let out = run_with_base(THESIS, tmp.path().to_path_buf());

    // Cover image referenced.
    assert!(
        out.typst.contains("figures/cover.jpg"),
        "cover image should be referenced in a #image(...); got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("image("),
        "cover should emit an #image call; got:\n{}",
        out.typst
    );
    // Title banner content.
    assert!(
        out.typst.contains("A Title to the Report"),
        "title should appear in the cover banner; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("A Catchy Optional Subtitle"),
        "subtitle should appear in the cover banner; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("Jane Author"),
        "author should appear in the cover banner; got:\n{}",
        out.typst
    );

    // Asset is registered so the project layer copies cover.jpg into the output.
    assert!(
        out.asset_refs
            .iter()
            .any(|a| a.typst_path.contains("cover.jpg")),
        "cover image must be registered as an asset for copying; refs: {:?}",
        out.asset_refs
    );
}

#[test]
fn article_with_coverimage_is_not_a_cover_page() {
    // Gating: a plain article must NOT grow a cover page even if it (oddly)
    // carries the directives — `\coverimage`/`\makecover` only mean something
    // for chapter-bearing thesis/report classes.
    let tmp = TempDir::new().unwrap();
    write_cover(tmp.path());
    let article = r"\documentclass{article}
\begin{document}
\title{Paper}
\author{Someone}
\coverimage{figures/cover.jpg}
\makecover
Body.
\end{document}";
    let out = run_with_base(article, tmp.path().to_path_buf());

    // No cover image should be emitted for an article.
    assert!(
        !out.typst.contains("figures/cover.jpg"),
        "article must not emit a cover image; got:\n{}",
        out.typst
    );
}

#[test]
fn thesis_makecover_missing_image_is_graceful() {
    // Missing cover image: still emit the title banner (no panic, no dangling
    // image() against a non-existent asset).
    let tmp = TempDir::new().unwrap();
    // NOTE: do not write the cover file.
    let out = run_with_base(THESIS, tmp.path().to_path_buf());

    assert!(
        out.typst.contains("A Title to the Report"),
        "title banner should still render when the cover image is missing; got:\n{}",
        out.typst
    );
    // Must not reference a cover image that doesn't exist as an asset.
    assert!(
        !out.asset_refs
            .iter()
            .any(|a| a.typst_path.contains("cover.jpg")),
        "missing cover image must not be registered as an asset; refs: {:?}",
        out.asset_refs
    );
}
