//! The gallery with something open over it, each in a fixed state for the snapshots of the tests: a context menu with
//! a submenu open, a dialog, the dialog of Pull with its choices, a confirmation, a picker with something typed, the
//! find bar with its matches, a branch being dragged over the timeline, and the Activity drawer.

use std::hash::Hash;

use render::{Point, Theme};

use super::menus::{FIND, RESET_ENTRY, ROW_MENU};
use super::overlays::PICKER;
use super::{GalleryState, Overlay, SCOPE, prepare};
use crate::frame::OverlayState;
use crate::id::WidgetId;
use crate::interaction::Active;
use crate::memory::UiMemory;
use crate::widget::drag::DragAndDrop;
use crate::widget::find::{FindState, MatchCase};
use crate::widget::menu::{OpenMenu, Placement};
use crate::widget::picker::PickerState;

/// What is open over the gallery in a scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// The context menu of a commit, with the submenu of the ways to reset open.
    Menu,
    /// The dialog that asks for the name of a new branch, with a name typed.
    Dialog,
    /// The dialog of Pull, with its radio buttons, check boxes and toggle switch.
    Pull,
    /// The confirmation of the deletion of a branch, with the commands it runs.
    Confirmation,
    /// The picker of several branches, with "fe" typed and two branches checked.
    Picker,
    /// The find bar, with "login" looked for and a match in the timeline current.
    Find,
    /// The pill of main dragged over the timeline.
    Drag,
    /// The Activity drawer, with what runs, the command log and the tasks.
    Activity,
}

/// The name of a widget of the gallery by any key.
fn scoped(key: impl Hash) -> WidgetId {
    WidgetId::ROOT.with(SCOPE).with(key)
}

/// Puts the memory in the fixed state of a scene, over that of [`prepare`], and returns the state of the gallery the
/// scene is drawn with.
pub fn prepare_scene(memory: &mut UiMemory, theme: &Theme, scene: Scene) -> GalleryState {
    prepare(memory, theme);
    let mut state = GalleryState::default();
    match scene {
        Scene::Menu => {
            state.menu_row = Some(MENU_ROW);
            memory.menu = Some(OpenMenu {
                id: scoped(ROW_MENU),
                placement: Placement::At(MENU_AT),
                levels: vec![Some(RESET_ENTRY), Some(2)],
                rects: Vec::new(),
                scrolls: Vec::new(),
                shown: Vec::new(),
                reveal: false,
            });
        }
        Scene::Dialog => {
            state.overlay = Some(Overlay::CreateBranch);
            state.new_branch = String::from("feature/file-history");
            let name = scoped("create branch").with("name");
            memory.set_input_text(name, &state.new_branch, (8, 20));
            open_overlay(memory, scoped("create branch"), Vec::new(), name);
        }
        Scene::Pull => {
            state.overlay = Some(Overlay::Pull);
            let id = scoped("pull");
            open_overlay(memory, id, Vec::new(), id.with(("strategy", 1_usize)));
        }
        Scene::Confirmation => state.overlay = Some(Overlay::ConfirmDelete),
        Scene::Picker => {
            state.overlay = Some(Overlay::PickBranches);
            let id = scoped(PICKER);
            let input = id.with("query");
            memory.set_input_text(input, "fe", (2, 2));
            let picker = PickerState {
                lit: 1,
                first_shown: 0,
                checked: [1, 2].into_iter().collect(),
                query: String::from("fe"),
                has_last: false,
            };
            let _ = memory.pickers.insert(id, picker);
            open_overlay(memory, id, vec![input, id], input);
        }
        Scene::Find => open_find(memory),
        Scene::Drag => drag(memory),
        Scene::Activity => state.is_activity_open = true,
    }
    state
}

/// The commit the context menu is open on.
const MENU_ROW: usize = 4;
/// Where the context menu opened, over the timeline.
const MENU_AT: Point = Point::new(430.0, 300.0);
/// Where the pill of main was pressed, on its commit, and where it is dragged to.
const DRAG: (Point, Point) = (Point::new(600.0, 268.0), Point::new(520.0, 470.0));
/// The number of the match of "login" made current: that in the subject of the first commit.
const FIND_CURRENT: usize = 0;

/// The find bar open with "login" looked for.
fn open_find(memory: &mut UiMemory) {
    let id = scoped(FIND);
    let input = id.with("query");
    memory.find = FindState {
        is_open: true,
        query: String::from("login"),
        case: MatchCase::Ignore,
        current: FIND_CURRENT,
        must_reveal: false,
    };
    memory.set_input_text(input, "login", (5, 5));
    open_overlay(memory, id, vec![input], input);
}

/// The pill of main dragged over the timeline.
fn drag(memory: &mut UiMemory) {
    let source = scoped(("reference", "main"));
    let (start, now) = DRAG;
    memory.drag_and_drop = Some(DragAndDrop {
        source,
        label: String::from("main"),
        payload: String::new(),
        start,
        is_dragging: true,
        dropped_at: None,
        is_refused: false,
    });
    memory.interaction.active = Some(Active { id: source });
    memory.interaction.pointer = Some(now);
}

/// Notes an overlay as open since an earlier frame, with the focus in one of its widgets.
fn open_overlay(memory: &mut UiMemory, id: WidgetId, owned: Vec<WidgetId>, focus: WidgetId) {
    let state = OverlayState {
        focus_before: memory.focus(),
        owned,
    };
    let _ = memory.overlays.insert(id, state);
    memory.set_focus(Some(focus));
}
