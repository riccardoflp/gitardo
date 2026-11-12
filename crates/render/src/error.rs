//! What can go wrong in drawing: a font file that cannot be read.

use std::fmt;

/// Something that went wrong while reading a font file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The file is neither a TrueType or OpenType font nor a collection of them.
    NotAFont,
    /// The file is a font whose outlines are not TrueType ones, such as the CFF outlines of some OpenType fonts.
    UnsupportedOutlines,
    /// A table every font must have is missing.
    MissingTable {
        /// The tag of the table, such as `cmap`.
        table: &'static str,
    },
    /// A table ends before what it says it holds, or holds a value it cannot hold: the file is damaged.
    Damaged {
        /// The tag of the table, such as `glyf`.
        table: &'static str,
    },
    /// A collection of fonts has no font at the position asked for.
    NoSuchFont {
        /// The position asked for, from 0.
        index: u32,
        /// How many fonts the collection holds.
        count: u32,
    },
    /// More fonts were loaded than the cache of glyphs can tell apart.
    TooManyFonts,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotAFont => write!(formatter, "The file is not a TrueType or OpenType font."),
            Error::UnsupportedOutlines => {
                write!(
                    formatter,
                    "The font has CFF outlines, which Gitardo does not draw yet: choose a TrueType font."
                )
            }
            Error::MissingTable { table } => {
                write!(formatter, "The font has no {table} table, which every font needs.")
            }
            Error::Damaged { table } => write!(formatter, "The {table} table of the font is damaged."),
            Error::NoSuchFont { index, count } => {
                write!(
                    formatter,
                    "The collection holds {count} fonts, so there is no font at position {index}."
                )
            }
            Error::TooManyFonts => write!(formatter, "Too many fonts are loaded at once."),
        }
    }
}

impl std::error::Error for Error {}
