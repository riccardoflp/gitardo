#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "a test fails loudly")]

use std::path::PathBuf;

use super::{GUIDE, STALE, check, make};
use crate::check::Workspace;
use crate::files::SourceFile;

/// A workspace of the files given, by their path from the root of the repository or of the workspace.
fn workspace(files: &[(&str, &str)]) -> Workspace {
    Workspace {
        members: Vec::new(),
        files: files
            .iter()
            .map(|(relative, text)| {
                let file = SourceFile {
                    relative: (*relative).to_owned(),
                    path: PathBuf::from(relative),
                };
                (file, Some((*text).to_owned()))
            })
            .collect(),
        repository: PathBuf::new(),
    }
}

/// An index of three cards: one Built in both apps, one Built in the desktop and Planned in the extension, one
/// Specified, and one Host only in the extension.
const INDEX: &str = "\
| [`stash.push`](08-stashes.md) | Stash changes | Operation | Built | Built |
| [`stash.only-here`](08-stashes.md) | Only here | Operation | Built | Planned |
| [`stash.later`](08-stashes.md) | Later | Operation | Specified | Planned |
| [`stash.vscode`](08-stashes.md) | Done by VS Code | Operation | Built | Host only |
";

/// The cards of the index, in an area of the catalog.
const CARDS: &str = "\
# Stashes

### `stash.push` — Stash changes

Puts the changes aside ([D12](../08-decisions.md#d12)).

- **Kind**: Operation
- **Function**: `features::stash::push(context, answers) -> Outcome`
- **Entry points**: **Stash Changes…** in the context menu of the draft; **Stash Changes** in Quick search
  (see [the draft](02-graph.md)).
- **Extension**: **Gitardo: Stash Changes** in the Command Palette, run by `Controller.stash()`.

Acceptance:

- [ ] Something.
      Its next line.

### `stash.only-here` — Only here

Only in the window.

- **Kind**: Operation
- **Entry points**: the branch rail.

### `stash.later` — Later

Planned.

- **Kind**: Operation

### `stash.vscode` — Done by VS Code

VS Code does it.

- **Kind**: Operation
";

/// The guide of the small catalog.
fn small_guide() -> String {
    make(&workspace(&[
        ("docs/features/README.md", INDEX),
        ("docs/features/08-stashes.md", CARDS),
    ]))
}

/// The entry of a card in the guide of an app, until the next.
fn entry<'a>(guide: &'a str, app: &str, identifier: &str) -> Option<&'a str> {
    let start = guide.find(&format!("@card {app} {identifier}\n"))?;
    let rest = &guide[start..];
    let end = rest[1..].find("\n@").map_or(rest.len(), |end| end + 2);
    Some(&rest[..end])
}

/// The guide of the catalog of the repository.
fn real_guide() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    make(&Workspace::read(&root).unwrap())
}

#[test]
fn help_guide__a_card_built_in_the_desktop_and_planned_in_the_extension_is_only_in_the_desktops_guide() {
    let guide = small_guide();
    assert!(entry(&guide, "desktop", "stash.only-here").is_some(), "{guide}");
    assert!(entry(&guide, "extension", "stash.only-here").is_none(), "{guide}");
    let pushed = entry(&guide, "extension", "stash.push").unwrap();
    assert!(
        pushed.contains("In VS Code: **Gitardo: Stash Changes** in the Command Palette, run by."),
        "{pushed}"
    );
}

#[test]
fn help_guide__a_card_specified_in_the_desktop_is_in_no_guide() {
    let guide = small_guide();
    assert!(!guide.contains("stash.later"), "{guide}");
}

#[test]
fn help_guide__squash_commits_is_in_the_desktops_guide_with_its_entry_points() {
    let guide = real_guide();
    let squash = entry(&guide, "desktop", "squash.commits").unwrap();
    assert!(squash.contains("title: Squash commits into one\n"), "{squash}");
    assert!(
        squash.contains(
            "Entry points: **Squash into one** in the selection bar; **Squash N commits into one…** in the context \
             menu of the selection;"
        ),
        "{squash}"
    );
    assert!(squash.contains("place: the timeline\n"), "{squash}");
}

#[test]
fn help_guide__a_card_host_only_in_the_extension_is_in_its_guide_by_its_title_alone() {
    let guide = small_guide();
    let vscode = entry(&guide, "extension", "stash.vscode").unwrap();
    assert!(vscode.contains("title: Done by VS Code\n"), "{vscode}");
    assert!(vscode.contains("Done by VS Code itself.\n"), "{vscode}");
    assert!(!vscode.contains("VS Code does it."), "{vscode}");
}

#[test]
fn help_guide__no_line_has_a_link_a_name_of_a_function_or_the_number_of_a_decision() {
    for guide in [small_guide(), real_guide()] {
        for line in guide.lines() {
            assert!(!line.contains("]("), "a link: {line}");
            assert!(!line.contains("::"), "a function: {line}");
            let has_number = line
                .split(|character: char| !character.is_ascii_alphanumeric())
                .any(|word| {
                    word.len() > 1
                        && word.starts_with('D')
                        && word[1..].chars().all(|character| character.is_ascii_digit())
                });
            assert!(!has_number, "a decision: {line}");
        }
    }
}

#[test]
fn help_guide__the_check_fails_with_its_message_when_entry_points_change_and_the_guide_is_not_made_again() {
    let made = small_guide();
    let files = [
        ("docs/features/README.md", INDEX),
        ("docs/features/08-stashes.md", CARDS),
        (GUIDE, made.as_str()),
    ];
    assert_eq!(check(&workspace(&files)).len(), 0);
    let changed = CARDS.replace("in the context menu of the draft", "in the menu of the draft");
    let files = [
        ("docs/features/README.md", INDEX),
        ("docs/features/08-stashes.md", changed.as_str()),
        (GUIDE, made.as_str()),
    ];
    let problems = check(&workspace(&files));
    assert_eq!(problems.len(), 1);
    assert!(problems[0].to_string().contains(STALE), "{}", problems[0]);
}
