//! The pull requests of GitHub when the remotes change: a remote on GitHub added, and the main remote renamed and back
//! (`remote.add`, `remote.manage`).

use app::store::State;
use engine::{GITHUB_TOKEN, SecretStore as _};
use model::contract::{Answer as AnswerGiven, FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use views::Intent;
use views::feature::FeatureIntent;

use super::super::support::{Window, has};
use super::{GitHubPlace, WIDE, pull_request_12, pull_request_of_feature};

/// An operation run from the window at `HEAD`.
fn run_at_head(feature: &'static str) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head))
}

/// The answer of a question: a text typed, or a choice picked.
fn answered(answer: AnswerGiven) -> Intent {
    Intent::Feature(FeatureIntent::Answer(Some(answer)))
}

/// The owners of the repositories GitHub was asked about, in order.
fn asked_owners(place: &GitHubPlace) -> Vec<String> {
    place
        .server
        .received()
        .iter()
        .filter_map(|received| {
            let body = json::parse(&String::from_utf8_lossy(&received.body)).ok()?;
            Some(body.get("variables")?.get("owner")?.as_str()?.to_owned())
        })
        .collect()
}

#[test]
fn remote_add__adding_origin_on_github_runs_git_remote_add_and_load_pull_requests_is_then_in_the_more_menu() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", false, Vec::new());
    let _ = place.repository.git(&["remote", "remove", "origin"]);
    let mut state = place.open();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.click(&state, Role::Button, "More");
    let (_, tree) = window.draw(&state, &[]);
    let load = "Load pull requests from GitHub";
    assert!(!has(&tree, Role::MenuItem, load), "not before");
    let add = run_at_head("remote.add");
    assert!(place.give(&mut state, vec![add]), "it asks the name");
    let name = answered(AnswerGiven::Text(String::from("origin")));
    assert!(place.give(&mut state, vec![name]), "it asks the URL");
    let url = answered(AnswerGiven::Text(String::from("https://github.com/owner/app.git")));
    assert!(!place.give(&mut state, vec![url]), "it asks nothing more");
    let added = "git remote add origin https://github.com/owner/app.git";
    assert!(state.log.entries().any(|entry| entry.line == added));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.click(&state, Role::Button, "More");
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::MenuItem, load));
}

#[test]
fn remote_manage__renaming_origin_to_github_and_back_reads_the_pull_requests_again_from_the_right_repository() {
    let answers = vec![pull_request_12(), pull_request_12(), pull_request_12()];
    let place = GitHubPlace::new("https://github.com/owner/app.git", false, answers);
    // A fork listed before origin, which is the main remote while origin has another name.
    let _ = place
        .repository
        .git(&["remote", "add", "fork", "https://github.com/ada/app.git"]);
    place.secrets.write(GITHUB_TOKEN, "kept-token").unwrap();
    let mut state = place.open();
    assert_eq!(asked_owners(&place), ["owner"]);
    let rename = |state: &mut State, from: &str, to: &str| {
        assert!(place.give(state, vec![run_at_head("remote.manage")]));
        assert!(place.give(state, vec![answered(AnswerGiven::Choice(from.to_owned()))]));
        assert!(place.give(state, vec![answered(AnswerGiven::Choice(String::from("rename")))]));
        assert!(!place.give(state, vec![answered(AnswerGiven::Text(to.to_owned()))]));
    };
    rename(&mut state, "origin", "github");
    assert_eq!(asked_owners(&place), ["owner", "ada"]);
    rename(&mut state, "github", "origin");
    assert_eq!(asked_owners(&place), ["owner", "ada", "owner"]);
    assert_eq!(pull_request_of_feature(&state), Some(12));
}
