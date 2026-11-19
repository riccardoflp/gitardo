//! The entries that rebase and rewrite commits, the words of the banner of a stop, and the rows of the interactive
//! rebase page, from rows and repositories made by hand.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::commit::Commit;
use model::contract::OpenIn;
use model::graph::Row;
use model::rebase::{RebaseStop, StopKind, TodoAction};
use model::state::{ChangeKind, FileChange, Head, RepositoryState};
use model::{CommitId, Timestamp};

use super::edit_stop::banner_words;
use super::{RebaseIntent, TodoPage, TodoPageBranch, can_squash, rewrite_menu};
use crate::branch::ReferenceMenus;
use crate::text::rebase as words;

/// The hash of a commit named by a letter.
fn id(letter: char) -> CommitId {
    CommitId::parse(&letter.to_string().repeat(40)).unwrap_or_else(|| panic!("{letter} makes no hash"))
}

/// Rows of commits, each `name: parents`, newest first.
fn rows(spec: &[(char, &[char])]) -> Vec<Row> {
    spec.iter()
        .map(|(name, parents)| {
            Row::Commit(Commit {
                id: id(*name),
                parents: parents.iter().map(|parent| id(*parent)).collect(),
                author_name: String::from("Ada"),
                author_email: String::new(),
                author_date: Timestamp(0),
                subject: format!("Commit {name}"),
            })
        })
        .collect()
}

/// `A→B→C→D→E`, `E` checked out on `main`, `A` the root.
fn line() -> Vec<Row> {
    rows(&[('e', &['d']), ('d', &['c']), ('c', &['b']), ('b', &['a']), ('a', &[])])
}

/// A repository checked out on `main` at a commit.
fn repository(head: char) -> RepositoryState {
    RepositoryState {
        head: Head {
            commit: Some(id(head)),
            branch: Some(String::from("refs/heads/main")),
        },
        ..RepositoryState::default()
    }
}

/// The labels of the entries of a commit, the interactive rebase page last, and the operation each runs.
fn entries(repository: &RepositoryState, rows: &[Row], commit: char) -> Vec<(String, Option<&'static str>)> {
    let menus = ReferenceMenus {
        repository,
        open_in: OpenIn::Checkout,
        window: crate::worktree::WorktreeWindow::default(),
        github: crate::github::GitHubShown::default(),
    };
    let menu = rewrite_menu(menus, rows, &id(commit));
    let mut labels: Vec<(String, Option<&'static str>)> = menu
        .operations
        .into_iter()
        .map(|entry| (entry.label, entry.run.map(|(feature, _)| feature.0)))
        .collect();
    labels.extend(menu.todo_page.map(|label| (label, None)));
    labels
}

/// Whether the menu of a commit has an entry.
fn offers(repository: &RepositoryState, rows: &[Row], commit: char, label: &str) -> bool {
    entries(repository, rows, commit).iter().any(|(each, _)| each == label)
}

/// A file changed, as the working tree lists it.
fn changed(path: &str) -> FileChange {
    FileChange {
        path: path.to_owned(),
        original_path: None,
        kind: ChangeKind::Modified,
    }
}

#[test]
fn rebase_onto__the_menu_of_another_commit_offers_rebase_main_onto_this_and_that_of_head_does_not() {
    let rows = line();
    let repository = repository('e');
    let onto = entries(&repository, &rows, 'c');
    assert_eq!(
        onto.first(),
        Some(&(String::from("Rebase main onto this"), Some("rebase.onto")))
    );
    assert!(!offers(&repository, &rows, 'e', "Rebase main onto this"));
}

#[test]
fn rebase_todo_page__the_menu_of_the_third_commit_under_head_opens_the_page_on_three_commits() {
    let rows = line();
    let repository = repository('e');
    assert!(offers(
        &repository,
        &rows,
        'c',
        "Interactive rebase of 3 commits\u{2026}"
    ));
    assert!(offers(&repository, &rows, 'e', words::EDIT_THIS_COMMIT));
    let mut rebasing = repository.clone();
    rebasing.operation = Some(model::state::Operation::Rebase);
    assert_eq!(
        entries(&rebasing, &rows, 'c'),
        [],
        "nothing while a rebase is under way"
    );
}

#[test]
fn rebase_todo_page__while_a_bisect_is_under_way_no_commit_is_rewritten_from_its_menu() {
    let rows = line();
    let mut bisecting = repository('e');
    bisecting.bisect = Some(model::bisect::Bisect::default());
    assert!(!offers(
        &bisecting,
        &rows,
        'c',
        "Interactive rebase of 3 commits\u{2026}"
    ));
    assert!(!offers(&bisecting, &rows, 'e', words::EDIT_THIS_COMMIT));
    assert!(!offers(&bisecting, &rows, 'e', words::SQUASH_INTO_PREVIOUS));
    assert!(!offers(&bisecting, &rows, 'e', words::SPLIT_THIS_COMMIT));
}

#[test]
fn squash_into_previous__on_head_whose_parent_is_not_a_merge_the_entry_is_there() {
    let rows = line();
    let squash = (String::from(words::SQUASH_INTO_PREVIOUS), Some("squash.into-previous"));
    assert!(entries(&repository('e'), &rows, 'e').contains(&squash));
}

#[test]
fn squash_into_previous__on_the_root_commit_the_entry_is_not_there() {
    assert!(!offers(&repository('e'), &line(), 'a', words::SQUASH_INTO_PREVIOUS));
}

#[test]
fn squash_into_previous__on_a_commit_whose_parent_is_a_merge_the_entry_is_not_there() {
    let rows = rows(&[
        ('e', &['d']),
        ('d', &['c', 'f']),
        ('f', &['b']),
        ('c', &['b']),
        ('b', &[]),
    ]);
    let repository = repository('e');
    assert!(!offers(&repository, &rows, 'e', words::SQUASH_INTO_PREVIOUS));
    assert!(offers(&repository, &rows, 'e', words::SPLIT_THIS_COMMIT));
}

#[test]
fn split_commit__the_entry_is_not_on_a_merge_commit_nor_on_the_root_commit() {
    let merge = rows(&[('d', &['c', 'f']), ('f', &['b']), ('c', &['b']), ('b', &[])]);
    assert!(!offers(&repository('d'), &merge, 'd', words::SPLIT_THIS_COMMIT));
    assert!(!offers(&repository('e'), &line(), 'a', words::SPLIT_THIS_COMMIT));
    assert!(offers(&repository('e'), &line(), 'c', words::SPLIT_THIS_COMMIT));
}

#[test]
fn fixup_commit__with_nothing_staged_the_entry_is_add_all_changes_and_with_a_file_staged_add_staged_changes() {
    let rows = line();
    let mut repository = repository('e');
    assert!(
        !offers(&repository, &rows, 'c', words::FIXUP_ALL),
        "no change, no fixup"
    );
    repository.working_tree.unstaged = vec![changed("b.txt")];
    assert!(entries(&repository, &rows, 'c').contains(&(String::from(words::FIXUP_ALL), Some("fixup.commit"))));
    repository.working_tree.staged = vec![changed("a.txt")];
    assert!(offers(&repository, &rows, 'c', words::FIXUP_STAGED));
    assert!(!offers(&repository, &rows, 'c', words::FIXUP_ALL));
}

#[test]
fn fixup_commit__on_the_commit_checked_out_the_entry_is_not_there() {
    let rows = line();
    let mut repository = repository('e');
    repository.working_tree.staged = vec![changed("a.txt")];
    assert!(!offers(&repository, &rows, 'e', words::FIXUP_STAGED));
    assert!(offers(&repository, &rows, 'd', words::FIXUP_STAGED));
}

#[test]
fn squash_commits__selecting_b_and_d_without_c_is_not_offered() {
    let rows = line();
    let repository = repository('e');
    assert!(!can_squash(&repository, &rows, &[id('b'), id('d')]));
    assert!(can_squash(&repository, &rows, &[id('d'), id('b'), id('c')]));
    assert!(can_squash(&repository, &rows, &[id('e'), id('d')]));
    assert!(!can_squash(&repository, &rows, &[id('c')]), "one commit alone");
}

#[test]
fn rebase_edit_stop__stopped_at_add_the_login_page_the_banner_says_so() {
    let stop = |kind| RebaseStop {
        kind,
        commit: id('c'),
        subject: String::from("Add the login page"),
    };
    assert_eq!(
        banner_words(&stop(StopKind::Edit)),
        "Stopped at \"Add the login page\" to edit it. Amend it or commit more, then Continue."
    );
    assert_eq!(
        banner_words(&stop(StopKind::Split)),
        "Splitting \"Add the login page\": commit its changes in as many commits as you like, then Continue."
    );
}

/// The page of `E`, `D`, `C`, newest first.
fn page() -> TodoPage {
    let commits = ['e', 'd', 'c'].map(|name| (id(name), format!("Commit {name}")));
    TodoPage::new(String::from("main"), commits.to_vec())
}

#[test]
fn rebase_todo_page__dragging_the_lowest_row_to_the_top_or_pressing_move_up_twice_puts_it_first() {
    let mut moved = page();
    moved.apply(&RebaseIntent::Move { from: 2, to: 1 });
    moved.apply(&RebaseIntent::Move { from: 1, to: 0 });
    let mut dragged = page();
    dragged.apply(&RebaseIntent::Move { from: 2, to: 0 });
    let order = |page: &TodoPage| page.rows.iter().map(|row| row.commit.clone()).collect::<Vec<_>>();
    assert_eq!(order(&moved), [id('c'), id('e'), id('d')]);
    assert_eq!(order(&dragged), order(&moved));
    assert!(moved.is_changed());
    assert!(!page().is_changed(), "nothing changed: Start rebase is disabled");
}

#[test]
fn rebase_todo_page__setting_every_row_to_drop_shows_the_first_message_and_disables_start_rebase() {
    let mut page = page();
    for index in 0..3 {
        page.apply(&RebaseIntent::SetAction(index, TodoAction::Drop));
    }
    assert_eq!(page.problem(), Some(words::EVERY_COMMIT_DROPPED));
    page.apply(&RebaseIntent::SetAction(2, TodoAction::Fixup));
    assert_eq!(page.problem(), Some(words::NOTHING_TO_MELD_INTO));
    page.apply(&RebaseIntent::SetAction(2, TodoAction::Reword));
    page.apply(&RebaseIntent::TypedSubject(2, String::from("  ")));
    assert_eq!(page.problem(), Some(words::ENTER_A_MESSAGE));
}

#[test]
fn rebase_todo_page__start_rebase_hands_the_rows_oldest_first_to_rebase_interactive() {
    let mut page = page();
    page.apply(&RebaseIntent::SetAction(0, TodoAction::Reword));
    page.apply(&RebaseIntent::TypedSubject(0, String::from("Better words")));
    page.apply(&RebaseIntent::SetAction(1, TodoAction::Drop));
    let expected = format!("pick {}\ndrop {}\nreword {} Better words\n", id('c'), id('d'), id('e'));
    assert_eq!(page.todo_rows(), expected);
    let read = model::rebase::read_todo_rows(&page.todo_rows()).unwrap_or_default();
    assert_eq!(read.len(), 3);
}

#[test]
fn rebase_todo_page__the_row_of_a_commit_shows_the_other_branches_on_it_and_which_stay() {
    let page = page().with_branches(|commit| {
        if *commit == id('d') {
            vec![
                TodoPageBranch {
                    name: String::from("p1"),
                    is_elsewhere: false,
                },
                TodoPageBranch {
                    name: String::from("p2"),
                    is_elsewhere: true,
                },
            ]
        } else {
            Vec::new()
        }
    });
    let names = |index: usize| {
        page.rows[index]
            .branches
            .iter()
            .map(|branch| branch.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(1), ["p1", "p2"]);
    assert!(names(0).is_empty() && names(2).is_empty());
    assert!(page.rows[1].branches[1].is_elsewhere);
    assert_eq!(words::moves_with_it("p1"), "p1 moves with the rebase");
    assert!(!page.is_changed(), "the branches change nothing of the rebase");
}
