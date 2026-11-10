//! Blocks that do not contain child blocks.
//!
//! Tables, lists, and cards contain structured data but are leaves because they do not contain
//! child blocks.

use serde::{Deserialize, Serialize};

use crate::error::ContentBlocksError;
use crate::limits::{
    HEADING_LEVEL,
    MAX_BADGE_TEXT_BYTES,
    MAX_FIELDS_ENTRIES,
    MAX_LIST_ITEMS,
    MAX_QR_VALUE_BYTES,
    MAX_TABLE_COLUMNS,
    MAX_TABLE_ROWS,
};
use crate::url::check_url;
use crate::validate::{check_count, check_range, check_spans, check_text, check_text_max};

/// A span within a text block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Span {
    /// Text to display.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub text: String,
    /// Whether to apply weak emphasis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emphasis: Option<bool>,
    /// Whether to apply strong emphasis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
}

impl Span {
    /// Creates an unstyled span.
    pub fn plain(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            emphasis: None,
            strong: None,
        }
    }

    /// Creates a span with strong emphasis.
    pub fn strong(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            emphasis: None,
            strong: Some(true),
        }
    }

    /// Creates a span with weak emphasis.
    pub fn emphasis(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            emphasis: Some(true),
            strong: None,
        }
    }
}

/// Presentation intent for alerts and badges.
///
/// Colors and other styling are determined by the display environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// Default style of the display environment.
    Auto,
    /// Informational content.
    Info,
    /// Successful outcome.
    Success,
    /// Warning or caution.
    Warning,
}

/// Marker style for a list block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ListStyle {
    /// A bulleted list.
    Bullet,
    /// A numbered list.
    Ordered,
}

/// Visual prominence of a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Prominence {
    /// A primary action.
    Primary,
    /// A secondary action.
    Secondary,
}

/// A label-value pair displayed in a fields block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FieldEntry {
    /// Label describing the value.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub label: String,
    /// Value to display.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub value: String,
}

/// A heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Heading {
    /// Heading text.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub text: String,
    /// Heading level.
    #[cfg_attr(feature = "schema", schemars(range(min = HEADING_LEVEL.0, max = HEADING_LEVEL.1)))]
    pub level: u8,
}

impl Heading {
    /// Creates a heading with the given level and text.
    pub fn new(level: u8, text: impl Into<String>) -> Heading {
        Heading {
            text: text.into(),
            level,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_text("heading.text", &self.text)?;
        check_range("heading.level", self.level, HEADING_LEVEL)
    }
}

/// A paragraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Text {
    /// Spans to display.
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
    pub spans: Vec<Span>,
}

impl Text {
    /// Creates a paragraph containing one unstyled span.
    pub fn plain(text: impl Into<String>) -> Text {
        Text {
            spans: vec![Span::plain(text)],
        }
    }

    /// Creates a paragraph from multiple spans.
    pub fn spans(spans: impl IntoIterator<Item = Span>) -> Text {
        Text {
            spans: spans.into_iter().collect(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_spans("text.spans", &self.spans)
    }
}

/// A block that displays labels alongside their values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Fields {
    /// Labels and values to display.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = MAX_FIELDS_ENTRIES)))]
    pub entries: Vec<FieldEntry>,
}

impl Fields {
    /// Creates a fields block from label-value pairs.
    pub fn new<L: Into<String>, V: Into<String>>(entries: impl IntoIterator<Item = (L, V)>) -> Fields {
        Fields {
            entries: entries
                .into_iter()
                .map(|(label, value)| FieldEntry {
                    label: label.into(),
                    value: value.into(),
                })
                .collect(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_count("fields.entries", self.entries.len(), MAX_FIELDS_ENTRIES)?;
        for entry in &self.entries {
            check_text("fields.entries.label", &entry.label)?;
            check_text("fields.entries.value", &entry.value)?;
        }
        Ok(())
    }
}

/// A value that provides a copy action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Copyable {
    /// Value to copy.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub value: String,
    /// Label describing the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub label: Option<String>,
}

impl Copyable {
    /// Creates a copyable block with the given value.
    pub fn new(value: impl Into<String>) -> Copyable {
        Copyable {
            value: value.into(),
            label: None,
        }
    }

    /// Adds a label describing the value.
    pub fn label(mut self, label: impl Into<String>) -> Copyable {
        self.label = Some(label.into());
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_text("copyable.value", &self.value)?;
        if let Some(label) = &self.label {
            check_text("copyable.label", label)?;
        }
        Ok(())
    }
}

/// A link to another page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Link {
    /// Link text.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub text: String,
    /// HTTPS destination URL.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = "^https://[^/?#]+")))]
    pub url: String,
    /// Visual prominence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prominence: Option<Prominence>,
}

impl Link {
    /// Creates a link block with the given text and URL.
    pub fn new(text: impl Into<String>, url: impl Into<String>) -> Link {
        Link {
            text: text.into(),
            url: url.into(),
            prominence: None,
        }
    }

    /// Sets the visual prominence.
    pub fn prominence(mut self, prominence: Prominence) -> Link {
        self.prominence = Some(prominence);
        self
    }

    pub(crate) fn validate(&self, text_field: &'static str, url_field: &'static str) -> Result<(), ContentBlocksError> {
        check_text(text_field, &self.text)?;
        check_url(url_field, &self.url)
    }
}

/// An image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Image {
    /// HTTPS URL of the image.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = "^https://[^/?#]+")))]
    pub url: String,
    /// Alternative text for the image.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub alt: String,
}

impl Image {
    /// Creates an image block with the given URL and alternative text.
    pub fn new(url: impl Into<String>, alt: impl Into<String>) -> Image {
        Image {
            url: url.into(),
            alt: alt.into(),
        }
    }

    pub(crate) fn validate(&self, url_field: &'static str, alt_field: &'static str) -> Result<(), ContentBlocksError> {
        check_url(url_field, &self.url)?;
        check_text(alt_field, &self.alt)
    }
}

/// A divider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Divider {}

impl Divider {
    /// Creates a divider.
    pub fn new() -> Divider {
        Divider {}
    }
}

/// A table containing column headings and rows.
///
/// [`Table::columns`] checks at compile time that the number of cells in each row matches the
/// number of columns. Use [`Table::try_new`] when the column count is determined at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Table {
    /// Column headings.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = MAX_TABLE_COLUMNS), schema_with = "crate::schema::text_array"))]
    pub columns: Vec<String>,
    /// Table rows. Each row must contain the same number of cells as there are columns.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = MAX_TABLE_ROWS), schema_with = "crate::schema::table_rows"))]
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// Creates a [`TableBuilder`] with the given column headings.
    pub fn columns<S: Into<String>, const N: usize>(columns: [S; N]) -> TableBuilder<N> {
        TableBuilder {
            columns: columns.map(Into::into),
            rows: Vec::new(),
        }
    }

    /// Creates a table block with the given column headings and rows.
    ///
    /// Returns [`ContentBlocksError::TableRowColumnMismatch`] when a row has the wrong number of
    /// cells.
    pub fn try_new(columns: Vec<String>, rows: Vec<Vec<String>>) -> Result<Table, ContentBlocksError> {
        for (row, cells) in rows.iter().enumerate() {
            if cells.len() != columns.len() {
                return Err(ContentBlocksError::TableRowColumnMismatch {
                    row,
                    expected: columns.len(),
                    got: cells.len(),
                });
            }
        }
        Ok(Table {
            columns,
            rows,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_count("table.columns", self.columns.len(), MAX_TABLE_COLUMNS)?;
        for column in &self.columns {
            check_text("table.columns", column)?;
        }
        check_count("table.rows", self.rows.len(), MAX_TABLE_ROWS)?;
        for (row, cells) in self.rows.iter().enumerate() {
            if cells.len() != self.columns.len() {
                return Err(ContentBlocksError::TableRowColumnMismatch {
                    row,
                    expected: self.columns.len(),
                    got: cells.len(),
                });
            }
            for cell in cells {
                check_text("table.rows.cell", cell)?;
            }
        }
        Ok(())
    }
}

/// Builder for a [`Table`] with a type-level column count `N`.
///
/// [`TableBuilder::row`] and [`TableBuilder::rows`] accept only arrays of length `N`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableBuilder<const N: usize> {
    columns: [String; N],
    rows: Vec<[String; N]>,
}

impl<const N: usize> TableBuilder<N> {
    /// Adds one row.
    pub fn row<S: Into<String>>(mut self, cells: [S; N]) -> TableBuilder<N> {
        self.rows.push(cells.map(Into::into));
        self
    }

    /// Adds multiple rows.
    pub fn rows<S: Into<String>>(mut self, rows: impl IntoIterator<Item = [S; N]>) -> TableBuilder<N> {
        self.rows
            .extend(rows.into_iter().map(|cells| cells.map(Into::into)));
        self
    }

    /// Returns the table block.
    pub fn build(self) -> Table {
        Table {
            columns: self.columns.into(),
            rows: self.rows.into_iter().map(Vec::from).collect(),
        }
    }
}

impl<const N: usize> From<TableBuilder<N>> for Table {
    fn from(builder: TableBuilder<N>) -> Table {
        builder.build()
    }
}

/// A bulleted or numbered list.
///
/// Each item is a vector of spans. Lists cannot be nested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct List {
    /// Marker style.
    pub style: ListStyle,
    /// List items.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = MAX_LIST_ITEMS), schema_with = "crate::schema::list_items"))]
    pub items: Vec<Vec<Span>>,
}

impl List {
    /// Creates a list block with the given marker style and items.
    pub fn new(style: ListStyle, items: impl IntoIterator<Item = Vec<Span>>) -> List {
        List {
            style,
            items: items.into_iter().collect(),
        }
    }

    /// Creates a bulleted list from unstyled strings.
    pub fn bullet(items: impl IntoIterator<Item = impl Into<String>>) -> List {
        List::new(ListStyle::Bullet, plain_items(items))
    }

    /// Creates a numbered list from unstyled strings.
    pub fn ordered(items: impl IntoIterator<Item = impl Into<String>>) -> List {
        List::new(ListStyle::Ordered, plain_items(items))
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_count("list.items", self.items.len(), MAX_LIST_ITEMS)?;
        for item in &self.items {
            check_spans("list.item.spans", item)?;
        }
        Ok(())
    }
}

// Converts strings into items containing a single span.
fn plain_items(items: impl IntoIterator<Item = impl Into<String>>) -> Vec<Vec<Span>> {
    items
        .into_iter()
        .map(|text| vec![Span::plain(text)])
        .collect()
}

/// A card containing a title, body, image, and link.
///
/// Images and links use [`Image`] and [`Link`], respectively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Card {
    /// Card title.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub title: String,
    /// Card body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
    pub body: Option<Vec<Span>>,
    /// Image displayed on the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    /// Link opened from the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
}

impl Card {
    /// Creates a card block with the given title.
    pub fn new(title: impl Into<String>) -> Card {
        Card {
            title: title.into(),
            body: None,
            image: None,
            link: None,
        }
    }

    /// Sets the body.
    pub fn body(mut self, spans: impl IntoIterator<Item = Span>) -> Card {
        self.body = Some(spans.into_iter().collect());
        self
    }

    /// Sets the image.
    pub fn image(mut self, image: Image) -> Card {
        self.image = Some(image);
        self
    }

    /// Sets the link.
    pub fn link(mut self, link: Link) -> Card {
        self.link = Some(link);
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_text("card.title", &self.title)?;
        if let Some(spans) = &self.body {
            check_spans("card.body", spans)?;
        }
        if let Some(image) = &self.image {
            image.validate("card.image.url", "card.image.alt")?;
        }
        if let Some(link) = &self.link {
            link.validate("card.link.text", "card.link.url")?;
        }
        Ok(())
    }
}

/// A short label representing a rank, restriction, status, or similar property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Badge {
    /// Presentation intent of the label.
    pub variant: Variant,
    /// Text displayed in the label.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = MAX_BADGE_TEXT_BYTES)))]
    pub text: String,
}

impl Badge {
    /// Creates a badge block with the given presentation intent and text.
    pub fn new(variant: Variant, text: impl Into<String>) -> Badge {
        Badge {
            variant,
            text: text.into(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_text_max("badge.text", &self.text, MAX_BADGE_TEXT_BYTES)
    }
}

/// A value rendered as a QR code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Qr {
    /// Value encoded in the QR code.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = MAX_QR_VALUE_BYTES)))]
    pub value: String,
    /// Caption displayed with the QR code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub caption: Option<String>,
}

impl Qr {
    /// Creates a QR code block with the given value.
    pub fn new(value: impl Into<String>) -> Qr {
        Qr {
            value: value.into(),
            caption: None,
        }
    }

    /// Sets the caption.
    pub fn caption(mut self, caption: impl Into<String>) -> Qr {
        self.caption = Some(caption.into());
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        // A QR code value is not used as a link, so URL validation does not apply.
        check_text_max("qr.value", &self.value, MAX_QR_VALUE_BYTES)?;
        if let Some(caption) = &self.caption {
            check_text("qr.caption", caption)?;
        }
        Ok(())
    }
}
