//! Destructive-behaviour guards for `byetex diagnose`.
//!
//! * Review finding #5: `diagnose --project --out DIR` materialised with
//!   `force = true` hardcoded, so it wiped every existing file in a
//!   user-supplied directory. `convert --project` gates the same wipe behind
//!   `--force`; `diagnose` must too.
//! * Review finding #9: diagnosing a `.typ` in place compiled it to a sibling
//!   `<stem>.pdf` and then deleted that file — clobbering a pre-existing PDF
//!   (the user's own render, and exactly the name `byetex review` looks for as
//!   a cached truth reference). The command documents the `.typ` as left
//!   untouched; nothing beside it should be destroyed either.

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

fn typst_available() -> bool {
    Command::new(std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".into()))
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

const DOC: &str = "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n";

#[test]
fn diagnose_project_refuses_to_wipe_a_non_empty_out_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let tex = tmp.path().join("paper.tex");
    std::fs::write(&tex, DOC).unwrap();

    let out = tmp.path().join("existing");
    std::fs::create_dir_all(&out).unwrap();
    let precious = out.join("precious.txt");
    std::fs::write(&precious, b"do not delete me").unwrap();

    let status = Command::new(bin())
        .arg("diagnose")
        .arg(&tex)
        .arg("--project")
        .arg("--out")
        .arg(&out)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run byetex diagnose");

    assert!(
        !status.success(),
        "diagnose should refuse a non-empty --out without --force"
    );
    assert!(
        precious.exists(),
        "diagnose deleted a pre-existing file in the --out directory"
    );
}

#[test]
fn diagnose_project_force_allows_the_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let tex = tmp.path().join("paper.tex");
    std::fs::write(&tex, DOC).unwrap();

    let out = tmp.path().join("existing");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("stale.txt"), b"x").unwrap();

    let status = Command::new(bin())
        .arg("diagnose")
        .arg(&tex)
        .arg("--project")
        .arg("--force")
        .arg("--out")
        .arg(&out)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run byetex diagnose --force");

    assert!(status.success(), "--force should proceed; got {status:?}");
    assert!(out.join("main.typ").is_file(), "project not materialised");
}

#[test]
fn diagnose_typ_does_not_delete_a_neighbouring_pdf() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let typ = tmp.path().join("main.typ");
    std::fs::write(&typ, "Hello from typst.\n").unwrap();
    // The user's own render (also the filename `byetex review` treats as a
    // cached truth reference).
    let pdf = tmp.path().join("main.pdf");
    std::fs::write(&pdf, b"%PDF-1.7 user's own file").unwrap();

    let status = Command::new(bin())
        .arg("diagnose")
        .arg(&typ)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run byetex diagnose on a .typ");
    assert!(status.success(), "diagnose exited {status:?}");

    assert!(pdf.exists(), "diagnose deleted the neighbouring main.pdf");
    assert_eq!(
        std::fs::read(&pdf).unwrap(),
        b"%PDF-1.7 user's own file",
        "diagnose overwrote the neighbouring main.pdf"
    );
}
