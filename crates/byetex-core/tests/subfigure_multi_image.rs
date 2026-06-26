//! A `subfigure` that stacks SEVERAL `\includegraphics` (the paper puts multiple
//! image rows in one subfigure) only rendered the FIRST image — the rest were
//! silently dropped, losing real figure panels (2605.22507's MNIST grids:
//! 22 `\includegraphics` in source → 10 `#image` out). `render_subfigure_panel`
//! captured `graphics` with an `is_none()` guard. It must emit ALL images in the
//! panel. Found by the visual grader on 2605.22507.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-sf-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn subfigure_with_multiple_images_emits_all() {
    let dir = tmpdir("multi");
    for n in ["a", "b", "c"] {
        fs::write(dir.join(format!("{n}.png")), b"\x89PNG\r\n\x1a\n").unwrap();
    }
    fs::write(
        dir.join("main.tex"),
        "\\documentclass{article}\n\\usepackage{subcaption,graphicx}\n\\begin{document}\n\
         \\begin{figure}\n\
         \\begin{subfigure}{.45\\textwidth}\n\
         \\includegraphics[width=\\linewidth]{a.png}\n\
         \\includegraphics[width=\\linewidth]{b.png}\n\
         \\includegraphics[width=\\linewidth]{c.png}\n\
         \\caption{Stacked panels}\n\
         \\end{subfigure}\n\
         \\caption{Fig}\n\
         \\end{figure}\n\
         \\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("main.tex")).unwrap(), &opts);
    for n in ["a.png", "b.png", "c.png"] {
        assert!(
            out.typst.contains(&format!("image(\"{n}\"")),
            "subfigure dropped image {n}; got:\n{}",
            out.typst
        );
    }
}
