//! Property-based tests: panic-freedom and warning offset invariants.

use byetex_core::{convert, ConvertOptions};
use proptest::prelude::*;

fn opts() -> ConvertOptions {
    ConvertOptions {
        source_name: Some("proptest".into()),
        ..Default::default()
    }
}

// Small alphabet that covers common LaTeX constructs without requiring unicode.
static CHARS: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\\{}[]()$_^&%#~";

fn arb_source() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(CHARS.chars().collect::<Vec<_>>()),
        0..=200,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

    #[test]
    fn prop_convert_never_panics(src in arb_source()) {
        // convert() must never panic regardless of input. The output may be
        // garbage; only the panic-freedom invariant is asserted.
        let _ = convert(&src, &opts());
    }

    #[test]
    fn prop_warnings_have_valid_byte_offsets(src in arb_source()) {
        // Every warning's byte range must be non-inverted and within the
        // source length. Guards against off-by-one regressions in warning
        // offset bookkeeping.
        let src_len = src.len() as u32;
        let out = convert(&src, &opts());
        for w in &out.warnings {
            prop_assert!(
                w.range.byte_start <= w.range.byte_end,
                "byte_start > byte_end in warning: {:?}",
                w
            );
            prop_assert!(
                w.range.byte_end <= src_len,
                "byte_end ({}) > source length ({}) in warning: {:?}",
                w.range.byte_end,
                src_len,
                w
            );
        }
    }
}
