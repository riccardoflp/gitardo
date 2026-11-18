//! The trusted folders in the window: the question asked before the commands of a new worktree run in a folder not
//! trusted, the answer kept in the window's stored state, and **Trusted folders…** in Quick search (`trust.ask`,
//! `trust.manage`).

use std::path::PathBuf;

use app::persisted::Persisted;
use app::store::{Effect, Message, State};
use model::contract::{Answer, Answers, FeatureId, Target};
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::search::{QuickChoice, QuickCommand, SearchIntent};

use super::showing;
use super::support::{OperationsDriver, run};

/// A repository with a commit whose `.gitardo/settings.json` asks new worktrees to run `echo one`, shown in a window
/// that trusts no folder.
fn with_a_command() -> (Repository, State) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let folder = repository.path().join(".gitardo");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("settings.json"),
        "{ \"worktrees.setupCommands\": [\"echo one\"] }",
    )
    .unwrap();
    let state = showing(repository.path().to_path_buf());
    (repository, state)
}

/// What an operation run from the window asked, and the effects of its end.
struct Ran {
    /// Whether it asked whether to trust the folder.
    asked_trust: bool,
    /// The effects of its end.
    ended: Vec<Effect>,
}

/// Runs an operation from the window until it ended, answering the question of `trust.ask` with `trust` and every
/// other question with its default, and saying yes to every plan shown.
fn run_answering(state: &mut State, (feature, target, answers): (&'static str, Target, Answers), trust: &str) -> Ran {
    let driver = OperationsDriver::new();
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::RunWith(FeatureId(feature), target, answers)),
    );
    let mut asked_trust = false;
    loop {
        let message = driver.raw_next();
        let reply = match &message {
            Message::FeatureAsks(question) if question.key == features::trust::TRUST => {
                asked_trust = true;
                Some(FeatureIntent::Answer(Some(Answer::Choice(trust.to_owned()))))
            }
            Message::FeatureAsks(question) => Some(FeatureIntent::Answer(question.default.clone())),
            Message::FeatureConfirms(_) => Some(FeatureIntent::Confirm(true)),
            _ => None,
        };
        let is_ended = matches!(message, Message::FeatureEnded { .. });
        let effects = run(state, message);
        driver.take(state, &effects);
        if let Some(reply) = reply {
            driver.intent(state, Intent::Feature(reply));
        }
        if is_ended {
            return Ran {
                asked_trust,
                ended: effects,
            };
        }
    }
}

/// Creates a worktree from `main` with a name, answering the question of `trust.ask` with `trust`.
fn create(state: &mut State, name: &str, trust: &str) -> Ran {
    let answers = Answers::new()
        .with(features::worktree::START, Answer::Choice(String::from("main")))
        .with(features::worktree::NAME, Answer::Text(name.to_owned()));
    run_answering(state, ("worktree.create", Target::Head, answers), trust)
}

/// The stored state the effects write, if they write it.
fn written(effects: &[Effect]) -> Option<&Persisted> {
    effects.iter().find_map(|effect| match effect {
        Effect::WritePersisted(persisted) => Some(persisted),
        _ => None,
    })
}

/// The commands of new worktrees the Tasks tab lists, in order.
fn commands(state: &State) -> Vec<String> {
    let tabs = state.tasks_tab.tabs.values();
    let tasks = tabs.flat_map(|tab| &tab.commands).flat_map(|commands| &commands.tasks);
    tasks.map(model::tasks::Task::command_line).collect()
}

/// The folder of the repository as the window shows it.
fn shown(state: &State) -> PathBuf {
    state.shown_folder().cloned().unwrap()
}

#[test]
fn trust_ask__trusting_the_folder_runs_the_commands_in_tasks_and_the_next_worktree_asks_nothing() {
    let (_repository, mut state) = with_a_command();
    let first = create(&mut state, "fix", "trust");
    assert!(first.asked_trust);
    let folder = shown(&state);
    assert_eq!(state.persisted.trusted_folders, std::slice::from_ref(&folder));
    let stored = written(&first.ended).expect("the folder trusted is written at once");
    let read_back = Persisted::from_text(&stored.to_text());
    assert_eq!(read_back.trusted_folders, [folder]);
    assert_eq!(commands(&state), ["echo one"], "the command is handed to the Tasks tab");
    let second = create(&mut state, "fix-again", "not-now");
    assert!(!second.asked_trust, "a trusted folder is not asked again");
    assert_eq!(commands(&state).len(), 2);
}

#[test]
fn trust_ask__not_now_creates_the_worktree_runs_nothing_trusts_nothing_and_asks_again() {
    let (_repository, mut state) = with_a_command();
    let first = create(&mut state, "fix", "not-now");
    assert!(first.asked_trust);
    let worktree = PathBuf::from(format!("{}.worktrees", shown(&state).display())).join("fix");
    assert!(worktree.join("README.md").exists(), "the worktree is created");
    assert_eq!(state.persisted.trusted_folders, Vec::<PathBuf>::new());
    assert!(written(&first.ended).is_none());
    assert_eq!(commands(&state).len(), 0, "no command runs");
    let said = state.operations.said.clone().unwrap();
    assert!(said.text.contains("this folder is not trusted"), "{}", said.text);
    let second = create(&mut state, "fix-again", "not-now");
    assert!(second.asked_trust, "the next worktree asks again");
}

#[test]
fn trust_manage__a_folder_unticked_is_no_longer_trusted_in_the_window_and_in_the_stored_state() {
    let (_repository, mut state) = with_a_command();
    let folder = shown(&state);
    let elsewhere = PathBuf::from("/elsewhere/tools");
    state.persisted.trusted_folders = vec![folder, elsewhere.clone()];
    let driver = OperationsDriver::new();
    let chosen = Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(
        QuickCommand::TrustedFolders,
    )));
    driver.intent(&mut state, chosen);
    let ended = loop {
        let message = driver.raw_next();
        let reply = match &message {
            Message::FeatureAsks(question) => {
                assert_eq!(question.key, features::trust::TRUSTED);
                Some(Answer::Choices(vec![elsewhere.to_string_lossy().into_owned()]))
            }
            _ => None,
        };
        let is_ended = matches!(message, Message::FeatureEnded { .. });
        let effects = run(&mut state, message);
        driver.take(&state, &effects);
        if let Some(reply) = reply {
            driver.intent(&mut state, Intent::Feature(FeatureIntent::Answer(Some(reply))));
        }
        if is_ended {
            break effects;
        }
    };
    assert_eq!(state.persisted.trusted_folders, std::slice::from_ref(&elsewhere));
    let stored = written(&ended).expect("the folders trusted are written");
    assert_eq!(Persisted::from_text(&stored.to_text()).trusted_folders, [elsewhere]);
    let said = state.operations.said.clone().unwrap();
    assert!(said.text.ends_with("is no longer trusted."), "{}", said.text);
}
