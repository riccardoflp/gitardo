//! What can go wrong in drawing: a typeface asked of fonts that do not have it.

use std::fmt;

/// Something that went wrong while choosing the fonts text is drawn in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// No font loaded belongs to the family asked for, such as a font of the system named in the settings that is not
    /// installed.
    UnknownFamily {
        /// The name of the family.
        name: String,
    },
    /// More typefaces were made than a [`TypefaceId`](crate::TypefaceId) can count.
    TooManyTypefaces,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownFamily { name } => {
                write!(formatter, "No font named {name} is installed: choose another font.")
            }
            Error::TooManyTypefaces => write!(formatter, "Too many typefaces are in use at once."),
        }
    }
}

impl std::error::Error for Error {}
