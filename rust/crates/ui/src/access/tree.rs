//! The tree of `accesskit` made of the widgets a frame described: a node for the window, a node for each widget under
//! the part of the window it was declared in, in the order declared, with its bounds in physical pixels.

use std::collections::{HashMap, HashSet};

use accesskit::{Action, Node, NodeId, Toggled, TreeId, TreeInfo, TreeUpdate};

use super::{AccessNode, AccessRole};
use crate::id::WidgetId;

/// The name of the toolkit, as the screen reader is told it.
const TOOLKIT: &str = "Gitardo";

/// The node of a widget.
fn node_id(id: WidgetId) -> NodeId {
    NodeId(id.number())
}

/// The node of the window, which holds every other.
pub(crate) fn root() -> NodeId {
    node_id(WidgetId::ROOT)
}

/// The role of `accesskit` for a role of the toolkit.
fn role(role: AccessRole) -> accesskit::Role {
    match role {
        AccessRole::Group => accesskit::Role::Group,
        AccessRole::Header => accesskit::Role::Banner,
        AccessRole::Navigation => accesskit::Role::Navigation,
        AccessRole::Main => accesskit::Role::Main,
        AccessRole::Complementary => accesskit::Role::Complementary,
        AccessRole::Region => accesskit::Role::Region,
        AccessRole::Button => accesskit::Role::Button,
        AccessRole::CheckBox => accesskit::Role::CheckBox,
        AccessRole::RadioButton => accesskit::Role::RadioButton,
        AccessRole::Switch => accesskit::Role::Switch,
        AccessRole::TextInput => accesskit::Role::TextInput,
        AccessRole::SearchInput => accesskit::Role::SearchInput,
        AccessRole::List => accesskit::Role::ListBox,
        AccessRole::ListItem => accesskit::Role::ListBoxOption,
        AccessRole::Menu => accesskit::Role::Menu,
        AccessRole::MenuItem => accesskit::Role::MenuItem,
        AccessRole::Dialog => accesskit::Role::Dialog,
        AccessRole::Label => accesskit::Role::Label,
        AccessRole::Heading => accesskit::Role::Heading,
        AccessRole::Progress => accesskit::Role::ProgressIndicator,
        AccessRole::Status => accesskit::Role::Status,
    }
}

/// The node of a widget, without its children.
fn node(widget: &AccessNode, scale: f32, is_focusable: bool) -> Node {
    let mut node = Node::new(role(widget.role));
    // The words of a label are its value, as the screen readers expect; the name of anything else is its label.
    if widget.role == AccessRole::Label {
        node.set_value(widget.name.as_str());
    } else if !widget.name.is_empty() {
        node.set_label(widget.name.as_str());
    }
    if let Some(value) = &widget.value {
        node.set_value(value.as_str());
    }
    let rect = widget.rect;
    let physical = |value: f32| f64::from(value * scale);
    node.set_bounds(accesskit::Rect::new(
        physical(rect.x),
        physical(rect.y),
        physical(rect.right()),
        physical(rect.bottom()),
    ));
    if let Some(is_selected) = widget.is_selected {
        node.set_selected(is_selected);
    }
    if widget.is_mixed {
        node.set_toggled(Toggled::Mixed);
    } else if let Some(is_on) = widget.is_on {
        node.set_toggled(if is_on { Toggled::True } else { Toggled::False });
    }
    if let Some(is_expanded) = widget.is_expanded {
        node.set_expanded(is_expanded);
    }
    if widget.is_modal {
        node.set_modal();
    }
    if widget.is_disabled {
        node.set_disabled();
    } else {
        if widget.role.is_clickable() {
            node.add_action(Action::Click);
        }
        if is_focusable {
            node.add_action(Action::Focus);
        }
    }
    node
}

/// The tree of the widgets of a frame at a scale, with the node that has the focus: the widget with the focus, or the
/// row with the keyboard's cursor of a list with the focus, or the window.
pub(crate) fn update(
    widgets: &[AccessNode],
    (scale, focus): (f32, Option<WidgetId>),
    is_focusable: impl Fn(WidgetId) -> bool,
) -> TreeUpdate {
    let known: HashSet<WidgetId> = widgets.iter().map(|widget| widget.id).collect();
    let mut children: HashMap<WidgetId, Vec<NodeId>> = HashMap::new();
    for widget in widgets {
        let parent = widget
            .parent
            .filter(|parent| known.contains(parent))
            .unwrap_or(WidgetId::ROOT);
        children.entry(parent).or_default().push(node_id(widget.id));
    }
    let mut nodes = Vec::with_capacity(widgets.len() + 1);
    let mut window = Node::new(accesskit::Role::Window);
    window.set_children(children.remove(&WidgetId::ROOT).unwrap_or_default());
    nodes.push((root(), window));
    for widget in widgets {
        let mut node = node(widget, scale, is_focusable(widget.owner));
        if let Some(inside) = children.remove(&widget.id) {
            node.set_children(inside);
        }
        nodes.push((node_id(widget.id), node));
    }
    let focused = focus.and_then(|focus| widgets.iter().find(|widget| widget.id == focus));
    let focus = focused.map_or(root(), |widget| {
        let row = widget.active_row.filter(|row| known.contains(row));
        node_id(row.unwrap_or(widget.id))
    });
    TreeUpdate {
        nodes,
        tree: Some(TreeInfo {
            root: root(),
            toolkit_name: Some(String::from(TOOLKIT)),
            toolkit_version: Some(String::from(env!("CARGO_PKG_VERSION"))),
        }),
        tree_id: TreeId::ROOT,
        focus,
    }
}
