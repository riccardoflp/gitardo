//! The pull requests of GitHub in the window (`github.load-pull-requests`): loaded from the **…** menu or Quick search,
//! read again after a restart with the token kept, and a token GitHub refuses taken out of the store. GitHub is a server
//! of the test on 127.0.0.1, the credential store is in memory, and git's credential helper is the test's own, in a
//! throwaway repository whose git reads no configuration of the user's.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use app::store::{Effect, Message, State};
use app::work::github::{GitHubAccess, Network};
use engine::{GITHUB_TOKEN, MemorySecrets, SecretStore as _};
use http::test_server::{Answer, TestServer, answer};
use http::{Http, Proxies};
use model::contract::{Answer as AnswerGiven, FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::search::{QuickChoice, QuickCommand, SearchIntent};

use super::support::{OperationsDriver, Window, has, run};
use super::window;

/// What the helper of the test prints for `get`, after writing a line in a file of the test, which tells it ran.
const HELPER: &str =
    "!f() { echo ran >> \"$GITARDO_TEST_MARKER\"; echo username=ada; echo password=token-from-helper; }; f";

/// The size of the window: tall enough for the whole **…** menu.
const WIDE: (u32, u32) = (1000, 1100);

/// A window wide enough for the label of `feature` and its badge beside `main`, on the line under the message.
const BADGES: (u32, u32) = (1200, 300);

/// A repository whose main remote is on GitHub, or elsewhere, with the server, the store and the git of the test.
struct GitHubPlace {
    /// The repository, with a branch `feature` whose upstream is `origin/feature`.
    repository: Repository,
    /// The server that answers for `api.github.com`.
    server: TestServer,
    /// The credential store.
    secrets: MemorySecrets,
    /// Where the pull requests are read from and the token kept.
    access: GitHubAccess,
    /// The thread of the operations.
    driver: OperationsDriver,
    /// The file the helper writes when it runs.
    marker: PathBuf,
}

impl GitHubPlace {
    /// A repository with `origin` at a URL, git's credential helper the test's own or none, and GitHub answering each
    /// request in turn.
    fn new(url: &str, has_helper: bool, answers: Vec<Answer>) -> GitHubPlace {
        let mut repository = Repository::new();
        let _ = repository.commit("Add a", &[("a.txt", "a\n")]);
        let _ = repository.git(&["remote", "add", "origin", url]);
        let _ = repository.git(&["branch", "feature"]);
        let _ = repository.git(&["update-ref", "refs/remotes/origin/feature", "HEAD"]);
        let _ = repository.git(&["branch", "--set-upstream-to=origin/feature", "feature"]);
        // An empty helper first forgets the helpers of other configurations; the test's own comes after it.
        let _ = repository.git(&["config", "credential.helper", ""]);
        if has_helper {
            let _ = repository.git(&["config", "--add", "credential.helper", HELPER]);
        }
        let git_folder = repository.path().join(".git");
        let global = git_folder.join("test-global-config");
        std::fs::write(&global, "").unwrap();
        let marker = git_folder.join("helper-ran");
        let server = TestServer::secure(answers);
        let secrets = MemorySecrets::default();
        let kept = secrets.clone();
        let http =
            Http::new(TestServer::trust(), Proxies::none()).with_address("api.github.com", 443, server.address());
        let access = GitHubAccess {
            network: Network::Given(http),
            secrets: Arc::new(move || Box::new(kept.clone())),
            git_variables: vec![
                (OsString::from("GIT_CONFIG_NOSYSTEM"), OsString::from("1")),
                (OsString::from("GIT_CONFIG_GLOBAL"), global.into_os_string()),
                (OsString::from("GITARDO_TEST_MARKER"), marker.clone().into_os_string()),
            ],
            copilot: engine::ai::CopilotCli::system(),
        };
        GitHubPlace {
            repository,
            server,
            secrets,
            driver: OperationsDriver::with_access(access.clone()),
            access,
            marker,
        }
    }

    /// The window opened on the repository, with the repository on GitHub looked for and what follows from it.
    fn open(&self) -> State {
        let mut state = window();
        let folders = vec![self.repository.path().to_path_buf()];
        let effects = run(&mut state, Message::Started { folders });
        self.serve(&mut state, &effects);
        state
    }

    /// Runs the jobs of the pull requests among the effects, at once on the thread of the test, and those they start.
    fn serve(&self, state: &mut State, effects: &[Effect]) {
        let mut waiting: Vec<_> = github_jobs(effects);
        while let Some(job) = waiting.pop() {
            let effects = run(state, app::work::github::run(&self.access, &job));
            self.driver.take(state, &effects);
            waiting.extend(github_jobs(&effects));
        }
    }

    /// Reduces intents, gives the thread of the operations what they ask of it, then waits until the operation asks
    /// a question or ends: whether it asked.
    fn give(&self, state: &mut State, intents: Vec<Intent>) -> bool {
        for intent in intents {
            let effects = run(state, Message::Intent(intent));
            self.driver.take(state, &effects);
            self.serve(state, &effects);
        }
        while state.is_operation_running {
            let message = self.driver.raw_next();
            let asks = matches!(message, Message::FeatureAsks(_));
            let effects = run(state, message);
            self.driver.take(state, &effects);
            self.serve(state, &effects);
            if asks {
                return true;
            }
        }
        false
    }

    /// Chooses **Load pull requests from GitHub** in the **…** menu: whether it asked for a token.
    fn load_from_menu(&self, state: &mut State) -> bool {
        let mut window = Window::new(ThemeName::Dark, WIDE);
        let opened = window.click(state, Role::Button, "More");
        let _ = self.give(state, opened);
        let chosen = window.click(state, Role::MenuItem, "Load pull requests from GitHub");
        self.give(state, chosen)
    }

    /// What GitHub was sent, by its `Authorization` header.
    fn sent_with(&self) -> Vec<String> {
        self.server
            .received()
            .iter()
            .map(|received| received.header("Authorization").unwrap_or_default())
            .collect()
    }

    /// Whether git's credential helper ran.
    fn helper_ran(&self) -> bool {
        self.marker.exists()
    }
}

/// The jobs of the pull requests among effects.
fn github_jobs(effects: &[Effect]) -> Vec<app::store::github::GitHubJob> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::GitHub(job) => Some(job.clone()),
            _ => None,
        })
        .collect()
}

/// GitHub's answer with pull request #12 of `feature`, whose checks failed.
fn pull_request_12() -> Answer {
    let body = "{\"data\": {\"repository\": {\"pullRequests\": {\"nodes\": [{\"number\": 12, \"title\": \"Add the parser\", \
                \"url\": \"https://github.com/owner/app/pull/12\", \"isDraft\": false, \"headRefName\": \"feature\", \
                \"headRepositoryOwner\": {\"login\": \"owner\"}, \"commits\": {\"nodes\": [{\"commit\": \
                {\"statusCheckRollup\": {\"state\": \"FAILURE\"}}}]}}]}}}}";
    Answer::Bytes(answer(
        "200 OK",
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    ))
}

/// GitHub refusing the token.
fn unauthorized() -> Answer {
    Answer::Bytes(answer("401 Unauthorized", &[], b"{\"message\": \"Bad credentials\"}"))
}

/// The number of the pull request of `feature`, as the window knows it.
fn pull_request_of_feature(state: &State) -> Option<u64> {
    let pull_requests = state.github.pull_requests.as_ref()?;
    pull_requests
        .of_branch("feature")
        .map(|pull_request| pull_request.number)
}

#[test]
fn github_load_pull_requests__nothing_is_sent_and_no_helper_runs_before_it_is_chosen_once() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", true, vec![pull_request_12()]);
    let mut state = place.open();
    let effects = run(&mut state, Message::Intent(Intent::Refresh));
    place.serve(&mut state, &effects);
    assert_eq!(place.sent_with(), [] as [String; 0]);
    assert!(!place.helper_ran(), "git credential fill does not run");
    assert_eq!(
        state
            .github
            .repository
            .as_ref()
            .map(|repository| repository.repo.as_str()),
        Some("app")
    );
}

#[test]
fn github_load_pull_requests__with_a_helper_that_knows_github_com_it_asks_nothing_and_the_pull_requests_show() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", true, vec![pull_request_12()]);
    let mut state = place.open();
    let asked = place.load_from_menu(&mut state);
    assert!(!asked, "nothing is asked");
    assert!(place.helper_ran());
    assert_eq!(place.sent_with(), ["Bearer token-from-helper"]);
    assert_eq!(pull_request_of_feature(&state), Some(12));
    assert_eq!(
        place.secrets.read(GITHUB_TOKEN).unwrap().as_deref(),
        Some("token-from-helper"),
        "the token is kept for the next times"
    );
}

#[test]
fn github_load_pull_requests__with_no_token_found_it_asks_and_backing_out_sends_and_keeps_nothing() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", false, vec![pull_request_12()]);
    let mut state = place.open();
    assert!(place.load_from_menu(&mut state), "it asks for a token");
    let backed_out = Intent::Feature(FeatureIntent::Answer(None));
    assert!(!place.give(&mut state, vec![backed_out]));
    assert_eq!(place.sent_with(), [] as [String; 0]);
    assert!(place.secrets.kept().is_empty(), "nothing is kept");
    assert_eq!(state.github.pull_requests, None);
}

#[test]
fn github_load_pull_requests__after_a_restart_the_pull_requests_are_read_with_the_kept_token_asking_nothing() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", true, vec![pull_request_12()]);
    place.secrets.write(GITHUB_TOKEN, "kept-token").unwrap();
    let state = place.open();
    assert!(!state.is_operation_running, "nothing is asked");
    assert!(!place.helper_ran());
    assert_eq!(place.sent_with(), ["Bearer kept-token"]);
    assert_eq!(pull_request_of_feature(&state), Some(12));
}

#[test]
fn github_load_pull_requests__a_token_github_refuses_is_not_kept_and_the_next_load_asks_again() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", false, vec![unauthorized()]);
    place.secrets.write(GITHUB_TOKEN, "refused-token").unwrap();
    let mut state = place.open();
    assert_eq!(place.sent_with(), ["Bearer refused-token"]);
    assert!(
        place.secrets.kept().is_empty(),
        "the token refused is taken out of the store"
    );
    let said = state
        .log
        .entries()
        .any(|entry| entry.line == "Could not read pull requests. GitHub answered 401 Unauthorized");
    assert!(
        said,
        "a read on its own is kept quiet, with a warning line in the Command log"
    );
    assert!(place.load_from_menu(&mut state), "the next load asks for a token");
}

#[test]
fn log_record__a_token_github_refuses_and_says_back_never_appears_in_the_log_kept_or_from_the_helper() {
    let refused = || super::log_secrets::refused("token-from-helper");
    let place = GitHubPlace::new("https://github.com/owner/app.git", true, vec![refused(), refused()]);
    place.secrets.write(GITHUB_TOKEN, "token-from-helper").unwrap();
    let mut state = place.open();
    assert!(!place.load_from_menu(&mut state));
    assert_eq!(
        place.sent_with(),
        ["Bearer token-from-helper", "Bearer token-from-helper"]
    );
    assert!(place.helper_ran(), "the second token came from the helper");
    let lines: Vec<&str> = state.log.entries().map(|entry| entry.line.as_str()).collect();
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("Could not read pull requests.")),
        "{lines:?}"
    );
    assert!(
        lines.iter().all(|line| !line.contains("token-from-helper")),
        "{lines:?}"
    );
    let said = state.operations.said.as_ref().map_or_default(|said| said.text.clone());
    assert!(!said.is_empty() && !said.contains("token-from-helper"), "{said}");
}

#[test]
fn github_load_pull_requests__a_token_refused_after_loading_is_said() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", true, vec![unauthorized()]);
    let mut state = place.open();
    assert!(!place.load_from_menu(&mut state));
    assert_eq!(place.sent_with(), ["Bearer token-from-helper"]);
    assert!(place.secrets.kept().is_empty());
    let said = state.operations.said.as_ref().map(|said| said.text.as_str());
    assert_eq!(
        said,
        Some("Could not read pull requests. GitHub answered 401 Unauthorized")
    );
}

#[test]
fn github_load_pull_requests__on_another_host_the_menu_has_no_entry_and_quick_search_says_why() {
    let place = GitHubPlace::new("https://gitlab.com/owner/app.git", true, Vec::new());
    let mut state = place.open();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.click(&state, Role::Button, "More");
    let (_, tree) = window.draw(&state, &[]);
    assert!(
        has(&tree, Role::MenuItem, "Switch identity\u{2026}"),
        "the menu is open"
    );
    assert!(!has(&tree, Role::MenuItem, "Load pull requests from GitHub"));
    let chosen = Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(
        QuickCommand::SignInToGitHub,
    )));
    let _ = place.give(&mut state, vec![chosen]);
    let said = state.operations.said.as_ref().map(|said| said.text.as_str());
    assert_eq!(said, Some("The main remote of this repository is not on github.com."));
    assert!(!place.helper_ran());
    assert_eq!(place.sent_with(), [] as [String; 0]);
}

/// The name the screen reader is told of the badge of #12, which is its tooltip.
const BADGE_12: &str = "Pull request #12: Add the parser \u{b7} checks failed\nClick to open it on GitHub";

/// A window showing a repository whose pull requests were read with the kept token: #12 of `feature`.
fn loaded() -> (GitHubPlace, State) {
    let place = GitHubPlace::new("https://github.com/owner/app.git", false, vec![pull_request_12()]);
    place.secrets.write(GITHUB_TOKEN, "kept-token").unwrap();
    let state = place.open();
    (place, state)
}

#[test]
fn graph_pull_request_badge__a_branch_whose_upstream_has_pull_request_12_with_checks_failing_has_12_in_the_color_of_failure()
 {
    let (_place, state) = loaded();
    let mut window = Window::new(ThemeName::Dark, BADGES);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, BADGE_12), "the label of feature has the badge");
    let read = state.shown_read().unwrap();
    let feature = read
        .state
        .branches
        .iter()
        .find(|branch| branch.name == "feature")
        .unwrap();
    let pull_request = app::store::github::shown(&state).of_branch(feature).unwrap();
    let badge = views::graph::PullRequestBadge {
        pull_request,
        lane: 0,
        service: model::hosting::HostingService::GitHub,
    };
    assert_eq!(views::graph::pull_request_badge_tone(&badge), ui::Tone::Error);
    window.assert_snapshot("pull-request-badge-dark");
}

#[test]
fn graph_pull_request_badge__clicking_the_badge_opens_the_pull_request_and_leaves_the_selection_as_it_was() {
    let (_place, mut state) = loaded();
    let selected = state.timeline.selected.clone();
    let mut window = Window::new(ThemeName::Dark, BADGES);
    let intents = window.click(&state, Role::Button, BADGE_12);
    let url = String::from("https://github.com/owner/app/pull/12");
    assert_eq!(intents, [Intent::OpenUrl(url.clone())]);
    let effects = run(&mut state, Message::Intent(Intent::OpenUrl(url.clone())));
    assert_eq!(effects, [Effect::OpenUrl(url)]);
    assert_eq!(state.timeline.selected, selected, "the selection is as it was");
}

#[test]
fn graph_pull_request_badge__before_the_pull_requests_are_loaded_no_label_has_a_badge() {
    let place = GitHubPlace::new("https://github.com/owner/app.git", false, vec![pull_request_12()]);
    let state = place.open();
    let mut window = Window::new(ThemeName::Dark, BADGES);
    let (_, tree) = window.draw(&state, &[]);
    let badges = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label().is_some_and(|label| label.contains("Pull request #")))
        .count();
    assert_eq!(badges, 0);
}

#[test]
fn github_open_pull_request__the_menu_of_a_branch_opens_its_pull_request_in_the_browser() {
    let (_place, state) = loaded();
    let mut window = Window::new(ThemeName::Dark, (1000, 800));
    let (_, tree) = window.draw(&state, &[]);
    let feature = super::support::find(&tree, Role::Button, "feature").bounds().unwrap();
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    let point = render::Point::new(
        f64::midpoint(feature.x0, feature.x1) as f32,
        f64::midpoint(feature.y0, feature.y1) as f32,
    );
    let button = ui::PointerButtonEvent {
        button: ui::PointerButton::Secondary,
        position: point,
        modifiers: ui::Modifiers::NONE,
        click_count: 1,
    };
    let pressed = [
        ui::InputEvent::PointerPressed(button),
        ui::InputEvent::PointerReleased(button),
    ];
    let _ = window.draw(&state, &pressed);
    let chosen = window.click(&state, Role::MenuItem, "Open pull request #12");
    assert_eq!(
        chosen,
        [Intent::OpenUrl(String::from("https://github.com/owner/app/pull/12"))]
    );
}

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
