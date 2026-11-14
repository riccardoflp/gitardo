//! Going to a commit from the search box (`search.go-to-hash`) and with **Go to commit…** (`search.go-to-commit`), over
//! repositories made for each test: wherever the commit is, what narrows the timeline taken away, a commit on no branch
//! or tag shown with its history alone.

use app::store::search::SearchJob;
use app::store::search::view::search_box;
use app::store::{Effect, JobKind, Message, reduce};
use model::CommitId;
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::search::SearchIntent;

use super::histories::{imported, line};
use super::pointer::{apply, says};
use super::search::{commit_of, search, selected};
use super::support::{Window, find};
use super::timeline::{pages_of_100, showing_with};
use super::{repository_with_history, showing};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

#[test]
fn search_go_to_hash__eight_characters_of_a_commit_4000_back_show_enter_and_enter_loads_up_to_it() {
    let repository = imported(&line(4_500));
    let mut state = showing(repository.path().to_path_buf());
    let wanted = commit_of(&repository, "main~4000");
    search(&mut state, SearchIntent::Type(wanted.as_str()[..8].to_owned()));
    assert!(search_box(&state).is_hash_offered);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, views::text::search::ENTER));
    search(&mut state, SearchIntent::Submit);
    assert_eq!(selected(&state), Some(wanted));
    assert_eq!(state.timeline.loaded(), 4_500);
}

#[test]
fn search_go_to_hash__a_commit_lost_to_a_reset_shows_its_history_alone_under_the_bar() {
    let (mut repository, _remote) = repository_with_history();
    let lost = repository.commit("Try the lanes", &[("lanes.txt", "lanes\n")]);
    let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let lost = CommitId::parse(&lost).unwrap();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::Type(lost.as_str()[..8].to_owned()));
    search(&mut state, SearchIntent::Submit);
    assert_eq!(state.search.alone.as_ref(), Some(&lost));
    assert_eq!(selected(&state), Some(lost.clone()));
    let listed: Vec<String> = super::timeline::subjects(&state);
    assert_eq!(
        listed,
        [
            "Try the lanes",
            "Show the branch rail",
            "Read the settings at startup",
            "Add the readme"
        ]
    );
    let bar = format!(
        "Only the history of {} \u{b7} on no branch or tag \u{b7} 4 commits",
        lost.short()
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, &bar), "the bar says {bar}");
    window.assert_snapshot("search-alone-dark");
    let intents = window.click(&state, Role::Button, views::text::search::SHOW_ALL_COMMITS);
    apply(&mut state, intents);
    assert_eq!(state.search.alone, None);
    assert!(!state.timeline.rows.iter().any(|row| row.id() == Some(&lost)));
}

#[test]
fn search_go_to_hash__a_commit_the_branches_shown_leave_out_shows_all_branches_again() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let only_feature = model::filter::BranchesShown::References(vec![String::from("refs/heads/feature")]);
    apply(
        &mut state,
        vec![Intent::Filter(views::FilterIntent::Show(Some(only_feature)))],
    );
    let wanted = commit_of(&repository, "main");
    assert!(!state.timeline.rows.iter().any(|row| row.id() == Some(&wanted)));
    let mut typed = wanted.as_str()[..8].to_owned();
    typed.make_ascii_uppercase();
    search(&mut state, SearchIntent::Type(typed));
    search(&mut state, SearchIntent::Submit);
    assert_eq!(state.timeline.shown, None);
    assert_eq!(state.search.alone, None);
    assert_eq!(selected(&state), Some(wanted));
}

#[test]
fn search_go_to_hash__upper_case_is_looked_for_in_lower_case() {
    let repository = imported(&line(150));
    let mut state = showing_with(repository.path().to_path_buf(), pages_of_100());
    let wanted = commit_of(&repository, "main~120");
    let typed = wanted.as_str()[..8].to_uppercase();
    search(&mut state, SearchIntent::Type(typed));
    let effects = reduce(&mut state, Message::Intent(Intent::Search(SearchIntent::Submit)));
    let asked: Vec<String> = effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Start(job) => match &job.kind {
                JobKind::Search(SearchJob::Revision { request, .. }) => Some(request.revision.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(asked, [wanted.as_str()[..8].to_owned()]);
}

#[test]
fn search_go_to_hash__three_characters_show_no_enter() {
    let repository = imported(&line(150));
    let mut state = showing_with(repository.path().to_path_buf(), pages_of_100());
    let wanted = commit_of(&repository, "main~120");
    search(&mut state, SearchIntent::Type(wanted.as_str()[..3].to_owned()));
    assert!(!search_box(&state).is_hash_offered);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, views::text::search::ENTER));
}

#[test]
fn search_go_to_hash__a_prefix_shared_by_two_commits_is_refused() {
    let repository = imported(&line(1_000));
    let mut state = showing_with(repository.path().to_path_buf(), pages_of_100());
    let unloaded = repository.git(&["rev-list", "--skip=100", "main"]);
    let mut prefixes: Vec<&str> = unloaded.lines().map(|hash| &hash[..4]).collect();
    prefixes.sort_unstable();
    let shared = prefixes
        .windows(2)
        .find(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
        .unwrap();
    let loaded = state.timeline.rows.iter().filter_map(|row| row.id());
    assert!(
        loaded.into_iter().all(|id| !id.as_str().starts_with(shared)),
        "no loaded commit has the prefix"
    );
    search(&mut state, SearchIntent::Type(shared.to_owned()));
    search(&mut state, SearchIntent::Submit);
    let refusal = format!("Gitardo: \"{shared}\" is not a commit of this repository, or is the start of several.");
    assert_eq!(state.search.failure.as_deref(), Some(refusal.as_str()));
}

#[test]
fn search_go_to_commit__main_at_yesterday_selects_the_commit_main_was_at_yesterday() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    search(&mut state, SearchIntent::GoToCommit);
    search(
        &mut state,
        SearchIntent::GoToCommitTyped(String::from("main@{yesterday}")),
    );
    let (_, tree) = window.draw(&state, &[]);
    let _dialog = find(&tree, Role::Dialog, views::text::search::GO_TO_COMMIT_TITLE);
    search(&mut state, SearchIntent::GoToCommitSubmit);
    assert_eq!(state.search.dialog, None);
    assert_eq!(selected(&state), Some(commit_of(&repository, "main@{yesterday}")));
}

#[test]
fn search_go_to_commit__nope_is_not_known_and_enter_does_nothing() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    search(&mut state, SearchIntent::GoToCommit);
    search(&mut state, SearchIntent::GoToCommitTyped(String::from("nope")));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, views::text::search::UNKNOWN_COMMIT));
    let before = state.timeline.selected.clone();
    search(&mut state, SearchIntent::GoToCommitSubmit);
    assert!(state.search.dialog.is_some());
    assert_eq!(state.timeline.selected, before);
}
