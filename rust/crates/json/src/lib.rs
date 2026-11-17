//! JSON and JSON with comments: parsing, writing, and editing a file in place while keeping its comments.
//!
//! Answers from web services are read as strict JSON with [`parse`]; settings files are read with
//! [`parse_with_comments`], which also allows `//` and `/* */` comments and trailing commas. [`set_value`] and
//! [`remove_value`] change one value in the text of such a file and keep everything else in it as it was;
//! [`position_of`] says where a value is written in it.

#![forbid(unsafe_code)]

mod edit;
mod error;
mod find;
mod layout;
mod parse;
mod value;
mod write;

pub use edit::{remove_value, set_value};
pub use error::{Error, Key, Position, Problem};
pub use find::position_of;
pub use parse::{parse, parse_with_comments};
pub use value::{Member, Number, Value, ValueKind};
pub use write::{to_compact_text, to_pretty_text};
