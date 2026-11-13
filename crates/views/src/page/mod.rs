//! The pages, opened in place of the timeline and the inspector with a trail above them, `Timeline › Welcome`, whose
//! first step comes back to the timeline.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Button, ButtonKind, Icon, Label, Rect, TextKind, Tone, Ui};

use crate::commit_page::{CommitPageId, CommitPageState, view as commit_page_view};
use crate::compare::{ComparePageId, CompareState, page as compare_page_view};
use crate::diff::{DiffPageId, DiffPageState, page_view as diff_page_view};
use crate::file_view::{FilePageId, FilePageState, page_view as file_page_view};
use crate::intent::Intent;
use crate::settings::{SettingsPageState, settings_view};
use crate::text::pages as words;
use crate::text::settings as settings_words;
use crate::welcome::{WelcomeState, page_view};

/// A page of the window. Each is open once: asking for it again shows the one there is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Page {
    /// The Welcome page (`welcome.page`).
    Welcome,
    /// The page of a commit (`commit-page.view`).
    Commit(CommitPageId),
    /// The settings page (`settings.edit`).
    Settings,
    /// The Changes page (`changes.pane`).
    Changes,
    /// A Diff page (`diff.page`).
    Diff(DiffPageId),
    /// The interactive rebase page (`rebase.todo-page`).
    Rebase,
    /// A comparison page (`compare.page`).
    Compare(ComparePageId),
    /// A Merge page (`merge-view.page`).
    Merge(crate::merge_view::MergePageId),
    /// The review of the resolutions of the conflicts (`review.page`).
    Review,
    /// A File page (`file-view.page`).
    File(FilePageId),
    /// The Files page: the tree of the working tree.
    Files,
}

impl Page {
    /// Its name in the trail.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Page::Welcome => words::WELCOME,
            Page::Commit(_) => crate::text::details::PAGE,
            Page::Settings => settings_words::PAGE,
            Page::Changes => crate::text::changes::PAGE,
            Page::Diff(_) => crate::text::diff::PAGE,
            Page::Rebase => crate::text::rebase::PAGE,
            Page::Compare(_) => crate::text::compare::PAGE,
            Page::Merge(_) => crate::text::merge_view::PAGE,
            Page::Review => crate::text::review::PAGE,
            Page::File(_) => crate::text::file_view::PAGE,
            Page::Files => crate::text::files::PAGE,
        }
    }
}

/// What a page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageState<'a> {
    /// The Welcome page.
    Welcome(WelcomeState<'a>),
    /// The page of a commit.
    Commit(CommitPageState<'a>),
    /// The settings page.
    Settings(SettingsPageState<'a>),
    /// The Changes page.
    Changes(crate::changes::ChangesState<'a>),
    /// A Diff page.
    Diff(DiffPageState<'a>),
    /// The interactive rebase page.
    Rebase(crate::rebase::TodoPageState<'a>),
    /// A comparison page.
    Compare(CompareState<'a>),
    /// A Merge page, with its title.
    Merge(crate::merge_view::MergePageState<'a>, &'a str),
    /// The review of the resolutions of the conflicts.
    Review(crate::review::ReviewState<'a>),
    /// A File page.
    File(FilePageState<'a>),
    /// The Files page.
    Files(crate::files::FilesPageState<'a>),
}

impl PageState<'_> {
    /// Which page it is.
    #[must_use]
    pub fn page(&self) -> Page {
        match self {
            PageState::Welcome(_) => Page::Welcome,
            PageState::Commit(commit) => Page::Commit(commit.id),
            PageState::Settings(_) => Page::Settings,
            PageState::Changes(_) => Page::Changes,
            PageState::Diff(diff) => Page::Diff(diff.id),
            PageState::Rebase(_) => Page::Rebase,
            PageState::Compare(compare) => Page::Compare(compare.id),
            PageState::Merge(merge, _) => Page::Merge(merge.id),
            PageState::Review(_) => Page::Review,
            PageState::File(file) => Page::File(file.id),
            PageState::Files(_) => Page::Files,
        }
    }

    /// Its name in the trail: the name of the page, or the short hash and the subject of a commit.
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            PageState::Commit(commit) => commit.title,
            PageState::Welcome(_)
            | PageState::Settings(_)
            | PageState::Changes(_)
            | PageState::Rebase(_)
            | PageState::Files(_) => self.page().title(),
            PageState::Diff(diff) => diff.title,
            PageState::Compare(compare) => compare.title,
            PageState::Merge(_, title) => title,
            PageState::Review(review) => review.title,
            PageState::File(file) => file.title,
        }
    }

    /// Whether it shows a file with edits not saved, which puts a dot after its name in the trail (`file-view.edit`).
    #[must_use]
    pub fn is_unsaved(&self) -> bool {
        match self {
            PageState::File(file) => file.is_unsaved,
            PageState::Diff(diff) => diff.is_unsaved,
            _ => false,
        }
    }
}

/// The page shown, with its trail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageHostState<'a> {
    /// Whether the trail is shown: not while the window shows no workspace, when there is no timeline to come back to.
    pub has_trail: bool,
    /// The page.
    pub page: PageState<'a>,
}

/// The page shown, over the rectangle its caller clipped it to, under its trail: **Timeline**, which comes back to the
/// timeline, then the name of the page, and a button that closes it.
///
/// Feature cards that fill it: every page of the catalog opens here (`commit-page.view`, `compare.page`, the Diff,
/// Merge, Conflicts, File, Files, Changes, Settings and Worktree setup pages), each adding its own steps to the trail
/// (`Timeline › Commit 4b830a6 › src/login/page.rs`).
pub fn host_view(state: &PageHostState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let mut area = Area::new(rect);
    if state.has_trail {
        let bar = area.top(ui.theme().sizes.header_height * 0.75);
        trail((state.page.title(), state.page.is_unsaved()), ui, bar, &mut intents);
    }
    let body = area.rest();
    intents.extend(ui.clip(body, |ui| match &state.page {
        PageState::Welcome(welcome) => page_view(welcome, ui),
        PageState::Commit(commit) => commit_page_view(commit, ui),
        PageState::Settings(settings) => settings_view(settings, ui),
        PageState::Changes(changes) => crate::changes::pane(changes, ui),
        PageState::Diff(diff) => diff_page_view(diff, ui),
        PageState::Rebase(rebase) => crate::rebase::todo_page_view(rebase, ui),
        PageState::Compare(compare) => compare_page_view(compare, ui),
        PageState::Merge(merge, _) => crate::merge_view::page_view(merge, ui),
        PageState::Review(review) => crate::review::page_view(review, ui),
        PageState::File(file) => file_page_view(file, ui),
        PageState::Files(files) => crate::files::page_view(files, ui),
    }));
    intents
}

/// The trail above a page, with a dot after the name of a file with edits not saved, and the button that closes it.
fn trail((title, is_unsaved): (&str, bool), ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    ui.region("trail", (rect, AccessRole::Navigation, words::TRAIL), |ui| {
        let inner = inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0));
        let mut parts = Area::with_gap(inner, spacing.medium);
        let timeline = Button::secondary(words::TIMELINE).kind(ButtonKind::Icon);
        let (width, height) = ui.button_size(&timeline);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button("trail timeline", cell, &timeline).clicked {
            intents.push(Intent::ShowTimeline);
        }
        let separator_width = ui.measure(words::TRAIL_SEPARATOR, ui.text_style(TextKind::Body));
        let _ = ui.label(
            parts.left(separator_width),
            &Label::new(words::TRAIL_SEPARATOR).tone(Tone::Muted),
        );
        let side = theme.sizes.control_height;
        let close_cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        let close = Button::icon(Icon::Close, words::CLOSE_PAGE).kind(ButtonKind::Icon);
        if ui.button("close page", close_cell, &close).clicked {
            intents.push(Intent::ClosePage);
        }
        if is_unsaved {
            let dot = crate::text::file_view::UNSAVED_DOT;
            let title_width = ui.measure(title, ui.text_style(TextKind::Strong));
            let _ = ui.label(parts.left(title_width), &Label::new(title).kind(TextKind::Strong));
            let dot_width = ui.measure(dot, ui.text_style(TextKind::Body));
            let _ = ui.label(parts.left(dot_width), &Label::new(dot));
        } else {
            let _ = ui.label(parts.rest(), &Label::new(title).kind(TextKind::Strong));
        }
    });
}
