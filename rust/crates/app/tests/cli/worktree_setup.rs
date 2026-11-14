//! What new worktrees get, saved from the Files page and brought into worktrees that are there already, on throwaway
//! repositories: the reads of git, the settings files and the links and copies, as the window runs them.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::{CommandLog, Engine, FixedClock, LogLevel};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::contract::{Answer, AnswerKey, Answers, Context, FeatureSettings, Outcome, Query, Target};
use testkit::Repository;

use crate::{cli, cli_done};

/// A repository that ignores `node_modules` and `.env`, with both on disk and the given settings for new worktrees.
fn repository_with(settings: &str) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Ignore what each machine makes",
        &[(".gitignore", "node_modules/\n.env\n/.gitardo/\n")],
    );
    write(
        repository.path(),
        "node_modules/left-pad/index.js",
        "module.exports = 1;\n",
    );
    write(
        repository.path(),
        "packages/api/node_modules/kit/index.js",
        "module.exports = 2;\n",
    );
    write(repository.path(), ".env", "KEY=1\n");
    write(repository.path(), ".gitardo/settings.json", settings);
    repository
}

/// Adds a worktree beside the repository, on a new branch.
fn add_worktree(repository: &Repository, name: &str) -> PathBuf {
    let worktree = repository
        .path()
        .parent()
        .map_or_else(PathBuf::new, |parent| parent.join("trees").join(name));
    let _ = repository.git(&["worktree", "add", "--quiet", "-b", name, &worktree.to_string_lossy()]);
    worktree
}

fn write(folder: &Path, name: &str, content: &str) {
    let path = folder.join(name);
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        panic!("could not make {}: {error}", parent.display());
    }
    if let Err(error) = fs::write(&path, content) {
        panic!("could not write {}: {error}", path.display());
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()))
}

#[test]
fn worktree_setup_link_into_new_worktrees__git_s_answers_give_the_pattern_and_the_sentence() {
    let repository = repository_with("{\n  // Kept.\n}\n");
    let ended = cli_done(
        &repository,
        &[
            "--target",
            "files:packages/api/node_modules",
            "worktree-setup.link-into-new-worktrees",
        ],
    );
    assert!(
        ended
            .out
            .contains("packages/api/node_modules will be linked into new worktrees."),
        "{}",
        ended.out
    );
    let settings = read(&repository.path().join(".gitardo").join("settings.json"));
    assert!(settings.contains("// Kept."), "{settings}");
    assert!(settings.contains("\"/packages/api/node_modules/\""), "{settings}");
    // Linked already now: nothing more is written, and the refusal says why.
    let again = cli(
        &repository,
        &[
            "--target",
            "files:packages/api/node_modules",
            "worktree-setup.link-into-new-worktrees",
        ],
    );
    assert_eq!(again.code, Some(1));
    assert!(
        again
            .errors
            .contains("packages/api/node_modules is linked into new worktrees already, by the pattern"),
        "{}",
        again.errors
    );
}

#[test]
fn worktree_setup_bring_into_worktree__node_modules_is_linked_and_the_worktree_s_own_env_is_left() {
    let repository =
        repository_with(r#"{"worktrees.linkFolders": ["node_modules/"], "worktrees.copyFiles": ["/.env"]}"#);
    let worktree = add_worktree(&repository, "feature");
    write(&worktree, ".env", "KEY=mine\n");
    let target = format!("worktree:{}", worktree.display());
    let ended = cli_done(
        &repository,
        &["--target", &target, "--yes", "worktree-setup.bring-into-worktree"],
    );
    assert!(ended.out.contains("symlink node_modules"), "{}", ended.out);
    assert_eq!(
        read(&worktree.join("node_modules").join("left-pad").join("index.js")),
        "module.exports = 1;\n"
    );
    assert_eq!(read(&worktree.join(".env")), "KEY=mine\n");
}

#[test]
fn worktree_setup_bring_into_other_worktrees__a_failure_in_the_first_does_not_stop_the_second() {
    let repository = repository_with(r#"{"worktrees.copyFiles": ["/.env", "/secrets.json"]}"#);
    let first = add_worktree(&repository, "first");
    let second = add_worktree(&repository, "second");
    let git = Git::new(repository.path());
    let state = match read_state(
        &git,
        &StateRequest {
            folder: repository.path().to_path_buf(),
        },
    ) {
        Ok(state) => state,
        Err(error) => panic!("could not read the repository: {error}"),
    };
    let context = Context {
        repository: Arc::new(state),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let mut answers = Answers::new();
    let plan = loop {
        match features::worktree_setup::bring_into_other_worktrees(&context, &answers) {
            Outcome::Read(key, query) => answers.insert(key, answer(&repository, key, &query)),
            Outcome::Run(plan) => break plan,
            other => panic!("expected a plan, got {other:?}"),
        }
    };
    assert_eq!(plan.title.as_str(), "Bring ignored files into 2 worktrees");
    // The file the patterns took goes away before the plan runs, so that the first copy into each worktree fails.
    let secrets = repository.path().join("secrets.json");
    assert!(!secrets.exists());
    let mut engine = Engine::new(git, CommandLog::new(LogLevel::Info), Box::new(FixedClock(Timestamp(0))));
    let ran = engine.run_plan(&plan).unwrap_or_else(|failure| panic!("{failure:?}"));
    assert_eq!(ran, ["copy .env", "copy .env"]);
    assert_eq!(read(&first.join(".env")), "KEY=1\n");
    assert_eq!(read(&second.join(".env")), "KEY=1\n");
    let failed: Vec<String> = engine
        .take_went_on_after()
        .into_iter()
        .map(|failure| failure.failed)
        .collect();
    assert_eq!(
        model::worktree_setup::failed_setup_steps(&failed).as_deref(),
        Some("2 folders or files failed: the log says why.")
    );
}

/// The answer of a read, from the files and from git, with `secrets.json` among what the patterns take.
fn answer(repository: &Repository, key: AnswerKey, query: &Query) -> Answer {
    match query {
        Query::FileTexts { paths } => Answer::Files(paths.iter().map(|path| fs::read(path).ok()).collect()),
        Query::SetupTakes(requests) => {
            let git = Git::new(repository.path());
            let takes = requests
                .iter()
                .map(|request| {
                    let request = git::read::worktree_setup::PatternsRequest {
                        folder: request.folder.clone(),
                        link_patterns: request.patterns.link.clone(),
                        copy_patterns: request.patterns.copy.clone(),
                        commit: request.commit.clone(),
                    };
                    let mut take = git::read::worktree_setup::read_what_patterns_take(&git, &request)
                        .unwrap_or_else(|error| panic!("{error}"));
                    take.copied.insert(0, String::from("secrets.json"));
                    take
                })
                .collect();
            Answer::SetupTakes(takes)
        }
        other => panic!("{} asked for {other:?}", key.0),
    }
}
