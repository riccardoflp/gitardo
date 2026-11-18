//! The words of the Search page: the search of files, folders and lines of text in the files of the worktree shown
//! (`file-search.page`).

use model::file_search::{Mode, Section};

use super::worktree_setup::thousands;

/// The name of the page, in its title row.
pub const PAGE: &str = "Search";
/// The button of the bar of views that opens the page: the header has a **Search** of its own, Quick search.
pub const BUTTON: &str = "Search in files";
/// The command of Quick search that opens the page.
pub const COMMAND: &str = "Search in files\u{2026}";
/// **Search here** in the context menu of a file or a folder of the Files page (`file-search.open`).
pub const SEARCH_HERE: &str = "Search here";
/// The switch that finds the text with its capitals as typed.
pub const MATCH_CASE: &str = "Match case";
/// The switch that finds the text as a word of its own.
pub const WHOLE_WORD: &str = "Whole word";
/// The switch that reads the text as a regular expression.
pub const REGULAR_EXPRESSION: &str = "Regular expression";
/// The scope of the worktree shown.
pub const REPOSITORY: &str = "Repository";
/// The scope of the files that differ from the last commit.
pub const CHANGES: &str = "Changes";
/// The tooltip of the button that takes away the scope **Search here** named.
pub const DROP_SCOPE: &str = "Stop searching here";
/// The box of the patterns of the files searched, empty.
pub const INCLUDE: &str = "Include: *.rs, src/";
/// The box of the patterns of the files left out, empty.
pub const EXCLUDE: &str = "Exclude: dist/, *.min.js";
/// The switch that leaves out what git ignores.
pub const GITIGNORE: &str = ".gitignore";
/// The switch that searches the files and the folders whose name starts with a dot.
pub const HIDDEN: &str = "Hidden";
/// The name of the list of the results, for the screen reader.
pub const RESULTS: &str = "Results";
/// What the results say while the files are listed.
pub const READING: &str = "Reading the files\u{2026}";
/// What the results say in **Text** while nothing is typed.
pub const TYPE_TO_SEARCH: &str = "Type to search the text of the files.";
/// What the results say when nothing was found.
pub const NO_RESULTS: &str = "No results";
/// What the line under the results says while the text is searched.
pub const SEARCHING: &str = "Searching\u{2026}";
/// What the results say when git could not list the files, before what it said.
pub const FAILED: &str = "Git could not list the files:";
/// The name of the preview, for the screen reader.
pub const PREVIEW: &str = "Preview";
/// What the preview says of a file with a NUL byte.
pub const NOT_TEXT: &str = "This file is not text.";
/// What the preview says of a file larger than it reads.
pub const TOO_LARGE: &str = "This file is too large to preview. Press Enter to open it.";

/// What the preview says of a file that could not be read.
#[must_use]
pub fn preview_failed(said: &str) -> String {
    format!("The file could not be read: {}", said.trim())
}

/// The name of a mode.
#[must_use]
pub fn mode(mode: Mode) -> &'static str {
    match mode {
        Mode::All => "All",
        Mode::Text => "Text",
        Mode::Files => "Files",
        Mode::Folders => "Folders",
    }
}

/// What the box of a mode shows while it is empty.
#[must_use]
pub fn placeholder(mode: Mode) -> &'static str {
    match mode {
        Mode::All => "Search files, folders and text",
        Mode::Text => "Search the text of the files",
        Mode::Files => "Search files by name",
        Mode::Folders => "Search folders by name",
    }
}

/// The heading of a section of the results of **All**.
#[must_use]
pub fn section(section: Section) -> &'static str {
    match section {
        Section::Folders => mode(Mode::Folders),
        Section::Files => mode(Mode::Files),
        Section::Text => mode(Mode::Text),
    }
}

/// What the results say when git could not list the files.
#[must_use]
pub fn failed(said: &str) -> String {
    format!("{FAILED} {}", said.trim())
}

/// How many lines were found, and in how many files: `12 lines in 3 files`, `5,000+ lines in 240 files` when there
/// are more lines than are listed.
#[must_use]
pub fn lines_found(lines: usize, files: usize, has_more: bool) -> String {
    let more = if has_more { "+" } else { "" };
    let lines_word = if lines == 1 && !has_more { "line" } else { "lines" };
    let files_word = if files == 1 { "file" } else { "files" };
    format!(
        "{}{more} {lines_word} in {} {files_word}",
        thousands(lines),
        thousands(files)
    )
}

/// How many names were found: `12 files`, `2,000 of 3,512 files` when more matched than are listed.
#[must_use]
pub fn names_found(listed: usize, total: usize, (one, many): (&str, &str)) -> String {
    let word = if total == 1 { one } else { many };
    if total > listed {
        format!("{} of {} {word}", thousands(listed), thousands(total))
    } else {
        format!("{} {word}", thousands(total))
    }
}

/// How many files were found by name.
#[must_use]
pub fn files_found(listed: usize, total: usize) -> String {
    names_found(listed, total, ("file", "files"))
}

/// How many folders were found by name.
#[must_use]
pub fn folders_found(listed: usize, total: usize) -> String {
    names_found(listed, total, ("folder", "folders"))
}

/// What a row of a file with lines found says after its name: how many of its lines were found.
#[must_use]
pub fn lines_of_file(lines: usize) -> String {
    thousands(lines)
}

/// What the screen reader is told of a line found: its file, its number and its text.
#[must_use]
pub fn line_said(path: &str, line: usize, text: &str) -> String {
    format!("{path}:{line} {text}")
}

#[cfg(test)]
mod tests {
    use super::{files_found, folders_found, lines_found};

    #[test]
    fn the_counts_are_said_with_their_words() {
        assert_eq!(lines_found(12, 3, false), "12 lines in 3 files");
        assert_eq!(lines_found(1, 1, false), "1 line in 1 file");
        assert_eq!(lines_found(5_000, 240, true), "5,000+ lines in 240 files");
        assert_eq!(files_found(1, 1), "1 file");
        assert_eq!(files_found(2_000, 3_512), "2,000 of 3,512 files");
        assert_eq!(folders_found(0, 0), "0 folders");
    }
}
