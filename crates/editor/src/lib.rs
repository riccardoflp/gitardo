//! The text editor of Gitardo Desktop: buffer, cursor, selection, undo, wrap and the hooks of highlighting.
//!
//! A [`Document`] is a file open in the editor: its text as a [`Buffer`] of lines, and the steps done to it that Undo
//! takes back. Each page that shows it has a [`Cursor`] of its own. What the keys and the pointer ask is a [`Command`],
//! which [`apply`] does, saying how the text changed for the other pages and the colors of the code.

#![forbid(unsafe_code)]

mod buffer;
mod command;
mod cursor;
mod document;

#[cfg(test)]
mod tests;

pub use buffer::{Buffer, Ending, LinesChanged, NotText, Position, Removed};
pub use command::{Command, Extent, Movement, apply};
pub use cursor::Cursor;
pub use document::{Change, Document};
