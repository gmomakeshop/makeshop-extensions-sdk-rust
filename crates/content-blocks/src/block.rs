//! Content Block types and the types used for container children.
//!
//! [`Leaf`], [`GridItem`], and [`SectionChild`] restrict which blocks may be added to each
//! container at compile time.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::container::{Accordion, Alert, Grid, Section};
use crate::error::ContentBlocksError;
use crate::leaf::{Badge, Card, Copyable, Divider, Fields, Heading, Image, Link, List, Qr, Table, Text};

// Declares the wire vocabulary and container membership once for all generated views.
macro_rules! define_blocks {
    ($( $(#[$doc:meta])* $ty:ident => $tag:literal : [$($slot:ident),*] ),* $(,)?) => {
        /// Any block supported by Content Blocks v1.
        ///
        /// In JSON, the `type` field identifies the block type.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(tag = "type")]
        pub enum Block {
            $( $(#[$doc])* #[serde(rename = $tag)] $ty($ty), )*
        }

        impl Block {
            /// Returns the name used in the JSON `type` field.
            pub fn type_name(&self) -> &'static str {
                match self { $( Self::$ty(_) => $tag, )* }
            }

            fn allowed_in(&self, slot: &str) -> bool {
                match self { $( Self::$ty(_) => [$(stringify!($slot)),*].contains(&slot), )* }
            }

            #[cfg(feature = "schema")]
            pub(crate) fn slot_types(slot: &str) -> Vec<&'static str> {
                let mut types = Vec::new();
                $( if [$(stringify!($slot)),*].contains(&slot) { types.push($tag); } )*
                types
            }

            #[cfg(feature = "schema")]
            pub(crate) fn payload_schemas(generator: &mut schemars::SchemaGenerator) -> serde_json::Value {
                serde_json::json!({ $( $tag: generator.subschema_for::<$ty>(), )* })
            }
        }

        $(
            impl From<$ty> for Block {
                fn from(block: $ty) -> Self { Self::$ty(block) }
            }
            $(
                impl From<$ty> for $slot {
                    fn from(block: $ty) -> Self { Self(Block::$ty(block)) }
                }
            )*
        )*
    };
}

define_blocks!(
    /// A heading.
    Heading => "heading": [Leaf, SectionChild],
    /// A paragraph.
    Text => "text": [Leaf, SectionChild],
    /// A collection of labels and values.
    Fields => "fields": [Leaf, SectionChild],
    /// A value that can be copied.
    Copyable => "copyable": [Leaf, SectionChild],
    /// A link.
    Link => "link": [Leaf, SectionChild],
    /// An image.
    Image => "image": [Leaf, GridItem, SectionChild],
    /// A divider.
    Divider => "divider": [Leaf, SectionChild],
    /// A table.
    Table => "table": [Leaf, SectionChild],
    /// A bulleted or numbered list.
    List => "list": [Leaf, SectionChild],
    /// A card.
    Card => "card": [Leaf, GridItem, SectionChild],
    /// A short label.
    Badge => "badge": [Leaf, SectionChild],
    /// A QR code.
    Qr => "qr": [Leaf, SectionChild],
    /// A container for notices and guidance.
    Alert => "alert": [SectionChild],
    /// A group of content with an optional heading.
    Section => "section": [],
    /// A container that displays cards or images in a grid.
    Grid => "grid": [SectionChild],
    /// Collapsible content.
    Accordion => "accordion": [SectionChild],
);

impl Block {
    /// Returns whether this block type has no child blocks.
    pub fn is_leaf(&self) -> bool {
        self.allowed_in("Leaf")
    }

    /// Validates this block and its child blocks.
    ///
    /// This does not validate the fragment count or the total block count across fragments.
    /// Use [`crate::validate_fragments`] to validate those limits as well.
    pub fn validate(&self) -> Result<(), ContentBlocksError> {
        match self {
            Block::Heading(block) => block.validate(),
            Block::Text(block) => block.validate(),
            Block::Fields(block) => block.validate(),
            Block::Copyable(block) => block.validate(),
            Block::Link(block) => block.validate("link.text", "link.url"),
            Block::Image(block) => block.validate("image.url", "image.alt"),
            Block::Divider(_) => Ok(()),
            Block::Table(block) => block.validate(),
            Block::List(block) => block.validate(),
            Block::Card(block) => block.validate(),
            Block::Badge(block) => block.validate(),
            Block::Qr(block) => block.validate(),
            Block::Alert(block) => block.validate(),
            Block::Section(block) => block.validate(),
            Block::Grid(block) => block.validate(),
            Block::Accordion(block) => block.validate(),
        }
    }

    /// Returns the total count of this block and all nested child blocks.
    ///
    /// Structured data inside tables, lists, cards, badges, and QR codes is not counted.
    pub fn count(&self) -> usize {
        let children: usize = match self {
            Block::Alert(block) => count_children(&block.blocks),
            Block::Accordion(block) => count_children(&block.blocks),
            Block::Section(block) => count_children(&block.blocks),
            Block::Grid(block) => count_children(&block.items),
            _ => 0,
        };
        1 + children
    }
}

fn count_children<C: AsRef<Block>>(children: &[C]) -> usize {
    children.iter().map(|child| child.as_ref().count()).sum()
}

// Generates conversions and serialization shared by container child types.
macro_rules! child_slot {
    ($name:ident, $doc:expr, $reject:literal) => {
        #[doc = $doc]
        ///
        /// When present, the value satisfies the container hierarchy rules.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(transparent)]
        pub struct $name(Block);

        impl $name {
            /// Returns a reference to the inner [`Block`].
            pub fn as_block(&self) -> &Block {
                &self.0
            }

            /// Returns the inner [`Block`].
            pub fn into_block(self) -> Block {
                self.0
            }
        }

        #[cfg(feature = "schema")]
        impl schemars::JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> { stringify!($name).into() }

            fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                schemars::json_schema!({
                    "allOf": [
                        generator.subschema_for::<Block>(),
                        {"properties": {"type": {"enum": Block::slot_types(stringify!($name))}}}
                    ]
                })
            }
        }

        impl AsRef<Block> for $name {
            fn as_ref(&self) -> &Block {
                &self.0
            }
        }

        impl From<$name> for Block {
            fn from(child: $name) -> Block {
                child.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let block = Block::deserialize(deserializer)?;
                if !block.allowed_in(stringify!($name)) {
                    return Err(D::Error::custom(format!("'{}' {}", block.type_name(), $reject)));
                }
                Ok($name(block))
            }
        }
    };
}

child_slot!(
    Leaf,
    "A [`Block`] without children that can be nested in an `alert` or `accordion` block.",
    "is not allowed here; only leaf blocks can be nested in alert or accordion"
);

child_slot!(
    GridItem,
    "A card or image block that can be nested in a `grid` block.",
    "is not allowed as a grid item; only card or image can be"
);

child_slot!(
    SectionChild,
    "A block that can be nested in a `section` block, excluding another `section` block.",
    "is not allowed as a section child; section cannot be nested"
);

/// Converts a leaf block into a section child.
impl From<Leaf> for SectionChild {
    fn from(leaf: Leaf) -> SectionChild {
        SectionChild(leaf.0)
    }
}

/// Converts a grid item into a leaf block.
impl From<GridItem> for Leaf {
    fn from(item: GridItem) -> Leaf {
        Leaf(item.0)
    }
}

/// Converts a table builder into a block.
impl<const N: usize> From<crate::leaf::TableBuilder<N>> for Block {
    fn from(builder: crate::leaf::TableBuilder<N>) -> Block {
        Block::Table(builder.build())
    }
}

impl<const N: usize> From<crate::leaf::TableBuilder<N>> for Leaf {
    fn from(builder: crate::leaf::TableBuilder<N>) -> Leaf {
        Leaf(Block::Table(builder.build()))
    }
}

impl<const N: usize> From<crate::leaf::TableBuilder<N>> for SectionChild {
    fn from(builder: crate::leaf::TableBuilder<N>) -> SectionChild {
        SectionChild(Block::Table(builder.build()))
    }
}

/// Creates a vector of blocks.
///
/// The element type is inferred from the call site. Supplying a block that is not allowed in the
/// target container causes a compile-time error.
///
/// ```
/// use content_blocks::{Alert, Block, Copyable, Text, Variant, blocks};
///
/// let blocks: Vec<Block> = blocks![
///     Text::plain("Your coupon is ready"),
///     Alert::new(Variant::Success, blocks![Copyable::new("CPN-9999").label("Coupon code")]),
/// ];
/// assert_eq!(blocks.len(), 2);
/// ```
#[macro_export]
macro_rules! blocks {
    () => { ::std::vec::Vec::new() };
    ($($block:expr),+ $(,)?) => {
        ::std::vec![$(::core::convert::Into::into($block)),+]
    };
}
