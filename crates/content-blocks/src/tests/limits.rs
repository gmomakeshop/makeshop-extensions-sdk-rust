//! Byte-size, item-count, and numeric-range limits from [`crate::limits`].

use crate::{
    Alert,
    Badge,
    Block,
    Card,
    ContentBlocksError,
    Copyable,
    Divider,
    Fields,
    Grid,
    Heading,
    List,
    ListStyle,
    MAX_TEXT_BYTES,
    Qr,
    Span,
    Table,
    Text,
    Variant,
    blocks,
    parse_blocks,
};

/// Validates one block and returns its error.
fn err_of(block: impl Into<Block>) -> ContentBlocksError {
    block.into().validate().expect_err("must be rejected")
}

/// Verifies that one block passes validation.
fn accepts(block: impl Into<Block>) {
    block.into().validate().expect("must be accepted");
}

// ------------------------------------------------------------------
// Text byte sizes
// ------------------------------------------------------------------

#[test]
fn rejects_empty_text() {
    assert!(matches!(err_of(Heading::new(1, "")), ContentBlocksError::EmptyText { .. }));
}

#[test]
fn rejects_span_over_1024_bytes_ascii() {
    let err = err_of(Text::plain("a".repeat(1025)));
    assert!(
        matches!(
            err,
            ContentBlocksError::TextTooLong {
                got: 1025,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn rejects_span_over_1024_bytes_multibyte() {
    // 342 characters fit below 1024 characters, but occupy 1026 bytes and must be rejected.
    let text = "あ".repeat(342);
    assert!(text.chars().count() < MAX_TEXT_BYTES);
    assert!(text.len() > MAX_TEXT_BYTES);
    let err = err_of(Text::plain(text));
    assert!(
        matches!(
            err,
            ContentBlocksError::TextTooLong {
                got: 1026,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn accepts_exactly_1024_bytes() {
    accepts(Text::plain("a".repeat(1024)));
}

#[test]
fn rejects_badge_text_over_64_bytes() {
    let err = err_of(Badge::new(Variant::Auto, "a".repeat(65)));
    assert!(
        matches!(
            err,
            ContentBlocksError::TextTooLong {
                field: "badge.text",
                max: 64,
                got: 65,
            }
        ),
        "{err}"
    );
}

#[test]
fn badge_accepts_exactly_64_bytes() {
    accepts(Badge::new(Variant::Info, "a".repeat(64)));
}

#[test]
fn rejects_qr_value_over_512_bytes() {
    let err = err_of(Qr::new("a".repeat(513)));
    assert!(
        matches!(
            err,
            ContentBlocksError::TextTooLong {
                field: "qr.value",
                max: 512,
                got: 513,
            }
        ),
        "{err}"
    );
}

#[test]
fn optional_text_fields_are_checked_too() {
    assert!(matches!(err_of(Copyable::new("v").label("")), ContentBlocksError::EmptyText { .. }));
    assert!(matches!(err_of(Qr::new("v").caption("")), ContentBlocksError::EmptyText { .. }));
}

// ------------------------------------------------------------------
// Item counts
// ------------------------------------------------------------------

#[test]
fn rejects_fields_entries_out_of_range() {
    let over: Vec<(String, String)> = (0..17)
        .map(|i| (format!("l{i}"), format!("v{i}")))
        .collect();
    let err = err_of(Fields::new(over));
    assert!(
        matches!(
            err,
            ContentBlocksError::CountOutOfRange {
                field: "fields.entries",
                max: 16,
                got: 17,
            }
        ),
        "{err}"
    );

    let err = err_of(Fields::new(Vec::<(String, String)>::new()));
    assert!(
        matches!(
            err,
            ContentBlocksError::CountOutOfRange {
                field: "fields.entries",
                got: 0,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn rejects_table_columns_and_rows_out_of_range() {
    let over_rows = Table::try_new(vec!["a".to_string()], (0..21).map(|i| vec![i.to_string()]).collect())
        .expect("row width matches");
    let err = err_of(over_rows);
    assert!(
        matches!(
            err,
            ContentBlocksError::CountOutOfRange {
                field: "table.rows",
                max: 20,
                got: 21,
            }
        ),
        "{err}"
    );

    let columns: Vec<String> = (0..7).map(|i| format!("c{i}")).collect();
    let over_columns =
        Table::try_new(columns, vec![(0..7).map(|i| i.to_string()).collect()]).expect("row width matches");
    let err = err_of(over_columns);
    assert!(
        matches!(
            err,
            ContentBlocksError::CountOutOfRange {
                field: "table.columns",
                max: 6,
                got: 7,
            }
        ),
        "{err}"
    );
}

#[test]
fn rejects_list_items_out_of_range() {
    let err = err_of(List::bullet(vec!["x"; 17]));
    assert!(
        matches!(
            err,
            ContentBlocksError::CountOutOfRange {
                field: "list.items",
                max: 16,
                got: 17,
            }
        ),
        "{err}"
    );
}

#[test]
fn rejects_grid_items_out_of_range() {
    let over: Vec<crate::GridItem> = (0..13)
        .map(|i| Card::new(format!("カード {i}")).into())
        .collect();
    let err = err_of(Grid::new(over));
    assert!(
        matches!(
            err,
            ContentBlocksError::CountOutOfRange {
                field: "grid.items",
                max: 12,
                got: 13,
            }
        ),
        "{err}"
    );

    let err = err_of(Grid::new(blocks![]));
    assert!(
        matches!(
            err,
            ContentBlocksError::CountOutOfRange {
                field: "grid.items",
                got: 0,
                ..
            }
        ),
        "{err}"
    );
}

// ------------------------------------------------------------------
// Non-empty arrays
// ------------------------------------------------------------------

#[test]
fn rejects_empty_spans_and_children() {
    assert!(matches!(
        err_of(Text::spans([])),
        ContentBlocksError::EmptyList {
            field: "text.spans"
        }
    ));
    assert!(matches!(
        err_of(List::new(ListStyle::Bullet, [Vec::<Span>::new()])),
        ContentBlocksError::EmptyList {
            field: "list.item.spans"
        }
    ));
    assert!(matches!(
        err_of(Card::new("t").body([])),
        ContentBlocksError::EmptyList {
            field: "card.body"
        }
    ));
    assert!(matches!(
        err_of(Alert::new(Variant::Info, blocks![])),
        ContentBlocksError::EmptyList {
            field: "alert.blocks"
        }
    ));
}

#[test]
fn rejects_empty_accordion_and_section_from_wire() {
    let blocks = parse_blocks(r#"[{"type":"accordion","summary":"q","blocks":[]}]"#).expect("parse");
    let err = crate::validate_blocks(&blocks).expect_err("empty accordion");
    assert!(
        matches!(
            err,
            ContentBlocksError::EmptyList {
                field: "accordion.blocks"
            }
        ),
        "{err}"
    );

    let blocks = parse_blocks(r#"[{"type":"section","blocks":[]}]"#).expect("parse");
    let err = crate::validate_blocks(&blocks).expect_err("empty section");
    assert!(
        matches!(
            err,
            ContentBlocksError::EmptyList {
                field: "section.blocks"
            }
        ),
        "{err}"
    );
}

// ------------------------------------------------------------------
// Numeric ranges
// ------------------------------------------------------------------

#[test]
fn rejects_heading_level_out_of_range() {
    let err = err_of(Heading::new(4, "x"));
    assert!(
        matches!(
            err,
            ContentBlocksError::NumberOutOfRange {
                field: "heading.level",
                min: 1,
                max: 3,
                got: 4,
            }
        ),
        "{err}"
    );
    accepts(Heading::new(3, "x"));
}

#[test]
fn rejects_grid_columns_hint_out_of_range() {
    let err = err_of(Grid::new(blocks![Card::new("c")]).columns_hint(5));
    assert!(
        matches!(
            err,
            ContentBlocksError::NumberOutOfRange {
                field: "grid.columns-hint",
                min: 2,
                max: 4,
                got: 5,
            }
        ),
        "{err}"
    );
    accepts(Grid::new(blocks![Card::new("c")]).columns_hint(2));
}

#[test]
fn nested_children_are_validated() {
    // Validation descends into container children and reports their violations.
    let err = err_of(Alert::new(Variant::Info, blocks![Heading::new(9, "x"), Divider::new()]));
    assert!(
        matches!(
            err,
            ContentBlocksError::NumberOutOfRange {
                field: "heading.level",
                got: 9,
                ..
            }
        ),
        "{err}"
    );
}
