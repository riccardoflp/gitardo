//! The file of the result under the keyboard, read for the preview of the Search page (`file-search.preview`): its
//! text from the working tree, colored as the File page colors it, and where git finds what is looked for in it.

use std::ops::Range;
use std::path::Path;

use git::read::file_search::{TextRequest, read_text};
use git::{Git, Taken};
use model::file_search::{Search, TextMatch};
use ui::CodeSpan;
use ui::editor::Buffer;

/// The largest file the preview reads, in bytes.
pub const MOST_BYTES: u64 = 1024 * 1024;

/// The most matches the preview marks in a file.
const MOST_MARKED_LINES: usize = 100_000;

/// A file to read for the preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewJob {
    /// Its path from the top of the worktree.
    pub path: String,
    /// What is looked for, when its matches are marked: the text was searched.
    pub search: Option<Search>,
}

/// What the preview shows of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewRead {
    /// Its text.
    Text {
        /// The text.
        buffer: Buffer,
        /// The colored parts of each line.
        colors: Vec<Vec<CodeSpan>>,
        /// Where what is looked for is, as bytes of the text, in order.
        marks: Vec<Range<usize>>,
    },
    /// A file with a NUL byte.
    NotText,
    /// A file larger than the preview reads.
    TooLarge,
    /// What the system or git said when the file could not be read.
    Failed(String),
}

/// Reads a file of the working tree whose top folder is `folder` for the preview: its text, its colors and the
/// matches of what is looked for, as `git` finds them in it.
#[must_use]
pub fn read_preview((folder, git): (&Path, &Git), job: &PreviewJob) -> PreviewRead {
    let mut on_disk = folder.to_path_buf();
    on_disk.extend(job.path.split('/').filter(|name| !name.is_empty()));
    match std::fs::metadata(&on_disk) {
        Ok(metadata) if metadata.len() > MOST_BYTES => return PreviewRead::TooLarge,
        Ok(_) => {}
        Err(error) => return PreviewRead::Failed(error.to_string()),
    }
    let bytes = match std::fs::read(&on_disk) {
        Ok(bytes) => bytes,
        Err(error) => return PreviewRead::Failed(error.to_string()),
    };
    let Ok(buffer) = Buffer::from_bytes(&bytes) else {
        return PreviewRead::NotText;
    };
    let colors = crate::store::file_page::colors_of(&job.path, buffer.text());
    let marks = match &job.search {
        Some(search) => match matches(git, &job.path, search) {
            Ok(found) => marks_in(&buffer, &found),
            Err(said) => return PreviewRead::Failed(said),
        },
        None => Vec::new(),
    };
    PreviewRead::Text { buffer, colors, marks }
}

/// The lines of one file where git finds what is looked for, each whole.
fn matches(git: &Git, path: &str, search: &Search) -> Result<Vec<TextMatch>, String> {
    let request = TextRequest {
        text: search.query.clone(),
        match_case: search.match_case,
        whole_word: search.whole_word,
        regex: search.regex,
        ignored: true,
        files: Some(vec![path.to_owned()]),
        most: MOST_MARKED_LINES,
        whole_lines: true,
        ..TextRequest::default()
    };
    let mut found = Vec::new();
    let _ended = read_text(git, &request, &mut |lines| {
        found.extend(lines);
        Taken::More
    })
    .map_err(|error| error.to_string())?;
    Ok(found)
}

/// The matches of some lines as bytes of the text of the file: a mark past the end of its line is cut there, and the
/// mark of the start of the file, which the text leaves out, is not counted.
fn marks_in(buffer: &Buffer, found: &[TextMatch]) -> Vec<Range<usize>> {
    let mut marks = Vec::new();
    for line in found {
        let Some(index) = line.line.checked_sub(1).filter(|index| *index < buffer.line_count()) else {
            continue;
        };
        let range = buffer.line_range(index);
        let skipped = if index == 0 && line.text.starts_with('\u{feff}') {
            '\u{feff}'.len_utf8()
        } else {
            0
        };
        let length = range.end - range.start;
        for mark in &line.marks {
            let start = mark.start.saturating_sub(skipped).min(length);
            let end = mark.end.saturating_sub(skipped).min(length);
            if start < end {
                marks.push(range.start + start..range.start + end);
            }
        }
    }
    marks.sort_by_key(|mark| mark.start);
    marks
}
