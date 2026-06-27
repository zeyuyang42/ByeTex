//! Figure image-emission fixes (code-review findings #1-#4):
//! - #1 multi-image figures pick stack direction by width (full-width → vertical,
//!   else side-by-side) instead of forcing horizontal (which overflows).
//! - #2 missing-file placeholder rect carries the `#` sigil for standalone use.
//! - #3 `render_caption_block` emits ALL images in a caption segment, not just
//!   the first.
//! - #4 a figure with a subfigure panel AND a direct `\includegraphics` keeps both.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-figemit-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ── #1: width-aware stack direction ──────────────────────────────────────────

#[test]
fn full_width_multi_image_stacks_vertically() {
    let t = typ("\\begin{figure}\n\\includegraphics[width=\\textwidth]{a.png}\n\\includegraphics[width=\\textwidth]{b.png}\n\\end{figure}");
    assert!(
        t.contains("stack(dir: ttb"),
        "two full-width panels must stack vertically (ltr overflows); got:\n{t}"
    );
}

#[test]
fn scaled_multi_image_stays_horizontal() {
    // svmult 2605.22312 idiom: several small `[scale=…]` panels, no width fraction.
    let t = typ("\\begin{figure}\n\\includegraphics[scale=0.13]{a.png}\n\\includegraphics[scale=0.13]{b.png}\n\\includegraphics[scale=0.13]{c.png}\n\\end{figure}");
    assert!(
        t.contains("stack(dir: ltr"),
        "small scaled panels should stay side-by-side; got:\n{t}"
    );
}

#[test]
fn small_fraction_multi_image_stays_horizontal() {
    let t = typ("\\begin{figure}\n\\includegraphics[width=0.3\\textwidth]{a.png}\n\\includegraphics[width=0.3\\textwidth]{b.png}\n\\end{figure}");
    assert!(
        t.contains("stack(dir: ltr"),
        "0.3+0.3 widths fit one row, should be side-by-side; got:\n{t}"
    );
}

// ── #2: missing-file placeholder sigil ───────────────────────────────────────

#[test]
fn missing_standalone_image_placeholder_has_sigil() {
    let dir = tmpdir("missing-img");
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\includegraphics{nope}\n\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
        ..Default::default()
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("#rect("),
        "standalone missing-image placeholder needs a leading # or Typst drops it; got:\n{}",
        out.typst
    );
}

// ── #3: render_caption_block emits all images ────────────────────────────────

#[test]
fn caption_block_emits_all_images() {
    // Pattern-B multi-caption float (#subpar.grid): two captioned minipages,
    // the first holding TWO direct \includegraphics.
    let t = typ(
        "\\begin{figure}\n\
         \\begin{minipage}{0.45\\textwidth}\\includegraphics{a.png}\\includegraphics{b.png}\\captionof{figure}{A}\\end{minipage}\n\
         \\begin{minipage}{0.45\\textwidth}\\includegraphics{c.png}\\captionof{figure}{B}\\end{minipage}\n\
         \\end{figure}",
    );
    assert!(
        t.matches("image(").count() >= 3,
        "all three images (a,b,c) must survive; got {} image() calls:\n{t}",
        t.matches("image(").count()
    );
}

// ── #4: subfigure panel + direct image both kept ─────────────────────────────

#[test]
fn figure_with_subfigure_and_direct_image_keeps_both() {
    let t = typ(
        "\\begin{figure}\n\
         \\begin{subfigure}{0.5\\textwidth}\\includegraphics{a.png}\\caption{A}\\end{subfigure}\n\
         \\includegraphics{b.png}\n\
         \\end{figure}",
    );
    assert!(
        t.matches("image(").count() >= 2,
        "both the subfigure image and the direct image must survive; got {}:\n{t}",
        t.matches("image(").count()
    );
}
