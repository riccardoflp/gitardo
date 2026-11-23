//! The notice of git's commit-graph file in the status bar (`upkeep.offer-commit-graph`), over histories of 10,000
//! commits made by `git fast-import`, which writes no commit-graph file. The operation **Write it** runs is checked by
//! the tests of `gitardo-cli`; here git writes the file as it would.

use std::path::Path;

use app::persisted::Persisted;
use app::store::{Message, State, ViewData};
use model::contract::{FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::text::repo as words;

use super::histories::{imported, line};
use super::support::{Window, run, says};

/// A window wide enough for the notice and its buttons in the status bar.
const SIZE: (u32, u32) = (1400, 800);

/// A window that started before, with what is kept between runs, that opened a folder and read its whole history.
fn opened(folder: &Path, persisted: Persisted) -> State {
    let mut state = State::new(
        Persisted {
            has_started: true,
            ..persisted
        },
        app::settings::Settings::default(),
        None,
    );
    let _ = run(
        &mut state,
        Message::Started {
            folders: vec![folder.to_path_buf()],
        },
    );
    state
}

/// The sentence of the notice for the repository a window shows.
fn notice_of(state: &State) -> String {
    let data = ViewData::new(state, Vec::new(), 0);
    words::commit_graph_notice(data.name().unwrap())
}

/// Whether the window says the notice in its status bar.
fn says_notice(window: &mut Window, state: &State) -> bool {
    let (_, tree) = window.draw(state, &[]);
    says(&tree, &notice_of(state))
}

#[test]
fn upkeep_offer_commit_graph__ten_thousand_commits_and_no_file_say_the_notice_once_and_write_it_writes_the_file() {
    let repository = imported(&line(10_000));
    let mut state = opened(repository.path(), Persisted::default());
    assert_eq!(state.timeline.loaded(), 10_000);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    assert!(
        says_notice(&mut window, &state),
        "the notice is said once the history is read"
    );
    let intents = window.click(&state, Role::Button, words::WRITE_COMMIT_GRAPH);
    let write = Intent::Feature(FeatureIntent::Run(FeatureId("upkeep.write-commit-graph"), Target::Head));
    assert!(intents.contains(&write), "{intents:?}");
    assert!(intents.contains(&Intent::DismissCommitGraph), "{intents:?}");
    let _ = run(&mut state, Message::Intent(Intent::DismissCommitGraph));
    assert!(!says_notice(&mut window, &state));
    let _ = repository.git(&["commit-graph", "write", "--reachable", "--changed-paths"]);
    let _ = run(&mut state, Message::Intent(Intent::Refresh));
    assert!(state.shown_read().unwrap().state.commit_graph.has_file);
    assert!(!says_notice(&mut window, &state), "read again, it does not come again");
    let again = opened(repository.path(), Persisted::default());
    assert_eq!(again.commit_graph.notice, None, "nor in a window opened after");
}

#[test]
fn upkeep_offer_commit_graph__not_for_this_repository_is_kept_and_opening_it_again_says_nothing() {
    let repository = imported(&line(10_000));
    let mut state = opened(repository.path(), Persisted::default());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    assert!(says_notice(&mut window, &state));
    let intents = window.click(&state, Role::Button, words::NOT_FOR_THIS_REPOSITORY);
    assert_eq!(intents, [Intent::DeclineCommitGraph]);
    let effects = run(&mut state, Message::Intent(Intent::DeclineCommitGraph));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, app::store::Effect::WritePersisted(_))),
        "{effects:?}"
    );
    assert!(!says_notice(&mut window, &state));
    let kept = Persisted::from_text(&state.persisted.to_text());
    assert_eq!(kept.commit_graph_declined.len(), 1, "state.json keeps it");
    let again = opened(repository.path(), kept);
    assert_eq!(again.timeline.loaded(), 10_000);
    assert_eq!(again.commit_graph.notice, None);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    assert!(!says_notice(&mut window, &again));
}

#[test]
fn upkeep_offer_commit_graph__fewer_commits_a_file_or_core_commit_graph_false_say_nothing() {
    let mut repository: Repository = imported(&line(9_999));
    let said = |repository: &Repository| opened(repository.path(), Persisted::default()).commit_graph.notice;
    assert_eq!(said(&repository), None, "9,999 commits");
    let _ = repository.commit("Change 10000", &[("src/last.txt", "last\n")]);
    let _ = repository.git(&["config", "core.commitGraph", "false"]);
    assert_eq!(said(&repository), None, "core.commitGraph is false");
    let _ = repository.git(&["config", "--unset", "core.commitGraph"]);
    let _ = repository.git(&["commit-graph", "write", "--reachable"]);
    assert_eq!(said(&repository), None, "git keeps the file");
    std::fs::remove_file(repository.path().join(".git/objects/info/commit-graph")).unwrap();
    assert!(said(&repository).is_some(), "10,000 commits and no file");
}
