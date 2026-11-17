#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, reason = "a test that cannot read its sources fails")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use model::changes::{ChangeRow, FileStatus, StagedBox};
use model::state::ChangeKind;

use super::*;

/// The meanings the table of `graph.icons` gives one icon together: the chevron that points down is a row unfolded
/// and, in the bar of a page, the next change or match.
const SHARED: [[&str; 2]; 1] = [["Unfolded", "The next change or match, in the bar of a page"]];

/// The meanings of the icons the widgets of `ui` draw for themselves: a box closed, an entry chosen, the matches of
/// the find bar, a search box, a menu that opens to the side.
const WIDGET_ICONS: [Icon; 6] = [CLOSE, CHOSEN, PREVIOUS, NEXT, SEARCH, FOLDED];

/// The Rust files under a folder, at any depth.
fn rust_files(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found
}

/// The names of the icons a source names as `Icon::<name>`.
fn icons_named(source: &str) -> Vec<String> {
    source
        .match_indices("Icon::")
        .map(|(start, found)| {
            let rest = source.get(start + found.len()..).unwrap_or_default();
            rest.chars().take_while(char::is_ascii_alphanumeric).collect()
        })
        .filter(|name: &String| name != "ALL")
        .collect()
}

#[test]
fn graph_icons__no_icon_is_drawn_for_two_meanings() {
    let mut meanings_of: BTreeMap<Icon, Vec<&str>> = BTreeMap::new();
    for (meaning, icon) in MEANINGS {
        meanings_of.entry(icon).or_default().push(meaning);
    }
    for (icon, meanings) in &meanings_of {
        let is_shared = SHARED.iter().any(|shared| shared.as_slice() == meanings.as_slice());
        assert!(meanings.len() == 1 || is_shared, "{icon:?} means {meanings:?}");
    }
    for icon in Icon::ALL {
        assert!(meanings_of.contains_key(&icon), "{icon:?} has no meaning in the table");
    }
    let views = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in rust_files(&views) {
        if file.starts_with(views.join("icons")) {
            continue;
        }
        let named = icons_named(&std::fs::read_to_string(&file).unwrap());
        assert!(
            named.is_empty(),
            "{} draws {named:?}: a view names the meaning, from crate::icons",
            file.display()
        );
    }
    let widgets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/widget");
    let widget_names: Vec<String> = WIDGET_ICONS.iter().map(|icon| format!("{icon:?}")).collect();
    for file in rust_files(&widgets) {
        for name in icons_named(&std::fs::read_to_string(&file).unwrap()) {
            assert!(widget_names.contains(&name), "{} draws {name}", file.display());
        }
    }
}

#[test]
fn graph_icons__discard_changes_and_revert_this_change_draw_the_same_icon_and_drop_another() {
    let row = ChangeRow {
        path: "src/parser.rs".to_owned(),
        original_path: None,
        status: FileStatus::Changed(ChangeKind::Modified),
        staged: StagedBox::Empty,
        is_unsaved: false,
    };
    let discard = crate::changes::file_buttons(&row)
        .into_iter()
        .find(|(said, _, _)| *said == crate::text::changes::DISCARD)
        .map(|(_, icon, _)| icon);
    let revert = crate::diff::revert_change_icon();
    let drop = crate::refs::drop_stash_icon();
    assert_eq!(discard, Some(revert));
    assert_eq!(revert, UNDO);
    assert_ne!(drop, revert);
    assert_eq!(drop, DELETE);
}
