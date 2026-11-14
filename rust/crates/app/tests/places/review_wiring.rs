//! What the review of the conflicts hands over to the rest of the window: the line number of a card and the files
//! also in conflict, opened where they can be resolved; **Resolve by hand**, which opens the Merge page at the
//! conflict; **Keep it** and **Delete it** on a file deleted on one side; and **Undo** and **Continue** in the report.

use app::store::{Message, State};
use model::merge_view::RegionKind;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::conflicts::ConflictsIntent;
use views::merge_view::MergeIntent;
use views::page::Page;
use views::review::ReviewIntent;

use super::compare::until_ended;
use super::review::{NEXT_TO_EACH_OTHER, SAME_LINE, cards, file, merged, review, reviewed};
use super::support::{OperationsDriver, Window, has, run, says};

/// A window wide enough for the list beside the cards.
fn window() -> Window {
    Window::new(ThemeName::Dark, (1400, 1000))
}

/// Gives each intent to the store, with the jobs it starts.
fn reduce_all(state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        let _effects = run(state, Message::Intent(intent));
    }
}

/// Gives each intent to a thread of the operations of its own, and confirms what it shows until it ends.
fn to_the_end(state: &mut State, intents: Vec<Intent>) {
    to_the_end_with(&OperationsDriver::new(), state, intents);
}

/// Gives each intent to the thread of the operations, which keeps the journal of Undo, and confirms what it shows until
/// it ends.
fn to_the_end_with(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
    until_ended(driver, state);
}

/// The git commands the operations ran, as the Command log has them.
fn logged(state: &State) -> Vec<String> {
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

/// A merge stopped on `added.txt`, which both sides added with the same first three lines and a different fourth.
fn both_added() -> Repository {
    let mut repository = Repository::new();
    let _base = repository.commit("Add the readme", &[("readme.txt", "readme\n")]);
    repository.branch("feature");
    let _main = repository.commit("Add the file on main", &[("added.txt", "one\ntwo\nthree\nmain\n")]);
    repository.checkout("feature");
    let _feature = repository.commit(
        "Add the file on feature",
        &[("added.txt", "one\ntwo\nthree\nfeature\n")],
    );
    repository.checkout("main");
    let _status = repository
        .command("git")
        .args(["merge", "--no-edit", "feature"])
        .output()
        .unwrap();
    repository
}

/// The File page shown: its path, and the line it was brought to, from 1.
fn file_page_shown(state: &State) -> (String, Option<usize>) {
    let Some(Page::File(id)) = state.page_shown() else {
        panic!("no File page is shown: {:?}", state.page_shown());
    };
    let page = state.file_pages.pages.iter().find(|page| page.id == id).unwrap();
    (page.path.clone(), page.reveal.map(|(_, line)| line + 1))
}

#[test]
fn conflicts_open_file__a_file_both_sides_added_opens_in_the_file_page_with_the_markers() {
    let repository = both_added();
    let mut state = super::showing(repository.path().to_path_buf());
    reduce_all(
        &mut state,
        vec![Intent::Conflicts(ConflictsIntent::Open(String::from("added.txt")))],
    );
    let (path, line) = file_page_shown(&state);
    assert_eq!((path.as_str(), line), ("added.txt", Some(1)));
    let page = state.file_pages.pages.first().unwrap();
    let text = page.document().unwrap().buffer().text().to_owned();
    assert!(text.contains("<<<<<<<"), "{text}");
}

#[test]
fn conflicts_open_file__from_a_review_card_a_file_both_sides_added_opens_at_the_line_of_the_conflict() {
    let repository = both_added();
    let mut state = reviewed(&repository);
    let card = cards(&state).remove(0);
    assert_eq!((card.key.path.as_str(), card.line), ("added.txt", 4));
    let intents = window().click(&state, Role::Button, "Conflict at line 4");
    assert_eq!(
        intents,
        [Intent::Conflicts(ConflictsIntent::OpenAtLine(
            String::from("added.txt"),
            4
        ))]
    );
    reduce_all(&mut state, intents);
    assert_eq!(file_page_shown(&state), (String::from("added.txt"), Some(4)));
}

#[test]
fn review_file_list__a_file_also_in_conflict_opens_where_it_can_be_resolved() {
    let repository = merged(&[
        file("one.txt", NEXT_TO_EACH_OTHER),
        ("image.bin", "a\0b\n", Some("a\0B\n"), Some("a\0C\n")),
    ]);
    let state = reviewed(&repository);
    let intents = window().click(&state, Role::Button, "image.bin");
    assert_eq!(
        intents,
        [Intent::Conflicts(ConflictsIntent::Open(String::from("image.bin")))]
    );
}

/// One file of three conflicts, the last of which has nothing proposed.
fn three_in_one() -> Repository {
    let base = "a\nb\nc\nd\n1\n2\n3\n4\nw\nx\ny\nz\n5\n6\n7\n8\nk\nl\nm\n";
    let main = "a\nB\nc\nd\n1\n2\n3\n4\nw\nX\ny\nz\n5\n6\n7\n8\nk\nL1\nm\n";
    let feature = "a\nb\nC\nd\n1\n2\n3\n4\nw\nx\nY\nz\n5\n6\n7\n8\nk\nL2\nm\n";
    merged(&[file("one.txt", (base, main, feature))])
}

#[test]
fn review_card__resolve_by_hand_opens_the_merge_page_of_the_file_scrolled_to_that_conflict() {
    let repository = three_in_one();
    let mut state = reviewed(&repository);
    let first = cards(&state).remove(0);
    let intents = window().click(&state, Role::Button, "Resolve by hand");
    assert_eq!(
        intents,
        [Intent::MergeView(MergeIntent::OpenAt(
            String::from("one.txt"),
            first.line
        ))]
    );
    let second = cards(&state).remove(1);
    reduce_all(
        &mut state,
        vec![Intent::MergeView(MergeIntent::OpenAt(
            second.key.path.clone(),
            second.line,
        ))],
    );
    let Some(Page::Merge(id)) = state.page_shown() else {
        panic!("no Merge page is shown: {:?}", state.page_shown());
    };
    let page = state.merge_pages.pages.iter().find(|page| page.id == id).unwrap();
    let conflicts: Vec<usize> = page
        .text
        .as_ref()
        .unwrap()
        .regions
        .iter()
        .enumerate()
        .filter(|(_, region)| region.kind == RegionKind::Conflict)
        .map(|(place, _)| place)
        .collect();
    assert_eq!(conflicts.len(), 3);
    assert_eq!(page.current, conflicts.get(1).copied());
    let rows = conflicts
        .get(1)
        .and_then(|place| page.layout.regions.get(*place))
        .unwrap();
    assert_eq!(page.reveal.map(|(_, row)| row), Some(rows.start));
}

/// A file deleted on `feature` and changed on `main`.
fn deleted() -> Repository {
    merged(&[("gone.txt", "one\ntwo\nthree\n", Some("one\nTWO\nthree\n"), None)])
}

#[test]
fn review_deleted_card__delete_it_asks_and_runs_git_rm() {
    let repository = deleted();
    let mut state = reviewed(&repository);
    let intents = window().click(&state, Role::Button, "Delete it");
    assert_eq!(
        intents,
        [Intent::Review(ReviewIntent::TakeSide(
            String::from("gone.txt"),
            model::review::Which::Incoming
        ))]
    );
    to_the_end(&mut state, intents);
    assert_eq!(logged(&state), ["git rm -- gone.txt"]);
    assert!(!repository.path().join("gone.txt").exists());
}

#[test]
fn review_deleted_card__keep_it_asks_and_runs_checkout_ours_and_add() {
    let repository = deleted();
    let mut state = reviewed(&repository);
    let intents = window().click(&state, Role::Button, "Keep it, with these changes");
    to_the_end(&mut state, intents);
    assert_eq!(
        logged(&state),
        ["git checkout --ours -- gone.txt", "git add -- gone.txt"]
    );
    assert_eq!(repository.git(&["show", ":gone.txt"]), "one\nTWO\nthree\n");
}

#[test]
fn review_deleted_card__the_button_beside_them_opens_the_base_beside_the_file() {
    let repository = deleted();
    let state = reviewed(&repository);
    let intents = window().click(&state, Role::Button, "Show what it was beside what it is now");
    assert_eq!(
        intents,
        [Intent::Conflicts(ConflictsIntent::Open(String::from("gone.txt")))]
    );
}

/// Writes what is accepted from the page, confirming what is asked, and gives the thread of the operations that ran it.
fn write_accepted(state: &mut State) -> OperationsDriver {
    review(state, ReviewIntent::AcceptAll);
    let driver = OperationsDriver::new();
    to_the_end_with(&driver, state, vec![Intent::Review(ReviewIntent::Write)]);
    driver
}

#[test]
fn review_report__after_writing_every_conflict_of_a_merge_continue_the_merge_runs_merge_continue() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    let _driver = write_accepted(&mut state);
    let mut window = window();
    let tree = window.draw(&state, &[]).1;
    assert!(says(&tree, "No file is left in conflict: the merge can continue."));
    let intents = window.click(&state, Role::Button, "Continue the merge");
    assert_eq!(intents, [Intent::Review(ReviewIntent::Continue)]);
    to_the_end(&mut state, intents);
    assert_eq!(
        logged(&state).last().map(String::as_str),
        Some("git -c core.editor=true merge --continue")
    );
    assert_eq!(repository.git(&["rev-list", "--count", "--merges", "HEAD"]).trim(), "1");
}

#[test]
fn review_report__with_a_file_left_in_conflict_continue_is_not_offered() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    let _driver = write_accepted(&mut state);
    let tree = window().draw(&state, &[]).1;
    assert!(says(&tree, "1 file is still in conflict."));
    assert!(!has(&tree, Role::Button, "Continue the merge"));
}

#[test]
fn review_report__undo_runs_conflicts_restore_of_the_files_written() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    let driver = write_accepted(&mut state);
    assert!(has(&window().draw(&state, &[]).1, Role::Button, "Undo"));
    to_the_end_with(&driver, &mut state, vec![Intent::Review(ReviewIntent::Undo)]);
    assert_eq!(
        logged(&state).last().map(String::as_str),
        Some("git checkout --merge -- one.txt")
    );
    let on_disk = std::fs::read_to_string(repository.path().join("one.txt")).unwrap();
    assert!(on_disk.contains("<<<<<<<"), "{on_disk}");
}
