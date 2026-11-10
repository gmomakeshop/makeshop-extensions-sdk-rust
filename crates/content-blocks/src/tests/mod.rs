//! Tests for content blocks.
//!
//! This module contains the all-type roundtrip and fixture/JSON Schema consistency checks.
//! Submodules group individual rules by concern.
//!
//! The JSON under `fixtures/` mirrors canonical files outside this crate. Referencing files above
//! the crate root with `include_str!` would require shipping them for tests in the packaged crate.
//! CI verifies that the local copies remain synchronized with their canonical files.

mod fragment;
mod hierarchy;
mod limits;
mod url;
mod wire;

use crate::{Block, FragmentInput, SCHEMA_VERSION, blocks_to_json, parse_blocks, validate_fragments};

/// Valid blocks JSON covering all 16 block types.
///
/// This constant fixes the wire format and must remain unchanged when Rust types are reorganized;
/// changing it would indicate a wire-compatibility break.
pub(crate) const VALID_BLOCKS_JSON: &str = r#"[
  {"type":"heading","text":"ご注文ありがとうございます","level":1},
  {"type":"text","spans":[
    {"text":"通常テキスト"},
    {"text":"強調","strong":true},
    {"text":"斜体","emphasis":true}
  ]},
  {"type":"fields","entries":[
    {"label":"注文番号","value":"P0001"},
    {"label":"合計","value":"1,200円"}
  ]},
  {"type":"copyable","value":"CPN-1234","label":"クーポンコード"},
  {"type":"link","text":"詳細を見る","url":"https://example.com/promo","prominence":"primary"},
  {"type":"image","url":"https://cdn.example.com/a.png","alt":"キャンペーン画像"},
  {"type":"divider"},
  {"type":"table","columns":["商品","数量"],"rows":[["りんご","2"],["みかん","3"]]},
  {"type":"list","style":"bullet","items":[
    [{"text":"1つめ"}],
    [{"text":"2つめ","strong":true}]
  ]},
  {"type":"card","title":"おすすめ","body":[{"text":"本文"}],
    "image":{"url":"https://cdn.example.com/c.png","alt":"カード画像"},
    "link":{"text":"詳細","url":"https://example.com/card","prominence":"secondary"}},
  {"type":"alert","variant":"success","blocks":[
    {"type":"text","spans":[{"text":"クーポンを配布しました"}]},
    {"type":"copyable","value":"CPN-9999"}
  ]},
  {"type":"section","heading":"おすすめ商品","blocks":[
    {"type":"grid","columns-hint":2,"items":[
      {"type":"card","title":"スタッフおすすめ A"},
      {"type":"card","title":"スタッフおすすめ B"}
    ]}
  ]},
  {"type":"grid","columns-hint":3,"items":[
    {"type":"card","title":"関連特集"},
    {"type":"image","url":"https://cdn.example.com/grid.png","alt":"関連商品画像"}
  ]},
  {"type":"badge","variant":"success","text":"在庫あり"},
  {"type":"qr","value":"https://example.com/redeem/abc123","caption":"レジでご提示ください"},
  {"type":"accordion","summary":"返品・交換について","blocks":[
    {"type":"text","spans":[{"text":"商品到着後7日以内にご連絡ください。"}]}
  ]}
]"#;

pub(crate) const PLACEMENTS: &[&str] = &["main"];

pub(crate) fn frag<'a>(placement: &'a str, blocks_json: &'a str) -> FragmentInput<'a> {
    FragmentInput {
        placement,
        blocks_json,
    }
}

#[test]
fn valid_blocks_roundtrip() {
    // deserialize -> validate -> serialize -> deserialize preserves equality.
    let blocks = parse_blocks(VALID_BLOCKS_JSON).expect("parse");
    crate::validate_blocks(&blocks).expect("validate");
    assert_eq!(blocks.len(), 16);

    let json = blocks_to_json(&blocks);
    let reparsed = parse_blocks(&json).expect("reparse");
    assert_eq!(blocks, reparsed, "roundtrip must be value-preserving");
}

#[test]
fn renderer_fixture_blocks_json_is_valid() {
    let json = include_str!("fixtures/order-completed-main.blocks.json");
    let blocks = parse_blocks(json).expect("parse renderer fixture blocks-json");
    crate::validate_blocks(&blocks).expect("validate renderer fixture blocks-json");
    assert!(
        blocks
            .iter()
            .any(|block| matches!(block, Block::Section(_))),
        "fixture should include container blocks for renderer coverage"
    );
}

#[test]
fn hook_output_fixture_contains_valid_fragments() {
    let json = include_str!("fixtures/order-completed-output.json");
    let value: serde_json::Value = serde_json::from_str(json).expect("parse hook output fixture");
    assert_eq!(value["schema-version"], SCHEMA_VERSION);

    let fragments = value["fragments"]
        .as_array()
        .expect("fragments must be an array")
        .iter()
        .map(|fragment| {
            let placement = fragment["placement"].as_str().expect("placement string");
            let blocks_json = fragment["blocks-json"]
                .as_str()
                .expect("blocks-json string");
            frag(placement, blocks_json)
        })
        .collect::<Vec<_>>();

    validate_fragments(&fragments, PLACEMENTS).expect("fixture fragments are valid");
}

#[test]
fn schema_embedded_examples_are_valid_blocks_json() {
    let json = include_str!("fixtures/content-blocks-v1.schema.json");
    let value: serde_json::Value = serde_json::from_str(json).expect("parse content-blocks schema");
    let examples = value["examples"].as_array().expect("schema examples array");

    for example in examples {
        let blocks: Vec<Block> = serde_json::from_value(example.clone()).expect("example is a block array");
        crate::validate_blocks(&blocks).expect("schema example validates");
    }
}

#[test]
fn schema_version_is_one() {
    assert_eq!(SCHEMA_VERSION, 1);
}
