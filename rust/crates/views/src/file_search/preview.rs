//! The preview of the Search page (`file-search.preview`): the file of the result under the keyboard drawn as the File
//! page draws it, read only, the line found lit and every match marked; or what a folder holds.

use std::ops::Range;

use ui::editor::{Buffer, Command, Cursor};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, CodeEditor, CodeSpan, FileOrFolder, Label, LineWrap, Rect, TextKind, Tone, Ui};

use super::FileSearchIntent;
use crate::intent::Intent;
use crate::text::file_search as words;

/// The preview of a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewState<'a> {
    /// The path of the file or the folder, from the top of the worktree.
    pub path: &'a str,
    /// What is shown of it.
    pub shown: PreviewShown<'a>,
    /// The line lit, from zero: the line found.
    pub lit_line: Option<usize>,
    /// The line to bring to the middle, from zero, once for each number.
    pub reveal: (u64, usize),
    /// How many columns a tab goes to.
    pub tab_width: usize,
    /// What is done with a line longer than the preview is wide.
    pub wrap: LineWrap,
}

/// What the preview shows of a file or a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewShown<'a> {
    /// The text of a file.
    Text {
        /// The text.
        buffer: &'a Buffer,
        /// The colored parts of each line.
        colors: &'a [Vec<CodeSpan>],
        /// Where what is looked for is, as bytes of the text, in order.
        marks: &'a [Range<usize>],
    },
    /// What a folder holds, by their names.
    Folder {
        /// Its folders.
        folders: &'a [String],
        /// Its files.
        files: &'a [String],
    },
    /// A file with a NUL byte.
    NotText,
    /// A file larger than the preview reads.
    TooLarge,
    /// What was said when the file could not be read.
    Failed(&'a str),
}

/// The preview over a rectangle: its path above, then the file, read only, the line found lit and in the middle, every
/// match marked and the one of that line outlined; or the folders and the files of a folder; or why the file is not
/// shown. A double click on a line of the file opens the File page there.
///
/// Feature: file-search.preview
pub fn preview_view(state: &PreviewState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let mut asked = Vec::new();
    ui.region("search preview", (rect, AccessRole::Region, words::PREVIEW), |ui| {
        let mut area = Area::with_gap(rect, theme.sizes.spacing.small);
        let heading = inset(
            area.top(theme.sizes.row_height),
            Insets::symmetric(theme.sizes.spacing.medium, 0.0),
        );
        let _ = ui.label(
            heading,
            &Label::new(state.path).kind(TextKind::Detail).tone(Tone::Muted),
        );
        let body = area.rest();
        let said = match state.shown {
            PreviewShown::Text { buffer, colors, marks } => {
                asked.extend(text_view(state, ui, body, (buffer, colors, marks)));
                return;
            }
            PreviewShown::Folder { folders, files } => {
                folder_view(ui, body, (folders, files));
                return;
            }
            PreviewShown::NotText => words::NOT_TEXT.to_owned(),
            PreviewShown::TooLarge => words::TOO_LARGE.to_owned(),
            PreviewShown::Failed(said) => words::preview_failed(said),
        };
        let line = Rect::new(body.x, body.y, body.width, theme.sizes.row_height.min(body.height));
        let _ = ui.label(
            inset(line, Insets::symmetric(theme.sizes.spacing.medium, 0.0)),
            &Label::new(&said).tone(Tone::Muted),
        );
    });
    asked.into_iter().map(Intent::FileSearch).collect()
}

/// The text of a file in a read only editor, the line found lit; a double click on a line asks to open it there.
fn text_view(
    state: &PreviewState<'_>,
    ui: &mut Ui<'_>,
    rect: Rect,
    (buffer, colors, marks): (&Buffer, &[Vec<CodeSpan>], &[Range<usize>]),
) -> Option<FileSearchIntent> {
    let lit = state.lit_line.filter(|line| *line < buffer.line_count());
    let current_mark = lit.and_then(|line| {
        let range = buffer.line_range(line);
        let first = marks.partition_point(|mark| mark.end <= range.start);
        marks.get(first).filter(|mark| mark.start <= range.end).map(|_| first)
    });
    let editor = CodeEditor {
        name: state.path,
        buffer,
        cursor: Cursor::at(lit.map_or(0, |line| buffer.line_range(line).start)),
        colors,
        marks,
        current_mark,
        lit_line: lit,
        is_read_only: true,
        reveal: Some(state.reveal),
        focus: None,
        gutter: 0,
        tab_width: state.tab_width,
        wrap: state.wrap,
    };
    let answer = ui.code_editor("search preview editor", rect, &editor);
    answer.commands.iter().find_map(|command| match command {
        Command::SelectWord(at) | Command::SelectLine(at) => {
            Some(FileSearchIntent::OpenAt(state.path.to_owned(), at.line + 1))
        }
        _ => None,
    })
}

/// What a folder holds: its folders, then its files, each with its icon.
fn folder_view(ui: &mut Ui<'_>, rect: Rect, (folders, files): (&[String], &[String])) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::new(inset(rect, Insets::symmetric(spacing.medium, 0.0)));
    let names = folders
        .iter()
        .map(|name| (name, FileOrFolder::Folder(name)))
        .chain(files.iter().map(|name| (name, FileOrFolder::File(name))));
    for (name, icon) in names {
        if area.rest().height < theme.sizes.row_height {
            break;
        }
        let mut row = Area::with_gap(area.top(theme.sizes.row_height), spacing.medium);
        let side = theme.sizes.icon;
        ui.file_icon_in(
            place(row.left(side), (side, side), (Align::Start, Align::Center)),
            icon,
            Tone::Muted,
        );
        let _ = ui.label(row.rest(), &Label::new(name));
    }
}
