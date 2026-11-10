//! Content Blocks parsing and validation errors.
//!
//! Field names in error messages use the kebab-case names from the JSON representation.

/// An error produced while parsing or validating Content Blocks.
///
/// Container hierarchy violations found in JSON are returned as [`ContentBlocksError::Parse`].
#[derive(Debug)]
pub enum ContentBlocksError {
    /// `blocks-json` could not be parsed as Content Blocks.
    Parse(String),
    /// A required text field is empty.
    EmptyText {
        /// Name of the invalid field.
        field: &'static str,
    },
    /// A text field exceeds its UTF-8 byte limit.
    TextTooLong {
        /// Name of the invalid field.
        field: &'static str,
        /// Maximum allowed byte length.
        max: usize,
        /// Actual byte length.
        got: usize,
    },
    /// An array that requires at least one element is empty.
    EmptyList {
        /// Name of the invalid field.
        field: &'static str,
    },
    /// An array length is outside the allowed range.
    CountOutOfRange {
        /// Name of the invalid field.
        field: &'static str,
        /// Maximum allowed number of elements.
        max: usize,
        /// Actual number of elements.
        got: usize,
    },
    /// A numeric value is outside the allowed range.
    NumberOutOfRange {
        /// Name of the invalid field.
        field: &'static str,
        /// Minimum allowed value.
        min: u8,
        /// Maximum allowed value.
        max: u8,
        /// Actual value.
        got: u8,
    },
    /// The number of cells in a table row does not match the number of columns.
    ///
    /// Use [`crate::TableBuilder`] to prevent column-count mismatches at compile time.
    TableRowColumnMismatch {
        /// Index of the invalid row.
        row: usize,
        /// Expected number of cells based on the column count.
        expected: usize,
        /// Actual number of cells.
        got: usize,
    },
    /// A URL violates the Content Blocks rules.
    Url {
        /// Name of the invalid field.
        field: &'static str,
        /// Reason URL validation failed.
        reason: UrlError,
    },
    /// The fragment count exceeds the limit.
    TooManyFragments {
        /// Maximum allowed number of fragments.
        max: usize,
        /// Actual number of fragments.
        got: usize,
    },
    /// A placement is not in the allowed set.
    UnknownPlacement {
        /// Index of the invalid fragment.
        idx: usize,
        /// Placement that was provided.
        placement: String,
    },
    /// The `blocks-json` in one fragment exceeds the byte limit.
    FragmentTooLarge {
        /// Index of the invalid fragment.
        idx: usize,
        /// Maximum allowed byte length.
        max: usize,
        /// Actual byte length.
        got: usize,
    },
    /// The total block count across all fragments exceeds the limit.
    TooManyBlocks {
        /// Maximum allowed number of blocks.
        max: usize,
        /// Actual number of blocks.
        got: usize,
    },
}

/// The reason URL validation failed.
///
/// [`ContentBlocksError::Url`] combines this value with the URL field name.
#[derive(Debug)]
pub enum UrlError {
    /// The URL does not begin with `https://`.
    NotHttps {
        /// URL that failed validation.
        value: String,
    },
    /// The URL contains a control character, whitespace, a backslash, a quotation mark (`"` or
    /// `'`), an angle bracket (`<` or `>`), or a backtick.
    ForbiddenChars,
    /// The authority contains user information.
    Userinfo {
        /// URL that failed validation.
        value: String,
    },
    /// The host is empty.
    MissingHost {
        /// URL that failed validation.
        value: String,
    },
}

impl core::fmt::Display for ContentBlocksError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "blocks-json is not valid content-blocks JSON: {message}"),
            Self::EmptyText {
                field,
            } => write!(f, "{field} must be non-empty"),
            Self::TextTooLong {
                field,
                max,
                got,
            } => write!(f, "{field} must be <= {max} bytes (got {got})"),
            Self::EmptyList {
                field,
            } => write!(f, "{field} must not be empty"),
            Self::CountOutOfRange {
                field,
                max,
                got,
            } => write!(f, "{field} must be within 1..={max} (got {got})"),
            Self::NumberOutOfRange {
                field,
                min,
                max,
                got,
            } => write!(f, "{field} must be within {min}..={max} (got {got})"),
            Self::TableRowColumnMismatch {
                row,
                expected,
                got,
            } => write!(f, "table.rows[{row}] must have {expected} cells to match columns (got {got})"),
            Self::Url {
                field,
                reason,
            } => write!(f, "{field} {reason}"),
            Self::TooManyFragments {
                max,
                got,
            } => write!(f, "fragments must be <= {max} entries (got {got})"),
            Self::UnknownPlacement {
                idx,
                placement,
            } => write!(f, "fragments[{idx}].placement '{placement}' is not allowed"),
            Self::FragmentTooLarge {
                idx,
                max,
                got,
            } => write!(f, "fragments[{idx}].blocks-json must be <= {max} bytes (got {got})"),
            Self::TooManyBlocks {
                max,
                got,
            } => write!(f, "total block count must be <= {max} (got {got})"),
        }
    }
}

impl core::fmt::Display for UrlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotHttps {
                value,
            } => write!(f, "must be an https:// URL (got '{value}')"),
            Self::ForbiddenChars => write!(
                f,
                "must not contain control, whitespace, backslash, quote, angle bracket, or backtick characters"
            ),
            Self::Userinfo {
                value,
            } => write!(f, "must not embed userinfo ('@') (got '{value}')"),
            Self::MissingHost {
                value,
            } => write!(f, "must include a host (got '{value}')"),
        }
    }
}

impl core::error::Error for ContentBlocksError {}
impl core::error::Error for UrlError {}
