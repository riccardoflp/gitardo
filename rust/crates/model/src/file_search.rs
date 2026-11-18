//! The search in the files of the worktree shown, for the Search page (decision D73): what is looked for and where,
//! the pathspecs that narrow it, the names that match best, the lines of text found, and the rows of the results.

mod open;
mod pathspecs;
mod rank;
mod results;

#[cfg(test)]
mod tests;

use std::ops::Range;

pub use open::{Asked, open_with};
pub use pathspecs::{Filters, pathspecs};
pub use rank::{RankRequest, RankedName, RankedNames, folders_of, rank_names};
pub use results::{
    Found, MOST_FILES_IN_ALL, MOST_FOLDERS_IN_ALL, NameFound, ResultRow, Section, counts, list_results, names_found,
};

/// The most names the results list, of files or of folders.
pub const MOST_NAMES: usize = 2_000;

/// The most lines of text the results list; the search of the text is stopped there.
pub const MOST_LINES: usize = 5_000;

/// What the results list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// The folders, the files and the lines of text, in sections.
    #[default]
    All,
    /// The lines of text.
    Text,
    /// The files, by name.
    Files,
    /// The folders, by name.
    Folders,
}

impl Mode {
    /// Every mode, in the order of the page.
    pub const EVERY: [Mode; 4] = [Mode::All, Mode::Text, Mode::Files, Mode::Folders];

    /// Whether the text of the files is searched.
    #[must_use]
    pub fn searches_text(self) -> bool {
        matches!(self, Mode::All | Mode::Text)
    }
}

/// Where the search looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Scope {
    /// The worktree shown.
    #[default]
    Repository,
    /// The files that differ from the last commit, the untracked ones too.
    Changes,
    /// The folder or the file **Search here** named.
    Named,
}

/// A folder or a file of the worktree, as **Search here** names it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    /// Its path from the top of the worktree, with `/`.
    pub path: String,
    /// Whether it is a file, searched alone, rather than a folder.
    pub is_file: bool,
}

/// What the Search page looks for, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools, reason = "each is a switch of the page, on or off")]
pub struct Search {
    /// What is typed in the box.
    pub query: String,
    /// **Match case**: the text is found with its capitals as typed.
    pub match_case: bool,
    /// **Whole word**: the text is found as a word of its own.
    pub whole_word: bool,
    /// **Regular expression**: the text is one.
    pub regex: bool,
    /// What the results list.
    pub mode: Mode,
    /// Where the search looks.
    pub scope: Scope,
    /// What **Search here** named last, a scope of the page until another takes its place.
    pub named: Option<Place>,
    /// **Include**: the patterns of the files searched, separated by commas.
    pub include: String,
    /// **Exclude**: the patterns of the files left out.
    pub exclude: String,
    /// **.gitignore**: what git ignores is left out.
    pub gitignore: bool,
    /// **Hidden**: the files and the folders whose name starts with a dot are searched too.
    pub hidden: bool,
}

impl Default for Search {
    fn default() -> Self {
        Search {
            query: String::new(),
            match_case: false,
            whole_word: false,
            regex: false,
            mode: Mode::All,
            scope: Scope::Repository,
            named: None,
            include: String::new(),
            exclude: String::new(),
            gitignore: true,
            hidden: false,
        }
    }
}

impl Search {
    /// The place the search is narrowed to: what **Search here** named, while that scope is the one chosen.
    #[must_use]
    pub fn place(&self) -> Option<&Place> {
        match self.scope {
            Scope::Named => self.named.as_ref(),
            Scope::Repository | Scope::Changes => None,
        }
    }

    /// The filters of the search, as the pathspecs are made from them.
    #[must_use]
    pub fn filters(&self) -> Filters<'_> {
        Filters {
            include: &self.include,
            exclude: &self.exclude,
            hidden: self.hidden,
        }
    }
}

/// A line of a file that holds what is looked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMatch {
    /// The file, by its path from the top of the worktree.
    pub path: String,
    /// The number of the line, from 1.
    pub line: usize,
    /// The line, without the white space at its start, cut when it is long.
    pub text: String,
    /// Where each match is in that text, in bytes.
    pub marks: Vec<Range<usize>>,
}
