use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 1) Compile the vendored tree-sitter-latex grammar.
    let src_dir = Path::new("vendor/tree-sitter-latex/src");
    let parser_c = src_dir.join("parser.c");
    let scanner_c = src_dir.join("scanner.c");

    let mut build = cc::Build::new();
    build
        .include(src_dir)
        .file(&parser_c)
        .file(&scanner_c)
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-label")
        .std("c11");
    build.compile("tree-sitter-latex");

    println!("cargo:rerun-if-changed={}", parser_c.display());
    println!("cargo:rerun-if-changed={}", scanner_c.display());

    // 2) Embed the workspace's `skills/*.md` directory into Rust source.
    generate_skill_catalogue();
}

fn generate_skill_catalogue() {
    // byetex-core lives at <workspace>/crates/byetex-core, so the skills
    // directory is two levels up.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("byetex-core under crates/");
    let skills_dir = workspace_root.join("skills");
    println!("cargo:rerun-if-changed={}", skills_dir.display());

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let out_path = Path::new(&out_dir).join("skills_generated.rs");

    let mut entries: Vec<(String, String, String)> = Vec::new();
    if skills_dir.is_dir() {
        let mut paths: Vec<_> = fs::read_dir(&skills_dir)
            .unwrap_or_else(|e| panic!("read_dir {}: {}", skills_dir.display(), e))
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension().and_then(|s| s.to_str()) == Some("md")
                    && p.file_stem().and_then(|s| s.to_str()) != Some("INDEX")
            })
            .collect();
        paths.sort();

        for path in paths {
            println!("cargo:rerun-if-changed={}", path.display());
            let body = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let (name, desc) = parse_frontmatter(&body).unwrap_or_else(|| {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                (stem, "(no description)".to_string())
            });
            entries.push((name, desc, body));
        }
    }

    let mut src = String::new();
    src.push_str("pub(crate) static SKILLS: &[Skill] = &[\n");
    for (name, desc, body) in &entries {
        src.push_str("    Skill {\n");
        src.push_str(&format!("        name: {},\n", rust_string_literal(name)));
        src.push_str(&format!(
            "        description: {},\n",
            rust_string_literal(desc)
        ));
        src.push_str(&format!("        body: {},\n", rust_string_literal(body)));
        src.push_str("    },\n");
    }
    src.push_str("];\n");

    fs::write(&out_path, src).unwrap_or_else(|e| panic!("write {}: {}", out_path.display(), e));
}

/// Extract `name:` and `description:` from a YAML-ish frontmatter block at the
/// start of the markdown file (`---` ... `---`). Returns `None` if the file
/// has no recognisable frontmatter.
fn parse_frontmatter(body: &str) -> Option<(String, String)> {
    let mut lines = body.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut name = None;
    let mut desc = None;
    for line in lines {
        let line = line.trim_end();
        if line.trim() == "---" {
            break;
        }
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("description:") {
            desc = Some(v.trim().trim_matches('"').to_string());
        }
    }
    Some((name?, desc.unwrap_or_default()))
}

fn rust_string_literal(s: &str) -> String {
    // Generate a Rust raw string literal that can hold arbitrary content.
    // Find the shortest `#` padding such that the closing delimiter is unique.
    let mut hashes = 0usize;
    let bytes = s.as_bytes();
    let mut needed = 0usize;
    for i in 0..bytes.len() {
        if bytes[i] == b'"' {
            let mut count = 0usize;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'#' {
                count += 1;
                j += 1;
            }
            if count + 1 > needed {
                needed = count + 1;
            }
        }
    }
    if needed > hashes {
        hashes = needed;
    }
    let pad: String = "#".repeat(hashes);
    format!("r{pad}\"{s}\"{pad}")
}
