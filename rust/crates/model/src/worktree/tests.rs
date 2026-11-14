use std::path::Path;

use super::{FolderInput, folder_path, worktree_label, worktrees_folder};

fn folder(setting: &str, name: &str) -> String {
    folder_path(&FolderInput {
        setting,
        main_worktree: Path::new("/home/someone/code/app"),
        home: Path::new("/home/someone"),
        name,
    })
    .to_string_lossy()
    .replace('\\', "/")
}

#[test]
fn worktree_folder_path__every_example_of_the_table() {
    let cases = [
        ("", "fix-login", "/home/someone/code/app.worktrees/fix-login"),
        ("", "feature/login", "/home/someone/code/app.worktrees/feature/login"),
        (
            "../trees/${repo}",
            "feature/login",
            "/home/someone/trees/app/feature/login",
        ),
        ("~/wt", "fix: the #42 bug", "/home/someone/wt/fix-the-42-bug"),
        ("D:/wt", "../escape", "D:/wt/escape"),
        ("", "...", "/home/someone/code/app.worktrees/worktree"),
        ("D:\\wt\\", "a", "D:/wt/a"),
    ];
    for (setting, name, expected) in cases {
        assert_eq!(folder(setting, name), expected, "{setting} {name}");
    }
}

#[test]
fn worktree_folder_path__from_another_worktree_it_is_beside_the_main_one() {
    // The main worktree is given whichever worktree asks: a worktree's own folder is never the base.
    let from_main = worktrees_folder("", Path::new("/home/someone/code/app"), Path::new("/home/someone"));
    assert_eq!(from_main, "/home/someone/code/app.worktrees");
}

#[test]
fn worktree_folder_path__no_name_leaves_the_folder_of_the_worktrees() {
    let base = "/home/someone/code/app.worktrees/";
    for name in [
        "../../etc",
        "..",
        "a/../../b",
        "/abs/olute",
        "C:\\x",
        "~/home",
        "a/./b",
        "\u{e9}t\u{e9}/..",
        "",
    ] {
        let found = folder("", name);
        assert!(found.starts_with(base), "{name} gave {found}");
        assert!(
            !found[base.len()..].split('/').any(|part| part == ".." || part == "."),
            "{name} gave {found}"
        );
    }
    assert_eq!(folder("", &"x".repeat(80)).len(), base.len() + 60);
}

#[test]
fn worktree_folder_path__the_label_is_the_path_inside_the_folder_of_the_worktrees() {
    let worktrees = "D:/Code/App.worktrees";
    assert_eq!(
        worktree_label(worktrees, Path::new("d:/code/app.worktrees/feature/login")),
        "feature/login"
    );
    assert_eq!(worktree_label(worktrees, Path::new("D:\\elsewhere\\hotfix")), "hotfix");
    assert_eq!(worktree_label(worktrees, Path::new("D:/Code/App.worktrees-old/x")), "x");
}

#[test]
fn worktree_branch_in_another_worktree__dragged_onto_a_commit_it_offers_no_rebase_and_still_offers_to_merge() {
    use crate::CommitId;
    use crate::drag::{Dragged, DropInput, DropRow, list_drop_entries};
    let hash = |digit: char| CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"));
    let (head, tip, other) = (hash('1'), hash('2'), hash('3'));
    let dragged = Dragged::Branch {
        name: "feature/login",
        tip: &tip,
        is_remote: false,
        is_checked_out_elsewhere: true,
    };
    let dropped = |row| {
        let input = DropInput {
            dragged,
            row,
            head: Some(&head),
            branch: Some("main"),
            is_operation_under_way: false,
        };
        list_drop_entries(&input)
            .into_iter()
            .map(|entry| entry.feature.0)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        dropped(DropRow::Commit(&other)),
        Vec::<&str>::new(),
        "no rebase onto another commit"
    );
    let on_head = dropped(DropRow::Commit(&head));
    assert!(on_head.contains(&"merge.into-current"), "{on_head:?}");
}
