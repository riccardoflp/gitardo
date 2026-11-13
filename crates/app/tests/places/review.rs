//! The review of the resolutions of the conflicts: the page, opened from Quick search and following the files, the list
//! of the files and its filters, and accepting one conflict, a file or all of them.

use app::store::{Message, State};
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::page::Page;
use views::review::{ConflictCard, ConflictKey, Filter, ReviewIntent};
use views::search::{QuickChoice, QuickCommand, SearchIntent};

use super::showing;
use super::support::{Window, has, run, says};

/// A file's path and its texts in the base, on `main` and on `feature`; a text of `None` deletes it on that side.
pub type Texts<'a> = (&'a str, &'a str, Option<&'a str>, Option<&'a str>);

/// Two lines next to each other changed one on each side: the rules propose to keep both changes.
pub const NEXT_TO_EACH_OTHER: (&str, &str, &str) = ("a\nb\nc\nd\n", "a\nB\nc\nd\n", "a\nb\nC\nd\n");
/// The same line changed in two ways: nothing is proposed.
pub const SAME_LINE: (&str, &str, &str) = ("a\nb\nc\n", "a\nB1\nc\n", "a\nB2\nc\n");

/// A repository whose merge of `feature` into `main` stopped on conflicts in some files.
pub fn merged(files: &[Texts<'_>]) -> Repository {
    let mut repository = Repository::new();
    let base: Vec<(&str, &str)> = files.iter().map(|(path, base, _, _)| (*path, *base)).collect();
    let _base = repository.commit("Add the files", &base);
    repository.branch("feature");
    side(&mut repository, files, |file| file.2, "Change them on main");
    repository.checkout("feature");
    side(&mut repository, files, |file| file.3, "Change them on feature");
    repository.checkout("main");
    let _status = repository
        .command("git")
        .args(["merge", "--no-ff", "--no-edit", "feature"])
        .output()
        .unwrap();
    repository
}

/// Commits the texts of one side, deleting the files it has none of.
fn side<'a>(
    repository: &mut Repository,
    files: &[Texts<'a>],
    text: impl Fn(&Texts<'a>) -> Option<&'a str>,
    message: &str,
) {
    let kept: Vec<(&str, &str)> = files
        .iter()
        .filter_map(|file| text(file).map(|text| (file.0, text)))
        .collect();
    for file in files.iter().filter(|file| text(file).is_none()) {
        let _ = repository.git(&["rm", "--quiet", "--", file.0]);
    }
    if kept.is_empty() {
        let _ = repository.git(&["commit", "--quiet", "-m", message]);
    } else {
        let _ = repository.commit(message, &kept);
    }
}

/// A file of texts each repeated, joined: `(path, base, main, feature)`.
pub fn file<'a>(path: &'a str, (base, main, feature): (&'a str, &'a str, &'a str)) -> Texts<'a> {
    (path, base, Some(main), Some(feature))
}

/// A window showing the repository, with the rules switched on, and the review page opened from Quick search.
pub fn reviewed(repository: &Repository) -> State {
    let mut state = showing(repository.path().to_path_buf());
    state.settings.features.conflicts_auto_resolve = true;
    open(&mut state);
    state
}

/// Chooses **Review the resolutions of the conflicts** in Quick search.
pub fn open(state: &mut State) {
    let choose = SearchIntent::ChooseQuick(QuickChoice::Command(QuickCommand::ReviewConflicts));
    let _effects = run(state, Message::Intent(Intent::Search(choose)));
}

/// Gives the page an intent, with the jobs it starts.
pub fn review(state: &mut State, intent: ReviewIntent) {
    let _effects = run(state, Message::Intent(Intent::Review(intent)));
}

/// Every card of the page.
pub fn cards(state: &State) -> Vec<ConflictCard> {
    let page = state.reviews.page.as_ref().unwrap();
    page.cards.iter().flat_map(|file| file.conflicts.clone()).collect()
}

/// The key of the card of a file at a place among its conflicts.
pub fn key(state: &State, path: &str, index: usize) -> ConflictKey {
    cards(state)
        .into_iter()
        .find(|card| card.key.path == path && card.key.index == index)
        .unwrap_or_else(|| panic!("no conflict {index} in {path}"))
        .key
}

/// The window drawn wide enough for the list beside the cards.
pub fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, (1400, 1000)).draw(state, &[]).1
}

/// Reads the repository again, as the watcher would once a file changed.
pub fn read_again(state: &mut State) {
    let _effects = run(state, Message::Intent(Intent::Refresh));
}

#[test]
fn review_page__opening_it_during_a_merge_shows_every_conflict_of_both_files() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", SAME_LINE)]);
    let state = reviewed(&repository);
    assert_eq!(state.page_shown(), Some(Page::Review));
    let folder = repository.path().file_name().unwrap().to_string_lossy().into_owned();
    let tree = drawn(&state);
    assert!(says(&tree, &format!("Conflicts \u{b7} {folder}")));
    assert!(says(&tree, "Conflicts of the merge"));
    let paths: Vec<String> = cards(&state).into_iter().map(|card| card.key.path).collect();
    assert_eq!(paths, ["one.txt", "two.txt"]);
    assert!(says(&tree, "Conflict at line 2"));
}

#[test]
fn review_page__staging_a_file_from_the_changes_page_removes_its_cards() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER), file("two.txt", SAME_LINE)]);
    let mut state = reviewed(&repository);
    std::fs::write(repository.path().join("two.txt"), "a\nB1\nc\n").unwrap();
    let _ = repository.git(&["add", "--", "two.txt"]);
    read_again(&mut state);
    let paths: Vec<String> = cards(&state).into_iter().map(|card| card.key.path).collect();
    assert_eq!(paths, ["one.txt"]);
}

#[test]
fn review_page__editing_one_conflict_by_hand_keeps_the_acceptance_of_the_others() {
    let base = "a\nb\nc\nd\n1\n2\n3\n4\n5\n6\nw\nx\ny\nz\n";
    let main = "a\nB\nc\nd\n1\n2\n3\n4\n5\n6\nw\nX\ny\nz\n";
    let feature = "a\nb\nC\nd\n1\n2\n3\n4\n5\n6\nw\nx\nY\nz\n";
    let repository = merged(&[file("one.txt", (base, main, feature))]);
    let mut state = reviewed(&repository);
    for index in 0..2 {
        let key = key(&state, "one.txt", index);
        review(&mut state, ReviewIntent::Accept(key));
    }
    let on_disk = std::fs::read_to_string(repository.path().join("one.txt")).unwrap();
    let edited = on_disk.replacen("X\n", "X by hand\n", 1);
    assert_ne!(edited, on_disk);
    std::fs::write(repository.path().join("one.txt"), edited).unwrap();
    read_again(&mut state);
    let accepted: Vec<bool> = cards(&state).iter().map(|card| card.is_accepted).collect();
    assert_eq!(accepted, [true, false]);
}

#[test]
fn review_page__with_both_settings_off_the_command_in_quick_search_refuses() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = showing(repository.path().to_path_buf());
    open(&mut state);
    assert_ne!(state.page_shown(), Some(Page::Review));
    let said = state.operations.said.as_ref().unwrap();
    assert_eq!(
        said.text,
        "Resolving conflicts automatically is experimental, and it is switched off in the settings."
    );
}

/// Four conflicts in two files, three proposed by the rules and one not.
fn four_conflicts() -> Repository {
    let base = "a\nb\nc\nd\n1\n2\n3\n4\n5\n6\nw\nx\ny\nz\n";
    let main = "a\nB\nc\nd\n1\n2\n3\n4\n5\n6\nw\nX\ny\nz\n";
    let feature = "a\nb\nC\nd\n1\n2\n3\n4\n5\n6\nw\nx\nY\nz\n";
    merged(&[
        file("one.txt", (base, main, feature)),
        file("two.txt", NEXT_TO_EACH_OTHER),
        file("three.txt", SAME_LINE),
    ])
}

#[test]
fn review_file_list__the_filters_count_the_conflicts_they_keep() {
    let repository = four_conflicts();
    let mut state = reviewed(&repository);
    assert_eq!(cards(&state).len(), 4);
    for index in 0..2 {
        let key = key(&state, "one.txt", index);
        review(&mut state, ReviewIntent::Accept(key));
    }
    let tree = drawn(&state);
    for name in ["All 4", "Not accepted 2", "Nothing proposed 1", "From the AI 0"] {
        assert!(has(&tree, Role::Button, name), "{name}");
    }
}

#[test]
fn review_file_list__a_conflict_accepted_under_not_accepted_stays_folded_until_another_filter() {
    let repository = four_conflicts();
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::Filter(Filter::NotAccepted));
    let first = key(&state, "two.txt", 0);
    review(&mut state, ReviewIntent::Accept(first.clone()));
    let shown = |state: &State| {
        let page = app::store::review::page_state(state).unwrap();
        cards(state)
            .iter()
            .filter(|card| page.is_shown(card))
            .map(|card| card.key.clone())
            .collect::<Vec<_>>()
    };
    assert!(shown(&state).contains(&first));
    assert!(
        cards(&state)
            .iter()
            .any(|card| card.key == first && card.shown.is_folded)
    );
    review(&mut state, ReviewIntent::Filter(Filter::All));
    review(&mut state, ReviewIntent::Filter(Filter::NotAccepted));
    assert!(!shown(&state).contains(&first));
}

#[test]
fn review_file_list__a_binary_file_is_listed_under_also_in_conflict_and_has_no_card() {
    let repository = merged(&[
        file("one.txt", NEXT_TO_EACH_OTHER),
        ("image.bin", "a\0b\n", Some("a\0B\n"), Some("a\0C\n")),
    ]);
    let state = reviewed(&repository);
    let tree = drawn(&state);
    assert!(says(&tree, "Also in conflict"));
    assert!(says(&tree, "image.bin"));
    assert!(cards(&state).iter().all(|card| card.key.path != "image.bin"));
}

#[test]
fn review_accept__accept_marks_the_card_accepted_folds_it_and_counts_it() {
    let repository = three_in_one();
    let mut state = reviewed(&repository);
    let mut window = Window::new(ThemeName::Dark, (1400, 1000));
    assert!(has(&window.draw(&state, &[]).1, Role::Button, "Write the accepted"));
    for intent in window.click(&state, Role::Button, "Accept") {
        let _effects = run(&mut state, Message::Intent(intent));
    }
    let card = cards(&state).remove(0);
    assert!(card.is_accepted && card.shown.is_folded);
    let tree = window.draw(&state, &[]).1;
    assert!(has(&tree, Role::Button, "Accepted"));
    assert!(has(&tree, Role::Button, "Write 1 resolution"));
}

#[test]
fn review_accept__accepted_takes_it_back_and_unfolds_it() {
    let repository = three_in_one();
    let mut state = reviewed(&repository);
    let key = key(&state, "one.txt", 0);
    review(&mut state, ReviewIntent::Accept(key));
    let mut window = Window::new(ThemeName::Dark, (1400, 1000));
    let _ = window.draw(&state, &[]);
    for intent in window.click(&state, Role::Button, "Accepted") {
        let _effects = run(&mut state, Message::Intent(intent));
    }
    let card = cards(&state).remove(0);
    assert!(!card.is_accepted && !card.shown.is_folded);
    assert!(has(&window.draw(&state, &[]).1, Role::Button, "Write the accepted"));
}

#[test]
fn review_accept__a_conflict_with_nothing_proposed_has_no_accept() {
    let repository = merged(&[file("one.txt", SAME_LINE)]);
    let state = reviewed(&repository);
    let tree = drawn(&state);
    assert!(says(&tree, "Nothing proposed yet"));
    assert!(!has(&tree, Role::Button, "Accept"));
}

/// One file of three conflicts, the last of which has nothing proposed.
fn three_in_one() -> Repository {
    let base = "a\nb\nc\nd\n1\n2\n3\n4\nw\nx\ny\nz\n5\n6\n7\n8\nk\nl\nm\n";
    let main = "a\nB\nc\nd\n1\n2\n3\n4\nw\nX\ny\nz\n5\n6\n7\n8\nk\nL1\nm\n";
    let feature = "a\nb\nC\nd\n1\n2\n3\n4\nw\nx\nY\nz\n5\n6\n7\n8\nk\nL2\nm\n";
    merged(&[file("one.txt", (base, main, feature))])
}

#[test]
fn review_accept_file__accepts_the_proposals_of_the_file_and_leaves_the_rest() {
    let repository = three_in_one();
    let mut state = reviewed(&repository);
    assert_eq!(cards(&state).len(), 3);
    review(&mut state, ReviewIntent::AcceptFile(String::from("one.txt")));
    let accepted: Vec<bool> = cards(&state).iter().map(|card| card.is_accepted).collect();
    assert_eq!(accepted, [true, true, false]);
    let page = app::store::review::page_state(&state).unwrap();
    let file = page.files.first().unwrap();
    assert_eq!(
        (file.accepted(), file.has_unproposed(), file.is_folded),
        (2, true, false)
    );
    assert!(says(&drawn(&state), "2 of 3"));
}

#[test]
fn review_accept_file__a_file_whose_conflicts_are_all_accepted_folds_to_its_name() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptFile(String::from("one.txt")));
    let page = state.reviews.page.as_ref().unwrap();
    assert!(page.cards.first().unwrap().is_folded);
    assert!(!says(&drawn(&state), "Conflict at line 2"));
}

#[test]
fn review_accept_all__accepts_every_proposal_and_is_then_disabled() {
    let repository = four_conflicts();
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptAll);
    let accepted = cards(&state).iter().filter(|card| card.is_accepted).count();
    assert_eq!(accepted, 3);
    let page = app::store::review::page_state(&state).unwrap();
    assert!(!page.conflicts().any(ConflictCard::is_acceptable_with_others));
}

#[test]
fn review_accept_all__conflicts_with_nothing_proposed_stay_unaccepted() {
    let repository = four_conflicts();
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptAll);
    let left: Vec<String> = cards(&state)
        .into_iter()
        .filter(|card| !card.is_accepted)
        .map(|card| card.key.path)
        .collect();
    assert_eq!(left, ["three.txt"]);
}

#[test]
fn review_clear__takes_back_every_acceptance_and_disables_the_write() {
    let repository = four_conflicts();
    let mut state = reviewed(&repository);
    review(&mut state, ReviewIntent::AcceptAll);
    review(&mut state, ReviewIntent::Clear);
    assert!(
        cards(&state)
            .iter()
            .all(|card| !card.is_accepted && !card.shown.is_folded)
    );
    let tree = drawn(&state);
    assert!(has(&tree, Role::Button, "Write the accepted"));
}
