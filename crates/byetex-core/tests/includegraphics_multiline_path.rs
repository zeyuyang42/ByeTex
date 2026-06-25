//! `\includegraphics[…]{` with the path on the FOLLOWING line (common when the
//! options make the line long) parses with the leading newline glued to the
//! path node (`path "\nimg/a/b.png"`). ByeTex fed that untrimmed string to the
//! asset resolver, so `base_dir.join("\nimg/…")` never matched and the existing
//! image was emitted as a `(missing)` placeholder — dropping real figures
//! (2605.22507 appendix lost several MNIST grids). Trim the extracted path.
//! Found by the visual grader on 2605.22507.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-ig-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn multiline_includegraphics_path_resolves() {
    let dir = tmpdir("multiline");
    fs::create_dir_all(dir.join("img/sub")).unwrap();
    // A tiny valid-enough PNG header; existence is what the resolver checks.
    fs::write(dir.join("img/sub/pic.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    fs::write(
        dir.join("main.tex"),
        // Path on the line AFTER `{` — the failing form.
        "\\documentclass{article}\n\\begin{document}\n\
         \\includegraphics[width=.4\\linewidth]{\nimg/sub/pic.png}\n\
         \\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("main.tex")).unwrap(), &opts);

    assert!(
        out.typst.contains(r#"image("img/sub/pic.png""#),
        "multi-line includegraphics path not resolved to a clean image() call; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("missing"),
        "existing image wrongly marked missing; got:\n{}",
        out.typst
    );
    // No stray newline glued to the path.
    assert!(
        !out.typst.contains("\\nimg/") && !out.typst.contains("image(\"\n"),
        "leading newline leaked into the path; got:\n{}",
        out.typst
    );
}
