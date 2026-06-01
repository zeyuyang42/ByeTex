//! Expanded-corpus defect (2605.31510): a BARE `algorithmic` environment (not
//! wrapped in an `algorithm` float) hit the unsupported-env arm and was dropped
//! whole — so `\State\label{alg:step:N}` labels were lost and `\cref{alg:step:N}`
//! references dangled (`label <alg:step:2> does not exist` → compile failure).
//! Pass the body through (like `multicols`) so the labels reach the orphan-label
//! anchor and the step text survives.

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn bare_algorithmic_labels_are_anchored() {
    // No enclosing \begin{algorithm} float — just \begin{algorithmic}.
    let src = "Steps \\cref{alg:step:1,alg:step:2} matter.\n\
        \\begin{algorithmic}[1]\n\
        \\State\\label{alg:step:1}\n\
        Sample the signals.\n\
        \\State\\label{alg:step:2}\n\
        Update the matrices.\n\
        \\end{algorithmic}\n";
    let t = typ(src);
    // Both referenced step labels must be anchored, or @alg:step:* dangles.
    assert!(t.contains("<alg:step:1>"), "alg:step:1 must be anchored; got:\n{t}");
    assert!(t.contains("<alg:step:2>"), "alg:step:2 must be anchored; got:\n{t}");
    // The step text must survive (body passed through, not dropped).
    assert!(
        t.contains("Sample the signals") && t.contains("Update the matrices"),
        "algorithmic body text must be preserved; got:\n{t}"
    );
}

#[test]
fn bare_algorithmic_body_not_dropped_silently() {
    let out = convert(
        "\\begin{algorithmic}\n\\State Compute the result.\n\\end{algorithmic}\n",
        &ConvertOptions::default(),
    );
    assert!(
        out.typst.contains("Compute the result"),
        "algorithmic content must be emitted, not dropped; got:\n{}",
        out.typst
    );
}

#[test]
fn algorithm_float_with_inputed_body_anchors_step_labels() {
    // The 2605.31510 shape: an `algorithm` float whose `algorithmic` body —
    // including `\State\label{alg:step:N}` — lives in an \input-ed file. Those
    // labels are not AST children of the float node, so the float emitter must
    // resolve the \input and harvest them, or `\cref{alg:step:N}` dangles.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::create_dir_all(root.join("Alg")).unwrap();
    fs::write(
        root.join("Alg/idanse.tex"),
        "\\begin{algorithmic}[1]\n\
         \\State\\label{alg:step:1}\nSample.\n\
         \\State\\label{alg:step:2}\nUpdate.\n\
         \\end{algorithmic}\n",
    )
    .unwrap();
    let main = "Steps \\cref{alg:step:1,alg:step:2}.\n\
        \\begin{algorithm}\n\\input{Alg/idanse}\n\\caption{iDANSE}\\label{alg:idanse}\n\\end{algorithm}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    );
    let t = &out.typst;
    assert!(t.contains("<alg:step:1>"), "input-ed alg:step:1 must anchor; got:\n{t}");
    assert!(t.contains("<alg:step:2>"), "input-ed alg:step:2 must anchor; got:\n{t}");
}
