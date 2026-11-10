//! Validation of Content Block values.
//!
//! Limit values are available from [`crate::limits`].

use crate::block::Block;
use crate::error::ContentBlocksError;
use crate::leaf::Span;
use crate::limits::MAX_TEXT_BYTES;

/// Validates a slice of parsed blocks.
///
/// This recursively validates text byte lengths, array lengths, and URLs. It does not validate the
/// fragment count, total block count, or `blocks-json` size. Use [`crate::validate_fragments`] to
/// validate those limits as well.
pub fn validate_blocks(blocks: &[Block]) -> Result<(), ContentBlocksError> {
    for block in blocks {
        block.validate()?;
    }
    Ok(())
}

/// Checks that a value is non-empty and within the common UTF-8 byte limit for text fields.
pub(crate) fn check_text(field: &'static str, value: &str) -> Result<(), ContentBlocksError> {
    check_text_max(field, value, MAX_TEXT_BYTES)
}

/// Checks that a value is non-empty and no longer than `max` UTF-8 bytes.
///
/// Pass a field-specific `max` for fields whose limits differ from the common limit, such as
/// `badge.text` (64 bytes) and `qr.value` (512 bytes).
pub(crate) fn check_text_max(field: &'static str, value: &str, max: usize) -> Result<(), ContentBlocksError> {
    if value.is_empty() {
        return Err(ContentBlocksError::EmptyText {
            field,
        });
    }
    // String and &str lengths are UTF-8 byte counts. Limits are defined in bytes, not characters.
    if value.len() > max {
        return Err(ContentBlocksError::TextTooLong {
            field,
            max,
            got: value.len(),
        });
    }
    Ok(())
}

/// Checks that a count is within `1..=max`.
pub(crate) fn check_count(field: &'static str, len: usize, max: usize) -> Result<(), ContentBlocksError> {
    if len == 0 || len > max {
        return Err(ContentBlocksError::CountOutOfRange {
            field,
            max,
            got: len,
        });
    }
    Ok(())
}

/// Checks that a numeric value is within the inclusive range `min..=max`.
pub(crate) fn check_range(field: &'static str, value: u8, (min, max): (u8, u8)) -> Result<(), ContentBlocksError> {
    if value < min || value > max {
        return Err(ContentBlocksError::NumberOutOfRange {
            field,
            min,
            max,
            got: value,
        });
    }
    Ok(())
}

/// Checks that a span slice is non-empty and that each span's text is within the limit.
pub(crate) fn check_spans(field: &'static str, spans: &[Span]) -> Result<(), ContentBlocksError> {
    if spans.is_empty() {
        return Err(ContentBlocksError::EmptyList {
            field,
        });
    }
    for span in spans {
        check_text("span.text", &span.text)?;
    }
    Ok(())
}

/// Checks that a container has children and that each child is valid.
///
/// The caller's types guarantee that each child type is permitted in the container.
pub(crate) fn check_children<C: AsRef<Block>>(field: &'static str, children: &[C]) -> Result<(), ContentBlocksError> {
    if children.is_empty() {
        return Err(ContentBlocksError::EmptyList {
            field,
        });
    }
    for child in children {
        child.as_ref().validate()?;
    }
    Ok(())
}
