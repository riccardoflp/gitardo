//! The head of the card of the inspector, a block over the files: the buttons that close the inspector and open the
//! commit as a page, the title of the message and the text under it, who made the commit and when, its hash and
//! parents, and the count of its files with the buttons of their layout.

use model::CommitId;
use model::details::{CommitDetail, Signature};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Label, Rect, Sense, TextKind, Tone, Ui};

use super::files::layout_buttons;
use super::message::{MessageState, show_more, text_height};
use super::{DetailsIntent, FileLayout, FilesOwner};
use crate::graph::GraphIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::dates::full_date;
use crate::text::details as words;

/// The width of the column of the names of the lines about who made the commit.
const KEY_WIDTH: f32 = 84.0;

/// What the head of the card shows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Head<'a> {
    /// The title of the message.
    pub(crate) title: &'a str,
    /// The text under it.
    pub(crate) message: MessageState<'a>,
    /// The commit, none for the draft.
    pub(crate) detail: Option<&'a CommitDetail>,
    /// What was read of the commit or the draft, whose files open in the Diff page.
    pub(crate) read: &'a CommitDetail,
    /// How many files, and how they are laid out.
    pub(crate) files: (usize, FileLayout),
    /// Whether the commit is a stash, whose parents after the first are where git keeps its index and its untracked
    /// files: only the commit it was made on is shown.
    pub(crate) is_stash: bool,
    /// The signature of the commit, when it has a line (`details.signature`).
    pub(crate) signature: Option<super::SignatureRead<'a>>,
}

/// The heights of the parts of the head at a width.
struct Heights {
    title: f32,
    text: f32,
    meta: f32,
    files: f32,
}

impl Heights {
    /// The parts of a head at a width.
    fn of(ui: &Ui<'_>, head: &Head<'_>, width: f32) -> Heights {
        let theme = ui.theme();
        let side = theme.sizes.control_height;
        let heading = ui.text_style(TextKind::Heading);
        let title_lines = ui
            .fonts()
            .wrap(head.title, heading, title_width(ui, width))
            .len()
            .max(1);
        let title = (crate::number::float_of_count(title_lines) * ui.line_height(heading)).max(side);
        let text = text_height(ui, &head.message, width);
        let meta = head.detail.map_or(0.0, |detail| {
            let lines = meta_lines(detail) + usize::from(head.signature.is_some());
            crate::number::float_of_count(lines) * theme.sizes.row_height
        });
        Heights {
            title,
            text,
            meta,
            files: side,
        }
    }

    /// The height of the whole head, with the gaps between its parts.
    fn total(&self, gap: f32) -> f32 {
        let gaps = [self.text, self.meta].iter().filter(|part| **part > 0.0).count();
        self.title + self.text + self.meta + self.files + crate::number::float_of_count(gaps + 1) * gap
    }
}

/// The width the title wraps at, beside the buttons.
fn title_width(ui: &Ui<'_>, width: f32) -> f32 {
    let theme = ui.theme();
    (width - 2.0 * theme.sizes.control_height - theme.sizes.spacing.medium).max(0.0)
}

/// How many lines about who made the commit there are: the author, the date, the committer when it differs, the hash
/// and the parents.
fn meta_lines(detail: &CommitDetail) -> usize {
    let parents = usize::from(!detail.parents.is_empty());
    3 + parents + usize::from(committer_differs(detail).is_some())
}

/// The committer, when its name or its date differs from the author's.
fn committer_differs(detail: &CommitDetail) -> Option<&Signature> {
    let (author, committer) = (detail.author.as_ref()?, detail.committer.as_ref()?);
    (author.name != committer.name || author.date != committer.date).then_some(committer)
}

/// The height of the head at a width.
pub(crate) fn head_height(ui: &Ui<'_>, head: &Head<'_>, width: f32) -> f32 {
    Heights::of(ui, head, width).total(ui.theme().sizes.spacing.large)
}

/// Draws the head from the top of a rectangle, and gives what was asked.
pub(crate) fn draw_head(ui: &mut Ui<'_>, head: &Head<'_>, rect: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let gap = ui.theme().sizes.spacing.large;
    let heights = Heights::of(ui, head, rect.width);
    let mut area = Area::with_gap(Rect::new(rect.x, rect.y, rect.width, heights.total(gap)), gap);
    title_line(ui, area.top(heights.title), head, &mut intents);
    if heights.text > 0.0 {
        intents.extend(ui.clip(area.top(heights.text), |ui| show_more(&head.message, ui)));
    }
    if let Some(detail) = head.detail.filter(|_| heights.meta > 0.0) {
        meta(ui, area.top(heights.meta), (detail, head), &mut intents);
    }
    files_header(ui, area.top(heights.files), head, &mut intents);
    intents
}

/// The title of the message, with the buttons that open the commit as a page and close the inspector at its right.
fn title_line(ui: &mut Ui<'_>, rect: Rect, head: &Head<'_>, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let mut buttons = Area::new(parts.right(2.0 * side - ui.theme().sizes.spacing.medium));
    let close = Button::icon(icons::CLOSE, words::CLOSE_INSPECTOR);
    if ui
        .button("close inspector", buttons.right(side).with_height(side), &close)
        .clicked
    {
        intents.push(Intent::Details(DetailsIntent::Close));
    }
    if let Some(commit) = head.detail.and_then(|detail| detail.id.as_ref()) {
        let open = Button::icon(icons::OPEN_AS_PAGE, words::OPEN_AS_PAGE);
        if ui
            .button("open as page", buttons.right(side).with_height(side), &open)
            .clicked
        {
            intents.push(Intent::Details(DetailsIntent::OpenPage(commit.clone())));
        }
    }
    let _ = ui.paragraph(parts.rest(), head.title, (TextKind::Heading, Tone::Normal));
}

/// The lines about who made the commit, when, its hash and its parents.
fn meta(ui: &mut Ui<'_>, rect: Rect, (detail, head): (&CommitDetail, &Head<'_>), intents: &mut Vec<Intent>) {
    let line = ui.theme().sizes.row_height;
    let mut area = Area::new(rect);
    if let Some(author) = &detail.author {
        let value = key_line(ui, area.top(line), words::AUTHOR);
        let _ = ui.label(value, &Label::new(&author.name).kind(TextKind::Medium));
        let response = ui.interact(ui.id("author email"), value, Sense::HOVER);
        ui.tooltip(&response, &author.email);
        let value = key_line(ui, area.top(line), words::DATE);
        let _ = ui.label(value, &Label::new(&full_date(author.date.0)).tone(Tone::Muted));
    }
    if let Some(committer) = committer_differs(detail) {
        let value = key_line(ui, area.top(line), words::COMMITTER);
        let said = format!("{} \u{b7} {}", committer.name, full_date(committer.date.0));
        let _ = ui.label(value, &Label::new(&said));
    }
    if let Some(commit) = &detail.id {
        let value = key_line(ui, area.top(line), words::COMMIT);
        let hash = Button::secondary(commit.short())
            .kind(ButtonKind::Icon)
            .with_tooltip(words::COPY_FULL_HASH);
        if ui.button("copy hash", fitted(ui, value, &hash), &hash).clicked {
            ui.copy_to_clipboard(commit.as_str().to_owned());
        }
    }
    if let Some(read) = head.signature {
        let state = super::SignatureState {
            read,
            key_width: KEY_WIDTH,
        };
        intents.extend(ui.clip(area.top(line), |ui| super::signature_view(&state, ui)));
    }
    let shown = shown_parents(detail, head.is_stash);
    if !shown.is_empty() {
        let key = if shown.len() > 1 { words::PARENTS } else { words::PARENT };
        let value = key_line(ui, area.top(line), key);
        parents(ui, value, shown, intents);
    }
}

/// The parents shown: of a stash, only the commit it was made on.
pub(crate) fn shown_parents(detail: &CommitDetail, is_stash: bool) -> &[CommitId] {
    if is_stash {
        detail.parents.get(..1).unwrap_or_default()
    } else {
        &detail.parents
    }
}

/// The parents, each a short hash that selects it in the timeline.
fn parents(ui: &mut Ui<'_>, rect: Rect, parents: &[CommitId], intents: &mut Vec<Intent>) {
    let mut cells = Area::new(outdent(ui, rect));
    for parent in parents {
        let button = Button::secondary(parent.short())
            .kind(ButtonKind::Icon)
            .with_tooltip(words::GO_TO_PARENT);
        let width = ui.button_size(&button).0;
        let cell = cells.left(width);
        if ui.button(("parent", parent.as_str()), cell, &button).clicked {
            intents.push(Intent::Graph(GraphIntent::GoTo(parent.clone())));
        }
    }
}

/// A button as wide as its words, at the start of a cell, its words where the words of the other lines start.
fn fitted(ui: &Ui<'_>, cell: Rect, button: &Button<'_>) -> Rect {
    let cell = outdent(ui, cell);
    let width = ui.button_size(button).0.min(cell.width);
    Rect::new(cell.x, cell.y, width, cell.height)
}

/// A cell moved left by the margin of a button, so that the words of the buttons in it start where the words of the
/// other lines do.
pub(crate) fn outdent(ui: &Ui<'_>, cell: Rect) -> Rect {
    let margin = ui.theme().sizes.spacing.large;
    Rect::new(cell.x - margin, cell.y, cell.width + margin, cell.height)
}

/// The name of a line in the muted tone, and the rest of the line for its value.
fn key_line(ui: &mut Ui<'_>, rect: Rect, key: &str) -> Rect {
    let mut parts = Area::new(rect);
    let _ = ui.label(
        parts.left(KEY_WIDTH),
        &Label::new(key).kind(TextKind::Detail).tone(Tone::Muted),
    );
    parts.rest()
}

/// The count of the files, **against the first parent** on a merge, the buttons that open them all in the Diff page and
/// that choose their layout.
fn files_header(ui: &mut Ui<'_>, rect: Rect, head: &Head<'_>, intents: &mut Vec<Intent>) {
    let (count, layout) = head.files;
    let side = ui.theme().sizes.control_height;
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    intents.extend(layout_buttons(ui, &mut parts, (FilesOwner::Inspector, layout)));
    if count > 1 {
        let open = Button::icon(icons::OPEN_ALL_CHANGES, words::OPEN_ALL_CHANGES);
        let cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        if ui.button("open all changes", cell, &open).clicked {
            let title = all_changes_title(head.read);
            intents.push(super::rows::open_diff(crate::diff::commit_files(head.read, title)));
        }
    }
    let mut said = words::changed_files(count);
    if !head.is_stash && head.detail.is_some_and(CommitDetail::is_merge) {
        said = format!("{said} \u{b7} {}", words::AGAINST_FIRST_PARENT);
    }
    let _ = ui.label(
        parts.rest(),
        &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted),
    );
}

/// The title of the Diff page of all the files of a commit: its short hash and subject, or the subject of the draft.
pub(crate) fn all_changes_title(detail: &CommitDetail) -> String {
    let subject = detail.message.lines().next().unwrap_or_default();
    match &detail.id {
        Some(commit) => words::page_title(commit.short(), subject),
        None => subject.to_owned(),
    }
}

/// A rectangle of a height, from its top.
trait WithHeight {
    /// The same rectangle, of another height.
    fn with_height(self, height: f32) -> Rect;
}

impl WithHeight for Rect {
    fn with_height(self, height: f32) -> Rect {
        Rect::new(self.x, self.y, self.width, height)
    }
}
