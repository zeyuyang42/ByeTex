//! Parse `typst compile` stderr into structured errors. Pure (no process
//! spawning) so it is unit-testable without the typst binary. Typst's
//! diagnostic format is:
//!     error: <message>
//!       ┌─ <file>:<line>:<col>
//! (optionally followed by source-snippet lines we ignore).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstError {
    pub message: String,
    /// 1-based line in the `.typ`, as typst reports.
    pub line: usize,
    /// 0-based column, as typst reports.
    pub col: usize,
}

/// Extract every `error:` diagnostic with a location line. Diagnostics without
/// a `┌─ file:line:col` location line are skipped (they can't be mapped).
pub fn parse_typst_errors(stderr: &str) -> Vec<TypstError> {
    let lines: Vec<&str> = stderr.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(msg) = lines[i].trim_start().strip_prefix("error: ") {
            // Look at the next line for the `┌─ file:line:col` location.
            if let Some(loc) = lines.get(i + 1).and_then(|l| parse_location(l)) {
                out.push(TypstError {
                    message: msg.trim().to_string(),
                    line: loc.0,
                    col: loc.1,
                });
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Parse a `  ┌─ main.typ:134:0` line into `(line, col)`. The box-drawing
/// prefix varies; key on the trailing `:<line>:<col>`.
fn parse_location(line: &str) -> Option<(usize, usize)> {
    let after = line.rsplit("─ ").next()?; // text after the box-drawing rule
    // after looks like `main.typ:134:0`
    let mut parts = after.rsplitn(3, ':');
    let col: usize = parts.next()?.trim().parse().ok()?;
    let ln: usize = parts.next()?.trim().parse().ok()?;
    parts.next()?; // the path (ignored)
    Some((ln, col))
}
