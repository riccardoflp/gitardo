//! The words of the window, one module per area, so that they can be read in one place and translated later (rule
//! R10.2). Where a card says the words, they are the card's, word for word.

pub mod activity;
pub mod branch;
pub mod changes;
pub mod dates;
pub mod details;
pub mod feature;
pub mod filter;
pub mod header;
pub mod pages;
pub mod rail;
pub mod repo;
pub mod search;
pub mod settings;
pub mod timeline;
pub mod welcome;

/// The name of the program, in the title bar of its window.
pub const PROGRAM: &str = "Gitardo Desktop";

/// The title of the window: the repository shown, then the program.
#[must_use]
pub fn window_title(repository: Option<&str>) -> String {
    match repository {
        Some(name) => format!("{name} \u{2014} {PROGRAM}"),
        None => PROGRAM.to_owned(),
    }
}
