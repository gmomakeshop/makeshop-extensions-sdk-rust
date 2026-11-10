//! Conversion between block slices and `blocks-json`.

use crate::block::Block;
use crate::error::ContentBlocksError;

/// Parses `blocks-json` and returns a vector of blocks.
///
/// Returns [`ContentBlocksError::Parse`] for unknown types, unknown fields, structural mismatches,
/// or invalid container hierarchies. This function does not validate constraints such as text
/// lengths or item counts.
pub fn parse_blocks(blocks_json: &str) -> Result<Vec<Block>, ContentBlocksError> {
    serde_json::from_str(blocks_json).map_err(|err| ContentBlocksError::Parse(err.to_string()))
}

/// Converts a slice of blocks to `blocks-json`.
pub fn blocks_to_json(blocks: &[Block]) -> String {
    serde_json::to_string(blocks)
        .expect("content-blocks types contain no map keys or floats that can fail to serialize")
}
