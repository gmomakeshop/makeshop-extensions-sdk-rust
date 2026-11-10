//! Containers that hold child blocks.
//!
//! Types restrict which child blocks may be added to each container.
//!
//! ```text
//! section   -> grid | alert | accordion | leaf
//! grid      -> card | image
//! alert     -> leaf
//! accordion -> leaf
//! ```

use serde::{Deserialize, Serialize};

use crate::block::{GridItem, Leaf, SectionChild};
use crate::error::ContentBlocksError;
use crate::leaf::Variant;
use crate::limits::{GRID_COLUMNS_HINT, MAX_GRID_ITEMS};
use crate::validate::{check_children, check_count, check_range, check_text};

/// A container for notices and guidance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Alert {
    /// Presentation intent.
    pub variant: Variant,
    /// Leaf blocks displayed inside the container.
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
    pub blocks: Vec<Leaf>,
}

impl Alert {
    /// Creates an alert with the given presentation intent and child blocks.
    pub fn new(variant: Variant, blocks: Vec<Leaf>) -> Alert {
        Alert {
            variant,
            blocks,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_children("alert.blocks", &self.blocks)
    }
}

/// Collapsible content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Accordion {
    /// Summary shown while the content is collapsed.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub summary: String,
    /// Leaf blocks shown when the content is expanded.
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
    pub blocks: Vec<Leaf>,
}

impl Accordion {
    /// Creates an accordion with the given summary and child blocks.
    pub fn new(summary: impl Into<String>, blocks: Vec<Leaf>) -> Accordion {
        Accordion {
            summary: summary.into(),
            blocks,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        check_text("accordion.summary", &self.summary)?;
        check_children("accordion.blocks", &self.blocks)
    }
}

/// A group of content with an optional heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Section {
    /// Optional heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = crate::MAX_TEXT_BYTES)))]
    pub heading: Option<String>,
    /// Blocks displayed in the section. A section cannot contain another section.
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
    pub blocks: Vec<SectionChild>,
}

impl Section {
    /// Creates a section with the given child blocks.
    pub fn new(blocks: Vec<SectionChild>) -> Section {
        Section {
            heading: None,
            blocks,
        }
    }

    /// Adds a heading.
    pub fn heading(mut self, heading: impl Into<String>) -> Section {
        self.heading = Some(heading.into());
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        if let Some(heading) = &self.heading {
            check_text("section.heading", heading)?;
        }
        check_children("section.blocks", &self.blocks)
    }
}

/// A container that displays cards or images in a grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Grid {
    /// Preferred column count. The rendered column count may vary by display environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(range(min = GRID_COLUMNS_HINT.0, max = GRID_COLUMNS_HINT.1)))]
    pub columns_hint: Option<u8>,
    /// Cards or images to display.
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = MAX_GRID_ITEMS)))]
    pub items: Vec<GridItem>,
}

impl Grid {
    /// Creates a grid with the given items.
    pub fn new(items: Vec<GridItem>) -> Grid {
        Grid {
            columns_hint: None,
            items,
        }
    }

    /// Sets the preferred column count.
    pub fn columns_hint(mut self, columns_hint: u8) -> Grid {
        self.columns_hint = Some(columns_hint);
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ContentBlocksError> {
        if let Some(hint) = self.columns_hint {
            check_range("grid.columns-hint", hint, GRID_COLUMNS_HINT)?;
        }
        check_count("grid.items", self.items.len(), MAX_GRID_ITEMS)?;
        check_children("grid.items", &self.items)
    }
}
