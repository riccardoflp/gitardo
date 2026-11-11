//! The environment every git command of a test runs in, away from the configuration of the person running the tests.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The name and address of the author and committer of every commit a test makes. No real person's.
const IDENTITY_NAME: &str = "Test Author";
/// The address that goes with [`IDENTITY_NAME`].
const IDENTITY_EMAIL: &str = "author@example.com";

/// The date of the first commit of every test repository, in seconds since 1970-01-01 00:00:00 UTC: it is
/// 2025-01-01 00:00:00 UTC. Each commit after it is one minute later.
pub const FIRST_COMMIT_TIME: u64 = 1_735_689_600;

/// The date a commit gets: the first commit's date, plus one minute for each commit made before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CommitDate {
    minutes: u64,
}

impl CommitDate {
    /// The date of the first commit.
    pub(crate) fn first() -> CommitDate {
        CommitDate { minutes: 0 }
    }

    /// The date of the commit after this one.
    pub(crate) fn next(self) -> CommitDate {
        CommitDate {
            minutes: self.minutes + 1,
        }
    }

    /// The date in git's internal format, seconds since the epoch and the time zone, which git reads without guessing.
    fn to_git(self) -> String {
        format!("{} +0000", FIRST_COMMIT_TIME + self.minutes * 60)
    }
}

/// Where a git command runs and with what date, for [`Environment::run`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Place<'a> {
    /// The folder the command runs in.
    pub(crate) directory: &'a Path,
    /// The date of a commit the command makes.
    pub(crate) date: CommitDate,
}

/// The variables that every git command of a test gets, in the test's folder: a global configuration file of its own,
/// no system configuration, a fixed identity, the C locale, and no prompt for a password.
#[derive(Clone, Debug)]
pub(crate) struct Environment {
    folder: PathBuf,
}

impl Environment {
    /// The environment of the tests in a folder, with an empty `global.gitconfig` made in it.
    pub(crate) fn new(folder: &Path) -> Environment {
        let global = folder.join("global.gitconfig");
        if let Err(error) = fs::write(&global, "") {
            panic!("could not write {}: {error}", global.display());
        }
        Environment {
            folder: folder.to_path_buf(),
        }
    }

    /// A program run in a place, in the environment of the tests, never through a shell.
    pub(crate) fn command(&self, program: impl AsRef<OsStr>, place: Place<'_>) -> Command {
        let mut command = Command::new(program);
        let _ = command
            .current_dir(place.directory)
            .env_clear()
            .envs(inherited_variables())
            .envs(self.variables(place.date));
        command
    }

    /// Runs git with arguments, never through a shell, and returns what it printed and how it ended.
    pub(crate) fn run<Argument: AsRef<OsStr>>(&self, place: Place<'_>, arguments: &[Argument]) -> Output {
        let output = self.command("git", place).args(arguments).output();
        match output {
            Ok(output) => output,
            Err(error) => panic!("could not run git, which the tests need on the PATH: {error}"),
        }
    }

    /// Runs git with arguments and returns what it printed on its standard output. When git fails, the test fails with
    /// the command and everything git printed.
    pub(crate) fn run_successfully<Argument: AsRef<OsStr>>(&self, place: Place<'_>, arguments: &[Argument]) -> String {
        let output = self.run(place, arguments);
        let command = shown_command(arguments);
        assert!(
            output.status.success(),
            "{command} failed in {} ({}):\n{}{}",
            place.directory.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        match String::from_utf8(output.stdout) {
            Ok(text) => text,
            Err(error) => panic!("{command} printed something that is not UTF-8: {error}"),
        }
    }

    /// The variables that isolate a command from the person running the tests, with the date of the commit it makes.
    fn variables(&self, date: CommitDate) -> Vec<(&'static str, OsString)> {
        let date = OsString::from(date.to_git());
        vec![
            (
                "GIT_CONFIG_GLOBAL",
                self.folder.join("global.gitconfig").into_os_string(),
            ),
            ("GIT_CONFIG_NOSYSTEM", OsString::from("1")),
            // The attributes and ignore files git reads from the home folder are the person's, not the test's.
            ("HOME", self.folder.clone().into_os_string()),
            ("XDG_CONFIG_HOME", self.folder.clone().into_os_string()),
            ("GIT_AUTHOR_NAME", OsString::from(IDENTITY_NAME)),
            ("GIT_AUTHOR_EMAIL", OsString::from(IDENTITY_EMAIL)),
            ("GIT_COMMITTER_NAME", OsString::from(IDENTITY_NAME)),
            ("GIT_COMMITTER_EMAIL", OsString::from(IDENTITY_EMAIL)),
            ("GIT_AUTHOR_DATE", date.clone()),
            ("GIT_COMMITTER_DATE", date),
            ("LC_ALL", OsString::from("C")),
            ("GIT_TERMINAL_PROMPT", OsString::from("0")),
        ]
    }
}

/// The variables of the process running the tests that a git command keeps: all but git's own, which could point it
/// at another repository (a hook running the tests sets `GIT_DIR` and `GIT_INDEX_FILE`) or change its configuration.
fn inherited_variables() -> impl Iterator<Item = (OsString, OsString)> {
    std::env::vars_os().filter(|(name, _)| !name.to_string_lossy().to_ascii_uppercase().starts_with("GIT_"))
}

/// A command as a failing test shows it.
fn shown_command<Argument: AsRef<OsStr>>(arguments: &[Argument]) -> String {
    let words: Vec<String> = arguments
        .iter()
        .map(|argument| argument.as_ref().to_string_lossy().into_owned())
        .collect();
    format!("git {}", words.join(" "))
}
