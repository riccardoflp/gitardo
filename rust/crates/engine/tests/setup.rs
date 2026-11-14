//! The links and copies a new worktree gets (`worktree-setup.link-and-copy-steps`), run by the engine in a worktree of
//! a throwaway repository: a junction on Windows, nothing made where the worktree has something, and a failure that
//! does not stop the steps after it.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::fs;
use std::path::{Path, PathBuf};

use engine::{CommandLog, Engine, FixedClock, LogKind, LogLevel};
use git::Git;
use model::Timestamp;
use model::contract::{Danger, Plan, Step, Text, Undo};
use model::worktree_setup::failed_setup_steps;
use testkit::Repository;

/// A repository that ignores `node_modules` and `.env`, with both on disk, and a worktree of it beside it on a branch
/// `feature`.
fn repository_and_worktree() -> (Repository, PathBuf) {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Ignore what each machine makes",
        &[(".gitignore", "node_modules/\n.env\n")],
    );
    write(
        repository.path(),
        "node_modules/left-pad/index.js",
        "module.exports = 1;\n",
    );
    write(repository.path(), ".env", "KEY=1\n");
    let worktree = repository.path().parent().unwrap().join("trees").join("feature");
    let _ = repository.git(&[
        "worktree",
        "add",
        "--quiet",
        "-b",
        "feature",
        worktree.to_str().unwrap(),
    ]);
    (repository, worktree)
}

fn write(folder: &Path, name: &str, content: &str) {
    let path = folder.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn plan(steps: Vec<Step>) -> Plan {
    Plan {
        title: Text::from("Set up the worktree"),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A test plan.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

fn engine(repository: &Repository) -> Engine {
    Engine::new(
        Git::new(repository.path()),
        CommandLog::new(LogLevel::Info),
        Box::new(FixedClock(Timestamp(0))),
    )
}

fn link(repository: &Repository, worktree: &Path, folder: &str) -> Step {
    Step::Link {
        from: worktree.join(folder),
        to: repository.path().join(folder),
    }
}

fn copy(repository: &Repository, worktree: &Path, path: &str) -> Step {
    Step::Copy {
        from: repository.path().join(path),
        to: worktree.join(path),
    }
}

fn log_lines(engine: &Engine) -> Vec<(LogKind, String)> {
    engine
        .log()
        .entries()
        .map(|entry| (entry.kind.clone(), entry.line.clone()))
        .collect()
}

#[test]
fn worktree_setup_link_and_copy_steps__node_modules_is_a_junction_to_the_window_s_node_modules() {
    let (repository, worktree) = repository_and_worktree();
    let mut engine = engine(&repository);
    let ran = engine
        .run_plan(&plan(vec![link(&repository, &worktree, "node_modules")]))
        .unwrap();
    assert_eq!(ran, ["symlink node_modules"]);
    let linked = worktree.join("node_modules");
    let file_type = fs::symlink_metadata(&linked).unwrap().file_type();
    assert!(file_type.is_symlink());
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        assert!(file_type.is_symlink_dir(), "a junction is a link to a folder");
    }
    let target = fs::canonicalize(&linked).unwrap();
    assert_eq!(
        target,
        fs::canonicalize(repository.path().join("node_modules")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(linked.join("left-pad").join("index.js")).unwrap(),
        "module.exports = 1;\n"
    );
    // Taking the junction away leaves the window's node_modules with all its files.
    fs::remove_dir(&linked).or_else(|_| fs::remove_file(&linked)).unwrap();
    assert!(repository.path().join("node_modules/left-pad/index.js").is_file());
}

#[test]
fn worktree_setup_link_and_copy_steps__what_the_worktree_has_is_left_and_the_log_says_why() {
    let (repository, worktree) = repository_and_worktree();
    write(&worktree, ".env", "KEY=mine\n");
    let mut engine = engine(&repository);
    let steps = vec![
        link(&repository, &worktree, "node_modules"),
        copy(&repository, &worktree, ".env"),
    ];
    let ran = engine.run_plan(&plan(steps)).unwrap();
    assert_eq!(ran, ["symlink node_modules"]);
    assert_eq!(fs::read_to_string(worktree.join(".env")).unwrap(), "KEY=mine\n");
    assert_eq!(
        log_lines(&engine),
        [
            (LogKind::Step, String::from("symlink node_modules")),
            (LogKind::Warning, String::from(".env is in the worktree already.")),
        ]
    );
}

#[test]
fn worktree_setup_link_and_copy_steps__a_copy_that_fails_does_not_stop_the_next_one() {
    let (repository, worktree) = repository_and_worktree();
    write(repository.path(), "config/local.json", "{}\n");
    let mut engine = engine(&repository);
    let steps = vec![
        copy(&repository, &worktree, "secrets.json"),
        copy(&repository, &worktree, ".env"),
        copy(&repository, &worktree, "config/local.json"),
    ];
    let ran = engine.run_plan(&plan(steps)).unwrap();
    assert_eq!(ran, ["copy .env", "copy config/local.json"]);
    assert_eq!(fs::read_to_string(worktree.join(".env")).unwrap(), "KEY=1\n");
    let failed: Vec<String> = engine
        .take_went_on_after()
        .into_iter()
        .map(|failure| failure.failed)
        .collect();
    assert_eq!(failed, ["copy secrets.json"]);
    assert_eq!(
        failed_setup_steps(&failed).as_deref(),
        Some("Gitardo: `copy secrets.json` failed: the log says why.")
    );
    let kinds: Vec<LogKind> = log_lines(&engine).into_iter().map(|(kind, _)| kind).collect();
    assert_eq!(kinds, [LogKind::Step, LogKind::Failure, LogKind::Step, LogKind::Step]);
}

#[test]
fn makes_nothing_under_a_link_of_the_worktree_and_copies_links_as_links() {
    let (repository, worktree) = repository_and_worktree();
    let store = repository.path().parent().unwrap().join("store");
    fs::create_dir_all(&store).unwrap();
    platform::link_folder(&worktree.join("config"), &store).unwrap();
    write(repository.path(), "config/local.json", "{}\n");
    write(repository.path(), "tools/real/run.txt", "run\n");
    platform::link_folder(
        &repository.path().join("tools").join("current"),
        &repository.path().join("tools/real"),
    )
    .unwrap();
    let mut engine = engine(&repository);
    let steps = vec![
        copy(&repository, &worktree, "config/local.json"),
        copy(&repository, &worktree, "tools"),
    ];
    let ran = engine.run_plan(&plan(steps)).unwrap();
    assert_eq!(ran, ["copy tools"]);
    assert!(
        !store.join("local.json").exists(),
        "nothing was written through the link"
    );
    let copied_link = worktree.join("tools").join("current");
    assert!(fs::symlink_metadata(&copied_link).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(copied_link.join("run.txt")).unwrap(), "run\n");
    assert_eq!(
        log_lines(&engine).first().map(|(_, line)| line.as_str()),
        Some("config/local.json is not made: config in the worktree is a link or a file.")
    );
}
