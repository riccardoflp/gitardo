//! `rebase.todo-page`: the list of the commits from a commit up to `HEAD`, to reorder and to pick, reword, edit,
//! squash, fixup or drop each one before the rebase starts.

use model::CommitId;
use model::rebase::{TodoAction, TodoRow, write_todo_rows};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, InputBox, Label, MenuAnchor, MenuEntry, MenuItem, Rect, TextKind, Tone, Ui};

use super::RebaseIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::rebase as words;

/// The widest the page is drawn.
const PAGE_WIDTH: f32 = 860.0;

/// The actions in the order the menu of a row lists them, the order of [`words::ACTIONS`].
const ACTIONS: [TodoAction; 6] = [
    TodoAction::Pick,
    TodoAction::Reword,
    TodoAction::Edit,
    TodoAction::Squash,
    TodoAction::Fixup,
    TodoAction::Drop,
];

/// A row of the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoPageRow {
    /// The commit.
    pub commit: CommitId,
    /// Its subject.
    pub subject: String,
    /// What the rebase does with it.
    pub action: TodoAction,
    /// What the box of a reworded commit holds; it starts with the subject.
    pub new_subject: String,
}

/// The page while it is open: the branch and the rows, newest first, as the timeline lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoPage {
    /// The branch checked out, or `HEAD` when detached.
    pub branch: String,
    /// The rows, newest first.
    pub rows: Vec<TodoPageRow>,
    /// The commits in the order the page started with, to tell whether anything changed.
    pub started: Vec<CommitId>,
}

impl TodoPage {
    /// The page of commits, newest first, each with its subject, every row **Pick**.
    #[must_use]
    pub fn new(branch: String, commits: Vec<(CommitId, String)>) -> TodoPage {
        let started = commits.iter().map(|(commit, _)| commit.clone()).collect();
        let rows = commits
            .into_iter()
            .map(|(commit, subject)| TodoPageRow {
                commit,
                new_subject: subject.clone(),
                subject,
                action: TodoAction::Pick,
            })
            .collect();
        TodoPage { branch, rows, started }
    }

    /// The page after an action, a move or a subject typed; opening, **Cancel** and **Start rebase** are the store's.
    pub fn apply(&mut self, intent: &RebaseIntent) {
        match intent {
            RebaseIntent::SetAction(index, action) => {
                if let Some(row) = self.rows.get_mut(*index) {
                    row.action = *action;
                }
            }
            RebaseIntent::Move { from, to } => {
                if *from < self.rows.len() && *to < self.rows.len() {
                    let row = self.rows.remove(*from);
                    self.rows.insert(*to, row);
                }
            }
            RebaseIntent::TypedSubject(index, text) => {
                if let Some(row) = self.rows.get_mut(*index) {
                    text.clone_into(&mut row.new_subject);
                }
            }
            RebaseIntent::Open(_) | RebaseIntent::Cancel | RebaseIntent::Start => {}
        }
    }

    /// What keeps the rebase from running, said in the footer.
    #[must_use]
    pub fn problem(&self) -> Option<&'static str> {
        let kept = self.rows.iter().rev().find(|row| row.action != TodoAction::Drop);
        match kept.map(|row| row.action) {
            None => return Some(words::EVERY_COMMIT_DROPPED),
            Some(TodoAction::Squash | TodoAction::Fixup) => return Some(words::NOTHING_TO_MELD_INTO),
            Some(_) => {}
        }
        self.rows
            .iter()
            .any(|row| row.action == TodoAction::Reword && row.new_subject.trim().is_empty())
            .then_some(words::ENTER_A_MESSAGE)
    }

    /// Whether anything changed since the page opened: an action other than **Pick**, or another order.
    #[must_use]
    pub fn is_changed(&self) -> bool {
        self.rows.iter().any(|row| row.action != TodoAction::Pick)
            || !self.rows.iter().map(|row| &row.commit).eq(self.started.iter())
    }

    /// The rows, oldest first, as `rebase.interactive` reads them.
    #[must_use]
    pub fn todo_rows(&self) -> String {
        let rows: Vec<TodoRow> = self
            .rows
            .iter()
            .rev()
            .map(|row| TodoRow {
                action: row.action,
                commit: row.commit.clone(),
                new_subject: (row.action == TodoAction::Reword).then(|| row.new_subject.trim().to_owned()),
            })
            .collect();
        write_todo_rows(&rows)
    }
}

/// What the page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TodoPageState<'a> {
    /// The page.
    pub page: &'a TodoPage,
}

/// The name and the help of an action.
fn action_words(action: TodoAction) -> (&'static str, &'static str) {
    let place = ACTIONS.iter().position(|each| *each == action).unwrap_or_default();
    words::ACTIONS.get(place).copied().unwrap_or(("", ""))
}

/// The interactive rebase page: its header, one row a commit with a grip to drag it by, its action, its short hash,
/// its subject or the box of its new subject, and **Move up** and **Move down**; and a footer with what keeps it from
/// running, **Cancel** and **Start rebase**.
///
/// Feature: rebase.todo-page
pub fn todo_page_view(state: &TodoPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let clip = ui.current_clip();
    let page = inset(
        Rect::new(clip.x, clip.y, PAGE_WIDTH.min(clip.width), clip.height),
        Insets::all(spacing.large + spacing.small),
    );
    let mut area = Area::with_gap(page, spacing.medium);
    let header = words::page_header(&state.page.branch, state.page.rows.len());
    let _ = ui.heading(area.top(ui.line_height(ui.text_style(TextKind::Heading))), &header);
    let under = Label::new(words::NEWEST_FIRST).tone(Tone::Muted);
    let _ = ui.label(area.top(ui.line_height(ui.text_style(TextKind::Body))), &under);
    footer(state.page, ui, area.bottom(theme.sizes.control_height), &mut intents);
    let list = area.rest();
    let row_height = theme.sizes.control_height + spacing.medium;
    let count = f32::from(u16::try_from(state.page.rows.len()).unwrap_or(u16::MAX));
    let (_, ()) = ui.scroll_area("todo rows", list, (list.width, row_height * count), |ui, inside| {
        let mut top = inside.y;
        for (index, row) in state.page.rows.iter().enumerate() {
            let rect = Rect::new(inside.x, top, inside.width, row_height);
            intents.extend(todo_row(state.page, (index, row), ui, rect));
            top += row_height;
        }
    });
    intents
}

/// A row: dropped on, it takes the row dragged to its place.
fn todo_row(page: &TodoPage, (index, row): (usize, &TodoPageRow), ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let dropped = ui.drop_target(rect).dropped;
    if let Some(from) =
        dropped.and_then(|source| (0..page.rows.len()).find(|each| ui.id(("todo grip", *each)) == source))
    {
        intents.push(Intent::Rebase(RebaseIntent::Move { from, to: index }));
    }
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(0.0, spacing.small)), spacing.medium);
    let grip = parts.left(theme.sizes.icon);
    let _ = ui.drag_source(("todo grip", index), grip, &row.subject);
    ui.icon_in(grip, icons::DRAG, Tone::Muted);
    intents.extend(action_button((index, row.action), ui, parts.left(110.0)));
    let last = page.rows.len().saturating_sub(1);
    for (key, icon, label, to, is_end) in [
        (
            "todo down",
            icons::MOVE_DOWN,
            words::MOVE_DOWN,
            index + 1,
            index == last,
        ),
        (
            "todo up",
            icons::MOVE_UP,
            words::MOVE_UP,
            index.saturating_sub(1),
            index == 0,
        ),
    ] {
        let mut button = Button::icon(icon, label);
        if is_end {
            button = button.disabled();
        }
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button((key, index), cell, &button).clicked {
            intents.push(Intent::Rebase(RebaseIntent::Move { from: index, to }));
        }
    }
    let short = Label::new(row.commit.short()).kind(TextKind::Code).tone(Tone::Muted);
    let _ = ui.label(parts.left(70.0), &short);
    let subject = parts.rest();
    if row.action == TodoAction::Reword {
        let input = InputBox::new(&row.new_subject, words::NEW_SUBJECT);
        if let Some(text) = ui.input_box(("todo subject", index), subject, &input).changed_text {
            intents.push(Intent::Rebase(RebaseIntent::TypedSubject(index, text)));
        }
    } else {
        let _ = ui.label(subject, &Label::new(&row.subject));
    }
    intents
}

/// The action of a row, with its help as tooltip, and the menu of the actions it opens.
fn action_button((index, action): (usize, TodoAction), ui: &mut Ui<'_>, cell: Rect) -> Option<Intent> {
    let (name, help) = action_words(action);
    let button = Button::secondary(name)
        .kind(ButtonKind::Outline)
        .with_trailing(icons::CHOOSE)
        .with_tooltip(help);
    if ui.button(("todo action", index), cell, &button).clicked {
        ui.open_menu(("todo actions", index), MenuAnchor::Below(cell));
    }
    let entries: Vec<MenuEntry<'_, TodoAction>> = ACTIONS
        .iter()
        .map(|each| MenuEntry::Item(MenuItem::new(action_words(*each).0, *each)))
        .collect();
    let chosen = ui.menu(("todo actions", index), &entries)?;
    Some(Intent::Rebase(RebaseIntent::SetAction(index, chosen)))
}

/// The footer: what keeps the rebase from running, **Cancel** and **Start rebase**, disabled while something keeps it
/// from running or nothing changed.
fn footer(page: &TodoPage, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(rect, spacing.medium);
    let problem = page.problem();
    let mut start = Button::primary(words::START_REBASE);
    if problem.is_some() || !page.is_changed() {
        start = start.disabled();
    }
    let (width, height) = ui.button_size(&start);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    if ui.button("start rebase", cell, &start).clicked {
        intents.push(Intent::Rebase(RebaseIntent::Start));
    }
    let cancel = Button::secondary(words::CANCEL).kind(ButtonKind::Outline);
    let (width, height) = ui.button_size(&cancel);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    if ui.button("cancel rebase", cell, &cancel).clicked {
        intents.push(Intent::Rebase(RebaseIntent::Cancel));
    }
    if let Some(problem) = problem {
        let _ = ui.label(parts.rest(), &Label::new(problem).tone(Tone::Error));
    }
}
