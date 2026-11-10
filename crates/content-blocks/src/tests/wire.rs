//! Wire format (`type` tags, key names, and strictness) and builder output shapes.

use crate::{
    Accordion,
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
    Image,
    Link,
    List,
    ListStyle,
    Prominence,
    Qr,
    Section,
    Span,
    Table,
    Text,
    Variant,
    blocks,
    blocks_to_json,
    parse_blocks,
};

/// Serializes one block to JSON and roundtrips it.
fn roundtrip(block: Block) -> String {
    block.validate().expect("validate");
    let json = blocks_to_json(std::slice::from_ref(&block));
    assert_eq!(block, parse_blocks(&json).expect("reparse")[0], "roundtrip must be value-preserving");
    json
}

// ------------------------------------------------------------------
// Strictness: unknown types, unknown fields, and missing required fields
// ------------------------------------------------------------------

#[test]
fn rejects_unknown_type() {
    let err = parse_blocks(r#"[{"type":"video","url":"https://x"}]"#).expect_err("unknown type");
    assert!(matches!(err, ContentBlocksError::Parse(_)), "{err}");
}

#[test]
fn rejects_unknown_field() {
    let err = parse_blocks(r#"[{"type":"heading","text":"x","level":1,"color":"red"}]"#).expect_err("unknown field");
    assert!(matches!(err, ContentBlocksError::Parse(_)), "{err}");
}

#[test]
fn rejects_unknown_span_field() {
    let err = parse_blocks(r#"[{"type":"text","spans":[{"text":"x","bold":true}]}]"#).expect_err("unknown span field");
    assert!(matches!(err, ContentBlocksError::Parse(_)), "{err}");
}

#[test]
fn rejects_unknown_field_on_divider() {
    // A divider has no fields and rejects extras instead of ignoring them. This is why it uses an
    // empty struct with deny_unknown_fields rather than a unit variant.
    let err = parse_blocks(r#"[{"type":"divider","x":1}]"#).expect_err("extra field on divider");
    assert!(matches!(err, ContentBlocksError::Parse(_)), "{err}");
}

#[test]
fn rejects_unknown_field_on_card_image() {
    // Card images and links share their block types, including the same strict parsing rules.
    let err = parse_blocks(
        r#"[{"type":"card","title":"t","image":{"url":"https://cdn.example.com/a.png","alt":"a","w":1}}]"#,
    )
    .expect_err("extra field on card.image");
    assert!(matches!(err, ContentBlocksError::Parse(_)), "{err}");
}

#[test]
fn rejects_image_without_alt() {
    // The schema requires alt text, so omission is a parse error.
    let err = parse_blocks(r#"[{"type":"image","url":"https://cdn.example.com/a.png"}]"#).expect_err("missing alt");
    assert!(matches!(err, ContentBlocksError::Parse(_)), "{err}");
}

// ------------------------------------------------------------------
// Roundtrips and key names for each type
// ------------------------------------------------------------------

#[test]
fn heading_and_text_roundtrip() {
    let json = roundtrip(Heading::new(1, "ご注文ありがとうございます").into());
    assert_eq!(json, r#"[{"type":"heading","text":"ご注文ありがとうございます","level":1}]"#);

    let json = roundtrip(Text::spans([Span::plain("a"), Span::strong("b"), Span::emphasis("c")]).into());
    assert!(json.contains(r#"{"text":"a"}"#), "{json}");
    assert!(json.contains(r#"{"text":"b","strong":true}"#), "{json}");
    assert!(json.contains(r#"{"text":"c","emphasis":true}"#), "{json}");

    let json = roundtrip(Text::plain("1 span だけ").into());
    assert_eq!(json, r#"[{"type":"text","spans":[{"text":"1 span だけ"}]}]"#);
}

#[test]
fn fields_builder_takes_str_pairs() {
    let json = roundtrip(Fields::new([("注文番号", "P0001"), ("合計", "1,200円")]).into());
    assert_eq!(
        json,
        r#"[{"type":"fields","entries":[{"label":"注文番号","value":"P0001"},{"label":"合計","value":"1,200円"}]}]"#
    );
}

#[test]
fn copyable_label_is_optional() {
    let json = roundtrip(Copyable::new("CPN-1234").into());
    assert_eq!(json, r#"[{"type":"copyable","value":"CPN-1234"}]"#);

    let json = roundtrip(Copyable::new("CPN-1234").label("クーポンコード").into());
    assert!(json.contains(r#""label":"クーポンコード""#), "{json}");
}

#[test]
fn link_prominence_serializes_lowercase() {
    let json = roundtrip(
        Link::new("詳細", "https://example.com/a")
            .prominence(Prominence::Secondary)
            .into(),
    );
    assert!(json.contains(r#""prominence":"secondary""#), "{json}");
}

#[test]
fn divider_serializes_without_optional_fields() {
    assert_eq!(blocks_to_json(&[Divider::new().into()]), r#"[{"type":"divider"}]"#);
}

#[test]
fn table_builder_roundtrips() {
    let json = roundtrip(
        Table::columns(["商品", "数量"])
            .row(["りんご", "2"])
            .row(["みかん", "3"])
            .into(),
    );
    assert_eq!(json, r#"[{"type":"table","columns":["商品","数量"],"rows":[["りんご","2"],["みかん","3"]]}]"#);
}

#[test]
fn table_builder_rows_takes_iterator() {
    let table = Table::columns(["c"]).rows((0..3).map(|i| [i.to_string()]));
    let json = roundtrip(table.into());
    assert!(json.contains(r#""rows":[["0"],["1"],["2"]]"#), "{json}");
}

#[test]
fn table_try_new_rejects_row_column_mismatch() {
    // Only the runtime-sized path returns Result; the builder fixes the column count at compile time.
    let err = Table::try_new(vec!["a".to_string(), "b".to_string()], vec![vec!["only-one".to_string()]])
        .expect_err("mismatch");
    assert!(
        matches!(
            err,
            ContentBlocksError::TableRowColumnMismatch {
                row: 0,
                expected: 2,
                got: 1,
            }
        ),
        "{err}"
    );
}

#[test]
fn validate_rejects_table_row_column_mismatch_from_wire() {
    // Parsing succeeds, but validation rejects the mismatched column count.
    let blocks = parse_blocks(r#"[{"type":"table","columns":["a","b"],"rows":[["x","y"],["z"]]}]"#).expect("parse");
    let err = crate::validate_blocks(&blocks).expect_err("mismatch");
    assert!(
        matches!(
            err,
            ContentBlocksError::TableRowColumnMismatch {
                row: 1,
                expected: 2,
                got: 1,
            }
        ),
        "{err}"
    );
}

#[test]
fn list_roundtrips_and_helpers() {
    let json = roundtrip(List::new(ListStyle::Ordered, [vec![Span::plain("a")], vec![Span::strong("b")]]).into());
    assert!(json.contains(r#""style":"ordered""#), "{json}");

    let json = roundtrip(List::bullet(["一つ", "二つ"]).into());
    assert_eq!(json, r#"[{"type":"list","style":"bullet","items":[[{"text":"一つ"}],[{"text":"二つ"}]]}]"#);

    let json = roundtrip(List::ordered(["一つ"]).into());
    assert!(json.contains(r#""style":"ordered""#), "{json}");
}

#[test]
fn card_title_only_is_valid() {
    assert_eq!(roundtrip(Card::new("おすすめ").into()), r#"[{"type":"card","title":"おすすめ"}]"#);
}

#[test]
fn card_full_roundtrips_with_link_prominence() {
    let card = Card::new("おすすめ")
        .body([Span::plain("本文")])
        .image(Image::new("https://cdn.example.com/c.png", "カード画像"))
        .link(Link::new("詳細", "https://example.com/card").prominence(Prominence::Primary));
    let json = roundtrip(card.into());
    // A card's image and link are objects without type tags.
    assert!(json.contains(r#""image":{"url":"https://cdn.example.com/c.png","alt":"カード画像"}"#), "{json}");
    assert!(json.contains(r#""prominence":"primary""#), "{json}");
}

#[test]
fn alert_variant_serializes_lowercase() {
    let json = roundtrip(Alert::new(Variant::Warning, blocks![Divider::new()]).into());
    assert!(json.contains(r#"{"type":"alert","variant":"warning""#), "{json}");
}

#[test]
fn variant_auto_roundtrips() {
    let blocks = parse_blocks(r#"[{"type":"alert","variant":"auto","blocks":[{"type":"divider"}]}]"#).expect("parse");
    crate::validate_blocks(&blocks).expect("validate");
    assert!(matches!(
        &blocks[0],
        Block::Alert(alert) if alert.variant == Variant::Auto
    ));
    let reserialized = blocks_to_json(&blocks);
    assert!(reserialized.contains(r#""variant":"auto""#), "{reserialized}");
    assert_eq!(blocks, parse_blocks(&reserialized).expect("reparse"));
}

#[test]
fn section_and_grid_roundtrip_with_kebab_case_keys() {
    let section = Section::new(blocks![
        Heading::new(2, "おすすめ"),
        Grid::new(blocks![Card::new("特集 A"), Card::new("特集 B")]).columns_hint(2),
    ])
    .heading("特集");
    let json = roundtrip(section.into());
    assert!(json.contains(r#""type":"section""#), "{json}");
    assert!(json.contains(r#""heading":"特集""#), "{json}");
    // Multiword fields use kebab-case; snake_case keys must not appear.
    assert!(json.contains(r#""columns-hint":2"#), "{json}");
    assert!(!json.contains("columns_hint"), "{json}");
}

#[test]
fn grid_accepts_card_and_image_items() {
    let grid = Grid::new(blocks![Card::new("c"), Image::new("https://cdn.example.com/a.png", "alt")]).columns_hint(3);
    let json = roundtrip(grid.into());
    assert!(json.contains(r#""columns-hint":3"#), "{json}");
}

#[test]
fn badge_roundtrips() {
    assert_eq!(
        roundtrip(Badge::new(Variant::Warning, "残少").into()),
        r#"[{"type":"badge","variant":"warning","text":"残少"}]"#
    );
}

#[test]
fn qr_roundtrips_with_caption() {
    let json = roundtrip(
        Qr::new("https://example.com/redeem/abc")
            .caption("店舗でご提示ください")
            .into(),
    );
    assert!(json.contains(r#""value":"https://example.com/redeem/abc""#), "{json}");
    assert!(json.contains(r#""caption":"店舗でご提示ください""#), "{json}");
}

#[test]
fn accordion_roundtrips() {
    let accordion = Accordion::new("よくある質問", blocks![Text::plain("回答本文"), Divider::new()]);
    let json = roundtrip(accordion.into());
    assert!(json.contains(r#"{"type":"accordion","summary":"よくある質問""#), "{json}");
}

#[test]
fn blocks_macro_infers_target_slot_type() {
    // The same macro constructs Vec<Block>, Vec<Leaf>, Vec<GridItem>, and Vec<SectionChild>.
    let page: Vec<Block> = blocks![
        Alert::new(Variant::Info, blocks![Text::plain("leaf のみ")]),
        Grid::new(blocks![Card::new("card のみ")]),
        Section::new(blocks![Divider::new()]),
    ];
    assert_eq!(page.len(), 3);
    crate::validate_blocks(&page).expect("validate");
}
