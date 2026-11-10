//! Validation of fragments within one output.

use crate::error::ContentBlocksError;
use crate::json::parse_blocks;
use crate::limits::{MAX_FRAGMENT_BLOCKS_JSON_BYTES, MAX_FRAGMENTS, MAX_TOTAL_BLOCKS};
use crate::validate::validate_blocks;

/// One fragment to validate.
#[derive(Debug, Clone, Copy)]
pub struct FragmentInput<'a> {
    /// Display placement of the fragment.
    pub placement: &'a str,
    /// `blocks-json` contained in the fragment.
    pub blocks_json: &'a str,
}

/// Validates a slice of fragments.
///
/// This validates the fragment count, placements, the size and contents of `blocks-json`, and the
/// total block count across all fragments. It does not validate output data outside `blocks-json`.
pub fn validate_fragments(
    fragments: &[FragmentInput<'_>],
    allowed_placements: &[&str],
) -> Result<(), ContentBlocksError> {
    if fragments.len() > MAX_FRAGMENTS {
        return Err(ContentBlocksError::TooManyFragments {
            max: MAX_FRAGMENTS,
            got: fragments.len(),
        });
    }

    let mut total_blocks: usize = 0;
    for (idx, fragment) in fragments.iter().enumerate() {
        if !allowed_placements.contains(&fragment.placement) {
            return Err(ContentBlocksError::UnknownPlacement {
                idx,
                placement: fragment.placement.to_string(),
            });
        }
        // Check the byte length before parsing to avoid the cost of parsing oversized input.
        if fragment.blocks_json.len() > MAX_FRAGMENT_BLOCKS_JSON_BYTES {
            return Err(ContentBlocksError::FragmentTooLarge {
                idx,
                max: MAX_FRAGMENT_BLOCKS_JSON_BYTES,
                got: fragment.blocks_json.len(),
            });
        }

        let blocks = parse_blocks(fragment.blocks_json)?;
        validate_blocks(&blocks)?;
        let count: usize = blocks.iter().map(|block| block.count()).sum();
        total_blocks = total_blocks.saturating_add(count);
    }

    if total_blocks > MAX_TOTAL_BLOCKS {
        return Err(ContentBlocksError::TooManyBlocks {
            max: MAX_TOTAL_BLOCKS,
            got: total_blocks,
        });
    }
    Ok(())
}
