//! The bar of views (`graph.view-bar`, D21): a column of icons on the left edge of the window, each the way to a
//! place of it, the place shown marked.

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, place};
use ui::{AccessRole, Badge, BadgeFill, Button, ButtonKind, Icon, Rect, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::page::Page;
use crate::text::views as words;

/// The width of the bar.
pub(crate) const VIEW_BAR_WIDTH: f32 = 48.0;

/// What the bar of views shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewBarState {
    /// The place shown, which is marked.
    pub shown: ViewShown,
    /// How many files are changed, once a repository is read; none before, when only **Activity** and **Settings**
    /// can be clicked.
    pub changed_files: Option<usize>,
    /// Whether the Activity drawer is open, which marks **Activity**.
    pub is_activity_open: bool,
    /// Whether the bottom panel is shown, which marks **Terminal**.
    pub is_panel_shown: bool,
}

/// The place of the window shown, as the bar of views marks it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewShown {
    /// The timeline.
    #[default]
    Timeline,
    /// The Changes page.
    Changes,
    /// The Files page.
    Files,
    /// The settings page.
    Settings,
    /// Another page, which none of the buttons stands for.
    Other,
}

/// A button of the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Timeline,
    Changes,
    Files,
    Compare,
    Terminal,
    Activity,
    Settings,
}

impl View {
    /// Its icon and its name.
    fn look(self) -> (Icon, &'static str) {
        match self {
            View::Timeline => (Icon::Commit, words::TIMELINE),
            View::Changes => (Icon::Pencil, words::CHANGES),
            View::Files => (Icon::Folder, words::FILES),
            View::Compare => (Icon::Compare, words::COMPARE),
            View::Terminal => (Icon::ChevronRight, crate::text::panel::TERMINAL),
            View::Activity => (Icon::Activity, words::ACTIVITY),
            View::Settings => (Icon::Gear, words::SETTINGS),
        }
    }

    /// Whether it is marked as the place shown.
    fn is_marked(self, state: &ViewBarState) -> bool {
        match self {
            View::Timeline => state.shown == ViewShown::Timeline,
            View::Changes => state.shown == ViewShown::Changes,
            View::Files => state.shown == ViewShown::Files,
            View::Compare => false,
            View::Terminal => state.is_panel_shown,
            View::Activity => state.is_activity_open,
            View::Settings => state.shown == ViewShown::Settings,
        }
    }

    /// Whether it needs a repository read.
    fn needs_repository(self) -> bool {
        matches!(
            self,
            View::Timeline | View::Changes | View::Files | View::Compare | View::Terminal
        )
    }

    /// What a click asks for.
    fn intent(self, state: &ViewBarState) -> Intent {
        match self {
            View::Timeline => Intent::ShowTimeline,
            View::Changes => Intent::ShowPage(Page::Changes),
            View::Files => Intent::ShowPage(Page::Files),
            View::Compare => Intent::Feature(FeatureIntent::Run(FeatureId("compare.choose"), Target::Head)),
            View::Terminal => Intent::Panel(crate::panel::PanelIntent::Toggle),
            View::Activity if state.is_activity_open => Intent::CloseActivity,
            View::Activity => Intent::OpenActivity,
            View::Settings => Intent::ShowPage(Page::Settings),
        }
    }
}

/// The bar of views over the rectangle its caller clipped it to: **Timeline**, **Changes** with the number of changed
/// files, **Files** and **Compare** from the top; **Terminal**, **Activity** and **Settings** at its foot. The place
/// shown has the ground of an island behind its icon.
///
/// Feature: graph.view-bar
pub fn view_bar_view(state: &ViewBarState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let side = theme.sizes.control_height + theme.sizes.spacing.small;
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    ui.region("view bar", (rect, AccessRole::Navigation, words::BAR), |ui| {
        let mut column = Area::with_gap(rect, spacing.small);
        for view in [View::Timeline, View::Changes, View::Files, View::Compare] {
            let cell = place(column.top(side), (side, side), (Align::Center, Align::Center));
            intents.extend(button(view, state, ui, cell));
        }
        for view in [View::Settings, View::Activity, View::Terminal] {
            let cell = place(column.bottom(side), (side, side), (Align::Center, Align::Center));
            intents.extend(button(view, state, ui, cell));
        }
    });
    intents
}

/// One button of the bar, with the ground of an island behind it when it is marked, and the number of changed files on
/// **Changes**.
fn button(view: View, state: &ViewBarState, ui: &mut Ui<'_>, cell: Rect) -> Option<Intent> {
    let theme = *ui.theme();
    let (icon, name) = view.look();
    let has_repository = state.changed_files.is_some();
    if view.is_marked(state) {
        ui.fill_rounded_rect(cell, theme.sizes.radius.medium + 2.0, theme.colors.page_background);
        ui.border(
            cell,
            ui::Border {
                width: theme.sizes.border,
                radius: theme.sizes.radius.medium + 2.0,
                color: theme.colors.border,
            },
        );
    }
    let mut button = Button::icon(icon, name).kind(ButtonKind::Icon);
    if view.needs_repository() && !has_repository {
        button = button.disabled();
    }
    let clicked = ui.button(("view bar", name), cell, &button).clicked;
    if let (View::Changes, Some(count)) = (view, state.changed_files)
        && count > 0
    {
        let text = words::changed_count(count);
        let badge = Badge {
            text: &text,
            icons: &[],
            tone: Tone::Accent,
            fill: BadgeFill::Solid,
        };
        let width = ui.badge_width(&badge);
        let height = ui.badge_height();
        let spot = Rect::new(cell.right() - width + 4.0, cell.y - 2.0, width, height);
        let _ = ui.badge(spot, &badge);
    }
    clicked.then(|| view.intent(state))
}
