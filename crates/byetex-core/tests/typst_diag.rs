use byetex_core::parse_typst_errors;

const STDERR: &str = "\
error: unknown variable: arrival
  ┌─ main.typ:134:0
  │
134 │ P(B_(tau_i)|arrival)
  │
error: unexpected argument
  ┌─ main.typ:200:12
";

#[test]
fn parses_message_line_col_for_each_error() {
    let errs = parse_typst_errors(STDERR);
    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].message, "unknown variable: arrival");
    assert_eq!(errs[0].line, 134);
    assert_eq!(errs[0].col, 0);
    assert_eq!(errs[1].message, "unexpected argument");
    assert_eq!(errs[1].line, 200);
    assert_eq!(errs[1].col, 12);
}

#[test]
fn empty_stderr_yields_no_errors() {
    assert!(parse_typst_errors("").is_empty());
}
