use byetex_core::{resolve_error_line, NodeOutput};

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
