//! The head of the page of a commit: the title of the message, its labels and the rest of it, who made the commit and
//! when, its full hash and its parents, **Show in the timeline** and **Open in the Diff page**, and the count of the
//! files with the lines gained and lost; and the heading of the changes, with the buttons that show and fold them.

use model::commit_page::{Fold, PatchLines};
use model::details::{CommitDetail, Signature};
use ui::layout::{Area, Insets, inset};
use ui::{Button, ButtonKind, Label, Rect, TextKind, Tone, Ui};

use super::rows::Page;
use super::{CommitPageIntent, CommitPageState, PageData, show_in_graph};
use crate::details::{FilesOwner, layout_buttons};
use crate::intent::Intent;
use crate::text::dates::full_date;
use crate::text::details as words;

/// The width of the column of the names of the lines about who made the commit.
const KEY_WIDTH: f32 = 96.0;

/// What the head of a page is drawn from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PageHead<'a> {
    /// The page.
    pub(crate) state: &'a CommitPageState<'a>,
    /// Its commit.
    pub(crate) data: &'a PageData,
}

/// The heights of the parts of the head at a width.
struct Heights {
    title: f32,
    labels: f32,
    text: f32,
    meta: f32,
    actions: f32,
    files: f32,
}

impl Heights {
    /// The parts of a head at a width.
    fn of(ui: &Ui<'_>, head: &PageHead<'_>, width: f32) -> Heights {
        let theme = ui.theme();
        let wrapped = |text: &str, kind| {
            let style = ui.text_style(kind);
            let lines = if text.is_empty() {
                0
            } else {
                ui.fonts().wrap(text, style, width).len()
            };
            crate::number::float_of_count(lines) * ui.line_height(style)
        };
        Heights {
            title: wrapped(&head.data.title, TextKind::Heading).max(ui.line_height(ui.text_style(TextKind::Heading))),
            labels: if head.state.labels.is_empty() {
                0.0
            } else {
                ui.badge_height()
            },
            text: wrapped(&head.data.text, TextKind::Body),
            meta: crate::number::float_of_count(meta_lines(&head.data.detail)) * theme.sizes.row_height,
            actions: theme.sizes.control_height,
            files: theme.sizes.control_height,
        }
    }

    /// The parts that take room, in order.
    fn parts(&self) -> [f32; 6] {
        [self.title, self.labels, self.text, self.meta, self.actions, self.files]
    }
}

/// The height of the head at a width, with its margins.
pub(crate) fn head_height(ui: &Ui<'_>, head: &PageHead<'_>, width: f32) -> f32 {
    let spacing = ui.theme().sizes.spacing;
    let parts = Heights::of(ui, head, width).parts();
    let shown = parts.iter().filter(|part| **part > 0.0).count();
    parts.iter().sum::<f32>()
        + crate::number::float_of_count(shown.saturating_sub(1)) * spacing.large
        + 2.0 * spacing.huge
}

/// Draws the head from the top of a rectangle, and gives what was asked.
pub(crate) fn draw_head(ui: &mut Ui<'_>, head: &PageHead<'_>, top: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let spacing = ui.theme().sizes.spacing;
    let heights = Heights::of(ui, head, top.width);
    let height = head_height(ui, head, top.width);
    let rect = inset(
        Rect::new(top.x, top.y, top.width, height),
        Insets::symmetric(0.0, spacing.huge),
    );
    let mut area = Area::with_gap(rect, spacing.large);
    let _ = ui.paragraph(
        area.top(heights.title),
        &head.data.title,
        (TextKind::Heading, Tone::Normal),
    );
    if heights.labels > 0.0 {
        crate::graph::draw_labels(ui, area.top(heights.labels), (head.state.labels, head.state.lane));
    }
    if heights.text > 0.0 {
        let _ = ui.paragraph(area.top(heights.text), &head.data.text, (TextKind::Body, Tone::Normal));
    }
    meta(ui, area.top(heights.meta), head, &mut intents);
    actions(ui, area.top(heights.actions), head, &mut intents);
    files_header(ui, area.top(heights.files), head, &mut intents);
    intents
}

/// How many lines about who made the commit there are.
fn meta_lines(detail: &CommitDetail) -> usize {
    3 + usize::from(!detail.parents.is_empty()) + usize::from(committer_differs(detail).is_some())
}

/// The committer, when its name, address or date differs from the author's.
fn committer_differs(detail: &CommitDetail) -> Option<&Signature> {
    let (author, committer) = (detail.author.as_ref()?, detail.committer.as_ref()?);
    (author != committer).then_some(committer)
}

/// The lines about who made the commit and when, its hash, which a click copies, and its parents, each of which opens
/// its own page.
fn meta(ui: &mut Ui<'_>, rect: Rect, head: &PageHead<'_>, intents: &mut Vec<Intent>) {
    let detail = &head.data.detail;
    let line = ui.theme().sizes.row_height;
    let mut area = Area::new(rect);
    if let Some(author) = &detail.author {
        let value = key_line(ui, area.top(line), words::AUTHOR);
        let _ = ui.label(value, &Label::new(&words::person(&author.name, &author.email)));
        let value = key_line(ui, area.top(line), words::DATE);
        let _ = ui.label(value, &Label::new(&full_date(author.date.0)).tone(Tone::Muted));
    }
    if let Some(committer) = committer_differs(detail) {
        let value = key_line(ui, area.top(line), words::COMMITTER);
        let said = format!(
            "{} \u{b7} {}",
            words::person(&committer.name, &committer.email),
            full_date(committer.date.0)
        );
        let _ = ui.label(value, &Label::new(&said));
    }
    if let Some(commit) = &detail.id {
        let value = key_line(ui, area.top(line), words::COMMIT);
        let value = crate::details::outdent(ui, value);
        let hash = Button::secondary(commit.as_str())
            .kind(ButtonKind::Icon)
            .with_tooltip(words::COPY_THE_HASH);
        let width = ui.button_size(&hash).0.min(value.width);
        if ui
            .button(
                "copy page hash",
                Rect::new(value.x, value.y, width, value.height),
                &hash,
            )
            .clicked
        {
            ui.copy_to_clipboard(commit.as_str().to_owned());
        }
    }
    let parents = crate::details::shown_parents(detail, head.state.is_stash);
    if !parents.is_empty() {
        let key = if parents.len() > 1 {
            words::PARENTS
        } else {
            words::PARENT
        };
        let value = key_line(ui, area.top(line), key);
        let mut cells = Area::new(crate::details::outdent(ui, value));
        for parent in parents {
            let button = Button::secondary(parent.short())
                .kind(ButtonKind::Icon)
                .with_tooltip(words::OPEN_PARENT);
            let cell = cells.left(ui.button_size(&button).0);
            if ui.button(("page parent", parent.as_str()), cell, &button).clicked {
                intents.push(Intent::CommitPage(
                    head.state.id,
                    CommitPageIntent::OpenParent(parent.clone()),
                ));
            }
        }
    }
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

/// **Show in the timeline** and, when the commit changed files, **Open in the Diff page**, which comes with the Diff
/// page (Q89).
fn actions(ui: &mut Ui<'_>, rect: Rect, head: &PageHead<'_>, intents: &mut Vec<Intent>) {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let show = Button::outline(words::SHOW_IN_TIMELINE).with_icon(ui::Icon::Branch);
    let width = ui.button_size(&show).0;
    intents.extend(ui.clip(parts.left(width), |ui| show_in_graph(head.state, ui)));
    if !head.data.detail.files.is_empty() {
        let open = Button::outline(words::OPEN_IN_DIFF_PAGE)
            .with_tooltip(words::NOT_YET)
            .disabled();
        let width = ui.button_size(&open).0;
        let _ = ui.button(("open in diff page", head.state.id), parts.left(width), &open);
    }
}

/// The count of the files with the lines all of them gained and lost, **against the first parent** on a merge, and the
/// buttons of their layout.
fn files_header(ui: &mut Ui<'_>, rect: Rect, head: &PageHead<'_>, intents: &mut Vec<Intent>) {
    let detail = &head.data.detail;
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    intents.extend(layout_buttons(
        ui,
        &mut parts,
        (FilesOwner::Page(head.state.id), head.state.layout),
    ));
    let mut said = words::changed_files(detail.files.len());
    if let PatchLines::Files(patches) = &head.data.patch {
        let added = patches.iter().map(|patch| patch.added).sum();
        let removed = patches.iter().map(|patch| patch.removed).sum();
        said = format!("{said}  {}", words::lines_changed(added, removed));
    }
    if !head.state.is_stash && detail.is_merge() {
        said = format!("{said} \u{b7} {}", words::AGAINST_FIRST_PARENT);
    }
    let _ = ui.label(parts.rest(), &Label::new(&said).kind(TextKind::Medium));
}

/// The heading of the changes: **Changes**, **Show the changes** or **Hide the changes**, and while they are shown
/// **Fold all** and **Unfold all**.
pub(crate) fn changes(ui: &mut Ui<'_>, page: &Page<'_>, top: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let state = page.state;
    let rect = Rect::new(
        top.x,
        top.y + theme.sizes.spacing.large,
        top.width,
        theme.sizes.control_height,
    );
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    let heading = ui.measure(words::CHANGES, ui.text_style(TextKind::Heading));
    let _ = ui.label(parts.left(heading), &Label::new(words::CHANGES).kind(TextKind::Heading));
    let (said, wanted) = if state.shows_changes {
        (words::HIDE_THE_CHANGES, false)
    } else {
        (words::SHOW_THE_CHANGES, true)
    };
    let toggle = Button::outline(said);
    let width = ui.button_size(&toggle).0;
    if ui
        .button(("show changes", state.id), parts.left(width), &toggle)
        .clicked
    {
        intents.push(Intent::CommitPage(state.id, CommitPageIntent::ShowChanges(wanted)));
    }
    if state.shows_changes {
        for (said, fold) in [(words::FOLD_ALL, Fold::Folded), (words::UNFOLD_ALL, Fold::Unfolded)] {
            let button = Button::secondary(said).kind(ButtonKind::Icon);
            let width = ui.button_size(&button).0;
            if ui
                .button(("fold all", state.id, said), parts.left(width), &button)
                .clicked
            {
                intents.push(Intent::CommitPage(state.id, CommitPageIntent::FoldAll(fold)));
            }
        }
    }
    intents
}
