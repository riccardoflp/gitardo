//! What the screen reader is told of the gallery: the tree of the window with its parts, its buttons, its input boxes
//! and the rows of the timeline, with the focus; and its requests, which act as a click or the keyboard would.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use accesskit::{Action, ActionRequest, Node, NodeId, Role, TreeId, TreeUpdate};
use render::ThemeName;
use support::Window;
use ui::gallery::{self, GalleryIntent, GalleryState};

/// The gallery drawn once in its fixed state, with the tree of that frame.
fn gallery() -> (Window, TreeUpdate) {
    let mut window = Window::new(ThemeName::Dark, (1280, 800), 1);
    gallery::prepare(&mut window.memory, &window.theme);
    let state = GalleryState::default();
    let (tree, _) = window.frame(&[], |ui| {
        let _ = gallery::view(&state, ui);
        ui.accessibility_update()
    });
    (window, tree)
}

/// The first node of a role with a label.
fn find<'tree>(tree: &'tree TreeUpdate, role: Role, label: &str) -> (NodeId, &'tree Node) {
    let found = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == role && node.label() == Some(label));
    assert!(found.is_some(), "no {role:?} named {label}");
    let (id, node) = found.unwrap();
    (*id, node)
}

#[test]
fn the_tree_has_the_commit_button_the_rows_of_the_timeline_and_the_focus() {
    let (_, tree) = gallery();
    let info = tree.tree.as_ref().unwrap();
    let (_, window) = tree.nodes.iter().find(|(id, _)| *id == info.root).unwrap();
    assert_eq!(window.role(), Role::Window);
    let (_, commit) = find(&tree, Role::Button, "Commit");
    assert!(commit.supports_action(Action::Click));
    let (_, history) = find(&tree, Role::ListBox, "History");
    let rows: Vec<&Node> = history
        .children()
        .iter()
        .filter_map(|child| tree.nodes.iter().find(|(id, _)| id == child).map(|(_, node)| node))
        .collect();
    assert!(rows.len() >= 10, "the rows in sight are in the tree: {}", rows.len());
    let first = rows.first().unwrap();
    assert_eq!(first.role(), Role::ListBoxOption);
    assert_eq!(first.label(), Some("4b830a6 Add the login page"));
    assert_eq!(first.is_selected(), Some(true));
    let (message, _) = find(&tree, Role::TextInput, "Describe the next commit");
    assert_eq!(tree.focus, message, "the message of the draft has the focus");
    let (_, search) = find(&tree, Role::SearchInput, "Commits, branches, files, commands");
    assert!(search.bounds().is_some());
    for part in ["Repository", "Branches", "Timeline", "Selected commit"] {
        assert!(
            tree.nodes.iter().any(|(_, node)| node.label() == Some(part)),
            "the part {part} is in the tree"
        );
    }
}

#[test]
fn a_click_and_the_focus_asked_by_the_screen_reader_act_as_the_pointer_and_the_keyboard_would() {
    let (mut window, tree) = gallery();
    let (commit, _) = find(&tree, Role::Button, "Commit");
    let (review, _) = find(&tree, Role::Button, "Review");
    let request = |action, target_node| ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node,
        data: None,
    };
    assert!(window.memory.accessibility_action(&request(Action::Click, commit)));
    let state = GalleryState::default();
    let (intents, _) = window.frame(&[], |ui| gallery::view(&state, ui));
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, GalleryIntent::Did(text) if text.starts_with("Committed"))),
        "{intents:?}"
    );
    assert!(window.memory.accessibility_action(&request(Action::Focus, review)));
    let tree = window.memory.accessibility_update();
    assert_eq!(tree.focus, review);
    assert!(
        !window.memory.accessibility_action(&request(Action::Click, NodeId(7))),
        "no such node"
    );
}
