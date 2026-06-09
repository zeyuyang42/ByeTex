use byetex_core::{default_skill_for, skills, Category};

#[test]
fn each_category_maps_to_an_existing_skill() {
    let cases = [
        Category::UnsupportedEnvironment { name: "x".into() },
        Category::Tikz,
        Category::CustomMacro { name: "x".into() },
        Category::ParseError { tree_sitter_node: "x".into() },
        Category::AmbiguousMath { reason: "x".into() },
        Category::UnsupportedCommand { name: "x".into() },
        Category::NeedsManualReview { reason: "x".into() },
        Category::UnknownPackage { name: "x".into() },
    ];
    for cat in cases {
        let name = default_skill_for(&cat).expect("every category should map to a skill");
        assert!(
            skills::read_skill(name).is_some(),
            "skill `{name}` for {cat:?} must exist in the catalogue"
        );
    }
    assert_eq!(
        default_skill_for(&Category::DropOnly { name: "x".into() }),
        None,
        "DropOnly is intentionally unactionable"
    );
}

#[test]
fn tikz_maps_to_tikz_skill() {
    assert_eq!(default_skill_for(&Category::Tikz), Some("byetex-tikz-to-typst"));
}

#[test]
fn unsupported_env_warning_gets_suggested_skill_filled() {
    let out = byetex_core::convert(
        r"\begin{unknownenvxyz}hi\end{unknownenvxyz}",
        &Default::default(),
    );
    let w = out
        .warnings
        .iter()
        .find(|w| matches!(&w.category, Category::UnsupportedEnvironment { .. }))
        .expect("expected an unsupported_environment warning");
    assert_eq!(w.suggested_skill.as_deref(), Some("byetex-unsupported-environment"));
}

#[test]
fn bibliography_warning_suggests_bibliography_skill() {
    use std::fs;
    let dir = std::env::temp_dir().join(format!("byetex-bibskill-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("p.tex"),
        "\\documentclass{article}\\begin{document}x\\bibliography{NoSuchFile}\\end{document}").unwrap();
    let out = byetex_core::convert(
        &fs::read_to_string(dir.join("p.tex")).unwrap(),
        &byetex_core::ConvertOptions { source_name: Some("p.tex".into()), base_dir: Some(dir.clone()) },
    );
    assert!(
        out.warnings.iter().any(|w| w.suggested_skill.as_deref() == Some("byetex-bibliography")),
        "a missing-.bib warning should suggest the bibliography skill; got {:?}",
        out.warnings
    );
    let _ = fs::remove_dir_all(&dir);
}
