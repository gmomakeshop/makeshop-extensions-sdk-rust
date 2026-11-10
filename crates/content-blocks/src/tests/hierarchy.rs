//! Container hierarchy defined by [`crate::container`].
//!
//! [`crate::Leaf`], [`crate::GridItem`], and [`crate::SectionChild`] close the hierarchy at the
//! type level. As a result:
//!
//! - Builder violations are **compile errors** because no matching `From` implementation exists.
//!   For example, `Alert::new(Variant::Info, blocks![Grid::new(…)])` has no `From<Grid> for Leaf`,
//!   and `Section::new(blocks![Section::new(…)])` fails for the same reason.
//! - Wire-format violations are rejected **during parsing**, as exercised by the tests below.
//!
//! Validation therefore needs no hierarchy check and exposes no `DisallowedChild`-style error.

use crate::{
    Accordion,
    Alert,
    Badge,
    Block,
    Card,
    ContentBlocksError,
    Divider,
    Grid,
    Image,
    List,
    Qr,
    Section,
    Table,
    Text,
    Variant,
    blocks,
    parse_blocks,
};

/// Verifies that parsing fails and reports both the offending type and the reason.
fn assert_parse_rejects(blocks_json: &str, offending_type: &str, reason: &str) {
    let err = parse_blocks(blocks_json).expect_err("hierarchy violation must be rejected at parse time");
    assert!(matches!(err, ContentBlocksError::Parse(_)), "{err}");
    let message = err.to_string();
    assert!(message.contains(&format!("'{offending_type}'")), "message must name the offending type: {message}");
    assert!(message.contains(reason), "message must explain the rule: {message}");
}

#[test]
fn rejects_nested_alert() {
    assert_parse_rejects(
        r#"[{"type":"alert","variant":"info","blocks":[
            {"type":"alert","variant":"warning","blocks":[{"type":"divider"}]}
        ]}]"#,
        "alert",
        "only leaf blocks",
    );
}

#[test]
fn rejects_grid_inside_alert() {
    assert_parse_rejects(
        r#"[{"type":"alert","variant":"info","blocks":[
            {"type":"grid","items":[{"type":"card","title":"t"}]}
        ]}]"#,
        "grid",
        "only leaf blocks",
    );
}

#[test]
fn rejects_alert_inside_accordion() {
    assert_parse_rejects(
        r#"[{"type":"accordion","summary":"q","blocks":[
            {"type":"alert","variant":"info","blocks":[{"type":"divider"}]}
        ]}]"#,
        "alert",
        "only leaf blocks",
    );
}

#[test]
fn rejects_nested_section() {
    assert_parse_rejects(
        r#"[{"type":"section","blocks":[{"type":"section","blocks":[{"type":"divider"}]}]}]"#,
        "section",
        "section cannot be nested",
    );
}

#[test]
fn rejects_text_as_grid_item() {
    assert_parse_rejects(
        r#"[{"type":"grid","items":[{"type":"text","spans":[{"text":"x"}]}]}]"#,
        "text",
        "only card or image",
    );
}

#[test]
fn rejects_grid_inside_grid() {
    // Grid items are limited to cards and images, so section > grid > grid cannot be represented.
    assert_parse_rejects(
        r#"[{"type":"section","blocks":[
            {"type":"grid","items":[{"type":"grid","items":[{"type":"card","title":"t"}]}]}
        ]}]"#,
        "grid",
        "only card or image",
    );
}

#[test]
fn accepts_all_leaf_types_inside_alert() {
    // All 12 leaf types can be alert children; tables, lists, and cards contain no child blocks.
    let alert = Alert::new(
        Variant::Auto,
        blocks![
            Text::plain("案内"),
            List::bullet(["a"]),
            Table::columns(["c"]).row(["v"]),
            Card::new("t"),
            Badge::new(Variant::Success, "在庫あり"),
            Qr::new("https://example.com/x"),
            Divider::new(),
        ],
    );
    Block::from(alert).validate().expect("validate");
}

#[test]
fn accepts_grid_alert_accordion_and_leaf_inside_section() {
    let section = Section::new(blocks![
        Divider::new(),
        Grid::new(blocks![Card::new("c"), Image::new("https://cdn.example.com/a.png", "alt")]),
        Alert::new(Variant::Info, blocks![Divider::new()]),
        Accordion::new("q", blocks![Divider::new()]),
    ]);
    Block::from(section).validate().expect("validate");
}

#[test]
fn max_depth_three_is_section_grid_card() {
    let section = Section::new(blocks![Grid::new(blocks![Card::new("c1"), Card::new("c2")])]);
    Block::from(section).validate().expect("depth 3 ok");
}

#[test]
fn nested_child_slots_parse_through_three_levels() {
    // section (SectionChild) > alert (Leaf) > leaf verifies that child-slot deserialization
    // composes across nested internally tagged enums.
    let json = r#"[{"type":"section","heading":"h","blocks":[
        {"type":"alert","variant":"info","blocks":[{"type":"copyable","value":"CPN"}]}
    ]}]"#;
    let blocks = parse_blocks(json).expect("parse");
    crate::validate_blocks(&blocks).expect("validate");

    let Block::Section(section) = &blocks[0] else {
        panic!("expected section, got {}", blocks[0].type_name());
    };
    let Block::Alert(alert) = section.blocks[0].as_block() else {
        panic!("expected alert as section child");
    };
    assert_eq!(alert.blocks[0].as_block().type_name(), "copyable");
}

#[test]
fn grid_item_is_also_a_leaf() {
    // Cards and images are both grid items and leaves, so either child slot accepts them.
    let alert = Alert::new(Variant::Info, blocks![Card::new("t")]);
    let grid = Grid::new(blocks![Card::new("t")]);
    assert_eq!(alert.blocks[0].as_block().type_name(), "card");
    assert_eq!(grid.items[0].as_block().type_name(), "card");
}

#[test]
fn block_count_includes_nested_children() {
    // section(1) + grid(1) + card×2 = 4。
    let section: Block = Section::new(blocks![Grid::new(blocks![Card::new("A"), Card::new("B")])]).into();
    assert_eq!(section.count(), 4);
    assert_eq!(Block::from(Divider::new()).count(), 1);
    // Tables, lists, and cards contain no child blocks, so each count is one.
    assert_eq!(Block::from(Table::columns(["c"]).row(["v"])).count(), 1);
}
