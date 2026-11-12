//! The gallery with something open over it, each in a fixed state for the snapshots of the tests: a context menu with
//! a submenu open, a dialog, a confirmation, a picker with something typed, the find bar with its matches, and a badge
//! being dragged over the list.

use std::hash::Hash;

use render::{Point, Theme};

use super::overlays::{FIND, PICKER, RESET_ENTRY, ROW_MENU};
use super::{GalleryState, Overlay, ROW_COUNT, SCOPE, prepare};
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
    /// The context menu of a row, with the submenu of the ways to reset open.
    Menu,
    /// The dialog that asks for the name of a new branch, with a name typed.
    Dialog,
    /// The confirmation of the deletion of a branch, with the commands it runs.
    Confirmation,
    /// The picker of several branches, with "fe" typed and two branches checked.
    Picker,
    /// The find bar, with "login" looked for and the match in sight in the list current.
    Find,
    /// The badge of main dragged over the list.
    Drag,
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
    let row = theme.sizes.row_height;
    match scene {
        Scene::Menu => {
            state.menu_row = Some(ROW_COUNT / 2);
            memory.menu = Some(OpenMenu {
                id: scoped(ROW_MENU),
                placement: Placement::At(Point::new(row * 20.0, row * 8.0)),
                levels: vec![Some(RESET_ENTRY), Some(2)],
                rects: Vec::new(),
            });
        }
        Scene::Dialog => {
            state.overlay = Some(Overlay::CreateBranch);
            state.new_branch = String::from("feature/file-history");
            let name = scoped("create branch").with("name");
            memory.set_input_text(name, &state.new_branch, (8, 20));
            open_overlay(memory, scoped("create branch"), Vec::new(), name);
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
            };
            let _ = memory.pickers.insert(id, picker);
            open_overlay(memory, id, vec![input, id], input);
        }
        Scene::Find => open_find(memory),
        Scene::Drag => drag(memory, theme),
    }
    state
}

/// The number of the match of "login" in the row of the list in sight with that word: the rows with it are one in
/// twelve, from the second.
const FIND_CURRENT: usize = (ROW_COUNT / 2 + 5) / 12;

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

/// The badge of main dragged over the list.
fn drag(memory: &mut UiMemory, theme: &Theme) {
    let row = theme.sizes.row_height;
    let source = scoped(("badge", "main"));
    memory.drag_and_drop = Some(DragAndDrop {
        source,
        label: String::from("main"),
        start: Point::new(row * 22.0, row * 26.0),
        is_dragging: true,
        dropped_at: None,
    });
    memory.interaction.active = Some(Active { id: source });
    memory.interaction.pointer = Some(Point::new(row * 30.0, row * 10.0));
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
