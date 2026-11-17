//! The whole window (D21): the bar of views on its left edge and the status bar at its foot, on the ground of the
//! window; beside them the islands: the tabs at the top (D50), the header under them, under it the branch rail, the timeline and the inspector, or
//! a page with its trail; the Activity drawer, the picker of the repositories, the question of an opened folder and a
//! notice over them. Below about 900 logical pixels the rail shows only icons, the header only its icons and the gaps
//! between the islands narrow; below about 700 the inspector is not beside the timeline (it opens over it once a
//! commit can be selected).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Icon, Key, Label, Modifiers, NamedKey, Rect, Tone, Ui};

use crate::activity::{ActivityState, StatusBarState, status_bar_view, what_is_running_view};
use crate::bisect::{BisectBanner, banner_view};
use crate::details::{DetailsState, show};
use crate::feature::{ConfirmState, QuestionState, SaidState, confirm_view, question_view, said_view};
use crate::filter::{BranchPicker, NarrowedBar, bar_height, bar_view, picker_button_width, picker_view};
use crate::graph::{
    GraphState, HoverTarget, PaneLayout, PaneSizes, ToolbarState, pane_rects, resize_panes_view, toolbar_view,
    view as graph_view,
};
use crate::intent::{Intent, Notice};
use crate::islands::{gap, island};
use crate::line::{ICON_RAIL_WIDTH, INSPECTOR_OVER_BELOW, control_in, is_narrow};
use crate::page::Page;
use crate::page::{PageHostState, host_view};
use crate::panel::{BottomPanelState, bottom_view, panel_height, panel_shortcut, resize_panel_view, split_column};
use crate::refs::ReferencesState;
use crate::repo::{
    NoRepositoryState, OpenFolderQuestion, RepositoryPickerState, TabsState, no_repository_view,
    open_folder_question_view, repository_picker_view, tabs_view,
};
use crate::search::{GoToCommitDialog, ListMode, SearchBox, go_to_commit_dialog, list_mode_view};
use crate::settings::{ThemePickerState, theme_picker_view};
use crate::text::repo as repo_words;
use crate::view_bar::{VIEW_BAR_WIDTH, ViewBarState, view_bar_view};

/// The room the search box at the top of the timeline keeps before the branch filter beside it gives up its words.
const SEARCH_BOX_ROOM: f32 = 360.0;

/// Everything the window shows, made from the store of the window for each frame.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowState<'a> {
    /// The bar of views, on the left edge.
    pub view_bar: ViewBarState,
    /// The island of the tabs, at the top, once a workspace is open (`repo.tabs`).
    pub tabs: Option<TabsState<'a>>,
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
    /// A repository, as read: the rail of its references, its commits and the inspector.
    Repository {
        /// The branch rail.
        rail: Box<ReferencesState<'a>>,
        /// The commits.
        graph: GraphState<'a>,
        /// The inspector.
        inspector: Box<DetailsState<'a>>,
        /// The sizes of the panes, as dragged.
        panes: PaneSizes,
        /// The branch filter, beside the search box at the top of the timeline (`filter.picker`).
        filter: Box<BranchPicker<'a>>,
        /// Whether the branch rail is put away (`graph.toggle-references`).
        is_rail_hidden: bool,
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
    /// No repository to show yet, or none at all: the rail is empty and the timeline says why.
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
    intents.extend(ui.clip(status, |ui| status_bar_view(&state.status, ui)));
    let islands = inset(
        area.rest(),
        Insets {
            top: gap,
            right: gap,
            ..Insets::all(0.0)
        },
    );
    let mut column = Area::with_gap(islands, gap);
    if let Some(tabs) = &state.tabs {
        let rect = column.top(bar_height);
        intents.extend(island(ui, rect, |ui| tabs_view(tabs, ui)));
    }
    // What is under the tabs is the tab's: its widgets remember how far they are scrolled and what is typed in them
    // apart from those of the other tabs (`repo.keep-open`).
    let shown_tab = state
        .tabs
        .as_ref()
        .and_then(|tabs| tabs.tabs.iter().find(|tab| tab.is_shown))
        .map(|tab| tab.folder.as_str());
    let (body, panel) = ui.scope(("tab", shown_tab), |ui| {
        tab_islands(state, ui, (column, gap, bar_height), &mut intents)
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

/// The islands of the tab shown: the header, what was said, a notice, the timeline or the page, and the bottom panel;
/// gives the rectangle under the notices, where the Activity drawer opens, and that of the bottom panel when it is shown.
fn tab_islands(
    state: &WindowState<'_>,
    ui: &mut Ui<'_>,
    (mut column, gap, bar_height): (Area, f32, f32),
    intents: &mut Vec<Intent>,
) -> (Rect, Option<Rect>) {
    let top = column.top(bar_height);
    intents.extend(island(ui, top, |ui| toolbar_view(&state.header, ui)));
    let mut body = column.rest();
    if let Some(said) = &state.said {
        let mut rows = Area::with_gap(body, gap);
        let bar = rows.top(bar_height);
        body = rows.rest();
        intents.extend(island(ui, bar, |ui| said_view(said, ui)));
    }
    if let Some(notice) = state.notice {
        let mut rows = Area::with_gap(body, gap);
        let bar = rows.top(bar_height);
        body = rows.rest();
        island(ui, bar, |ui| notice_bar(notice, ui, bar, intents));
    }
    let panel = state.panel.as_ref();
    let shown = match &state.content {
        Content::Timeline(timeline) => timeline_with_rail(timeline, panel, ui, (body, gap), intents),
        Content::Page(page) => {
            let above = above_panel(panel, ui, (body, gap), intents);
            intents.extend(island(ui, above.0, |ui| host_view(page, ui)));
            above.1
        }
    };
    (body, shown)
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

/// The branch rail, the timeline and the inspector side by side, each an island with a gap between them that resizes
/// them, and the bottom panel under the timeline and the inspector when it is shown, the rail keeping its whole height;
/// in a narrow window the rail shows its icons and the inspector opens over the commits while something is selected.
/// Gives where the bottom panel is.
fn timeline_with_rail(
    state: &TimelineState<'_>,
    panel: Option<&BottomPanelState<'_>>,
    ui: &mut Ui<'_>,
    (body, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) -> Option<Rect> {
    match state {
        TimelineState::Repository { .. } => repository_panes(state, panel, ui, (body, gap), intents),
        TimelineState::NoRepository(no_repository) => {
            let icon_rail = is_narrow(ui).then_some(ICON_RAIL_WIDTH);
            let mut columns = Area::with_gap(body, gap);
            let rail = columns.left(icon_rail.unwrap_or(crate::graph::RAIL_START));
            island(ui, rail, |_| ());
            let (above, shown) = above_panel(panel, ui, (columns.rest(), gap), intents);
            intents.extend(island(ui, above, |ui| no_repository_view(no_repository, ui)));
            shown
        }
    }
}

/// The panes of a repository read: the branch rail, the commits, the inspector and the bottom panel under the last two.
fn repository_panes(
    state: &TimelineState<'_>,
    panel: Option<&BottomPanelState<'_>>,
    ui: &mut Ui<'_>,
    (body, gap): (Rect, f32),
    intents: &mut Vec<Intent>,
) -> Option<Rect> {
    let TimelineState::Repository {
        rail: references,
        graph,
        inspector: details,
        panes,
        is_rail_hidden,
        search,
        filter,
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
        icon_rail: is_narrow(ui).then_some(ICON_RAIL_WIDTH),
        is_rail_hidden: *is_rail_hidden,
        is_inspector_over,
        has_inspector: (!is_inspector_over && beside.is_none()) || details.selected.is_some(),
        gap,
        bottom,
    };
    let rects = pane_rects(&layout, gap);
    let mut rail_hover = None;
    if let Some(rail) = rects.rail {
        let (rail_intents, hover) = island(ui, rail, |ui| crate::branch::pane_view(references, ui));
        intents.extend(rail_intents);
        rail_hover = hover;
    }
    island(ui, rects.main, |ui| {
        if let Some(beside) = beside {
            intents.extend(crate::diff::beside_inspector_view(beside, ui));
        } else {
            let main = search_row((&search.0, filter), ui, rects.main, intents);
            let mut main = Area::new(main);
            banners(state, ui, &mut main, intents);
            let graph = with_rail_hover(graph, rail_hover);
            intents.extend(ui.clip(main.rest(), |ui| graph_view(&graph, ui)));
        }
    });
    if let Some(column) = rects.inspector {
        intents.extend(island(ui, column, |ui| show(details, ui)));
    }
    intents.extend(resize_panes_view(&layout, ui));
    let column = Rect::new(
        rects.main.x,
        body.y,
        (body.right() - rects.main.x).max(0.0),
        body.height,
    );
    above_panel(panel, ui, (column, gap), intents).1
}

/// The bars above the commits, each the height of one: the list mode of the search or what the timeline is narrowed
/// to, then the banner of an operation stopped, then that of a bisect.
fn banners(state: &TimelineState<'_>, ui: &mut Ui<'_>, main: &mut Area, intents: &mut Vec<Intent>) {
    let TimelineState::Repository {
        bar,
        search,
        conflicts,
        bisect,
        worktree,
        ..
    } = state
    else {
        return;
    };
    if let Some(list) = &search.1 {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| list_mode_view(list, ui)));
    } else if let Some(bar) = bar {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| bar_view(bar, ui)));
    }
    if let Some(banner) = worktree {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| crate::worktree::banner(banner, ui)));
    }
    if let Some(banner) = conflicts {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| crate::conflicts::banner_view(banner, ui)));
    }
    if let Some(banner) = bisect {
        let top = main.top(bar_height(ui));
        intents.extend(ui.clip(top, |ui| banner_view(banner, ui)));
    }
}

/// The timeline with the reference of the branch rail the pointer rests on, whose history it brings out.
fn with_rail_hover<'a>(graph: &GraphState<'a>, hover: Option<(HoverTarget, std::time::Duration)>) -> GraphState<'a> {
    match (graph, hover) {
        (GraphState::Listed(history), Some(hover)) => {
            let mut history = history.as_ref().clone();
            history.rail_hover = Some(hover);
            GraphState::Listed(Box::new(history))
        }
        _ => graph.clone(),
    }
}

/// The branch filter and the search box at the top of the timeline; gives what is left under them, for the bar of what
/// the timeline is narrowed to and the commits.
fn search_row(
    (search_box, filter): (&SearchBox<'_>, &BranchPicker<'_>),
    ui: &mut Ui<'_>,
    main: Rect,
    intents: &mut Vec<Intent>,
) -> Rect {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::new(main);
    let top = area.top(theme.sizes.control_height + 2.0 * spacing.medium);
    let row = inset(top, Insets::symmetric(spacing.medium, spacing.medium));
    let mut parts = Area::with_gap(row, spacing.medium);
    // The filter gives up its words where the search box would have less than its room beside them.
    let words = picker_button_width(filter, ui);
    let width = if row.width - words < SEARCH_BOX_ROOM {
        theme.sizes.control_height
    } else {
        words
    };
    let cell = control_in(ui, parts.left(width), width);
    intents.extend(ui.clip(cell, |ui| picker_view(filter, ui)));
    intents.extend(ui.clip(parts.rest(), |ui| crate::search::box_view(search_box, ui)));
    area.rest()
}

/// A notice across the top of what is under the header, with a button that puts it away.
fn notice_bar(notice: Notice, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let said = match notice {
        Notice::RecentGone => repo_words::RECENT_GONE,
        Notice::SingleRepository => repo_words::SINGLE_REPOSITORY,
        Notice::NoRepositoryToSelect => repo_words::NO_REPOSITORY_TO_SELECT,
        Notice::DiffLooksBinary => crate::text::diff::LOOKS_BINARY,
        Notice::DiffTooComplex => crate::text::diff::TOO_MANY_DIFFERENCES,
        Notice::MergeNotInConflict => crate::text::merge_view::NOT_IN_CONFLICT,
    };
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
        spacing.medium,
    );
    ui.icon_in(parts.left(theme.sizes.icon), Icon::Info, Tone::Accent);
    let dismiss = Button::secondary(repo_words::DISMISS).kind(ButtonKind::Outline);
    let (width, height) = ui.button_size(&dismiss);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    if ui.button("dismiss notice", cell, &dismiss).clicked {
        intents.push(Intent::DismissNotice);
    }
    let _ = ui.label(parts.rest(), &Label::new(said));
}

/// The keys of the window that no widget used: `Ctrl+Tab` goes through the timeline and the pages open, `Ctrl+,`
/// opens the settings page, and `Escape` closes the Activity drawer while it is open, else brings the timeline back
/// from the Diff page beside the inspector (`diff.beside-inspector`).
fn shortcuts(ui: &Ui<'_>, state: &WindowState<'_>) -> Vec<Intent> {
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
            (Key::Named(NamedKey::Escape), Modifiers::NONE) if is_drawer_open => Some(Intent::CloseActivity),
            (Key::Named(NamedKey::Escape), Modifiers::NONE) if is_beside => Some(Intent::ShowTimeline),
            _ => None,
        })
        .collect()
}
