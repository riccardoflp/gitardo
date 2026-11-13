//! The whole window: the header at the top; under it the branch rail, the timeline and the inspector, or a page with
//! its trail; the Activity drawer, the picker of the repositories, the question of an opened folder and a notice over
//! them. Below about 900 logical pixels the rail shows only icons and the header only its icons; below about 700 the
//! inspector is not beside the timeline (it opens over it once a commit can be selected).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Icon, Key, Label, Modifiers, NamedKey, Rect, Tone, Ui};

use crate::activity::{ActivityState, what_is_running_view};
use crate::bisect::{BisectBanner, banner_view};
use crate::details::{DetailsState, show};
use crate::feature::{ConfirmState, QuestionState, SaidState, confirm_view, question_view, said_view};
use crate::filter::{NarrowedBar, bar_height, bar_view};
use crate::graph::{
    GraphState, HoverTarget, PaneLayout, PaneSizes, ToolbarState, pane_rects, resize_panes_view, toolbar_view,
    view as graph_view,
};
use crate::intent::{Intent, Notice};
use crate::line::{ICON_RAIL_WIDTH, INSPECTOR_OVER_BELOW, is_narrow};
use crate::page::Page;
use crate::page::{PageHostState, host_view};
use crate::refs::{ReferencesState, rail_view};
use crate::repo::{
    NoRepositoryState, OpenFolderQuestion, RepositoryPickerState, no_repository_view, open_folder_question_view,
    repository_picker_view,
};
use crate::search::{GoToCommitDialog, ListMode, SearchBox, go_to_commit_dialog, list_mode_view};
use crate::settings::{ThemePickerState, theme_picker_view};
use crate::text::repo as repo_words;

/// Everything the window shows, made from the store of the window for each frame.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowState<'a> {
    /// The header.
    pub header: ToolbarState<'a>,
    /// What is under the header.
    pub content: Content<'a>,
    /// The Activity drawer, while it is open.
    pub drawer: Option<ActivityState<'a>>,
    /// The picker of the repositories, while it is open.
    pub picker: Option<RepositoryPickerState<'a>>,
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
        /// Whether the branch rail is put away (`graph.toggle-references`).
        is_rail_hidden: bool,
        /// The bar above the commits, while the timeline is narrowed to some branches (`filter.bar`).
        bar: Option<NarrowedBar<'a>>,
        /// The search box at the top of the timeline, and the bar of a search shown as a list or of the history of a
        /// commit alone, drawn in place of `bar`.
        search: Box<(SearchBox<'a>, Option<ListMode<'a>>)>,
        /// The banner of a rebase stopped to edit or split a commit, above the commits (`rebase.edit-stop`).
        stop: Option<crate::rebase::EditStopState<'a>>,
        /// The banner of the bisect under way, above the commits (`bisect.banner`).
        bisect: Option<BisectBanner<'a>>,
    },
    /// No repository to show yet, or none at all: the rail is empty and the timeline says why.
    NoRepository(NoRepositoryState),
}

/// The window, over all of it.
pub fn view(state: &WindowState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let window = ui.window();
    ui.fill_rect(window, theme.colors.page_background);
    let mut area = Area::new(window);
    let top = area.top(theme.sizes.header_height);
    intents.extend(ui.clip(top, |ui| toolbar_view(&state.header, ui)));
    let mut body = area.rest();
    if let Some(said) = &state.said {
        let bar = Area::new(body).top(theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium);
        body = Rect::new(body.x, bar.bottom(), body.width, (body.height - bar.height).max(0.0));
        intents.extend(ui.clip(bar, |ui| said_view(said, ui)));
    }
    if let Some(notice) = state.notice {
        let bar = Area::new(body).top(theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium);
        body = Rect::new(body.x, bar.bottom(), body.width, (body.height - bar.height).max(0.0));
        notice_bar(notice, ui, bar, &mut intents);
    }
    match &state.content {
        Content::Timeline(timeline) => timeline_with_rail(timeline, ui, body, &mut intents),
        Content::Page(page) => intents.extend(ui.clip(body, |ui| host_view(page, ui))),
    }
    if let Some(drawer) = &state.drawer {
        intents.extend(ui.clip(body, |ui| what_is_running_view(drawer, ui)));
    }
    if let Some(picker) = &state.picker {
        intents.extend(repository_picker_view(picker, ui));
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
    match &state.feature {
        Some(FeatureDialog::Question(question)) => intents.extend(question_view(question, ui)),
        Some(FeatureDialog::Plan(plan)) => intents.extend(confirm_view(plan, ui)),
        None => {}
    }
    intents.extend(shortcuts(ui, state.drawer.is_some()));
    intents
}

/// The branch rail, the timeline and the inspector, side by side; in a narrow window the rail shows its icons and the
/// inspector opens over the commits while something is selected.
fn timeline_with_rail(state: &TimelineState<'_>, ui: &mut Ui<'_>, body: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let icon_rail = is_narrow(ui).then_some(ICON_RAIL_WIDTH);
    match state {
        TimelineState::Repository {
            rail: references,
            graph,
            inspector: details,
            panes,
            is_rail_hidden,
            bar,
            search,
            stop,
            bisect,
        } => {
            let is_inspector_over = ui.window().width < INSPECTOR_OVER_BELOW;
            let layout = PaneLayout {
                body,
                sizes: *panes,
                icon_rail,
                is_rail_hidden: *is_rail_hidden,
                is_inspector_over,
                has_inspector: !is_inspector_over || details.selected.is_some(),
            };
            let rects = pane_rects(&layout, theme.sizes.border);
            let mut rail_hover = None;
            if let Some(rail) = rects.rail {
                let (rail_intents, hover) = ui.clip(rail, |ui| rail_view(references, ui));
                intents.extend(rail_intents);
                rail_hover = hover;
            }
            let main = search_row(&search.0, ui, rects.main, intents);
            let mut main = Area::new(main);
            if let Some(list) = &search.1 {
                let top = main.top(bar_height(ui));
                intents.extend(ui.clip(top, |ui| list_mode_view(list, ui)));
            } else if let Some(bar) = bar {
                let top = main.top(bar_height(ui));
                intents.extend(ui.clip(top, |ui| bar_view(bar, ui)));
            }
            if let Some(stop) = stop {
                let top = main.top(bar_height(ui));
                intents.extend(ui.clip(top, |ui| crate::rebase::edit_stop_view(stop, ui)));
            }
            if let Some(banner) = bisect {
                let top = main.top(bar_height(ui));
                intents.extend(ui.clip(top, |ui| banner_view(banner, ui)));
            }
            let graph = with_rail_hover(graph, rail_hover);
            intents.extend(ui.clip(main.rest(), |ui| graph_view(&graph, ui)));
            if let Some(column) = rects.inspector {
                if is_inspector_over {
                    ui.fill_rect(column, theme.colors.page_background);
                }
                intents.extend(ui.clip(column, |ui| show(details, ui)));
            }
            intents.extend(resize_panes_view(&layout, ui));
        }
        TimelineState::NoRepository(no_repository) => {
            let mut columns = Area::new(body);
            let rail = columns.left(icon_rail.unwrap_or(crate::graph::RAIL_START));
            ui.fill_rect(rail, theme.colors.rail_background);
            intents.extend(ui.clip(columns.rest(), |ui| no_repository_view(no_repository, ui)));
        }
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

/// The search box at the top of the timeline; gives what is left under it, for the bar of what the timeline is
/// narrowed to and the commits.
fn search_row(search_box: &SearchBox<'_>, ui: &mut Ui<'_>, main: Rect, intents: &mut Vec<Intent>) -> Rect {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::new(main);
    let top = area.top(theme.sizes.control_height + 2.0 * spacing.medium);
    let row = inset(top, Insets::symmetric(spacing.large + spacing.small, spacing.medium));
    intents.extend(ui.clip(row, |ui| crate::search::box_view(search_box, ui)));
    area.rest()
}

/// A notice across the top of what is under the header, with a button that puts it away.
fn notice_bar(notice: Notice, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let said = match notice {
        Notice::RecentGone => repo_words::RECENT_GONE,
        Notice::SingleRepository => repo_words::SINGLE_REPOSITORY,
        Notice::NoRepositoryToSelect => repo_words::NO_REPOSITORY_TO_SELECT,
        Notice::DiffLooksBinary => crate::text::diff::LOOKS_BINARY,
        Notice::DiffTooComplex => crate::text::diff::TOO_MANY_DIFFERENCES,
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
/// opens the settings page, and `Escape` closes the Activity drawer while it is open.
fn shortcuts(ui: &Ui<'_>, is_drawer_open: bool) -> Vec<Intent> {
    ui.unclaimed_keys()
        .into_iter()
        .filter_map(|key| match (key.key, key.modifiers) {
            (Key::Named(NamedKey::Tab), Modifiers::CONTROL) => Some(Intent::NextPage),
            (Key::Character(','), Modifiers::CONTROL) => Some(Intent::ShowPage(Page::Settings)),
            (Key::Named(NamedKey::Escape), Modifiers::NONE) if is_drawer_open => Some(Intent::CloseActivity),
            _ => None,
        })
        .collect()
}
