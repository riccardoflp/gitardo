//! Going to a commit in the timeline (`graph.show-in-graph`) from the branch rail and the reflog, over repositories
//! made for each test: the pages loaded up to it, what narrows the timeline taken away or kept, a folded merge
//! unfolded; and what the timeline shows while it is narrowed to the history of a file.

use app::store::{Message, State};
use model::CommitId;
use model::contract::{Answer, Answers, FeatureId, Target};
use model::filter::BranchesShown;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::feature::FeatureIntent;
use views::graph::{GraphIntent, Selected};
use views::search::SearchIntent;
use views::{FilterIntent, Intent};

use super::blame::{give, three_commits};
use super::folds::merged_feature;
use super::histories::{imported, line};
use super::pointer::apply;
use super::search::{commit_of, search, selected};
use super::support::{self, OperationsDriver, Window, says, says_in};
use super::timeline::subjects;
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// Hands an intent to the store, running the reads it starts.
fn ask(state: &mut State, intent: Intent) {
    let _effects = support::run(state, Message::Intent(intent));
}

/// Runs **Show in the timeline** of the reflog on the entry of a selector, `HEAD@{1}`, and waits until it ended.
fn reflog_show(state: &mut State, selector: &str) {
    let answers = Answers::new()
        .with(features::reflog::ENTRY, Answer::Choice(selector.to_owned()))
        .with(features::reflog::ACTION, Answer::Choice(String::from("show")));
    let driver = OperationsDriver::new();
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::RunWith(
            FeatureId("reflog.browse"),
            Target::Head,
            answers,
        )),
    );
    while !matches!(driver.next(state), Message::FeatureEnded { .. }) {}
}

/// The selector of the newest entry of the reflog of `HEAD` at the commit of a subject.
fn selector_of(repository: &Repository, subject: &str) -> String {
    let reflog = repository.git(&["log", "--walk-reflogs", "--format=%gd%x09%s"]);
    reflog
        .lines()
        .find_map(|entry| entry.strip_suffix(&format!("\t{subject}")))
        .unwrap_or_else(|| panic!("no entry of the reflog at {subject}"))
        .to_owned()
}

/// Shows the history of a file of the repository in the timeline, as **Show file history** does.
fn file_history(state: &mut State, repository: &Repository, path: &str) {
    let driver = OperationsDriver::new();
    let target = Target::Path(repository.path().join(path));
    let intent = Intent::Feature(FeatureIntent::Run(FeatureId("file-history.file"), target));
    give(&driver, state, vec![intent]);
}

/// `three_commits`, with `README.md` written by a fourth commit and a stash on top.
fn three_commits_a_readme_and_a_stash() -> Repository {
    let (mut repository, _commits) = three_commits();
    let _ = repository.commit("Write the readme", &[("README.md", "Gitardo\n")]);
    std::fs::write(repository.path().join("README.md"), "Gitardo Desktop\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet"]);
    repository
}

#[test]
fn graph_show_in_graph__show_in_the_timeline_on_a_tag_3500_commits_back_loads_four_pages_and_selects_its_commit() {
    let repository = imported(&line(3_600));
    let _ = repository.git(&["tag", "v1.0", "main~3500"]);
    let mut state = showing(repository.path().to_path_buf());
    let tagged = commit_of(&repository, "v1.0");
    assert_eq!(state.timeline.pages, 1);
    assert!(!state.timeline.layout.shown_of.contains_key(&tagged), "not loaded yet");
    ask(&mut state, Intent::Graph(GraphIntent::GoTo(tagged.clone())));
    assert_eq!(state.timeline.pages, 4);
    assert_eq!(selected(&state), Some(tagged));
}

#[test]
fn graph_show_in_graph__show_in_the_timeline_on_a_branch_the_timeline_is_not_narrowed_to_shows_all_branches_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let only_feature = BranchesShown::References(vec![String::from("refs/heads/feature")]);
    ask(&mut state, Intent::Filter(FilterIntent::Show(Some(only_feature))));
    let main = commit_of(&repository, "main");
    assert!(
        !state.timeline.layout.shown_of.contains_key(&main),
        "narrowed to feature"
    );
    ask(&mut state, Intent::Graph(GraphIntent::GoTo(main.clone())));
    assert_eq!(state.timeline.shown, None);
    assert_eq!(selected(&state), Some(main));
}

#[test]
fn graph_show_in_graph__a_commit_of_the_reflog_no_branch_reaches_shows_its_history_alone_until_show_all_commits() {
    let (mut repository, _remote) = repository_with_history();
    let lost = repository.commit("Try the lanes", &[("lanes.txt", "lanes\n")]);
    let lost = CommitId::parse(&lost).unwrap();
    let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let mut state = showing(repository.path().to_path_buf());
    reflog_show(&mut state, "HEAD@{1}");
    assert_eq!(state.search.alone.as_ref(), Some(&lost));
    assert_eq!(selected(&state), Some(lost.clone()));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, views::text::search::SHOW_ALL_COMMITS);
    apply(&mut state, intents);
    assert_eq!(state.search.alone, None);
    assert!(!state.timeline.rows.iter().any(|row| row.id() == Some(&lost)));
}

#[test]
fn graph_show_in_graph__going_to_a_commit_of_the_history_of_a_file_keeps_that_history_when_the_commit_is_in_it() {
    let repository = three_commits_a_readme_and_a_stash();
    let mut state = showing(repository.path().to_path_buf());
    file_history(&mut state, &repository, "src/app.rs");
    let second = commit_of(&repository, "HEAD~2");
    ask(&mut state, Intent::Graph(GraphIntent::GoTo(second.clone())));
    assert!(state.search.history.is_some(), "the history of the file is kept");
    assert_eq!(
        subjects(&state),
        ["Add the end", "Change the second line", "Write the first lines"]
    );
    assert_eq!(selected(&state), Some(second));
}

#[test]
fn graph_fold_merge__going_to_one_of_the_folded_commits_from_the_reflog_unfolds_the_merge_and_selects_it() {
    let repository = merged_feature();
    let mut state = showing(repository.path().to_path_buf());
    let merge = commit_of(&repository, "main");
    ask(&mut state, Intent::Graph(GraphIntent::Fold(merge.clone())));
    let folded = commit_of(&repository, "feature~1");
    assert_eq!(state.timeline.layout.folded.folded_by.get(&folded), Some(&merge));
    reflog_show(&mut state, &selector_of(&repository, "Feature 2"));
    assert!(!state.timeline.folded.contains(&merge), "the merge is unfolded");
    assert_eq!(state.timeline.selected, Some(Selected::Commit(folded)));
}

#[test]
fn graph_stashes__while_the_timeline_shows_the_history_of_a_file_no_stash_is_in_it() {
    let repository = three_commits_a_readme_and_a_stash();
    let mut state = showing(repository.path().to_path_buf());
    let stash = commit_of(&repository, "stash@{0}");
    assert!(
        state.timeline.layout.shown_of.contains_key(&stash),
        "the stash is in the timeline"
    );
    file_history(&mut state, &repository, "src/app.rs");
    assert!(!state.timeline.layout.shown_of.contains_key(&stash));
    assert!(!state.timeline.rows.iter().any(|row| row.id() == Some(&stash)));
}

#[test]
fn filter_bar__the_history_of_a_file_narrowed_to_local_says_so_and_show_all_commits_leaves_the_local_branches() {
    let repository = three_commits_a_readme_and_a_stash();
    let mut state = showing(repository.path().to_path_buf());
    let local = model::filter::preset(model::filter::Preset::Local);
    ask(&mut state, Intent::Filter(FilterIntent::Show(local)));
    file_history(&mut state, &repository, "src/app.rs");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "History of src/app.rs"));
    assert!(says_in(&tree, "in the local branches"));
    let intents = window.click(&state, Role::Button, views::text::search::SHOW_ALL_COMMITS);
    apply(&mut state, intents);
    assert_eq!(state.search.history, None);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says_in(&tree, "Only the history of the local branches"));
    assert!(!says_in(&tree, "History of src/app.rs"));
}

#[test]
fn search_go_to_commit__head_3_with_the_history_of_a_file_it_did_not_change_shows_all_commits_and_selects_it() {
    let (mut repository, _commits) = three_commits();
    let _ = repository.commit("Write the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.commit("Write the guide", &[("GUIDE.md", "The timeline\n")]);
    let _ = repository.commit("Write the notes", &[("NOTES.md", "Notes\n")]);
    let _ = repository.commit("Write the licence", &[("LICENSE", "MIT\n")]);
    let mut state = showing(repository.path().to_path_buf());
    file_history(&mut state, &repository, "src/app.rs");
    let wanted = commit_of(&repository, "HEAD~3");
    assert!(
        !state.timeline.layout.shown_of.contains_key(&wanted),
        "not in the history of the file"
    );
    search(&mut state, SearchIntent::GoToCommit);
    search(&mut state, SearchIntent::GoToCommitTyped(String::from("HEAD~3")));
    search(&mut state, SearchIntent::GoToCommitSubmit);
    assert_eq!(state.search.history, None);
    assert_eq!(selected(&state), Some(wanted));
}
