//! Phase 2c / defect D5: a `figure` containing several `subfigure`s must emit
//! ALL their `\includegraphics`, not just the last one. `emit_figure` kept a
//! single `graphics_include`, so an N-subfigure figure collapsed to one image
//! (corpus 2605.22765: 37 includegraphics / 21 subfigures → 0-1 images;
//! 2605.22800 14 → 0). Emit each subfigure image so the figure's panels survive.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn all_subfigure_images_are_emitted() {
    let t = typ(
        "\\begin{figure}\n\
         \\begin{subfigure}{0.5\\textwidth}\\includegraphics{a.png}\\caption{Panel A}\\end{subfigure}\n\
         \\begin{subfigure}{0.5\\textwidth}\\includegraphics{b.png}\\caption{Panel B}\\end{subfigure}\n\
         \\caption{Main caption}\\label{fig:multi}\n\
         \\end{figure}",
    );
    assert!(
        t.contains("image(\"a.png\")") || t.contains("image(\"a.png\","),
        "first subfigure image must be emitted; got:\n{t}"
    );
    assert!(
        t.contains("image(\"b.png\")") || t.contains("image(\"b.png\","),
        "second subfigure image must be emitted; got:\n{t}"
    );
    // Exactly the two panel images (no spurious extras).
    assert_eq!(
        t.matches("image(").count(),
        2,
        "exactly two images expected; got:\n{t}"
    );
    // Outer caption + label still attach to the figure.
    assert!(
        t.contains("caption: [Main caption]"),
        "outer caption kept; got:\n{t}"
    );
    assert!(t.contains("<fig:multi>"), "outer label kept; got:\n{t}");
}

#[test]
fn subfigure_subcaptions_are_preserved() {
    let t = typ(
        "\\begin{figure}\n\
         \\begin{subfigure}{0.5\\textwidth}\\includegraphics{a.png}\\caption{Panel A}\\end{subfigure}\n\
         \\begin{subfigure}{0.5\\textwidth}\\includegraphics{b.png}\\caption{Panel B}\\end{subfigure}\n\
         \\caption{Main}\\end{figure}",
    );
    assert!(
        t.contains("Panel A") && t.contains("Panel B"),
        "subcaptions must survive; got:\n{t}"
    );
}

#[test]
fn referenced_label_of_imageless_subfigure_is_anchored() {
    // Regression (corpus 2605.22507): a subfigure whose body is prose (no
    // \includegraphics) is dropped as a panel, but if its \label is \ref'd the
    // label must still be anchored or `@fig:x` dangles ("label does not exist").
    let src = "Look at \\cref{fig:panel_c} for details.\n\
        \\begin{figure}\n\
        \\begin{subfigure}{0.5\\textwidth}\\includegraphics{a.png}\\caption{A}\\end{subfigure}\n\
        \\begin{subfigure}{0.5\\textwidth}(c) text-only panel\\label{fig:panel_c}\\end{subfigure}\n\
        \\caption{Main}\\label{fig:main}\\end{figure}";
    let out = convert(src, &ConvertOptions::default());
    let t = &out.typst;
    // The image panel still renders.
    assert!(
        t.contains("image(\"a.png\")") || t.contains("image(\"a.png\","),
        "image panel renders; got:\n{t}"
    );
    // The referenced text-only subfigure label must be present somewhere as an anchor.
    assert!(
        t.contains("<fig:panel_c>"),
        "referenced label of an image-less subfigure must be anchored; got:\n{t}"
    );
}

#[test]
fn single_graphics_figure_unchanged() {
    // Regression guard: an ordinary one-image figure is still a single image.
    let t =
        typ("\\begin{figure}\\includegraphics{solo.png}\\caption{Solo}\\label{fig:s}\\end{figure}");
    assert_eq!(
        t.matches("image(").count(),
        1,
        "single image preserved; got:\n{t}"
    );
    assert!(
        t.contains("caption: [Solo]") && t.contains("<fig:s>"),
        "caption+label; got:\n{t}"
    );
}
