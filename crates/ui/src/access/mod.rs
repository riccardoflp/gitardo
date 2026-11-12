//! What is on screen, told to the screen reader: every widget the frame declared, with what it is, its name, its value
//! and its state, in a tree of the parts of the window, rebuilt each frame (rule R10.4).
//!
//! The widgets of the toolkit describe themselves; a view that draws a widget of its own describes it with
//! [`Ui::describe`] and names the parts of its window with [`Ui::region`]. The application hands the tree to the
//! platform with [`Ui::accessibility_update`] or [`UiMemory::accessibility_update`], and the requests of the screen
//! reader back with [`UiMemory::accessibility_action`], which act as a click or the keyboard would. Only the types of
//! `accesskit` cross between `ui` and `platform` (rule R1.6).

mod action;
mod tree;

use std::hash::Hash;

use render::Rect;

use crate::frame::Ui;
use crate::id::WidgetId;
use crate::interaction::WidgetEvent;
use crate::memory::UiMemory;

/// What a widget or a part of the window is, for the screen reader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AccessRole {
    /// Widgets that belong together, with no other meaning.
    #[default]
    Group,
    /// The bar at the top of the window.
    Header,
    /// A list of places to go, such as the branch rail.
    Navigation,
    /// The main content of the window, such as the timeline.
    Main,
    /// What goes beside the main content, such as the inspector.
    Complementary,
    /// A part of the window with a name of its own, such as the Activity drawer or a card.
    Region,
    /// A button.
    Button,
    /// A check box.
    CheckBox,
    /// A radio button.
    RadioButton,
    /// A toggle switch.
    Switch,
    /// An input box.
    TextInput,
    /// An input box that searches.
    SearchInput,
    /// A list whose rows can be selected.
    List,
    /// A row of a list.
    ListItem,
    /// A menu.
    Menu,
    /// An item of a menu.
    MenuItem,
    /// A dialog, or a picker.
    Dialog,
    /// Text.
    Label,
    /// A title.
    Heading,
    /// A progress bar or a spinner.
    Progress,
    /// A message that says how something went.
    Status,
}

impl AccessRole {
    /// Whether a widget of this role acts when it is clicked.
    fn is_clickable(self) -> bool {
        matches!(
            self,
            AccessRole::Button
                | AccessRole::CheckBox
                | AccessRole::RadioButton
                | AccessRole::Switch
                | AccessRole::TextInput
                | AccessRole::SearchInput
                | AccessRole::ListItem
                | AccessRole::MenuItem
        )
    }
}

/// What the screen reader is told about a widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Description<'text> {
    /// What it is.
    pub role: AccessRole,
    /// Its name: the words on a button, the label of an input box, the text of a row.
    pub name: &'text str,
    /// Its value, such as the text typed in an input box.
    pub value: Option<&'text str>,
    /// Whether it is selected, for a row.
    pub is_selected: Option<bool>,
    /// Whether it is on, for a check box, a radio button or a toggle switch.
    pub is_on: Option<bool>,
    /// Whether what it holds is shown, for a row that unfolds or a button that opens a menu.
    pub is_expanded: Option<bool>,
    /// Whether it cannot be used now.
    pub is_disabled: bool,
}

impl<'text> Description<'text> {
    /// A widget of a role and a name, with no value and no state.
    #[must_use]
    pub fn new(role: AccessRole, name: &'text str) -> Description<'text> {
        Description {
            role,
            name,
            value: None,
            is_selected: None,
            is_on: None,
            is_expanded: None,
            is_disabled: false,
        }
    }

    /// The same description with a value.
    #[must_use]
    pub fn value(self, value: &'text str) -> Description<'text> {
        Description {
            value: Some(value),
            ..self
        }
    }

    /// The same description, selected or not.
    #[must_use]
    pub fn selected(self, is_selected: bool) -> Description<'text> {
        Description {
            is_selected: Some(is_selected),
            ..self
        }
    }

    /// The same description, on or off.
    #[must_use]
    pub fn on(self, is_on: bool) -> Description<'text> {
        Description {
            is_on: Some(is_on),
            ..self
        }
    }

    /// The same description, unfolded or folded.
    #[must_use]
    pub fn expanded(self, is_expanded: bool) -> Description<'text> {
        Description {
            is_expanded: Some(is_expanded),
            ..self
        }
    }

    /// The same description, of a widget that cannot be used now.
    #[must_use]
    pub fn disabled(self, is_disabled: bool) -> Description<'text> {
        Description { is_disabled, ..self }
    }
}

/// A widget as the screen reader is told of it, kept from the frame that declared it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AccessNode {
    /// Its name, which names its node.
    pub(crate) id: WidgetId,
    /// The part of the window it is in; nothing for the window itself.
    pub(crate) parent: Option<WidgetId>,
    /// The widget a click or the focus asked for it goes to: itself, or the list of a row.
    pub(crate) owner: WidgetId,
    /// Where it is, in logical pixels.
    pub(crate) rect: Rect,
    /// What it is.
    pub(crate) role: AccessRole,
    /// Its name.
    pub(crate) name: String,
    /// Its value.
    pub(crate) value: Option<String>,
    /// Whether it is selected.
    pub(crate) is_selected: Option<bool>,
    /// Whether it is on.
    pub(crate) is_on: Option<bool>,
    /// Whether it is unfolded.
    pub(crate) is_expanded: Option<bool>,
    /// Whether it cannot be used now.
    pub(crate) is_disabled: bool,
    /// Whether it is a dialog that holds the keyboard.
    pub(crate) is_modal: bool,
    /// The row of a list that has the keyboard's cursor, which has the focus when the list has it.
    pub(crate) active_row: Option<WidgetId>,
}

/// The widgets a frame described, and the parts of the window open while it declares more.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessRecorder {
    /// The widgets, in the order they were declared.
    pub(crate) nodes: Vec<AccessNode>,
    /// The parts of the window open, the innermost last.
    parents: Vec<WidgetId>,
}

/// What the toolkit keeps for the screen reader between frames: the tree of the last frame, and what the screen reader
/// asked for since, to apply in the next frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessMemory {
    /// The widgets of the last frame.
    pub(crate) nodes: Vec<AccessNode>,
    /// Physical pixels for each logical pixel, in the last frame.
    pub(crate) scale: f32,
    /// The events asked for by the screen reader, for the widget each goes to.
    pub(crate) pending: Vec<(WidgetId, WidgetEvent)>,
}

impl AccessRecorder {
    /// The part of the window open now, if any.
    fn parent(&self) -> Option<WidgetId> {
        self.parents.last().copied()
    }
}

impl Ui<'_> {
    /// Tells the screen reader what a widget the view drew itself is, such as a row of its own: its name, where it is
    /// and its state. A click or the focus the screen reader asks for goes to the widget, as the pointer's would.
    pub fn describe(&mut self, id: WidgetId, rect: Rect, description: &Description<'_>) {
        self.access_node(id, rect, description);
    }

    /// Declares a part of the window for the screen reader, such as the header or the timeline, with the widgets
    /// `inside` declares in it.
    pub fn region<R>(
        &mut self,
        key: impl Hash,
        (rect, role, name): (Rect, AccessRole, &str),
        inside: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        let id = self.id(key);
        self.access_within(id, rect, &Description::new(role, name), inside)
    }

    /// Notes a widget for the screen reader, in the part of the window open, when any of it can be seen.
    pub(crate) fn access_node(&mut self, id: WidgetId, rect: Rect, description: &Description<'_>) {
        self.push_access(id, id, rect, description);
    }

    /// Notes a row of a list for the screen reader: a click on it goes to its list, at the row.
    pub(crate) fn access_row(&mut self, (list, row): (WidgetId, WidgetId), rect: Rect, description: &Description<'_>) {
        self.push_access(row, list, rect, description);
    }

    /// Notes a widget that holds others, such as a list, a menu or a dialog, with the widgets `inside` declares in it.
    pub(crate) fn access_within<R>(
        &mut self,
        id: WidgetId,
        rect: Rect,
        description: &Description<'_>,
        inside: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        self.push_access(id, id, rect, description);
        self.access.parents.push(id);
        let result = inside(self);
        let _ = self.access.parents.pop();
        result
    }

    /// Opens a part of the window for the screen reader: the widgets noted until [`Ui::access_leave`] are in it.
    pub(crate) fn access_enter(&mut self, id: WidgetId) {
        self.access.parents.push(id);
    }

    /// Closes the part of the window opened last.
    pub(crate) fn access_leave(&mut self) {
        let _ = self.access.parents.pop();
    }

    /// Says whether what a widget noted for the screen reader opens is shown, such as the menu of a button.
    pub(crate) fn access_expanded(&mut self, id: WidgetId, is_expanded: bool) {
        if let Some(node) = self.access.nodes.iter_mut().rev().find(|node| node.id == id) {
            node.is_expanded = Some(is_expanded);
        }
    }

    /// Says which row of a list noted for the screen reader has the keyboard's cursor.
    pub(crate) fn access_active_row(&mut self, list: WidgetId, row: WidgetId) {
        if let Some(node) = self.access.nodes.iter_mut().rev().find(|node| node.id == list) {
            node.active_row = Some(row);
        }
    }

    /// Notes a widget for the screen reader, unless it is out of sight or already noted in this frame.
    fn push_access(&mut self, id: WidgetId, owner: WidgetId, rect: Rect, description: &Description<'_>) {
        if rect.intersection(self.current_clip()).is_empty() || self.access.nodes.iter().any(|node| node.id == id) {
            return;
        }
        let is_modal = description.role == AccessRole::Dialog && self.current_layer() == crate::layer::Layer::Modal;
        self.access.nodes.push(AccessNode {
            id,
            parent: self.access.parent(),
            owner,
            rect,
            role: description.role,
            name: description.name.to_owned(),
            value: description.value.map(str::to_owned),
            is_selected: description.is_selected,
            is_on: description.is_on,
            is_expanded: description.is_expanded,
            is_disabled: description.is_disabled,
            is_modal,
            active_row: None,
        });
    }

    /// The tree of what this frame declared so far, for the screen reader: the window, its parts and their widgets,
    /// with the focus. The application calls it once the view declared its widgets and hands it to the platform.
    #[must_use]
    pub fn accessibility_update(&self) -> accesskit::TreeUpdate {
        let focus = self.memory_view().focus();
        let hits = self.hits();
        tree::update(&self.access.nodes, (self.scale(), focus), |id| hits.is_focusable(id))
    }

    /// Keeps the widgets of this frame for the requests of the screen reader until the next frame.
    pub(crate) fn keep_access(&mut self) {
        let nodes = std::mem::take(&mut self.access.nodes);
        let scale = self.scale();
        let memory = &mut self.memory().access;
        memory.nodes = nodes;
        memory.scale = scale;
    }
}

impl UiMemory {
    /// The tree of what the last frame drew, for the screen reader, with the focus as it is now: for an application
    /// that hands the tree over once the frame is finished.
    #[must_use]
    pub fn accessibility_update(&self) -> accesskit::TreeUpdate {
        let hits = &self.interaction.hits;
        tree::update(&self.access.nodes, (self.access.scale, self.focus()), |id| {
            hits.is_focusable(id)
        })
    }
}
