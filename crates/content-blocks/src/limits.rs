//! Limits for Content Blocks v1.
//!
//! Count constants define upper bounds. Arrays that require elements have a lower bound of one.

/// Major version of the Content Blocks schema used by `blocks-json`.
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum number of fragments in one output.
pub const MAX_FRAGMENTS: usize = 8;
/// Maximum total number of blocks across all fragments, including nested children.
pub const MAX_TOTAL_BLOCKS: usize = 64;
/// Maximum size in bytes of `blocks-json` in one fragment.
pub const MAX_FRAGMENT_BLOCKS_JSON_BYTES: usize = 8192;
/// Common maximum UTF-8 byte length for text fields.
pub const MAX_TEXT_BYTES: usize = 1024;
/// Maximum number of entries in `fields.entries`.
pub const MAX_FIELDS_ENTRIES: usize = 16;
/// Maximum number of entries in `table.columns`.
pub const MAX_TABLE_COLUMNS: usize = 6;
/// Maximum number of entries in `table.rows`.
pub const MAX_TABLE_ROWS: usize = 20;
/// Maximum number of entries in `list.items`.
pub const MAX_LIST_ITEMS: usize = 16;
/// Maximum number of entries in `grid.items`.
pub const MAX_GRID_ITEMS: usize = 12;
/// Maximum UTF-8 byte length of `badge.text`.
pub const MAX_BADGE_TEXT_BYTES: usize = 64;
/// Maximum UTF-8 byte length of `qr.value`.
pub const MAX_QR_VALUE_BYTES: usize = 512;

/// Inclusive range allowed for `heading.level`.
pub const HEADING_LEVEL: (u8, u8) = (1, 3);
/// Inclusive range allowed for `grid.columns-hint`.
pub const GRID_COLUMNS_HINT: (u8, u8) = (2, 4);
