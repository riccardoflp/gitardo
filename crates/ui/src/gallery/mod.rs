//! The gallery: a window shaped as the Timeline of Gitardo Desktop, over a made-up repository, that shows every widget
//! of the toolkit where the application will use it, for the demo window and for the snapshots of the tests. It is
//! written as a view would be: a function of its state that declares widgets and returns what the person asked for,
//! and a function that applies that to the state.
//!
//! The header has the repository, the branch checked out, Quick search, what runs, Sync and the Activity button; the
//! branch rail is on the left, the timeline of a million commits with the draft of the next one in the middle, and the
//! inspector of the commit picked on the right. Below about 900 logical pixels the rail shows only icons, and below
//! about 700 the inspector opens over the timeline when a commit is picked.

mod activity;
mod data;
mod graph;
mod header;
mod inspector;
mod menus;
mod overlays;
mod rail;
mod scenes;
mod timeline;

pub use menus::MenuCommand;
pub use scenes::{Scene, prepare_scene};

use render::{Rect, Theme, ThemeName};

use crate::access::AccessRole;
use crate::frame::Ui;
use crate::id::WidgetId;
use crate::layout::Area;
use crate::memory::UiMemory;
use crate::widget::list::Selection;

/// The key of the scope every widget of the gallery is declared in.
const SCOPE: &str = "gallery";

/// How many commits the timeline has.
pub const ROW_COUNT: usize = 1_000_000;

/// The width of the window, in logical pixels, below which the branch rail shows only icons.
const ICON_RAIL_BELOW: f32 = 900.0;
/// The width of the window, in logical pixels, below which the inspector opens over the timeline.
const INSPECTOR_OVER_BELOW: f32 = 700.0;
/// The width of the branch rail when it shows only icons.
const ICON_RAIL_WIDTH: f32 = 56.0;

/// The message of the draft, as the gallery opens.
const MESSAGE: &str = "Remember the e-mail of the last login";

/// The switches of the dialog of Pull.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Switch {
    /// Fetch before pulling, a check box that is on.
    FetchFirst,
    /// Prune the remote branches, a check box that is off.
    Prune,
    /// Keep the changes aside while pulling, a toggle switch.
    Autostash,
}

/// The input boxes of the gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    /// The message of the draft.
    Message,
    /// Quick search, in the header.
    Search,
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
    /// Quick search: commits, branches, files and commands.
    QuickSearch,
    /// The dialog of Pull, with its choices.
    Pull,
    /// The dialog that says a commit needs a message.
    EmptyMessage,
}

/// How the branch rail is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RailMode {
    /// With the names of the branches.
    #[default]
    Full,
    /// With icons only, as in a narrow window.
    Icons,
    /// Not at all.
    Hidden,
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
    /// A commit of the timeline was opened, by a double click or Enter.
    Opened(usize),
    /// A commit of the timeline was picked, which opens the inspector over a narrow window.
    Picked(usize),
    /// The context menu was opened on a commit of the timeline.
    MenuOn(usize),
    /// An overlay is to open.
    Open(Overlay),
    /// The overlay open was closed without an answer.
    Close,
    /// Something was done, as the header says, which closes the overlay open.
    Did(String),
    /// A theme was chosen.
    ChoseTheme(ThemeName),
    /// The branch rail is to be shown so.
    ShowRail(RailMode),
    /// A section of what is kept aside was unfolded or folded, by its place.
    Unfold(usize),
    /// The Activity drawer is to open.
    OpenActivity,
    /// The Activity drawer is to close.
    CloseActivity,
    /// The inspector open over the timeline is to close.
    CloseInspector,
}

/// The choices of the dialog of Pull.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PullChoices {
    /// The way chosen: merge, rebase or fast-forward.
    pub strategy: usize,
    /// Whether Pull fetches first.
    pub fetch_first: bool,
    /// Whether Pull prunes the remote branches.
    pub prune: bool,
    /// Whether Pull keeps the changes aside while it runs.
    pub autostash: bool,
}

impl Default for PullChoices {
    fn default() -> PullChoices {
        PullChoices {
            strategy: 1,
            fetch_first: true,
            prune: false,
            autostash: true,
        }
    }
}

/// What the gallery shows that a person can change.
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryState {
    /// The message of the draft.
    pub message: String,
    /// What is typed in Quick search.
    pub search: String,
    /// The text of the box of the dialog that creates a branch.
    pub new_branch: String,
    /// The choices of the dialog of Pull.
    pub pull: PullChoices,
    /// How the branch rail is shown.
    pub rail: RailMode,
    /// Which sections of what is kept aside are unfolded: stashes, worktrees and tags.
    pub unfolded: [bool; 3],
    /// Whether the Activity drawer is open.
    pub is_activity_open: bool,
    /// Whether the inspector is open over the timeline, in a narrow window.
    pub is_inspector_open: bool,
    /// The last commit opened.
    pub opened: Option<usize>,
    /// The commit the context menu was last opened on.
    pub menu_row: Option<usize>,
    /// What is open over the gallery.
    pub overlay: Option<Overlay>,
    /// The theme chosen in the menu.
    pub theme: ThemeName,
    /// What was done last, which the header shows in place of what runs.
    pub status: Option<String>,
}

impl Default for GalleryState {
    fn default() -> GalleryState {
        GalleryState {
            message: String::from(MESSAGE),
            search: String::new(),
            new_branch: String::new(),
            pull: PullChoices::default(),
            rail: RailMode::Full,
            unfolded: [false; 3],
            is_activity_open: false,
            is_inspector_open: false,
            opened: None,
            menu_row: None,
            overlay: None,
            theme: ThemeName::System,
            status: None,
        }
    }
}

impl GalleryState {
    /// Applies what the person asked for.
    pub fn apply(&mut self, intent: &GalleryIntent) {
        match intent {
            GalleryIntent::Clicked(_) => {}
            GalleryIntent::Toggled(Switch::FetchFirst) => self.pull.fetch_first = !self.pull.fetch_first,
            GalleryIntent::Toggled(Switch::Prune) => self.pull.prune = !self.pull.prune,
            GalleryIntent::Toggled(Switch::Autostash) => self.pull.autostash = !self.pull.autostash,
            GalleryIntent::Chose(choice) => self.pull.strategy = *choice,
            GalleryIntent::Typed(Field::Message, text) => self.message.clone_from(text),
            GalleryIntent::Typed(Field::Search, text) => self.search.clone_from(text),
            GalleryIntent::Typed(Field::NewBranch, text) => self.new_branch.clone_from(text),
            GalleryIntent::Opened(row) => self.opened = Some(*row),
            GalleryIntent::Picked(_) => self.is_inspector_open = true,
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
            GalleryIntent::ShowRail(mode) => self.rail = *mode,
            GalleryIntent::Unfold(section) => {
                if let Some(unfolded) = self.unfolded.get_mut(*section) {
                    *unfolded = !*unfolded;
                }
            }
            GalleryIntent::OpenActivity => self.is_activity_open = true,
            GalleryIntent::CloseActivity => self.is_activity_open = false,
            GalleryIntent::CloseInspector => self.is_inspector_open = false,
        }
    }
}

/// The name of a widget of the gallery, by its key, to find it in tests: such as `"draft message"`, `"history"` or
/// `"your branches"`.
#[must_use]
pub fn widget_id(key: &str) -> WidgetId {
    WidgetId::ROOT.with(SCOPE).with(key)
}

/// Puts the memory in the fixed state the gallery is shown in: the focus at the end of the message of the draft, the
/// first commit of the timeline picked, and the branch checked out picked in the rail. The state is the same in every
/// theme, which the gallery is given for the window it is shown in.
pub fn prepare(memory: &mut UiMemory, _theme: &Theme) {
    let message = widget_id("draft message");
    let end = MESSAGE.len();
    memory.set_input_text(message, MESSAGE, (end, end));
    memory.set_focus(Some(message));
    memory.set_list(widget_id("history"), Selection::single(0), 0.0);
    memory.set_list(widget_id("your branches"), Selection::single(0), 0.0);
}

/// The commit picked in the timeline, if any.
fn picked_commit(ui: &Ui<'_>) -> Option<usize> {
    ui.memory_view()
        .list_selection(widget_id("history"))
        .and_then(Selection::cursor)
}

/// Declares the gallery over the whole window.
pub fn view(state: &GalleryState, ui: &mut Ui<'_>) -> Vec<GalleryIntent> {
    let mut intents = Vec::new();
    ui.scope(SCOPE, |ui| {
        let theme = *ui.theme();
        let window = ui.window();
        ui.fill_rect(window, theme.colors.page_background);
        let mut area = Area::new(window);
        let top = area.top(theme.sizes.header_height);
        header::header(state, ui, top, &mut intents);
        let body = area.rest();
        let mut columns = Area::new(body);
        let mode = rail_mode(state.rail, window.width);
        let rail_width = match mode {
            RailMode::Full => theme.sizes.rail_width,
            RailMode::Icons => ICON_RAIL_WIDTH,
            RailMode::Hidden => 0.0,
        };
        if rail_width > 0.0 {
            rail::rail(state, ui, (columns.left(rail_width), mode), &mut intents);
        }
        let is_narrow = window.width < INSPECTOR_OVER_BELOW;
        let beside = (!is_narrow).then(|| columns.right(theme.sizes.inspector_width));
        let main = columns.rest();
        ui.region("timeline region", (main, AccessRole::Main, "Timeline"), |ui| {
            timeline::timeline(state, ui, main, &mut intents);
        });
        match beside {
            Some(rect) => inspector::inspector(ui, rect, &mut intents),
            None if state.is_inspector_open => inspector::inspector_over(ui, main, &mut intents),
            None => {}
        }
        if state.is_activity_open {
            let beside_rail = Rect::new(main.x, main.y, body.right() - main.x, main.height);
            activity::drawer(state, ui, beside_rail, &mut intents);
        }
        menus::shortcuts(state, ui, &mut intents);
        let _ = ui.find_in_page(menus::FIND, body);
        overlays::overlay(state, ui, &mut intents);
        menus::close_on_escape(state, ui, &mut intents);
    });
    intents
}

/// How the branch rail is shown in a window of a width: with icons only when the window is narrow.
fn rail_mode(chosen: RailMode, width: f32) -> RailMode {
    match chosen {
        RailMode::Full if width < ICON_RAIL_BELOW => RailMode::Icons,
        RailMode::Full | RailMode::Icons | RailMode::Hidden => chosen,
    }
}
