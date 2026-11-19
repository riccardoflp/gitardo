//! The merge requests of a GitLab in the window (`hosting.load-merge-requests`): a token GitLab refuses taken out of
//! the store, and asked for again. The GitLab is one of one's own at `example.test` (`hosting.selfHosted`), a server of
//! the test on 127.0.0.1; the credential store is in memory, and git, in a throwaway repository, has no credential
//! helper and reads no configuration of the user's.

use std::ffi::OsString;
use std::sync::Arc;

use app::store::{Effect, Message, State};
use app::work::github::{GitHubAccess, Network};
use engine::{MemorySecrets, SecretStore as _};
use http::test_server::{Answer, TestServer, answer};
use http::{Http, Proxies};
use model::hosting::{HostingService, KnownHosts, SelfHosted};
use render::ThemeName;
use testkit::Repository;
use views::Intent;

use super::support::{OperationsDriver, Window, run};
use super::window;

/// The size of the window: tall enough for the whole **…** menu.
const WIDE: (u32, u32) = (1000, 1100);

/// A repository whose main remote is on the GitLab of the test, with the server, the store and the git of the test.
struct GitLabPlace {
    /// The repository, with a branch `feature` whose upstream is `origin/feature`.
    repository: Repository,
    /// The server that answers for `example.test`.
    server: TestServer,
    /// The credential store.
    secrets: MemorySecrets,
    /// Where the merge requests are read from and the token kept.
    access: GitHubAccess,
    /// The thread of the operations.
    driver: OperationsDriver,
}

impl GitLabPlace {
    /// A repository with `origin` on `example.test`, and the GitLab answering each request in turn.
    fn new(answers: Vec<Answer>) -> GitLabPlace {
        let mut repository = Repository::new();
        let _ = repository.commit("Add a", &[("a.txt", "a\n")]);
        let _ = repository.git(&["remote", "add", "origin", "https://example.test/group/app.git"]);
        let _ = repository.git(&["branch", "feature"]);
        let _ = repository.git(&["update-ref", "refs/remotes/origin/feature", "HEAD"]);
        let _ = repository.git(&["branch", "--set-upstream-to=origin/feature", "feature"]);
        let _ = repository.git(&["config", "credential.helper", ""]);
        let global = repository.path().join(".git").join("test-global-config");
        std::fs::write(&global, "").unwrap();
        let server = TestServer::secure(answers);
        let secrets = MemorySecrets::default();
        let kept = secrets.clone();
        let http = Http::new(TestServer::trust(), Proxies::none()).with_address("example.test", 443, server.address());
        let access = GitHubAccess {
            network: Network::Given(http),
            secrets: Arc::new(move || Box::new(kept.clone())),
            git_variables: vec![
                (OsString::from("GIT_CONFIG_NOSYSTEM"), OsString::from("1")),
                (OsString::from("GIT_CONFIG_GLOBAL"), global.into_os_string()),
            ],
            copilot: engine::ai::CopilotCli::system(),
        };
        GitLabPlace {
            repository,
            server,
            secrets,
            driver: OperationsDriver::with_access(access.clone()),
            access,
        }
    }

    /// The window opened on the repository, with `example.test` known as a GitLab of one's own.
    fn open(&self) -> State {
        let mut state = window();
        let gitlab = SelfHosted {
            service: HostingService::GitLab,
            web: None,
        };
        state.settings.features.hosting = KnownHosts {
            by_host: [(String::from("example.test"), gitlab)].into(),
        };
        let folders = vec![self.repository.path().to_path_buf()];
        let effects = run(&mut state, Message::Started { folders });
        self.serve(&mut state, &effects);
        state
    }

    /// Runs the jobs of the merge requests among the effects, at once on the thread of the test, and those they start.
    fn serve(&self, state: &mut State, effects: &[Effect]) {
        let mut waiting: Vec<_> = jobs(effects);
        while let Some(job) = waiting.pop() {
            let effects = run(state, app::work::github::run(&self.access, &job));
            self.driver.take(state, &effects);
            waiting.extend(jobs(&effects));
        }
    }

    /// Reduces intents, then waits until the operation asks a question or ends: whether it asked.
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

    /// Chooses **Load merge requests from GitLab** in the **…** menu: whether it asked for a token.
    fn load_from_menu(&self, state: &mut State) -> bool {
        let mut window = Window::new(ThemeName::Dark, WIDE);
        let opened = window.click(state, platform::accesskit::Role::Button, "More");
        let _ = self.give(state, opened);
        let chosen = window.click(
            state,
            platform::accesskit::Role::MenuItem,
            "Load merge requests from GitLab",
        );
        self.give(state, chosen)
    }
}

/// The jobs of the merge requests among effects.
fn jobs(effects: &[Effect]) -> Vec<app::store::github::GitHubJob> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::GitHub(job) => Some(job.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn hosting_load_merge_requests__a_token_gitlab_refuses_is_not_kept_and_the_next_load_asks_again() {
    let refused = Answer::Bytes(answer("401 Unauthorized", &[], b"{\"message\": \"401 Unauthorized\"}"));
    let place = GitLabPlace::new(vec![refused]);
    place.secrets.write("example.test", "refused-token").unwrap();
    let mut state = place.open();
    let sent: Vec<_> = place.server.received();
    assert_eq!(sent.len(), 1, "the kept token is tried once");
    assert_eq!(sent[0].request_line(), "POST /api/graphql HTTP/1.1");
    assert_eq!(sent[0].header("Authorization").as_deref(), Some("Bearer refused-token"));
    assert!(
        place.secrets.kept().is_empty(),
        "the token refused is taken out of the store"
    );
    let said = state
        .log
        .entries()
        .any(|entry| entry.line == "Could not read merge requests. GitLab answered 401 Unauthorized");
    assert!(
        said,
        "a read on its own is kept quiet, with a warning line in the Command log"
    );
    assert!(place.load_from_menu(&mut state), "the next load asks for a token");
    assert_eq!(
        place.server.received().len(),
        1,
        "nothing more is sent before a token is given"
    );
}
