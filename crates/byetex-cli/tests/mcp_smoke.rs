//! MCP smoke test: spawn `byetex serve`, perform the standard `initialize`
//! handshake, then call `tools/list` and one tool. Asserts protocol responses.
//!
//! This is intentionally a minimal client written by hand — we don't pull
//! `rmcp`'s client features into the test, since the goal is to verify the
//! wire protocol our server speaks.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

fn cargo_bin() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // .../crates
    p.pop(); // .../ByeTex
    p.push("target/debug/byetex");
    p
}

/// Read JSON-RPC frames from the given reader until one with the requested id
/// arrives. Times out after a few seconds.
fn read_response_for_id(
    reader: &mut BufReader<std::process::ChildStdout>,
    id: u64,
) -> serde_json::Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        if std::time::Instant::now() > deadline {
            panic!("timeout waiting for MCP response id={id}");
        }
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => panic!("MCP server closed stdout before answering id={id}"),
            Ok(_) => {}
            Err(e) => panic!("read MCP server stdout: {e}"),
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue, // ignore non-JSON lines
        };
        if v.get("id").and_then(|x| x.as_u64()) == Some(id) {
            return v;
        }
    }
}

#[test]
fn mcp_server_handshakes_and_lists_tools() {
    let bin = cargo_bin();
    if !bin.exists() {
        // The test binary may run before the CLI is built. Build it.
        let status = Command::new("cargo")
            .args(["build", "-p", "byetex-cli"])
            .status()
            .expect("cargo build");
        assert!(status.success(), "cargo build failed");
    }

    let mut child = Command::new(&bin)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn byetex serve");

    let stdin = child.stdin.as_mut().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    // 1) initialize
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "smoke-test", "version": "0" }
        }
    });
    writeln!(stdin, "{}", init).expect("write init");
    let resp1 = read_response_for_id(&mut reader, 1);
    assert!(resp1.get("result").is_some(), "initialize result: {resp1}");

    // 2) notifications/initialized
    let initd = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    writeln!(stdin, "{}", initd).expect("write initd");

    // 3) tools/list
    let list = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    writeln!(stdin, "{}", list).expect("write tools/list");
    let resp2 = read_response_for_id(&mut reader, 2);
    let tools = resp2
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("no tools array: {resp2}"));
    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();
    for expected in [
        "convert",
        "convert_file",
        "convert_fragment",
        "list_skills",
        "read_skill",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "tool '{expected}' missing; got {names:?}"
        );
    }

    // 4) tools/call list_skills
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": { "name": "list_skills", "arguments": {} }
    });
    writeln!(stdin, "{}", call).expect("write list_skills call");
    let resp3 = read_response_for_id(&mut reader, 3);
    let content = resp3
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("no content text: {resp3}"));
    assert!(content.contains("byetex-using-warnings-json"));

    // 5) tools/call convert
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "convert",
            "arguments": { "tex": "Hello \\textbf{world}." }
        }
    });
    writeln!(stdin, "{}", call).expect("write convert call");
    let resp4 = read_response_for_id(&mut reader, 4);
    let content = resp4
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("no content text: {resp4}"));
    assert!(
        content.contains("*world*"),
        "expected `*world*` in converted output; got: {content}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
