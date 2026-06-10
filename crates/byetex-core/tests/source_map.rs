use byetex_core::{resolve_error_line, NodeOutput};
use byetex_core::{convert, convert_capturing_source_map, resolve_error_line as resolve, ConvertOptions};

fn n(src: (usize, usize), out: &str) -> NodeOutput {
    NodeOutput { src, output: out.to_string() }
}

#[test]
fn shortest_containing_output_wins() {
    let map = vec![
        n((0, 100), "= Heading\n\nP(B_(tau_i)|arrival)\n"), // parent
        n((40, 60), "P(B_(tau_i)|arrival)"),                 // the math node
    ];
    let span = resolve_error_line(&map, "P(B_(tau_i)|arrival)");
    assert_eq!(span, Some((40, 60)));
}

#[test]
fn whitespace_is_normalized() {
    let map = vec![n((5, 9), "a + b")];
    assert_eq!(resolve_error_line(&map, "   a + b   "), Some((5, 9)));
}

#[test]
fn token_fallback_when_no_full_line_match() {
    let map = vec![n((3, 8), "#hide[$arrival$]")];
    assert_eq!(resolve_error_line(&map, "(#hide[$arrival$])"), Some((3, 8)));
}

#[test]
fn no_match_returns_none() {
    let map = vec![n((0, 4), "abcd")];
    assert_eq!(resolve_error_line(&map, "totally unrelated zzz"), None);
}

#[test]
fn default_convert_has_empty_source_map_and_unchanged_output() {
    let src = r"\section{Intro}\nHello world.";
    let plain = convert(src, &ConvertOptions::default());
    let mapped = convert_capturing_source_map(src, &ConvertOptions::default());
    assert!(plain.source_map.is_empty(), "default convert must not capture a map");
    assert_eq!(plain.typst, mapped.typst, "capture must not change the output");
}

#[test]
fn captured_map_resolves_a_body_line_to_its_source() {
    let src = "\\section{Intro}\n\nThe quick brown fox.\n";
    let out = convert_capturing_source_map(src, &ConvertOptions::default());
    assert!(!out.source_map.is_empty());
    let span = resolve(&out.source_map, "The quick brown fox.").expect("should resolve");
    let frag = &src[span.0..span.1];
    assert!(frag.contains("quick brown fox"), "resolved fragment was: {frag:?}");
}
