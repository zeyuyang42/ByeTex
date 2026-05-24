//! Tests that the emitter records AssetRefs for resolved images and bib files.

use std::path::PathBuf;

use bytetex_core::{convert, AssetKind, ConvertOptions};

fn fixture(rel: &str) -> PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    root.join("tests/fixtures").join(rel)
}

#[test]
fn asset_discovery_finds_image_and_bib() {
    let base = fixture("asset-discovery");
    let source = std::fs::read_to_string(base.join("main.tex")).unwrap();

    let out = convert(
        &source,
        &ConvertOptions {
            source_name: Some("asset-discovery/main.tex".into()),
            base_dir: Some(base.clone()),
        },
    );

    // Should find exactly 2 assets: one image, one bibliography.
    assert_eq!(
        out.asset_refs.len(),
        2,
        "expected 2 asset refs, got {}: {:?}",
        out.asset_refs.len(),
        out.asset_refs
    );

    let image_ref = out.asset_refs.iter().find(|r| r.kind == AssetKind::Image);
    let bib_ref = out
        .asset_refs
        .iter()
        .find(|r| r.kind == AssetKind::Bibliography);

    assert!(image_ref.is_some(), "no Image AssetRef found");
    assert!(bib_ref.is_some(), "no Bibliography AssetRef found");

    // The image source path should point to the fixture file.
    let img = image_ref.unwrap();
    assert!(
        img.source_path.is_file(),
        "image source_path {:?} is not a file",
        img.source_path
    );
    assert!(
        img.source_path.ends_with("fig/diagram.pdf"),
        "unexpected image source path: {:?}",
        img.source_path
    );

    // The bib source path should point to refs.bib.
    let bib = bib_ref.unwrap();
    assert!(
        bib.source_path.is_file(),
        "bib source_path {:?} is not a file",
        bib.source_path
    );
    assert!(
        bib.source_path.ends_with("refs.bib"),
        "unexpected bib source path: {:?}",
        bib.source_path
    );
}

#[test]
fn asset_discovery_empty_without_base_dir() {
    let base = fixture("asset-discovery");
    let source = std::fs::read_to_string(base.join("main.tex")).unwrap();

    let out = convert(
        &source,
        &ConvertOptions {
            source_name: Some("inline".into()),
            base_dir: None,
        },
    );

    assert!(
        out.asset_refs.is_empty(),
        "expected no asset refs without base_dir, got: {:?}",
        out.asset_refs
    );
}
