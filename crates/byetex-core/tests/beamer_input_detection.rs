//! Regression: a beamer deck that splits its `frame`s into `\input`ed files
//! (the common multi-file deck layout) must still convert them to slides. The
//! `\input` sub-emitter inherited macros / chapter_based / natbib_mode etc. but
//! NOT `detected_class`, so the included file defaulted to article and every
//! `\begin{frame}` was flagged `unsupported_environment` and rendered as plain
//! content instead of a touying slide (corpus gh-klb2-beamer: 12 such warnings).

use byetex_core::{convert, ConvertOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn beamer_frames_in_input_file_are_recognized() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("slides.tex"),
        "\\begin{frame}{Title A}\nContent A.\n\\end{frame}\n\
         \\begin{frame}{Title B}\nContent B.\n\\end{frame}\n",
    )
    .unwrap();
    let main = "\\documentclass{beamer}\n\\begin{document}\n\\input{slides}\n\\end{document}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(tmp.path().to_path_buf()),
        },
    );

    let unsupported_frames = out
        .warnings
        .iter()
        .filter(|w| format!("{:?}", w).contains("frame"))
        .count();
    assert_eq!(
        unsupported_frames, 0,
        "frames in an \\input'ed file should be recognized as beamer; warnings: {:?}",
        out.warnings
    );
    assert!(out.typst.contains("Content A."), "lost frame body; got:\n{}", out.typst);
    assert!(out.typst.contains("Title A"), "lost frame title; got:\n{}", out.typst);
}
