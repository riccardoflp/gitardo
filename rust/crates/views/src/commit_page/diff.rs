//! The rows of the files and of the diffs of the page of a commit: a file with its lines gained and lost and its eye; the
//! header of a diff, kept at the top while its lines scroll; the hunks and the lines, numbered before and after; the
//! notes of a diff with no lines; and the round button that goes back to the top.

use model::commit_page::{DiffLine, Fold, LineKind};
use model::state::ChangeKind;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Color, Label, MenuEntry, Point, Rect, Sense, TextKind, TextRun, Tone, Ui};

use super::CommitPageIntent;
use super::rows::{HEADER_ROWS, Page, PageRow};
use crate::details::{FileRow, FilesOwner, commit_list, compare_file, directory_row, file_start, show_all_files};
use crate::icons;
use crate::intent::Intent;
use crate::text::details as words;

/// How far the page is scrolled down before **Back to the top** shows.
const BACK_TO_TOP_AFTER: f32 = 400.0;

/// What a frame drew of the rows: the first row and where it is, and where the header of each diff in sight is.
#[derive(Clone, Debug, Default)]
pub(crate) struct Drawn {
    /// The first row drawn, and its top.
    first: Option<(usize, f32)>,
    /// The diffs whose header was drawn, and where each starts.
    headers: Vec<(usize, f32)>,
}

impl Drawn {
    /// Notes a row drawn, and says whether it is the first of the frame.
    pub(crate) fn note(&mut self, row: &ui::Row) -> bool {
        if self.first.is_some() {
            return false;
        }
        self.first = Some((row.index, row.rect.y));
        true
    }

    /// Notes where the header of a diff starts.
    pub(crate) fn header(&mut self, file: usize, top: f32) {
        if !self.headers.iter().any(|(drawn, _)| *drawn == file) {
            self.headers.push((file, top));
        }
    }
}

/// A row that is not a block: a file, the link that shows them all, the untracked files, a hunk header, a line.
pub(crate) fn simple_row(ui: &mut Ui<'_>, page: &Page<'_>, (row, kind): (&ui::Row, PageRow<'_>)) -> Vec<Intent> {
    let owner = FilesOwner::Page(page.state.id);
    match kind {
        PageRow::File(file_row @ FileRow::Directory { .. }) => directory_row(ui, row.rect, (owner, &file_row)),
        PageRow::File(FileRow::File(index, depth)) => file_row(ui, page, (row, index, depth)),
        PageRow::ShowAll => ui.clip(row.rect, |ui| show_all_files(&page.files.capped, ui)),
        PageRow::UntrackedHeading => {
            let said = words::untracked_files(page.data.detail.untracked_files.len());
            let _ = ui.label(row.rect, &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted));
            Vec::new()
        }
        PageRow::Untracked(index) => untracked_row(ui, page, (row, index)),
        PageRow::Hunk(file, hunk) => {
            if let Some(hunk) = page.patch(file).and_then(|patch| patch.hunks.get(hunk)) {
                ui.fill_rect(row.rect, ui.theme().colors.card_background);
                code_text(ui, row.rect, (&hunk.header, ui.theme().colors.muted_foreground));
            }
            Vec::new()
        }
        PageRow::Line(file, hunk, line) => {
            if let Some(line) = page
                .patch(file)
                .and_then(|patch| patch.hunks.get(hunk))
                .and_then(|hunk| hunk.lines.get(line))
            {
                diff_line(ui, row.rect, line);
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// An untracked file of a stash, which opens in the Diff page against nothing when clicked.
fn untracked_row(ui: &mut Ui<'_>, page: &Page<'_>, (row, index): (&ui::Row, usize)) -> Vec<Intent> {
    let detail = &page.data.detail;
    let Some(file) = detail.untracked_files.get(index) else {
        return Vec::new();
    };
    let theme = *ui.theme();
    let response = ui.interact(
        ui.id(("page untracked file", index)),
        row.rect,
        Sense::CLICK | Sense::HOVER,
    );
    if response.hovered {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let rest = file_start(ui, row.rect, (file, 0));
    let _ = ui.label(rest, &Label::new(&file.path));
    match crate::diff::untracked_file(detail, file) {
        Some(target) if response.clicked => vec![open(target)],
        _ => Vec::new(),
    }
}

/// A file of the list: its status, icon and path, its lines gained and lost or **binary**, and the eye that shows its
/// changes in the page; a click opens it in the Diff page; **Open the file at this revision** in its context menu, and
/// **Compare with…** (`details.compare-file-with`).
fn file_row(ui: &mut Ui<'_>, page: &Page<'_>, (row, index, depth): (&ui::Row, usize, usize)) -> Vec<Intent> {
    let Some(file) = page.data.detail.files.get(index) else {
        return Vec::new();
    };
    let theme = *ui.theme();
    let response = ui.interact(ui.id(("page file", index)), row.rect, Sense::CLICK | Sense::HOVER);
    if response.hovered {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let mut intents = Vec::new();
    let rest = file_start(ui, row.rect, (file, depth));
    let mut parts = Area::with_gap(rest, theme.sizes.spacing.medium);
    let side = row.rect.height;
    let eye = Button::icon(icons::SHOW, words::SHOW_ITS_CHANGES).kind(ButtonKind::Icon);
    if ui.button(("page file eye", index), parts.right(side), &eye).clicked {
        intents.push(Intent::CommitPage(page.state.id, CommitPageIntent::ShowFile(index)));
    }
    lines_of(ui, &mut parts, (page, index));
    let path = match (&file.original_path, depth) {
        (Some(original), 0) => words::renamed(original, &file.path),
        (_, 0) => file.path.clone(),
        _ => file.path.rsplit('/').next().unwrap_or_default().to_owned(),
    };
    let _ = ui.label(parts.rest(), &Label::new(&path));
    let at_revision = page
        .data
        .detail
        .id
        .clone()
        .filter(|_| file.kind != model::state::ChangeKind::Deleted)
        .map(|commit| {
            Intent::FileView(crate::file_view::FileViewIntent::Open(crate::file_view::FileTarget {
                path: file.path.clone(),
                revision: Some(commit),
                line: None,
            }))
        });
    let open_at_revision = icons::action(icons::OPEN_FILE, words::OPEN_AT_REVISION, true);
    let entries = [
        MenuEntry::Item(if at_revision.is_some() {
            open_at_revision
        } else {
            open_at_revision.disabled()
        }),
        MenuEntry::Item(icons::action(icons::COMPARE, words::COMPARE_WITH, false)),
    ];
    match ui.context_menu(("page file menu", index), &response, &entries) {
        Some(true) => intents.extend(at_revision),
        Some(false) => intents.push(compare_file(file, commit_list(&page.data.detail))),
        None => {}
    }
    if response.clicked && intents.is_empty() {
        intents.push(open_file(page, index));
    }
    intents
}

/// What opens the Diff page of a file of the page, by its place.
fn open_file(page: &Page<'_>, index: usize) -> Intent {
    let detail = &page.data.detail;
    match detail.files.get(index) {
        Some(file) => open(crate::diff::commit_file(detail, file)),
        None => open_all(page.state, page.data),
    }
}

/// What opens all the changes of a commit in one Diff page, titled as the page of the commit is.
pub(crate) fn open_all(state: &super::CommitPageState<'_>, data: &super::PageData) -> Intent {
    open(crate::diff::commit_files(&data.detail, state.title.to_owned()))
}

/// What opens the Diff page of a target.
fn open(target: crate::diff::DiffTarget) -> Intent {
    Intent::Diff(crate::diff::DiffIntent::Open(Box::new(target)))
}

/// The lines a file gained and lost, or **binary**, at the right of a row.
fn lines_of(ui: &mut Ui<'_>, parts: &mut Area, (page, index): (&Page<'_>, usize)) {
    let Some(patch) = page.patch(index) else {
        return;
    };
    let colors = ui.theme().colors;
    let style = ui.text_style(TextKind::Detail);
    let pieces: Vec<(String, Color)> = if patch.is_binary {
        vec![(words::BINARY.to_owned(), colors.muted_foreground)]
    } else {
        vec![
            (format!("\u{2212}{}", patch.removed), colors.removed),
            (format!("+{}", patch.added), colors.added),
        ]
    };
    for (text, color) in &pieces {
        let width = ui.measure(text, style);
        let cell = parts.right(width);
        let line = place(cell, (width, ui.line_height(style)), (Align::Start, Align::Center));
        let run = TextRun {
            text,
            style,
            color: *color,
            origin: Point::new(line.x, line.y),
            max_width: None,
        };
        let _ = ui.text(&run);
    }
}

/// The header of a diff: the arrow that folds it, its status, icon and path, which folds it too, its lines gained and
/// lost, and the button that opens it in the Diff page.
pub(crate) fn header(ui: &mut Ui<'_>, page: &Page<'_>, (file, rect): (usize, Rect)) -> Vec<Intent> {
    let Some(change) = page.data.detail.files.get(file) else {
        return Vec::new();
    };
    let theme = *ui.theme();
    let id = page.state.id;
    let card = inset(rect, Insets::symmetric(0.0, theme.sizes.spacing.small));
    ui.fill_rect(rect, theme.colors.page_background);
    ui.fill_rounded_rect(card, theme.sizes.radius.medium, theme.colors.card_background);
    let response = ui.interact(ui.id(("diff header", file)), card, Sense::CLICK | Sense::HOVER);
    let mut parts = Area::with_gap(inset(card, Insets::symmetric(theme.sizes.spacing.small, 0.0)), 4.0);
    let is_folded = page.state.folds.is_folded(file);
    let (arrow, said, wanted) = if is_folded {
        (icons::FOLDED, words::UNFOLD_DIFF, Fold::Unfolded)
    } else {
        (icons::UNFOLDED, words::FOLD_DIFF, Fold::Folded)
    };
    let mut intents = Vec::new();
    let side = theme.sizes.control_height.min(card.height);
    let fold = Button::icon(arrow, said);
    if ui
        .button(
            ("diff fold", file),
            place(parts.left(side), (side, side), (Align::Start, Align::Center)),
            &fold,
        )
        .clicked
        || response.clicked
    {
        intents.push(Intent::CommitPage(id, CommitPageIntent::Fold(file, wanted)));
    }
    let open_button = Button::icon(icons::CHANGES, words::OPEN_IN_DIFF_PAGE);
    if ui
        .button(
            ("diff open", file),
            place(parts.right(side), (side, side), (Align::Start, Align::Center)),
            &open_button,
        )
        .clicked
    {
        intents.push(open_file(page, file));
    }
    lines_of(ui, &mut parts, (page, file));
    let name_cell = file_start(ui, parts.rest(), (change, 0));
    let path = change.original_path.as_ref().map_or_else(
        || change.path.clone(),
        |original| words::renamed(original, &change.path),
    );
    let _ = ui.label(name_cell, &Label::new(&path).kind(TextKind::Medium));
    let entries = [MenuEntry::Item(icons::action(icons::COMPARE, words::COMPARE_WITH, ()))];
    if ui
        .context_menu(("diff header menu", file), &response, &entries)
        .is_some()
    {
        intents.push(compare_file(change, commit_list(&page.data.detail)));
    }
    intents
}

/// The note of a diff with no lines, with the link that opens it in the Diff page when it has some elsewhere; or, with
/// no file, what the page says when the lines cannot be matched to the files.
pub(crate) fn note(ui: &mut Ui<'_>, page: &Page<'_>, (file, rect): (Option<usize>, Rect)) -> Vec<Intent> {
    let (said, link) = match file.and_then(|file| page.patch(file)) {
        None => (words::UNMATCHED, Some(words::OPEN_IN_DIFF_PAGE)),
        Some(patch) if patch.is_binary => (words::BINARY_FILE, Some(words::OPEN_IT_IN_DIFF_PAGE)),
        Some(patch) if patch.is_past_limit => (words::TOO_LONG, Some(words::OPEN_IT_IN_DIFF_PAGE)),
        Some(patch) if patch.kind == ChangeKind::Renamed => (words::RENAMED_NO_CHANGE, None),
        Some(_) => (words::NO_CHANGE_TO_TEXT, None),
    };
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.large, 0.0)), spacing.medium);
    let width = ui.measure(said, ui.text_style(TextKind::Body));
    let _ = ui.label(parts.left(width), &Label::new(said).tone(Tone::Muted));
    if let Some(link) = link {
        let button = Button::secondary(link).kind(ButtonKind::Icon);
        let width = ui.button_size(&button).0;
        let cell = place(
            parts.left(width),
            (width, ui.theme().sizes.control_height),
            (Align::Start, Align::Center),
        );
        if ui.button(("note link", file), cell, &button).clicked {
            return vec![match file {
                Some(file) => open_file(page, file),
                None => open_all(page.state, page.data),
            }];
        }
    }
    Vec::new()
}

/// A line of a hunk: its numbers before and after, its mark, and its text, on the color of lines added or removed.
fn diff_line(ui: &mut Ui<'_>, rect: Rect, line: &DiffLine) {
    let colors = ui.theme().colors;
    let (background, mark) = match line.kind {
        LineKind::Added => (Some(colors.added_background), "+"),
        LineKind::Removed => (Some(colors.removed_background), "\u{2212}"),
        LineKind::Context | LineKind::NoNewline => (None, " "),
    };
    if let Some(background) = background {
        ui.fill_rect(rect, background);
    }
    let style = ui.text_style(TextKind::Code);
    let number_width = ui.measure("00000", style);
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    for number in [line.before, line.after] {
        let cell = parts.left(number_width);
        if let Some(number) = number {
            let said = number.to_string();
            let width = ui.measure(&said, style);
            let right = Rect::new(cell.right() - width, cell.y, width, cell.height);
            code_text(ui, right, (&said, colors.muted_foreground));
        }
    }
    let mark_cell = parts.left(ui.measure("+", style));
    code_text(ui, mark_cell, (mark, colors.muted_foreground));
    let color = if line.kind == LineKind::NoNewline {
        colors.muted_foreground
    } else {
        colors.foreground
    };
    code_text(ui, parts.rest(), (&line.text, color));
}

/// Code, in the middle of a rectangle down, cut at its right.
fn code_text(ui: &mut Ui<'_>, rect: Rect, (text, color): (&str, Color)) {
    let style = ui.text_style(TextKind::Code);
    let line = place(rect, (rect.width, ui.line_height(style)), (Align::Start, Align::Center));
    let run = TextRun {
        text: &text.replace('\t', "    "),
        style,
        color,
        origin: Point::new(line.x, line.y),
        max_width: Some(rect.width),
    };
    let _ = ui.text(&run);
}

/// The header of the diff whose lines are at the top of the page, kept there while they scroll, pushed up by the header
/// of the next diff.
pub(crate) fn sticky_header(ui: &mut Ui<'_>, page: &Page<'_>, (rect, drawn): (Rect, &Drawn)) -> Vec<Intent> {
    let Some((first, _)) = drawn.first else {
        return Vec::new();
    };
    let file = match page.rows.get(first) {
        Some(PageRow::Hunk(file, _) | PageRow::Line(file, _, _) | PageRow::Note(file, _)) => *file,
        _ => return Vec::new(),
    };
    let height = crate::number::float_of_count(HEADER_ROWS) * page.height;
    let next = drawn
        .headers
        .iter()
        .filter(|(drawn_file, _)| *drawn_file > file)
        .map(|(_, top)| *top)
        .fold(f32::INFINITY, f32::min);
    let top = rect.y.min(next - height);
    ui.clip(rect, |ui| {
        header(ui, page, (file, Rect::new(rect.x, top, rect.width, height)))
    })
}

/// The round button in the corner of the page, once it is scrolled down more than 400 pixels, that takes it back to the
/// top.
pub(crate) fn back_to_top(ui: &mut Ui<'_>, page: &Page<'_>, (rect, drawn): (Rect, &Drawn)) -> Vec<Intent> {
    let Some((first, top)) = drawn.first else {
        return Vec::new();
    };
    let scrolled = crate::number::float_of_count(first) * page.height + (rect.y - top);
    if scrolled <= BACK_TO_TOP_AFTER {
        return Vec::new();
    }
    let theme = *ui.theme();
    let side = theme.sizes.control_height + theme.sizes.spacing.medium;
    let margin = theme.sizes.spacing.huge;
    let cell = Rect::new(rect.right() - side - margin, rect.bottom() - side - margin, side, side);
    ui.fill_circle(cell.center(), side * 0.5, theme.colors.secondary_button_background);
    let button = Button::icon(icons::BACK_TO_TOP, words::BACK_TO_TOP);
    if ui.button(("back to top", page.state.id), cell, &button).clicked {
        return vec![Intent::CommitPage(page.state.id, CommitPageIntent::BackToTop)];
    }
    Vec::new()
}
