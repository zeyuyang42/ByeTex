//! A `figure` with several `\includegraphics` placed DIRECTLY in it (multi-panel
//! figures that don't use subfigure) only rendered the FIRST image —
//! `emit_figure` captured `graphics` as an `Option`. Emit ALL of them (a single
//! image stays bare; several become a horizontal `stack`). Mirrors the subfigure
//! fix (#411). Found by the visual grader on 2605.22312 (3-panel figure).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn figure_with_three_direct_images_emits_all() {
    let t = typ(r"\begin{figure}\centering\includegraphics{a.png}\includegraphics{b.png}\includegraphics{c.png}\caption{Three panels}\end{figure}");
    for n in ["a.png", "b.png", "c.png"] {
        assert!(t.contains(&format!("image(\"{n}\"")), "image {n} dropped; got:\n{t}");
    }
}

#[test]
fn single_image_figure_unchanged() {
    let t = typ(r"\begin{figure}\includegraphics{solo.png}\caption{One}\end{figure}");
    assert!(t.contains("image(\"solo.png\")") && !t.contains("stack(dir"), "single image should stay bare; got:\n{t}");
}
