use byetex_core::{convert, ConvertOptions};

fn has_env_warning(src: &str, env_name: &str) -> bool {
    let out = convert(src, &ConvertOptions::default());
    out.warnings.iter().any(|w| {
        matches!(
            &w.category,
            byetex_core::warnings::Category::UnsupportedEnvironment { name }
            if name == env_name
        )
    })
}

fn output(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn tikzpicture_no_warning() {
    let src = r"\begin{tikzpicture}\draw (0,0) -- (1,1);\end{tikzpicture}";
    assert!(!has_env_warning(src, "tikzpicture"), "tikzpicture should not warn");
}

#[test]
fn tikzpicture_content_dropped() {
    let src = r"Before.\begin{tikzpicture}\draw (0,0) circle (1);\end{tikzpicture}After.";
    let out = output(src);
    assert!(!out.contains("draw"), "tikzpicture body should be dropped");
    assert!(out.contains("Before") && out.contains("After"), "surrounding text preserved");
}

#[test]
fn multicols_no_warning() {
    let src = r"\begin{multicols}{2}Some text in columns.\end{multicols}";
    assert!(!has_env_warning(src, "multicols"), "multicols should not warn");
}

#[test]
fn multicols_content_preserved() {
    let src = r"\begin{multicols}{2}Important content here.\end{multicols}";
    let out = output(src);
    assert!(out.contains("Important content here"), "multicols body should be passed through");
}
