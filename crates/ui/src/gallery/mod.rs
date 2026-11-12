//! The gallery: every widget of the toolkit in one fixed layout, for the demo window of the application and for the
//! snapshots of the tests. It is written as a view would be: a function of its state that declares widgets and returns
//! what the person asked for, and a function that applies that to the state.

mod controls;
mod overlays;
mod rows;
mod scenes;

pub use overlays::MenuCommand;
pub use scenes::{Scene, prepare_scene};

use render::{Icon, Rect, Theme, ThemeName};

use crate::frame::Ui;
use crate::id::WidgetId;
use crate::layout::{Area, Insets, inset};
use crate::memory::UiMemory;
use crate::widget::Tone;
use crate::widget::badge::Badge;
use crate::widget::button::Button;
use crate::widget::label::Label;
use crate::widget::list::Selection;
use crate::widget::metrics;
use crate::widget::splitter::{Split, Splitter};

/// The key of the scope every widget of the gallery is declared in.
const SCOPE: &str = "gallery";

/// How many rows the list of the gallery has.
pub const ROW_COUNT: usize = 1_000_000;

/// The switches of the gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Switch {
    /// The check box that is on.
    FetchFirst,
    /// The check box that is off.
    Prune,
    /// The toggle switch.
    ShowRemotes,
}

/// The input boxes of the gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    /// The box with the focus.
    BranchName,
    /// The empty box with a placeholder.
    Search,
    /// The box with an error under it.
    Existing,
    /// The box of the dialog that creates a branch.
    NewBranch,
}

/// What opens over the gallery and waits for an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Overlay {
    /// The dialog that asks for the name of a new branch.
    CreateBranch,
    /// The confirmation of the deletion of a branch.
    ConfirmDelete,
    /// The picker of one branch.
    PickBranch,
    /// The picker of several branches.
    PickBranches,
}

/// What the person asked for in the gallery.
#[derive(Clone, Debug, PartialEq)]
pub enum GalleryIntent {
    /// A button was clicked, by its words or its tooltip.
    Clicked(String),
    /// A check box or a toggle switch was turned.
    Toggled(Switch),
    /// A radio button was chosen, by its place.
    Chose(usize),
    /// Text was typed in an input box.
    Typed(Field, String),
    /// A row of the list was opened.
    Opened(usize),
    /// The context menu was opened on a row of the list.
    MenuOn(usize),
    /// An overlay is to open.
    Open(Overlay),
    /// The overlay open was closed without an answer.
    Close,
    /// Something was done, as the status bar says, which closes the overlay open.
    Did(String),
    /// A theme was chosen.
    ChoseTheme(ThemeName),
}

/// What the gallery shows that a person can change.
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryState {
    /// Whether the first check box is on.
    pub fetch_first: bool,
    /// Whether the second check box is on.
    pub prune: bool,
    /// Whether the toggle switch is on.
    pub show_remotes: bool,
    /// The radio button chosen.
    pub strategy: usize,
    /// The text of the box with the focus.
    pub branch_name: String,
    /// The text of the empty box.
    pub search: String,
    /// The text of the box with an error.
    pub existing: String,
    /// How far the progress bar is, from 0 to 1.
    pub progress: f32,
    /// The last row opened.
    pub opened: Option<usize>,
    /// The row the context menu was last opened on.
    pub menu_row: Option<usize>,
    /// What is open over the gallery.
    pub overlay: Option<Overlay>,
    /// The text of the box of the dialog that creates a branch.
    pub new_branch: String,
    /// The theme chosen in the menu.
    pub theme: ThemeName,
    /// What was done last, which the status bar shows in place of what runs.
    pub status: Option<String>,
}

impl Default for GalleryState {
    fn default() -> GalleryState {
        GalleryState {
            fetch_first: true,
            prune: false,
            show_remotes: true,
            strategy: 1,
            branch_name: String::from(BRANCH_NAME),
            search: String::new(),
            existing: String::from("main"),
            progress: 0.62,
            opened: None,
            menu_row: None,
            overlay: None,
            new_branch: String::new(),
            theme: ThemeName::System,
            status: None,
        }
    }
}

/// The text of the box with the focus, of which a word is selected.
const BRANCH_NAME: &str = "feature/login-page";

impl GalleryState {
    /// Applies what the person asked for.
    pub fn apply(&mut self, intent: &GalleryIntent) {
        match intent {
            GalleryIntent::Clicked(_) => {}
            GalleryIntent::Toggled(Switch::FetchFirst) => self.fetch_first = !self.fetch_first,
            GalleryIntent::Toggled(Switch::Prune) => self.prune = !self.prune,
            GalleryIntent::Toggled(Switch::ShowRemotes) => self.show_remotes = !self.show_remotes,
            GalleryIntent::Chose(choice) => self.strategy = *choice,
            GalleryIntent::Typed(Field::BranchName, text) => self.branch_name.clone_from(text),
            GalleryIntent::Typed(Field::Search, text) => self.search.clone_from(text),
            GalleryIntent::Typed(Field::Existing, text) => self.existing.clone_from(text),
            GalleryIntent::Typed(Field::NewBranch, text) => self.new_branch.clone_from(text),
            GalleryIntent::Opened(row) => self.opened = Some(*row),
            GalleryIntent::MenuOn(row) => self.menu_row = Some(*row),
            GalleryIntent::Open(overlay) => {
                self.overlay = Some(*overlay);
                self.new_branch.clear();
            }
            GalleryIntent::Close => self.overlay = None,
            GalleryIntent::Did(text) => {
                self.overlay = None;
                self.status = Some(text.clone());
            }
            GalleryIntent::ChoseTheme(theme) => self.theme = *theme,
        }
    }
}

/// The name of a widget of the gallery, by its key, to find it in tests.
#[must_use]
pub fn widget_id(key: &str) -> WidgetId {
    WidgetId::ROOT.with(SCOPE).with(key)
}

/// Puts the memory in the fixed state the gallery is shown in: the focus in the first input box with a word selected,
/// the list scrolled to its middle with three rows selected.
pub fn prepare(memory: &mut UiMemory, theme: &Theme) {
    let name = widget_id("branch name");
    memory.set_input_text(name, BRANCH_NAME, (8, 13));
    memory.set_focus(Some(name));
    let middle = ROW_COUNT / 2;
    let first = crate::number::wide_of_count(middle - 4) * f64::from(theme.sizes.row_height);
    let mut selection = Selection::span(middle, middle + 2);
    selection.toggle(middle + 5);
    memory.set_list(widget_id("commits"), selection, first);
}

/// Declares the gallery over the whole window.
pub fn view(state: &GalleryState, ui: &mut Ui<'_>) -> Vec<GalleryIntent> {
    let mut intents = Vec::new();
    ui.scope(SCOPE, |ui| {
        let theme = *ui.theme();
        let mut area = Area::new(ui.window());
        let bar = area.top(theme.sizes.bar_height);
        toolbar(state, ui, bar, &mut intents);
        let status = area.bottom(theme.sizes.status_bar_height);
        status_bar(state, ui, status);
        let body = area.rest();
        ui.fill_rect(body, theme.colors.page_background);
        let split = Splitter {
            split: Split::Columns,
            default_size: theme.sizes.row_height * 20.0,
            least_first: theme.sizes.row_height * 12.0,
            least_second: theme.sizes.row_height * 12.0,
        };
        let panes = ui.splitter("columns", body, &split);
        ui.fill_rect(panes.first, theme.colors.side_bar_background);
        controls::controls(state, ui, panes.first, &mut intents);
        rows::rows(state, ui, panes.second, &mut intents);
        overlays::shortcuts(state, ui, &mut intents);
        let _ = ui.find_in_page(overlays::FIND, body);
        overlays::overlay(state, ui, &mut intents);
    });
    intents
}

/// The toolbar: a heading, buttons of each kind, and buttons that are only an icon, the first of which opens a menu.
fn toolbar(state: &GalleryState, ui: &mut Ui<'_>, bar: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    ui.fill_rect(bar, theme.colors.toolbar_background);
    let spacing = theme.sizes.spacing;
    let inner = inset(bar, Insets::symmetric(spacing.medium, spacing.tiny));
    let mut area = Area::with_gap(inner, spacing.small);
    let heading = area.left(theme.sizes.row_height * 6.0);
    let _ = ui.label(
        heading,
        &Label::new("Gitardo widgets").kind(crate::frame::TextKind::Strong),
    );
    let buttons = [
        Button::primary("Fetch")
            .with_icon(Icon::ArrowDown)
            .with_tooltip("Fetch from every remote"),
        Button::secondary("Push").with_icon(Icon::ArrowUp),
        Button::secondary("Undo").disabled(),
    ];
    for button in &buttons {
        let (width, height) = ui.button_size(button);
        let cell = area.left(width);
        let rect = crate::layout::place(
            cell,
            (width, height),
            (crate::layout::Align::Start, crate::layout::Align::Center),
        );
        if ui.button(button.label, rect, button).clicked {
            intents.push(GalleryIntent::Clicked(button.label.to_owned()));
        }
    }
    let icons = [
        Button::icon(Icon::More, "More actions"),
        Button::icon(Icon::Gear, "Open the settings"),
        Button::icon(Icon::Refresh, "Read the repository again"),
    ];
    let side = metrics::control_height(&theme);
    let themes = overlays::theme_menu();
    let entries = overlays::more_menu(&themes);
    for (index, button) in icons.iter().enumerate() {
        let cell = area.right(side);
        let rect = crate::layout::place(
            cell,
            (side, side),
            (crate::layout::Align::Start, crate::layout::Align::Center),
        );
        if index == 0 {
            if let Some(command) = ui.menu_button(MORE, rect, button, &entries) {
                overlays::run_command(ui, command, state.menu_row, intents);
            }
        } else if ui.button(button.tooltip, rect, button).clicked {
            intents.push(GalleryIntent::Clicked(button.tooltip.unwrap_or_default().to_owned()));
        }
    }
}

/// The key of the menu of the toolbar.
const MORE: &str = "more";

/// The status bar: a spinner with what runs, or what was done last, and the branch with how far it is from its
/// upstream.
fn status_bar(state: &GalleryState, ui: &mut Ui<'_>, bar: Rect) {
    let theme = *ui.theme();
    ui.fill_rect(bar, theme.colors.status_bar_background);
    let spacing = theme.sizes.spacing;
    let mut area = Area::with_gap(inset(bar, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
    let spinner = area.left(theme.sizes.icon);
    if state.status.is_none() {
        ui.spinner(spinner);
    }
    let ahead = Badge {
        text: "2 ahead",
        icon: Some(Icon::ArrowUp),
        tone: Tone::Normal,
    };
    let width = ui.badge_width(&ahead);
    let _ = ui.badge(area.right(width), &ahead);
    let branch = area.right(theme.sizes.row_height * 3.0);
    let _ = ui.label(branch, &Label::new("main").align(crate::layout::Align::End));
    let text = area.rest();
    let shown = state.status.as_deref().unwrap_or("Reading the repository (0.8 s)");
    let _ = ui.label(text, &Label::new(shown).tone(Tone::Normal));
}
