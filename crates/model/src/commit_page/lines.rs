//! The output of a diff split into its files, hunks and lines, with the lines each file gained and lost, and the cap of
//! lines a page is given.

use std::iter::Peekable;

use super::names::quoted_name;
use super::{DiffLine, DiffOutput, FilePatch, Hunk, LineKind, PatchLines};
use crate::state::{ChangeKind, FileChange};

/// What starts the part of a diff about one file.
const SECTION_START: &[u8] = b"diff --git ";

/// The files of a diff with their hunks and the lines each gained and lost, named after the files listed for the same
/// diff. A file whose type changed, which git prints in two parts, is one file. Files are given their hunks in order
/// while their sizes, added up, fit under the limit; a file that does not fit has none, its counts still given, and a
/// smaller file after it may still fit. Each line is numbered as it is before and after, from its hunk header.
///
/// When the output names other files than those listed, or is not a diff this reads, the lines are not shown.
///
/// Feature: commit-page.diff-lines
#[must_use]
pub fn diff_lines(input: &DiffOutput<'_>) -> PatchLines {
    let Some(sections) = sections(input.output) else {
        return PatchLines::Unmatched;
    };
    let Some(matched) = matched_files(input.files, sections) else {
        return PatchLines::Unmatched;
    };
    let patches: Option<Vec<FilePatch>> = matched.iter().map(|(file, parts)| file_patch(file, parts)).collect();
    match patches {
        Some(mut files) => {
            apply_limit(&mut files, input.limit);
            PatchLines::Files(files)
        }
        None => PatchLines::Unmatched,
    }
}

/// The part of a diff about one file: its header line, and the lines after it.
struct Section<'a> {
    header: &'a [u8],
    lines: Vec<&'a [u8]>,
}

/// The output split where each file starts; none when something comes before the first file.
fn sections(output: &[u8]) -> Option<Vec<Section<'_>>> {
    let mut lines: Vec<&[u8]> = output.split(|byte| *byte == b'\n').collect();
    if lines.last().is_some_and(|last| last.is_empty()) {
        let _ = lines.pop();
    }
    let mut found: Vec<Section<'_>> = Vec::new();
    for line in lines {
        match found.last_mut() {
            _ if line.starts_with(SECTION_START) => found.push(Section {
                header: line,
                lines: Vec::new(),
            }),
            Some(section) => section.lines.push(line),
            None if line.is_empty() => {}
            None => return None,
        }
    }
    Some(found)
}

/// Each file listed with the parts of the output about it, in order; none when they do not match one for one. A file
/// whose type changed may have a second part with the same header.
fn matched_files<'a, 'files>(
    files: &'files [FileChange],
    sections: Vec<Section<'a>>,
) -> Option<Vec<(&'files FileChange, Vec<Section<'a>>)>> {
    let mut sections: Peekable<_> = sections.into_iter().peekable();
    let mut matched = Vec::with_capacity(files.len());
    for file in files {
        let header = expected_header(file);
        let first = sections.next().filter(|section| section.header == header.as_bytes())?;
        let mut parts = vec![first];
        if file.kind == ChangeKind::TypeChanged
            && let Some(second) = sections.next_if(|section| section.header == header.as_bytes())
        {
            parts.push(second);
        }
        matched.push((file, parts));
    }
    sections.peek().is_none().then_some(matched)
}

/// The header git prints for a file listed: `diff --git a/<path before> b/<path after>`, each name quoted when it must
/// be.
fn expected_header(file: &FileChange) -> String {
    let before = file.original_path.as_deref().unwrap_or(&file.path);
    format!(
        "diff --git {} {}",
        quoted_name(&format!("a/{before}")),
        quoted_name(&format!("b/{}", file.path))
    )
}

/// The patch of a file from the parts of the output about it; none when a part is not a diff this reads.
fn file_patch(file: &FileChange, parts: &[Section<'_>]) -> Option<FilePatch> {
    let mut patch = FilePatch {
        path: file.path.clone(),
        original_path: file.original_path.clone(),
        kind: file.kind,
        hunks: Vec::new(),
        added: 0,
        removed: 0,
        is_binary: false,
        is_past_limit: false,
    };
    for part in parts {
        read_part(&mut patch, &part.lines)?;
    }
    Some(patch)
}

/// Adds what one part says to the patch: whether the file is binary, and its hunks with their counts. Before the first
/// hunk come the lines that say what happened to the file (modes, renames, the index).
fn read_part(patch: &mut FilePatch, lines: &[&[u8]]) -> Option<()> {
    let mut hunk: Option<Numbered> = None;
    for line in lines {
        if let Some(current) = hunk.as_mut() {
            if line.starts_with(b"@@ ") {
                patch.hunks.push(hunk.take()?.hunk);
                hunk = Some(Numbered::start(line)?);
            } else {
                current.add(line, patch)?;
            }
        } else if line.starts_with(b"@@ ") {
            hunk = Some(Numbered::start(line)?);
        } else if line.starts_with(b"Binary files ") || *line == b"GIT binary patch" {
            patch.is_binary = true;
        }
    }
    if let Some(last) = hunk {
        patch.hunks.push(last.hunk);
    }
    Some(())
}

/// A hunk being read, with the numbers its next line has before and after.
struct Numbered {
    hunk: Hunk,
    before: usize,
    after: usize,
}

impl Numbered {
    /// A hunk from its header, `@@ -a,b +c,d @@`, whose first line is `a` before and `c` after.
    fn start(line: &[u8]) -> Option<Numbered> {
        let header = String::from_utf8_lossy(line).into_owned();
        let mut ranges = header.strip_prefix("@@ -")?.split(' ');
        let before = ranges.next()?.split(',').next()?.parse().ok()?;
        let after = ranges.next()?.strip_prefix('+')?.split(',').next()?.parse().ok()?;
        Some(Numbered {
            hunk: Hunk {
                header,
                lines: Vec::new(),
            },
            before,
            after,
        })
    }

    /// Adds a line, numbered, and counts it in the patch when it was added or removed. An empty line is an empty line
    /// both sides share, as git prints it with `diff.suppressBlankEmpty`.
    fn add(&mut self, line: &[u8], patch: &mut FilePatch) -> Option<()> {
        let (mark, text) = line.split_first().unwrap_or((&b' ', &[][..]));
        let (kind, before, after) = match mark {
            b' ' => (LineKind::Context, Some(self.before), Some(self.after)),
            b'+' => (LineKind::Added, None, Some(self.after)),
            b'-' => (LineKind::Removed, Some(self.before), None),
            b'\\' => (LineKind::NoNewline, None, None),
            _ => return None,
        };
        self.before += usize::from(before.is_some());
        self.after += usize::from(after.is_some());
        patch.added += usize::from(kind == LineKind::Added);
        patch.removed += usize::from(kind == LineKind::Removed);
        self.hunk.lines.push(DiffLine {
            kind,
            text: String::from_utf8_lossy(text).into_owned(),
            before,
            after,
        });
        Some(())
    }
}

/// Leaves out the hunks of each file that does not fit, with the files before it that kept theirs, under the limit.
fn apply_limit(files: &mut [FilePatch], limit: usize) {
    let mut taken = 0;
    for file in files {
        let size = file.size();
        if taken + size <= limit {
            taken += size;
        } else {
            file.hunks.clear();
            file.is_past_limit = true;
        }
    }
}

#[cfg(test)]
mod tests;
