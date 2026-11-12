//! The whole window: the header at the top; under it the branch rail, the timeline and the inspector, or a page with
//! its trail; the Activity drawer, the picker of the repositories, the question of an opened folder and a notice over
//! them. Below about 900 logical pixels the rail shows only icons and the header only its icons; below about 700 the
//! inspector is not beside the timeline (it opens over it once a commit can be selected).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Icon, Key, Label, Modifiers, NamedKey, Rect, Tone, Ui};

use crate::activity::{ActivityState, what_is_running_view};
use crate::details::{DetailsState, show};
use crate::filter::{NarrowedBar, bar_height, bar_view};
use crate::graph::{
    GraphState, HoverTarget, PaneLayout, PaneSizes, ToolbarState, pane_rects, resize_panes_view, toolbar_view,
    view as graph_view,
};
use crate::intent::{Intent, Notice};
use crate::line::{ICON_RAIL_WIDTH, INSPECTOR_OVER_BELOW, is_narrow};
use crate::page::{PageHostState, host_view};
use crate::refs::{ReferencesState, rail_view};
use crate::repo::{
    NoRepositoryState, OpenFolderQuestion, RepositoryPickerState, no_repository_view, open_folder_question_view,
    repository_picker_view,
};
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
    /// A sentence said once, until it is put away.
    pub notice: Option<Notice>,
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
        inspector: DetailsState<'a>,
        /// The sizes of the panes, as dragged.
        panes: PaneSizes,
        /// Whether the branch rail is put away (`graph.toggle-references`).
        is_rail_hidden: bool,
        /// The bar above the commits, while the timeline is narrowed to some branches (`filter.bar`).
        bar: Option<NarrowedBar<'a>>,
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
    intents.extend(shortcuts(ui));
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
            let mut main = Area::new(rects.main);
            if let Some(bar) = bar {
                let top = main.top(bar_height(ui));
                intents.extend(ui.clip(top, |ui| bar_view(bar, ui)));
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

/// A notice across the top of what is under the header, with a button that puts it away.
fn notice_bar(notice: Notice, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let said = match notice {
        Notice::RecentGone => repo_words::RECENT_GONE,
        Notice::SingleRepository => repo_words::SINGLE_REPOSITORY,
        Notice::NoRepositoryToSelect => repo_words::NO_REPOSITORY_TO_SELECT,
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

/// The keys of the window that no widget used: `Ctrl+Tab` goes through the timeline and the pages open.
fn shortcuts(ui: &Ui<'_>) -> Vec<Intent> {
    ui.unclaimed_keys()
        .into_iter()
        .filter(|key| key.key == Key::Named(NamedKey::Tab) && key.modifiers == Modifiers::CONTROL)
        .map(|_| Intent::NextPage)
        .collect()
}
