//! Column-packing heuristic for subpar.grid (Thread 5). Sub-block width
//! fractions are greedily packed into rows of cumulative width <= ~1.05; the
//! column count is the widest row's block count. No widths => single column.
use byetex_core::emit_testing::columns_for_widths;

#[test]
fn two_half_width_blocks_make_two_columns() {
    assert_eq!(columns_for_widths(&[Some(0.41), Some(0.58)]), 2);
}

#[test]
fn three_third_width_blocks_make_three_columns() {
    assert_eq!(columns_for_widths(&[Some(0.32), Some(0.32), Some(0.32)]), 3);
}

#[test]
fn third_plus_two_thirds_pack_into_two() {
    assert_eq!(columns_for_widths(&[Some(0.32), Some(0.65)]), 2);
}

#[test]
fn no_widths_is_single_column() {
    assert_eq!(columns_for_widths(&[None, None]), 1);
}

#[test]
fn overflowing_widths_wrap_to_max_row_count() {
    // 0.5 + 0.5 fills a row; the third starts a new row → max row = 2.
    assert_eq!(columns_for_widths(&[Some(0.5), Some(0.5), Some(0.5)]), 2);
}
