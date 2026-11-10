//! Aggregate fragment validation for counts, placements, sizes, and total blocks.

use super::{PLACEMENTS, VALID_BLOCKS_JSON, frag};
use crate::{
    Alert,
    Block,
    Card,
    ContentBlocksError,
    Divider,
    Grid,
    Heading,
    MAX_FRAGMENT_BLOCKS_JSON_BYTES,
    Section,
    Variant,
    blocks,
    blocks_to_json,
    validate_fragments,
};

#[test]
fn accepts_valid_fragments() {
    validate_fragments(&[frag("main", VALID_BLOCKS_JSON)], PLACEMENTS).expect("valid fragments");
}

#[test]
fn rejects_over_8_fragments() {
    let json = r#"[{"type":"divider"}]"#;
    let fragments: Vec<_> = (0..9).map(|_| frag("main", json)).collect();
    let err = validate_fragments(&fragments, PLACEMENTS).expect_err("9 fragments");
    assert!(
        matches!(
            err,
            ContentBlocksError::TooManyFragments {
                max: 8,
                got: 9,
            }
        ),
        "{err}"
    );
}

#[test]
fn rejects_unknown_placement() {
    let err =
        validate_fragments(&[frag("sidebar", r#"[{"type":"divider"}]"#)], PLACEMENTS).expect_err("unknown placement");
    assert!(matches!(err, ContentBlocksError::UnknownPlacement { .. }), "{err}");
}

#[test]
fn rejects_blocks_json_over_8kib() {
    let big_text = "a".repeat(1000);
    let blocks: Vec<Block> = (0..9)
        .map(|_| Heading::new(1, big_text.clone()).into())
        .collect();
    let json = blocks_to_json(&blocks);
    assert!(json.len() > MAX_FRAGMENT_BLOCKS_JSON_BYTES);
    let err = validate_fragments(&[frag("main", &json)], PLACEMENTS).expect_err("8KiB");
    assert!(matches!(err, ContentBlocksError::FragmentTooLarge { .. }), "{err}");
}

#[test]
fn rejects_over_64_blocks_in_one_fragment() {
    let blocks: Vec<Block> = (0..65).map(|_| Divider::new().into()).collect();
    let json = blocks_to_json(&blocks);
    let err = validate_fragments(&[frag("main", &json)], PLACEMENTS).expect_err("65 blocks");
    assert!(
        matches!(
            err,
            ContentBlocksError::TooManyBlocks {
                max: 64,
                got: 65,
            }
        ),
        "{err}"
    );
}

#[test]
fn block_total_counts_alert_children() {
    // One alert and two children make 3 blocks; 62 dividers bring the total to 65.
    let mut blocks: Vec<Block> = blocks![Alert::new(Variant::Info, blocks![Divider::new(), Divider::new()])];
    blocks.extend((0..62).map(|_| Block::from(Divider::new())));
    let json = blocks_to_json(&blocks);
    let err = validate_fragments(&[frag("main", &json)], PLACEMENTS).expect_err("nested counts");
    assert!(
        matches!(
            err,
            ContentBlocksError::TooManyBlocks {
                got: 65,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn block_total_counts_section_and_grid_nesting() {
    // One section, one grid, and two cards make 4 blocks; 61 dividers bring the total to 65.
    let mut blocks: Vec<Block> = blocks![Section::new(blocks![Grid::new(blocks![Card::new("A"), Card::new("B")])])];
    blocks.extend((0..61).map(|_| Block::from(Divider::new())));
    let json = blocks_to_json(&blocks);
    let err = validate_fragments(&[frag("main", &json)], PLACEMENTS).expect_err("nested counts");
    assert!(
        matches!(
            err,
            ContentBlocksError::TooManyBlocks {
                got: 65,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn block_total_sums_across_fragments() {
    // Two fragments with 32 blocks each meet the limit of 64; one more block exceeds it.
    let thirty_two = blocks_to_json(
        &(0..32)
            .map(|_| Block::from(Divider::new()))
            .collect::<Vec<_>>(),
    );
    validate_fragments(&[frag("main", &thirty_two), frag("main", &thirty_two)], PLACEMENTS)
        .expect("64 blocks across fragments is the limit");

    let one = blocks_to_json(&[Block::from(Divider::new())]);
    let err =
        validate_fragments(&[frag("main", &thirty_two), frag("main", &thirty_two), frag("main", &one)], PLACEMENTS)
            .expect_err("65 blocks across fragments");
    assert!(
        matches!(
            err,
            ContentBlocksError::TooManyBlocks {
                got: 65,
                ..
            }
        ),
        "{err}"
    );
}
