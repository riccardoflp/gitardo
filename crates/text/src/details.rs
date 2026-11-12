//! How the message of a commit is shown in the inspector and in the page of a commit: its lines put back together
//! where they were cut to fit 72 columns, and a title short enough to be read at a glance.

mod title;
mod unwrap;

#[cfg(test)]
mod tests;

pub use title::{RawMessage, ShownMessage, message_title};
pub use unwrap::unwrap_message;
