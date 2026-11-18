//! What a search found, and the rows the list of the Search page shows it as.

use super::{Mode, RankedNames, TextMatch};

/// The most folders **All** lists.
pub const MOST_FOLDERS_IN_ALL: usize = 50;

/// The most files **All** lists.
pub const MOST_FILES_IN_ALL: usize = 100;

/// A file or a folder found by its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameFound {
    /// Its path from the top of the worktree.
    pub path: String,
    /// Where the letters that matched start in the path, in bytes, in order.
    pub marks: Vec<usize>,
}

/// What a search found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Found {
    /// The folders kept, the best first.
    pub folders: Vec<NameFound>,
    /// How many folders matched in all.
    pub folders_total: usize,
    /// The files kept, the best first.
    pub files: Vec<NameFound>,
    /// How many files matched in all.
    pub files_total: usize,
    /// The lines of text found, those of a file together, in the order git gave them.
    pub text: Vec<TextMatch>,
    /// Whether there are more lines than these.
    pub more_text: bool,
}

/// A heading of the results of **All**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    /// **Folders**.
    Folders,
    /// **Files**.
    Files,
    /// **Text**.
    Text,
}

/// A row of the results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultRow {
    /// The heading of a section of **All**.
    Heading(Section),
    /// A folder found by its name: which of the folders found.
    Folder(usize),
    /// A file found by its name: which of the files found.
    File(usize),
    /// A file with lines found: the first of its lines among those found, and how many they are.
    TextFile {
        /// The first of its lines, among the lines found.
        first: usize,
        /// How many of its lines were found.
        lines: usize,
    },
    /// A line of text: which of the lines found.
    Line(usize),
}

impl ResultRow {
    /// Whether the keyboard stops on the row: every row but a heading.
    #[must_use]
    pub fn is_result(self) -> bool {
        !matches!(self, ResultRow::Heading(_))
    }
}

/// The rows of the results for a mode: the names in **Files** and **Folders**, the lines under their files in
/// **Text**, and all three under their headings in **All**, with fewer names.
///
/// Feature: file-search.list-results
#[must_use]
pub fn list_results(found: &Found, mode: Mode) -> Vec<ResultRow> {
    match mode {
        Mode::Files => (0..found.files.len()).map(ResultRow::File).collect(),
        Mode::Folders => (0..found.folders.len()).map(ResultRow::Folder).collect(),
        Mode::Text => text_rows(&found.text),
        Mode::All => {
            let folders = (0..found.folders.len().min(MOST_FOLDERS_IN_ALL)).map(ResultRow::Folder);
            let files = (0..found.files.len().min(MOST_FILES_IN_ALL)).map(ResultRow::File);
            let mut rows = Vec::new();
            section(&mut rows, Section::Folders, folders.collect());
            section(&mut rows, Section::Files, files.collect());
            section(&mut rows, Section::Text, text_rows(&found.text));
            rows
        }
    }
}

/// How many lines of text were found, and in how many files.
#[must_use]
pub fn counts(found: &Found) -> (usize, usize) {
    let mut files = 0;
    let mut before: Option<&str> = None;
    for line in &found.text {
        if before != Some(line.path.as_str()) {
            files += 1;
            before = Some(&line.path);
        }
    }
    (found.text.len(), files)
}

/// The names a ranking kept, with their paths.
#[must_use]
pub fn names_found(names: &[String], ranked: RankedNames) -> Vec<NameFound> {
    ranked
        .names
        .into_iter()
        .filter_map(|ranked| {
            names.get(ranked.index).map(|path| NameFound {
                path: path.clone(),
                marks: ranked.marks,
            })
        })
        .collect()
}

/// Adds a section under its heading, when it has rows.
fn section(rows: &mut Vec<ResultRow>, heading: Section, of_section: Vec<ResultRow>) {
    if !of_section.is_empty() {
        rows.push(ResultRow::Heading(heading));
        rows.extend(of_section);
    }
}

/// The rows of the lines of text: for each file, its row, then its lines.
fn text_rows(text: &[TextMatch]) -> Vec<ResultRow> {
    let mut rows = Vec::with_capacity(text.len());
    let mut first = 0;
    while let Some(line) = text.get(first) {
        let lines = text
            .get(first..)
            .unwrap_or_default()
            .iter()
            .take_while(|other| other.path == line.path)
            .count()
            .max(1);
        rows.push(ResultRow::TextFile { first, lines });
        rows.extend((first..first + lines).map(ResultRow::Line));
        first += lines;
    }
    rows
}
