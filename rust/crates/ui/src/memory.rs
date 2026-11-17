//! What the toolkit keeps from one frame to the next: the pointer, the press held, the focus, the text of input boxes,
//! the scroll offsets, the selections of lists, the sizes of splitters, and the tiles of the last frame.
//!
//! The application keeps one [`UiMemory`] per window and hands it to every frame. It is the only state of the toolkit:
//! views stay functions of the state of the application.

use std::collections::HashMap;
use std::time::Duration;

use render::{Point, ThemeName};

use crate::access::AccessMemory;
use crate::frame::OverlayState;
use crate::id::WidgetId;
use crate::interaction::Interaction;
use crate::paint::Tiles;
use crate::widget::drag::DragAndDrop;
use crate::widget::find::{FindState, RowMatches};
use crate::widget::input_box::EditableLine;
use crate::widget::list::Selection;
use crate::widget::menu::OpenMenu;
use crate::widget::picker::PickerState;
use crate::widget::scroll::ScrollAxis;

/// The state of an input box between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct InputBoxState {
    /// The text, the caret and the selection, and the steps to undo.
    pub(crate) line: EditableLine,
    /// The text the view gave last, to see when the view changes it.
    pub(crate) given: Option<String>,
    /// How far the text is scrolled to the left, in logical pixels, to keep the caret in sight.
    pub(crate) scroll: f32,
    /// When the text or the caret last changed, so that the caret shows at once and blinks after.
    pub(crate) changed_at: Duration,
    /// Whether the box hides what is typed, as its view said in the last frame.
    pub(crate) is_hidden: bool,
    /// Whether the box had the keyboard in the last frame, to say when it loses it.
    pub(crate) had_focus: bool,
}

/// The state of a scroll area between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ScrollState {
    /// Across.
    pub(crate) x: ScrollAxis,
    /// Down.
    pub(crate) y: ScrollAxis,
    /// The number of the last place its view asked it to go to, and the time it went there, so that it goes to each
    /// once.
    pub(crate) revealed: Option<(u64, std::time::Duration)>,
}

/// The state of a list between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ListState {
    /// The rows selected, and the row of the keyboard.
    pub(crate) selection: Selection,
    /// How far the rows are scrolled.
    pub(crate) scroll: ScrollAxis,
    /// The number of the last row its view asked to bring into sight, so that each is brought once.
    pub(crate) revealed: Option<u64>,
    /// The row the main button is held on, while it is, for a drag that starts from it.
    pub(crate) press: Option<crate::widget::list::RowPress>,
}

/// The state of an editor of code between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CodeEditorState {
    /// How far the lines are scrolled.
    pub(crate) scroll: ScrollAxis,
    /// The number of the last line its view asked to bring to the middle, so that each is brought once.
    pub(crate) revealed: Option<u64>,
    /// The number of the last time its view asked it to take the keyboard, so that it takes it once each time.
    pub(crate) focused: Option<u64>,
    /// Where the caret was in the last frame, to keep it in sight when it moves.
    pub(crate) head: Option<usize>,
    /// When the caret last moved, which its blink starts from.
    pub(crate) changed_at: Duration,
}

/// A drag in progress: where it started and the value it moves, such as the size of a splitter or the offset of a
/// scroll bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragStart {
    /// The widget dragged.
    pub(crate) id: WidgetId,
    /// Where the pointer was pressed.
    pub(crate) position: Point,
    /// The value when it was pressed.
    pub(crate) value: f64,
}

/// What a frame was drawn at: when one of these changes, all of the window is drawn again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Surface {
    /// The width of the frame, in physical pixels.
    pub(crate) width: u32,
    /// The height of the frame, in physical pixels.
    pub(crate) height: u32,
    /// The scale of the screen.
    pub(crate) scale: f32,
    /// The theme.
    pub(crate) theme: ThemeName,
}

/// What the toolkit keeps from one frame to the next, for one window.
#[derive(Clone, Debug, Default)]
pub struct UiMemory {
    /// The pointer, the press held and the focus.
    pub(crate) interaction: Interaction,
    /// The input boxes.
    pub(crate) input_boxes: HashMap<WidgetId, InputBoxState>,
    /// The scroll areas.
    pub(crate) scrolls: HashMap<WidgetId, ScrollState>,
    /// The lists.
    pub(crate) lists: HashMap<WidgetId, ListState>,
    /// The editors of code.
    pub(crate) code_editors: HashMap<WidgetId, CodeEditorState>,
    /// The number of the last time each input box was asked to take the keyboard once.
    pub(crate) focus_serials: HashMap<WidgetId, u64>,
    /// The size of the first pane of each splitter that was moved.
    pub(crate) splitters: HashMap<WidgetId, f32>,
    /// The drag in progress of a splitter or a scroll bar.
    pub(crate) drag: Option<DragStart>,
    /// The overlays open: menus, dialogs, pickers and the find bar.
    pub(crate) overlays: HashMap<WidgetId, OverlayState>,
    /// The menu open.
    pub(crate) menu: Option<OpenMenu>,
    /// The pickers open.
    pub(crate) pickers: HashMap<WidgetId, PickerState>,
    /// The lists of pickers whose rows their views draw.
    pub(crate) pick_lists: HashMap<WidgetId, crate::widget::picker::list::PickListState>,
    /// Find in page.
    pub(crate) find: FindState,
    /// The matches of the lists that search their rows, by list.
    pub(crate) list_searches: HashMap<WidgetId, RowMatches>,
    /// The drag and drop in progress.
    pub(crate) drag_and_drop: Option<DragAndDrop>,
    /// The hashes of the tiles of the last frame.
    pub(crate) tiles: Option<Tiles>,
    /// What the last frame was drawn at.
    pub(crate) surface: Option<Surface>,
    /// The time of the last frame.
    pub(crate) last_frame: Option<Duration>,
    /// What the screen reader was told of the last frame, and what it asked for since.
    pub(crate) access: AccessMemory,
}

impl UiMemory {
    /// Nothing remembered: no focus, nothing scrolled, every widget as its view first declares it.
    #[must_use]
    pub fn new() -> UiMemory {
        UiMemory::default()
    }

    /// The widget with the keyboard focus.
    #[must_use]
    pub fn focus(&self) -> Option<WidgetId> {
        self.interaction.focus
    }

    /// Gives the keyboard focus to a widget, or takes it from all of them.
    pub fn set_focus(&mut self, id: Option<WidgetId>) {
        self.interaction.focus = id;
    }

    /// Sets the text of an input box, with the caret at `caret` and the selection from `anchor` to it, in bytes of
    /// the text; positions inside a character or past the end move to the nearest end of a character.
    pub fn set_input_text(&mut self, id: WidgetId, text: &str, (anchor, caret): (usize, usize)) {
        let state = self.input_boxes.entry(id).or_default();
        state.line = EditableLine::new(text);
        state.line.select(anchor, caret);
        state.given = Some(text.to_owned());
    }

    /// The text of an input box, when it was declared or set.
    #[must_use]
    pub fn input_text(&self, id: WidgetId) -> Option<&str> {
        self.input_boxes.get(&id).map(|state| state.line.text())
    }

    /// Sets the rows selected in a list and how far it is scrolled, in logical pixels from its top.
    pub fn set_list(&mut self, id: WidgetId, selection: Selection, offset: f64) {
        let state = self.lists.entry(id).or_default();
        state.selection = selection;
        state.scroll.jump(offset);
    }

    /// The rows selected in a list.
    #[must_use]
    pub fn list_selection(&self, id: WidgetId) -> Option<&Selection> {
        self.lists.get(&id).map(|state| &state.selection)
    }

    /// How far a list is scrolled, or is gliding to, in logical pixels from its top.
    #[must_use]
    pub fn list_offset(&self, id: WidgetId) -> Option<f64> {
        self.lists.get(&id).map(|state| state.scroll.target())
    }

    /// Sets the size of the first pane of a splitter, in logical pixels.
    pub fn set_splitter(&mut self, id: WidgetId, size: f32) {
        let _ = self.splitters.insert(id, size);
    }

    /// Forgets what the last frame drew, so that the next one draws all of the window: for when the frame buffer was
    /// changed by something else.
    pub fn redraw_everything(&mut self) {
        self.tiles = None;
    }

    /// Forgets the state of a widget that is gone for good, such as the input box of a dialog that closed.
    pub fn forget(&mut self, id: WidgetId) {
        let _ = self.input_boxes.remove(&id);
        let _ = self.scrolls.remove(&id);
        let _ = self.lists.remove(&id);
        let _ = self.code_editors.remove(&id);
        let _ = self.focus_serials.remove(&id);
        let _ = self.splitters.remove(&id);
        let _ = self.pickers.remove(&id);
        let _ = self.list_searches.remove(&id);
    }
}
