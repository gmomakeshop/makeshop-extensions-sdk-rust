//! JSON Schema for the Content Blocks wire format.
//!
//! Schema constraints approximate UTF-8 byte limits with character limits. Runtime validation
//! also checks URL hygiene, table row widths, and limits across fragments.

use schemars::{Schema, SchemaGenerator, json_schema};

use crate::{Block, MAX_TABLE_COLUMNS, MAX_TEXT_BYTES, MAX_TOTAL_BLOCKS, SCHEMA_VERSION, Span};

/// Returns the Content Blocks schema, including payload metadata for code generators.
pub fn content_blocks_schema() -> Schema {
    let mut generator = SchemaGenerator::default();
    let payloads = Block::payload_schemas(&mut generator);
    let mut schema = generator.into_root_schema_for::<Vec<Block>>();
    schema.insert("$id".into(), "https://makeshop.dev/schemas/managed-runtime/content-blocks-v1.schema.json".into());
    schema.insert("title".into(), "ContentBlocksV1".into());
    schema.insert("maxItems".into(), MAX_TOTAL_BLOCKS.into());
    schema.insert("description".into(), "A single fragment's Content Blocks array. Runtime validation additionally checks UTF-8 byte limits, URL hygiene, table row widths, and limits across fragments.".into());
    schema.insert("x-schema-version".into(), SCHEMA_VERSION.into());
    schema.insert("x-block-payloads".into(), payloads);
    schema
        .insert("examples".into(), serde_json::json!([[Block::from(crate::Text::plain("Thank you for your order."))]]));
    schema
}

pub(crate) fn text_array(_: &mut SchemaGenerator) -> Schema {
    json_schema!({"type": "array", "items": {"type": "string", "minLength": 1, "maxLength": MAX_TEXT_BYTES}})
}

pub(crate) fn table_rows(generator: &mut SchemaGenerator) -> Schema {
    let mut row = text_array(generator);
    row.insert("minItems".into(), 1.into());
    row.insert("maxItems".into(), MAX_TABLE_COLUMNS.into());
    json_schema!({"type": "array", "items": row})
}

pub(crate) fn list_items(generator: &mut SchemaGenerator) -> Schema {
    json_schema!({"type": "array", "items": {"type": "array", "minItems": 1, "items": generator.subschema_for::<Span>()}})
}

#[cfg(test)]
mod tests {
    #[test]
    fn schema_is_up_to_date() {
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("tests/fixtures/content-blocks-v1.schema.json")).unwrap();
        assert_eq!(serde_json::to_value(super::content_blocks_schema()).unwrap(), expected);
    }
}
