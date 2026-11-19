//! Repositories cloned and created from the window (`repo.clone`, `repo.init`): the clone opened as the workspace when
//! none is open, the folder of the next clone remembered across a restart, the row of the **Activity** tab while git
//! clones, and **Create a repository here**. Every clone is of a throwaway repository on disk, and every git runs away
//! from the configuration of the person running the tests.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use app::persisted::Persisted;
use app::store::{Choosing, Effect, Message, State};
use engine::{Asker, CommandLog, Engine, FixedClock, LogLevel};
use git::Git;
use model::Timestamp;
use model::contract::{Answer, AnswerKey, Answers, Context, FeatureId, FeatureSettings, Plan, Question, Target};
use model::state::RepositoryState;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;

use super::support::{OperationsDriver, run};

/// A repository with one commit, to clone, and a folder beside it that holds the clones.
fn origin() -> (Repository, PathBuf) {
    let mut origin = Repository::new();
    let _commit = origin.commit("Add the readme", &[("README.md", "lib\n")]);
    let parent = origin.path().parent().unwrap().join("clones");
    std::fs::create_dir_all(&parent).unwrap();
    (origin, parent)
}

/// The URL of a repository on disk, as `file://`.
fn file_url(repository: &Repository) -> String {
    format!(
        "file:///{}",
        repository
            .path()
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
    )
}

/// The variables the git of the tests runs with: a global configuration of its own, in the test's folder.
fn variables(repository: &Repository) -> Vec<(OsString, OsString)> {
    let command = repository.command("git");
    command
        .get_envs()
        .filter_map(|(name, value)| Some((name.to_os_string(), value?.to_os_string())))
        .collect()
}

/// The thread of the operations, running git with the variables of the tests.
fn driver(repository: &Repository) -> OperationsDriver {
    OperationsDriver::with_access(app::work::github::GitHubAccess {
        git_variables: variables(repository),
        ..app::work::github::GitHubAccess::system()
    })
}

/// Gives the thread what the store asked of it; an operation with no folder runs where the window would run it, in
/// a folder of the user's, here the one given.
fn give(driver: &OperationsDriver, state: &State, effects: Vec<Effect>, home: &Path) {
    let effects: Vec<Effect> = effects
        .into_iter()
        .map(|effect| match effect {
            Effect::RunFeature {
                folder,
                feature,
                preset,
                context,
            } if folder.as_os_str().is_empty() => Effect::RunFeature {
                folder: home.to_path_buf(),
                feature,
                preset,
                context,
            },
            effect => effect,
        })
        .collect();
    driver.take(state, &effects);
}

/// Runs an operation from the window until it ends, confirming its plan; what it asks is said back.
fn until_ended(driver: &OperationsDriver, state: &mut State, intent: Intent, home: &Path) -> Vec<Effect> {
    let effects = run(state, Message::Intent(intent));
    give(driver, state, effects, home);
    let mut all = Vec::new();
    loop {
        let message = driver.raw_next();
        let effects = run(state, message.clone());
        all.extend(effects.iter().cloned());
        give(driver, state, effects, home);
        match message {
            Message::FeatureConfirms(_) => {
                let effects = run(state, Message::Intent(Intent::Feature(FeatureIntent::Confirm(true))));
                give(driver, state, effects, home);
            }
            Message::FeatureEnded { .. } | Message::FeatureAsks(_) => return all,
            _ => {}
        }
    }
}

/// The answers of a clone of a URL into `<parent>/lib`, with nothing ticked.
fn clone_answers(url: &str, parent: &Path) -> Answers {
    Answers::new()
        .with(AnswerKey("clipboard"), Answer::Clipboard(None))
        .with(features::repo::URL, Answer::Text(url.to_owned()))
        .with(features::repo::PARENT, Answer::Path(parent.to_path_buf()))
        .with(features::repo::NAME, Answer::Text(String::from("lib")))
        .with(features::repo::OPTIONS, Answer::Choices(Vec::new()))
}

/// A window that started with no workspace open.
fn no_workspace(persisted: Persisted) -> State {
    State::new(
        Persisted {
            has_started: true,
            ..persisted
        },
        app::settings::Settings::default(),
        None,
    )
}

/// Whether two folders are the same once the links and the short names of Windows are followed.
fn same(first: &Path, second: &Path) -> bool {
    first.canonicalize().unwrap() == second.canonicalize().unwrap()
}

#[test]
fn repo_clone__with_no_workspace_open_the_clone_becomes_the_workspace_and_is_first_among_the_recent_ones() {
    let (origin, parent) = origin();
    let driver = driver(&origin);
    let mut state = no_workspace(Persisted::default());
    let clone = FeatureIntent::RunWith(
        FeatureId("repo.clone"),
        Target::Head,
        clone_answers(&file_url(&origin), &parent),
    );
    let _effects = until_ended(&driver, &mut state, Intent::Feature(clone), &parent);
    let lib = parent.join("lib");
    assert!(lib.join("README.md").is_file(), "the clone has the files of its origin");
    let folders = &state.workspace.as_ref().expect("a workspace is open").folders;
    assert_eq!(folders.len(), 1);
    assert!(same(&folders[0], &lib), "{folders:?}");
    assert!(state.shown_folder().is_some_and(|shown| same(shown, &lib)));
    let first = state.persisted.recent.first().expect("a recent workspace");
    assert!(
        first.folders.len() == 1 && same(&first.folders[0], &lib),
        "{:?}",
        state.persisted.recent
    );
}

#[test]
fn repo_clone__the_folder_dialog_of_the_next_clone_starts_where_the_last_one_went_after_the_window_opens_again() {
    let (origin, parent) = origin();
    let driver = driver(&origin);
    let mut state = no_workspace(Persisted::default());
    let clone = FeatureIntent::RunWith(
        FeatureId("repo.clone"),
        Target::Head,
        clone_answers(&file_url(&origin), &parent),
    );
    let _effects = until_ended(&driver, &mut state, Intent::Feature(clone), &parent);
    assert_eq!(state.persisted.clone_folder.as_deref(), Some(parent.as_path()));
    let mut again = no_workspace(Persisted::from_text(&state.persisted.to_text()));
    let answers = Answers::new()
        .with(AnswerKey("clipboard"), Answer::Clipboard(None))
        .with(features::repo::URL, Answer::Text(file_url(&origin)));
    let next = FeatureIntent::RunWith(FeatureId("repo.clone"), Target::Head, answers);
    let effects = until_ended(&driver, &mut again, Intent::Feature(next), &parent);
    let dialog = effects.iter().find_map(|effect| match effect {
        Effect::ChooseFile { question, choosing, .. } => Some((question, *choosing)),
        _ => None,
    });
    let (question, choosing) = dialog.expect("the folder dialog is opened");
    assert_eq!(choosing, Choosing::Folder);
    assert_eq!(question.folder, parent);
    let effects = run(
        &mut again,
        Message::Intent(Intent::Feature(FeatureIntent::Answer(None))),
    );
    give(&driver, &again, effects, &parent);
    assert!(
        matches!(driver.raw_next(), Message::FeatureEnded { .. }),
        "backing out runs nothing"
    );
}

/// Answers the questions of an operation from answers given, and runs its plan.
struct Answering(Answers);

impl Asker for Answering {
    fn ask(&mut self, question: &Question) -> Option<Answer> {
        self.0.get(question.key).cloned()
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

#[test]
fn repo_clone__during_a_clone_the_activity_tab_lists_it_with_the_last_line_of_progress_git_printed() {
    let (origin, parent) = origin();
    let script = parent.parent().unwrap().join("slow.sh");
    std::fs::write(&script, "sleep 2\nexec \"$@\"\n").unwrap();
    let hook = format!("sh '{}'", script.to_string_lossy().replace('\\', "/"));
    let _set = origin.git(&["config", "--global", "uploadpack.packObjectsHook", &hook]);
    let url = file_url(&origin);
    let context = Context {
        repository: std::sync::Arc::new(RepositoryState {
            folder: parent.clone(),
            ..RepositoryState::default()
        }),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let git = variables(&origin)
        .into_iter()
        .fold(Git::new(&parent), |git, (name, value)| git.with_variable(name, value));
    let answers = clone_answers(&url, &parent);
    let (sender, receiver) = std::sync::mpsc::channel();
    let cloning = thread::spawn(move || {
        let mut engine = Engine::new(git, CommandLog::new(LogLevel::Info), Box::new(FixedClock(Timestamp(0))));
        sender.send(engine.activities().clone()).unwrap();
        let mut asker = Answering(answers.clone());
        engine
            .run_feature_from(
                FeatureId("repo.clone"),
                features::repo::clone,
                (&context, answers),
                &mut asker,
            )
            .is_ok()
    });
    let activities = receiver.recv().unwrap();
    let started = Instant::now();
    let mut seen: Vec<(String, String)> = Vec::new();
    while started.elapsed() < Duration::from_secs(20) {
        thread::sleep(Duration::from_millis(50));
        let running = activities.running();
        let Some(entry) = running.first() else {
            if seen.is_empty() {
                continue;
            }
            break;
        };
        if let Some(progress) = &entry.progress {
            seen.push((entry.description.to_string(), progress.clone()));
        }
    }
    assert!(cloning.join().unwrap(), "the clone ends well");
    let (title, line) = seen.first().expect("a line of progress while git clones");
    assert_eq!(*title, format!("Clone {url} into lib"));
    assert_eq!(line, "Cloning into 'lib'...");
}

#[test]
fn repo_init__create_a_repository_here_runs_git_init_in_the_folder_of_the_workspace_and_shows_its_files_untracked() {
    let origin = Repository::new();
    let _set = origin.git(&["config", "--global", "init.defaultBranch", "main"]);
    let here = origin.path().parent().unwrap().join("here");
    std::fs::create_dir_all(&here).unwrap();
    std::fs::write(here.join("notes.txt"), "notes\n").unwrap();
    let driver = driver(&origin);
    let mut state = no_workspace(Persisted::default());
    let _effects = run(
        &mut state,
        Message::Started {
            folders: vec![here.clone()],
        },
    );
    assert!(state.shown_folder().is_none(), "no repository yet");
    let _effects = until_ended(&driver, &mut state, Intent::CreateRepositoryHere, &here);
    assert!(here.join(".git").is_dir(), "git init ran in the folder");
    assert!(state.log.entries().any(|entry| entry.line.starts_with("git init")));
    let read = state.shown_read().expect("the new repository is shown");
    assert!(same(&read.folder, &here));
    assert_eq!(read.state.working_tree.untracked, ["notes.txt"]);
}
