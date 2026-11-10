// Use the README as the crate landing page.
#![doc = include_str!("../README.md")]
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod block;
mod container;
mod error;
mod fragment;
mod json;
mod leaf;
mod limits;
mod url;
mod validate;

#[cfg(feature = "schema")]
pub mod schema;

pub use block::{Block, GridItem, Leaf, SectionChild};
pub use container::{Accordion, Alert, Grid, Section};
pub use error::{ContentBlocksError, UrlError};
pub use fragment::{FragmentInput, validate_fragments};
pub use json::{blocks_to_json, parse_blocks};
pub use leaf::{
    Badge,
    Card,
    Copyable,
    Divider,
    FieldEntry,
    Fields,
    Heading,
    Image,
    Link,
    List,
    ListStyle,
    Prominence,
    Qr,
    Span,
    Table,
    TableBuilder,
    Text,
    Variant,
};
pub use limits::{
    GRID_COLUMNS_HINT,
    HEADING_LEVEL,
    MAX_BADGE_TEXT_BYTES,
    MAX_FIELDS_ENTRIES,
    MAX_FRAGMENT_BLOCKS_JSON_BYTES,
    MAX_FRAGMENTS,
    MAX_GRID_ITEMS,
    MAX_LIST_ITEMS,
    MAX_QR_VALUE_BYTES,
    MAX_TABLE_COLUMNS,
    MAX_TABLE_ROWS,
    MAX_TEXT_BYTES,
    MAX_TOTAL_BLOCKS,
    SCHEMA_VERSION,
};
pub use url::HttpsUrl;
pub use validate::validate_blocks;

#[cfg(test)]
mod tests;
