#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot read its sources fails"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use model::changes::{ChangeRow, FileStatus, StagedBox};
use model::filter::{Kinds, ReferenceKind};
use model::state::ChangeKind;
use ui::{Icon, MenuEntry};

use super::menu::operation_icon;
use super::*;

/// The items made without an icon, by the file that makes them and the words it gives them, and why no icon says them.
const WITHOUT_ICON: [(&str, &str, &str); 14] = [
    (
        "graph/commit_menu.rs",
        "label.as_str()",
        "the interactive rebase page: a rewrite of the history",
    ),
    ("settings/controls.rs", "name", "a font: a value of a setting"),
    ("settings/row.rs", "choice", "a value of a setting"),
    ("page/trail.rs", "name.words", "a page of the way back"),
    (
        "rebase/todo_page.rs",
        "action_words(*each).0",
        "the action of a step of a rebase: a value, named as git names it",
    ),
    ("file_view/edit.rs", "words::GO_TO_LINE", "a line: no icon says it"),
    (
        "tasks/list.rs",
        "words::COLLAPSE_ALL",
        "every heading of a list folded: no icon says it",
    ),
    (
        "tasks/list.rs",
        "words::EXPAND_ALL",
        "every heading of a list unfolded: no icon says it",
    ),
    ("tasks/menu.rs", "words::RUN", "a task run: no icon says it"),
    (
        "tasks/menu.rs",
        "words::RESTART",
        "a task stopped and run again: no icon says it",
    ),
    ("tasks/menu.rs", "name", "a group of tasks: a name the user gave it"),
    (
        "tasks/groups.rs",
        "words::IMPORT_GROUPS",
        "groups read from a file: no icon says it",
    ),
    (
        "tasks/groups.rs",
        "words::EXPORT_GROUPS",
        "groups saved to a file to import elsewhere: no icon says it",
    ),
    (
        "tasks/groups.rs",
        "words::RUN_ALL",
        "the tasks of a group run: no icon says it",
    ),
];

/// The operations offered in a menu that have no icon: they rewrite the history (rebase, reset, squash, split,
/// fixup), or no icon of Lucide says them, as a branch that stops following its upstream.
const OPERATIONS_WITHOUT_ICON: [&str; 14] = [
    "rebase.onto",
    "rebase.branch-onto",
    "reset.to-commit",
    "squash.into-previous",
    "squash.commits",
    "split.commit",
    "fixup.commit",
    "patch.apply",
    "identity.switch",
    "submodule.sync-urls",
    "branch.unset-upstream",
    "worktree.unlock",
    "upkeep.start-background",
    "upkeep.stop-background",
];

/// The sources of the views, tests left out, with their paths from `src/`.
fn view_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let mut folders = vec![root.clone()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap() {
            let path: PathBuf = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != "tests" {
                    folders.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && name != "tests.rs"
                && !name.ends_with("_tests.rs")
            {
                let relative = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
                found.push((relative, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    found
}

/// The text of a source without its comments, line by line.
fn code_of(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first argument of each call of a function in a source, as written.
fn first_arguments<'a>(source: &'a str, call: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    for (start, _) in source.match_indices(call) {
        let rest = source.get(start + call.len()..).unwrap_or_default();
        let mut depth = 0_i32;
        let end = rest
            .char_indices()
            .find(|(_, character)| {
                match character {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' | ',' if depth == 0 => return true,
                    ')' | ']' | '}' => depth -= 1,
                    _ => {}
                }
                false
            })
            .map_or(rest.len(), |(at, _)| at);
        found.push(rest.get(..end).unwrap_or_default().trim());
    }
    found
}

/// The identifiers of the cards of the catalog.
fn cards() -> BTreeSet<String> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/features");
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "md") {
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if let Some(identifier) = line.strip_prefix("### `").and_then(|rest| rest.split('`').next()) {
                    let _ = found.insert(identifier.to_owned());
                }
            }
        }
    }
    found
}

/// The string literals of a source that are the identifier of a card.
fn features_named(source: &str, cards: &BTreeSet<String>) -> Vec<String> {
    source
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|literal| cards.contains(*literal))
        .map(str::to_owned)
        .collect()
}

/// The icons of the items of a menu, in order, separators left out.
fn icons_of<T>(entries: &[MenuEntry<'_, T>]) -> Vec<Option<Icon>> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some(item.icon),
            MenuEntry::Submenu { icon, .. } => Some(*icon),
            MenuEntry::Separator => None,
        })
        .collect()
}

/// Checks that a source of the views makes its items through `crate::icons`, and takes out of `unused` the items without
/// an icon it makes, each of which must be on [`WITHOUT_ICON`].
fn check_items(file: &str, code: &str, unused: &mut BTreeSet<(&str, &str)>) {
    if file == "icons/menu.rs" {
        return;
    }
    assert!(
        !code.contains("MenuItem::new("),
        "{file} makes an item with MenuItem::new: make it with crate::icons, with the icon of its action"
    );
    let makes_submenu = code
        .match_indices("MenuEntry::Submenu {")
        .any(|(at, found)| !code.get(at + found.len()..).unwrap_or_default().starts_with(" .."));
    assert!(
        !makes_submenu || code.contains("icon: Some("),
        "{file} makes a submenu without an icon"
    );
    for words in first_arguments(code, "without_icon(") {
        let listed = WITHOUT_ICON
            .iter()
            .find(|(listed_file, listed_words, _)| *listed_file == file && *listed_words == words);
        let Some((listed_file, listed_words, _)) = listed else {
            panic!("{file} makes an item without an icon, saying {words}: give it the icon of its action");
        };
        let _ = unused.remove(&(*listed_file, *listed_words));
    }
}

/// The operations a source that makes menus names that have no icon and are not on [`OPERATIONS_WITHOUT_ICON`]; those
/// that are on it are taken out of `unused`.
fn operations_missing(file: &str, code: &str, cards: &BTreeSet<String>, unused: &mut BTreeSet<&str>) -> Vec<String> {
    let builds_menus = ["MenuEntry", "OperationEntry", "DropEntry", "SelectionEntry"]
        .iter()
        .any(|name| code.contains(name));
    if !builds_menus {
        return Vec::new();
    }
    let mut missing = Vec::new();
    for feature in features_named(code, cards) {
        if operation_icon(&feature).is_none() {
            if !OPERATIONS_WITHOUT_ICON.contains(&feature.as_str()) {
                missing.push(format!("{feature} in {file}"));
            }
            let _ = unused.remove(feature.as_str());
        }
    }
    missing
}

#[test]
fn ui_menu_icons__every_item_of_every_menu_has_the_icon_of_its_action_or_is_on_the_list_without_one() {
    let cards = cards();
    let mut unused: BTreeSet<(&str, &str)> = WITHOUT_ICON.iter().map(|(file, words, _)| (*file, *words)).collect();
    let mut operations_unused: BTreeSet<&str> = OPERATIONS_WITHOUT_ICON.into_iter().collect();
    let model_drag = Path::new(env!("CARGO_MANIFEST_DIR")).join("../model/src/drag.rs");
    let mut sources = view_sources();
    sources.push(("model/drag.rs".to_owned(), std::fs::read_to_string(model_drag).unwrap()));
    let mut missing = Vec::new();
    for (file, source) in &sources {
        let code = code_of(source);
        check_items(file, &code, &mut unused);
        missing.extend(operations_missing(file, &code, &cards, &mut operations_unused));
    }
    assert!(
        missing.is_empty(),
        "operations offered with no icon in operation_icon: {missing:?}"
    );
    assert!(
        unused.is_empty(),
        "items listed without an icon that no view makes: {unused:?}"
    );
    assert!(
        operations_unused.is_empty(),
        "operations listed without an icon that no menu offers: {operations_unused:?}"
    );
    for feature in OPERATIONS_WITHOUT_ICON {
        assert_eq!(operation_icon(feature), None, "{feature} is listed but has an icon");
    }
}

#[test]
fn ui_menu_icons__an_action_draws_in_a_menu_the_icon_its_button_draws() {
    let row = ChangeRow {
        path: "src/parser.rs".to_owned(),
        original_path: None,
        status: FileStatus::Changed(ChangeKind::Modified),
        staged: StagedBox::Empty,
        is_unsaved: false,
    };
    let item_of = |said: &str| {
        crate::changes::file_actions(&row)
            .into_iter()
            .find(|action| action.words == said)
            .map(|action| action.icon)
    };
    let changes = crate::text::changes::DISCARD;
    assert_eq!(operation_icon("changes.discard-all"), item_of(changes));
    assert_eq!(operation_icon("changes.discard"), item_of(changes));
    assert_eq!(operation_icon("stash.push"), item_of(crate::text::changes::STASH_FILE));
    assert_eq!(
        operation_icon("stash.push-file"),
        item_of(crate::text::changes::STASH_FILE)
    );
    assert_eq!(operation_icon("changes.stage"), item_of(crate::text::changes::STAGE));
    assert_eq!(operation_icon("changes.unstage"), Some(UNSTAGE));
    assert_eq!(operation_icon("conflicts.mark-resolved"), Some(ADD));
    let stash = icons_of(&crate::graph::stash_menu());
    assert_eq!(stash, [Some(APPLY), Some(POP), Some(CREATE_BRANCH), Some(DELETE)]);
    assert_eq!(stash.last().copied().flatten(), Some(crate::refs::drop_stash_icon()));
    assert_eq!(operation_icon("blame.reveal-line-commit"), Some(TIMELINE));
    assert_eq!(operation_icon("sync.push"), Some(PUSH));
    assert_eq!(operation_icon("compare.with"), Some(COMPARE));
    assert_eq!(operation_icon("file-history.file"), Some(HISTORY));
}

#[test]
fn ui_menu_icons__every_copy_item_draws_the_copy_icon_and_every_delete_item_the_bin() {
    for copy in [
        "commits.copy-hash",
        "commits.copy-message",
        "patch.copy",
        "worktree.copy-path",
        "submodule.copy-path",
        "submodule.copy-url",
    ] {
        assert_eq!(operation_icon(copy), Some(COPY), "{copy}");
    }
    for delete in [
        "branch.delete",
        "tag.delete",
        "remote.delete-branch",
        "worktree.remove",
        "submodule.remove",
        "stash.drop",
    ] {
        assert_eq!(operation_icon(delete), Some(DELETE), "{delete}");
    }
    assert_eq!(icons_of(&crate::graph::stash_menu()).last(), Some(&Some(DELETE)));
}

#[test]
fn ui_menu_icons__a_switch_keeps_its_check_while_it_is_on_and_has_no_icon_of_its_own() {
    let kinds = Kinds::ALL.with(ReferenceKind::Remote, false);
    let entries = crate::graph::kind_entries(Some(kinds));
    assert_eq!(icons_of(&entries), [Some(CHOSEN), None, Some(CHOSEN)]);
    let narrowed = crate::graph::kind_entries(None);
    assert_eq!(icons_of(&narrowed), [None, None, None]);
}
