//! The whole window (D21, D56): the bar of views on its left edge, the header across the top on its right and the status
//! bar at its foot, on the ground of the window; under the header the islands: the side column of the repositories and
//! the branches, and beside it the timeline and the inspector, or a page with its trail; the Activity drawer, the
//! picker of the repositories, the question of an opened folder and a notice over them. Below about 900 logical pixels
//! the side column is put away (its button opens it over the timeline), the header shows only its icons and the gaps
//! between the islands narrow; below about 700 the inspector is not beside the timeline (it opens over it once a commit
//! can be selected).

use ui::layout::{Area, Insets, inset};
use ui::{Key, Modifiers, NamedKey, Rect, Ui};

use crate::activity::{ActivityState, StatusBarState, status_bar_view, what_is_running_view};
use crate::bisect::BisectBanner;
use crate::details::{DetailsState, show};
use crate::feature::{ConfirmState, QuestionState, SaidState, confirm_view, question_view};
use crate::filter::{BranchPicker, NarrowedBar};
use crate::graph::{
    GraphIntent, GraphState, HoverTarget, PaneLayout, PaneSizes, ReferencesShown, ToolbarState, pane_rects,
    resize_panes_view, toolbar_view,
};
use crate::intent::{Intent, Notice};
use crate::islands::{gap, island};
use crate::line::{INSPECTOR_OVER_BELOW, is_narrow};
use crate::page::Page;
use crate::page::{PageHostState, host_view};
use crate::panel::{BottomPanelState, bottom_view, panel_height, panel_shortcut, resize_panel_view, split_column};
use crate::repo::{
    NoRepositoryState, OpenFolderQuestion, RepositoryPickerState, SideColumnState, no_repository_view,
    open_folder_question_view, repository_picker_view,
};
use crate::search::{GoToCommitDialog, ListMode, SearchBox, go_to_commit_dialog};
use crate::settings::{ThemePickerState, theme_picker_view};
use crate::text::repo as repo_words;
use crate::view_bar::{VIEW_BAR_WIDTH, ViewBarState, view_bar_view};

mod side;
mod timeline;

use side::{puts_side_away, side_beside, side_over, side_toggle};

use timeline::main_island;

/// The room the search box at the top of the timeline keeps before the branch filter beside it gives up its words.
const SEARCH_BOX_ROOM: f32 = 360.0;

/// Everything the window shows, made from the store of the window for each frame.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowState<'a> {
    /// The bar of views, on the left edge.
    pub view_bar: ViewBarState,
    /// The side column of the repositories and the branches, once a workspace is open (`repo.side-column`).
    pub side: Option<SideColumnState<'a>>,
    /// Where the side column is: beside what is shown, put away, or over the timeline in the narrow layout.
    pub side_place: SidePlace,
    /// The header.
    pub header: ToolbarState<'a>,
    /// The status bar, at the foot.
    pub status: StatusBarState<'a>,
    /// What is under the header.
    pub content: Content<'a>,
    /// The bottom panel, while it is shown (`panel.bottom`).
    pub panel: Option<BottomPanelState<'a>>,
    /// The Activity drawer, while it is open.
    pub drawer: Option<ActivityState<'a>>,
    /// The picker of the repositories, while it is open.
    pub picker: Option<RepositoryPickerState<'a>>,
    /// The picker of the repositories to open in tabs, or of one to open in a tab, while it is open
    /// (`repo.choose-tabs`, `repo.open-tab`).
    pub tabs_picker: Option<crate::repo::TabsPickerState<'a>>,
    /// The question asked about a folder opened, while it waits for its answer.
    pub question: Option<OpenFolderQuestion<'a>>,
    /// The picker of the themes, while it is open (`settings.theme`).
    pub theme_picker: Option<ThemePickerState<'a>>,
    /// A sentence said once, until it is put away.
    pub notice: Option<Notice>,
    /// **Go to commit**, while it is open (`search.go-to-commit`).
    pub go_to_commit: Option<GoToCommitDialog<'a>>,
    /// The comparison chooser, while it is open (`compare.chooser`).
    pub chooser: Option<crate::compare::ChooserState<'a>>,
    /// What the operation running asks, or the plan it is about to run (`preview.confirm`).
    pub feature: Option<FeatureDialog<'a>>,
    /// What was said once an operation ended, until it is put away.
    pub said: Option<SaidState<'a>>,
    /// The question asked before a very large diff is computed (`diff.page`).
    pub large_diff: Option<crate::diff::LargeDiffQuestion<'a>>,
    /// The question asked before a very large file in conflict is shown (`merge-view.page`).
    pub large_merge: Option<crate::merge_view::LargeMergeQuestion<'a>>,
    /// The question asked before closing the page of a file with edits not saved (`file-view.edit`).
    pub close_file: Option<crate::file_view::CloseQuestion<'a>>,
}

/// Where the side column is, as the user left it (`graph.toggle-references`, `graph.resize-panes`).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SidePlace {
    /// Its width, as dragged; none where it starts.
    pub width: Option<f32>,
    /// Whether it is put away.
    pub is_hidden: bool,
    /// Whether it is opened over the timeline, in the narrow layout.
    pub is_open_over: bool,
}

impl SidePlace {
    /// Whether the side column is shown: beside what is shown while the window is wide and it is not put away, over the
    /// timeline while the window is narrow and it was opened over it.
    #[must_use]
    pub fn is_shown(self, is_narrow: bool) -> bool {
        if is_narrow { self.is_open_over } else { !self.is_hidden }
    }
}

/// What the operation running shows over the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureDialog<'a> {
    /// A question.
    Question(QuestionState<'a>),
    /// The plan, before it runs.
    Plan(ConfirmState<'a>),
}

/// What is under the header.
#[derive(Clone, Debug, PartialEq)]
pub enum Content<'a> {
    /// The timeline, with the branch rail and the inspector beside it.
    Timeline(TimelineState<'a>),
    /// A page, in place of the timeline and the inspector.
    Page(PageHostState<'a>),
}

/// The timeline and what is beside it.
#[derive(Clone, Debug, PartialEq)]
pub enum TimelineState<'a> {
    /// A repository, as read: its commits and the inspector.
    Repository {
        /// The commits.
        graph: GraphState<'a>,
        /// The inspector.
        inspector: Box<DetailsState<'a>>,
        /// The sizes of the panes, as dragged.
        panes: PaneSizes,
        /// The branch filter, beside the search box at the top of the timeline (`filter.picker`).
        filter: Box<BranchPicker<'a>>,
        /// The bar above the commits, while the timeline is narrowed to some branches (`filter.bar`).
        bar: Option<NarrowedBar<'a>>,
        /// The search box at the top of the timeline, and the bar of a search shown as a list or of the history of a
        /// commit alone, drawn in place of `bar`.
        search: Box<(SearchBox<'a>, Option<ListMode<'a>>)>,
        /// The banner of an operation that stopped, or of a rebase stopped to edit or split a commit, above the
        /// commits (`conflicts.banner`, `rebase.edit-stop`).
        conflicts: Option<crate::conflicts::BannerState<'a>>,
        /// The banner of the bisect under way, above the commits (`bisect.banner`).
        bisect: Option<BisectBanner<'a>>,
        /// The banner of another worktree shown in this window, above the commits (`worktree.banner`).
        worktree: Option<Box<crate::worktree::BannerState<'a>>>,
        /// The Diff page opened from the inspector, shown in place of the commits while it is
        /// (`diff.beside-inspector`).
        beside: Option<Box<crate::diff::BesideInspectorState<'a>>>,
    },
    /// No repository to show yet, or none at all: the timeline says why.
    NoRepository(NoRepositoryState),
}

/// The window, over all of it.
pub fn view(state: &WindowState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let window = ui.window();
    ui.fill_rect(window, theme.colors.ground_background);
    let gap = gap(ui);
    let bar_height = theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium;
    let mut area = Area::new(window);
    let status = area.bottom(theme.sizes.row_height);
    let bar = area.left(VIEW_BAR_WIDTH);
    let bar = inset(
        bar,
        Insets {
            top: gap,
            ..Insets::all(0.0)
        },
    );
    intents.extend(ui.clip(bar, |ui| view_bar_view(&state.view_bar, ui)));
    // What was said and the notices of the window are said in the status bar, which keeps its height (D56).
    let status_state = StatusBarState {
        said: state.said.or_else(|| state.notice.map(notice_said)),
        ..state.status
    };
    intents.extend(ui.clip(status, |ui| status_bar_view(&status_state, ui)));
    let rest = inset(
        area.rest(),
        Insets {
            right: gap,
            ..Insets::all(0.0)
        },
    );
    let mut column = Area::new(rest);
    let header = column.top(bar_height);
    let body = column.rest();
    // What is right of the bar of views and above the status bar is the tab's: its widgets remember how far they are
    // scrolled and what is typed in them apart from those of the other tabs (`repo.keep-open`).
    let shown_tab = state
        .side
        .as_ref()
        .and_then(|side| side.tabs.tabs.iter().find(|tab| tab.is_shown))
        .map(|tab| tab.folder.as_str());
    let panel = ui.scope(("tab", shown_tab), |ui| {
        intents.extend(ui.clip(header, |ui| toolbar_view(&state.header, ui)));
        tab_islands(state, ui, (body, gap), &mut intents)
    });
    if let Some(drawer) = &state.drawer {
        // The drawer opens over what is above the bottom panel, which it would hide.
        let above = panel.map_or(body, |panel| {
            Rect::new(body.x, body.y, body.width, (panel.y - gap - body.y).max(0.0))
        });
        intents.extend(ui.clip(above, |ui| what_is_running_view(drawer, ui)));
    }
    dialogs(state, ui, &mut intents);
    intents.extend(shortcuts(ui, state));
    intents.extend(panel_shortcut(ui, panel));
    intents.extend(crate::terminal::new_terminal_shortcut(ui));
    intents
}

/// The islands of the tab shown under the header: the side column beside the timeline or the page, and the bottom
/// panel; gives where the bottom panel is when it is shown.
fn tab_islands(
    state: &WindowState<'_>,
    ui: &mut Ui<'_>,
    (body, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) -> Option<Rect> {
    let is_narrow = is_narrow(ui);
    let side = state
        .side
        .as_ref()
        .filter(|_| !is_narrow && !state.side_place.is_hidden && !puts_side_away(&state.content));
    let (content, hover) = match side {
        Some(side) => side_beside(side, state.side_place, ui, (body, gap), intents),
        None => (body, None),
    };
    let panel = state.panel.as_ref();
    let shown = match &state.content {
        Content::Timeline(timeline) => {
            let toggle = side_toggle(state, ui);
            timeline_with_rail(timeline, (panel, hover, toggle), ui, (content, gap), intents)
        }
        Content::Page(page) => {
            let above = above_panel(panel, ui, (content, gap), intents);
            intents.extend(island(ui, above.0, |ui| host_view(page, ui)));
            above.1
        }
    };
    if let Some(side) = state
        .side
        .as_ref()
        .filter(|_| is_narrow && state.side_place.is_open_over)
    {
        side_over(side, state.side_place, ui, (content, gap), intents);
    }
    shown
}

/// The bottom panel at the foot of a column, when it is shown, with the gap above it that changes its height; gives
/// what is left above it and where the panel is.
fn above_panel(
    panel: Option<&BottomPanelState<'_>>,
    ui: &mut Ui<'_>,
    (column, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) -> (Rect, Option<Rect>) {
    let Some(panel) = panel else {
        return (column, None);
    };
    let height = panel_height(panel.height, column.height, ui.window().height);
    let (above, below) = split_column(column, height, gap);
    intents.extend(island(ui, below, |ui| bottom_view(panel, ui)));
    intents.extend(resize_panel_view(panel, (column, gap), ui));
    (above, Some(below))
}

/// The dialogs over the window, those that are open.
fn dialogs(state: &WindowState<'_>, ui: &mut Ui<'_>, intents: &mut Vec<Intent>) {
    if let Some(picker) = &state.picker {
        intents.extend(repository_picker_view(picker, ui));
    }
    if let Some(picker) = &state.tabs_picker {
        intents.extend(crate::repo::tabs_picker_view(picker, ui));
    }
    if let Some(question) = &state.question {
        intents.extend(open_folder_question_view(question, ui));
    }
    if let Some(dialog) = &state.go_to_commit {
        intents.extend(go_to_commit_dialog(dialog, ui));
    }
    if let Some(chooser) = &state.chooser {
        intents.extend(crate::compare::chooser_view(chooser, ui));
    }
    if let Some(theme_picker) = &state.theme_picker {
        intents.extend(theme_picker_view(theme_picker, ui));
    }
    if let Some(question) = &state.large_diff {
        intents.extend(crate::diff::large_diff_question_view(question, ui));
    }
    if let Some(question) = &state.large_merge {
        intents.extend(crate::merge_view::large_merge_question_view(question, ui));
    }
    if let Some(question) = &state.close_file {
        intents.extend(crate::file_view::close_question_view(question, ui));
    }
    match &state.feature {
        Some(FeatureDialog::Question(question)) => intents.extend(question_view(question, ui)),
        Some(FeatureDialog::Plan(plan)) => intents.extend(confirm_view(plan, ui)),
        None => {}
    }
}

/// The timeline and the inspector side by side, each an island with a gap between them that resizes them, and the
/// bottom panel under them when it is shown; in a narrow window the inspector opens over the commits while something
/// is selected. Gives where the bottom panel is.
fn timeline_with_rail(
    state: &TimelineState<'_>,
    beside: (
        Option<&BottomPanelState<'_>>,
        Option<(HoverTarget, std::time::Duration)>,
        Option<ReferencesShown>,
    ),
    ui: &mut Ui<'_>,
    (body, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) -> Option<Rect> {
    match state {
        TimelineState::Repository { .. } => repository_panes(state, beside, ui, (body, gap), intents),
        TimelineState::NoRepository(no_repository) => {
            let (above, shown) = above_panel(beside.0, ui, (body, gap), intents);
            intents.extend(island(ui, above, |ui| no_repository_view(no_repository, ui)));
            shown
        }
    }
}

/// The panes of a repository read: the commits, the inspector and the bottom panel under them.
fn repository_panes(
    state: &TimelineState<'_>,
    (panel, rail_hover, side): (
        Option<&BottomPanelState<'_>>,
        Option<(HoverTarget, std::time::Duration)>,
        Option<ReferencesShown>,
    ),
    ui: &mut Ui<'_>,
    (body, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) -> Option<Rect> {
    let TimelineState::Repository {
        inspector: details,
        panes,
        beside,
        ..
    } = state
    else {
        return None;
    };
    let is_inspector_over = ui.window().width < INSPECTOR_OVER_BELOW;
    if let Some(beside) = beside.as_deref().filter(|_| is_inspector_over) {
        // In the narrow layout the page takes the place of the timeline and of the inspector, as before.
        let (above, below) = above_panel(panel, ui, (body, gap), intents);
        intents.extend(island(ui, above, |ui| crate::diff::beside_inspector_view(beside, ui)));
        return below;
    }
    let bottom = panel.map_or(0.0, |panel| {
        panel_height(panel.height, body.height, ui.window().height) + gap
    });
    let layout = PaneLayout {
        body,
        sizes: *panes,
        icon_rail: None,
        is_rail_hidden: true,
        is_inspector_over,
        has_inspector: (!is_inspector_over && beside.is_none()) || details.selected.is_some(),
        gap,
        bottom,
    };
    let rects = pane_rects(&layout, gap);
    island(ui, rects.main, |ui| {
        main_island(state, (rail_hover, side), ui, rects.main, intents);
    });
    if let Some(column) = rects.inspector {
        intents.extend(island(ui, column, |ui| show(details, ui)));
    }
    intents.extend(resize_panes_view(&layout, ui));
    above_panel(panel, ui, (body, gap), intents).1
}

/// A notice of the window, said in the status bar as a warning, with **×** that puts it away (`activity.said`).
fn notice_said(notice: Notice) -> SaidState<'static> {
    let text = match notice {
        Notice::RecentGone => repo_words::RECENT_GONE,
        Notice::SingleRepository => repo_words::SINGLE_REPOSITORY,
        Notice::NoRepositoryToSelect => repo_words::NO_REPOSITORY_TO_SELECT,
        Notice::DiffLooksBinary => crate::text::diff::LOOKS_BINARY,
        Notice::DiffTooComplex => crate::text::diff::TOO_MANY_DIFFERENCES,
        Notice::MergeNotInConflict => crate::text::merge_view::NOT_IN_CONFLICT,
    };
    SaidState {
        text,
        is_error: false,
        can_undo: false,
        offers_stashes: false,
        found: None,
        offer: None,
        stopped: None,
        runs: &[],
        link: None,
        is_notice: true,
        log_at: None,
        shown_at: None,
    }
}

/// The keys of the window that no widget used: `Ctrl+Tab` goes through the timeline and the pages open, `Ctrl+,`
/// opens the settings page, and `Escape` closes the side column opened over the timeline, else the Activity drawer
/// while it is open, else brings the timeline back from the Diff page beside the inspector (`diff.beside-inspector`).
fn shortcuts(ui: &Ui<'_>, state: &WindowState<'_>) -> Vec<Intent> {
    let is_side_over = state.side.is_some() && is_narrow(ui) && state.side_place.is_open_over;
    let is_drawer_open = state.drawer.is_some();
    let is_beside = matches!(
        &state.content,
        Content::Timeline(TimelineState::Repository { beside: Some(_), .. })
    );
    ui.unclaimed_keys()
        .into_iter()
        .filter_map(|key| match (key.key, key.modifiers) {
            (Key::Named(NamedKey::Tab), Modifiers::CONTROL) => Some(Intent::NextPage),
            (Key::Character(','), Modifiers::CONTROL) => Some(Intent::ShowPage(Page::Settings)),
            (Key::Named(NamedKey::Escape), Modifiers::NONE) if is_side_over => {
                Some(Intent::Graph(GraphIntent::CloseSideColumn))
            }
            (Key::Named(NamedKey::Escape), Modifiers::NONE) if is_drawer_open => Some(Intent::CloseActivity),
            (Key::Named(NamedKey::Escape), Modifiers::NONE) if is_beside => Some(Intent::ShowTimeline),
            _ => None,
        })
        .collect()
}
