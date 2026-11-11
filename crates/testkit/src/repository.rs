//! Throwaway repositories: an empty git repository for one test, with helpers in the words of the domain.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::environment::{CommitDate, Environment, Place};
use crate::folder::TemporaryFolder;

/// The file that [`Repository::conflict`] changes on both sides.
pub const CONFLICT_FILE: &str = "conflict.txt";

/// The branch that [`Repository::conflict`] merges into what is checked out.
pub const CONFLICT_BRANCH: &str = "conflicting";

/// An empty git repository for one test, on a branch named `main`, deleted with its folder when it is dropped.
///
/// Its folder holds the repository in `repository/`, the empty global configuration of its git commands in
/// `global.gitconfig`, and the bare repository of a [`crate::Remote`] in `remote.git`. Every git command it runs has
/// a fixed identity, `Test Author <author@example.com>`, and a fixed date: [`crate::FIRST_COMMIT_TIME`] for the first
/// commit, one minute later for each commit after it.
#[derive(Debug)]
pub struct Repository {
    path: PathBuf,
    environment: Environment,
    date: CommitDate,
    // Dropped last, after everything that points into it.
    folder: TemporaryFolder,
}

impl Repository {
    /// Makes an empty repository with `git init -b main`, in a folder of its own under the temporary folder, with
    /// `core.autocrlf`, `commit.gpgsign` and `tag.gpgsign` turned off in its configuration.
    ///
    /// # Panics
    ///
    /// When the folder cannot be made or git fails: the test cannot go on without its repository.
    #[must_use]
    pub fn new() -> Repository {
        let folder = TemporaryFolder::new();
        let environment = Environment::new(folder.path());
        let path = folder.path().join("repository");
        let date = CommitDate::first();
        let place = Place {
            directory: folder.path(),
            date,
        };
        let _ = environment.run_successfully(place, &["init", "--quiet", "-b", "main", "repository"]);
        let repository = Repository {
            path,
            environment,
            date,
            folder,
        };
        for (key, value) in [
            ("core.autocrlf", "false"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
        ] {
            let _ = repository.git(&["config", key, value]);
        }
        repository
    }

    /// The folder of the repository: its working tree, with `.git` in it.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A program to run in the repository's folder, in the environment its git commands run in: the same identity, the
    /// date of the next commit, and none of the configuration of the person running the tests. For a test that runs
    /// Gitardo's own programs on the repository.
    #[must_use]
    pub fn command(&self, program: impl AsRef<OsStr>) -> Command {
        self.environment.command(program, self.place())
    }

    /// Runs any git command in the repository and returns what it printed on its standard output. A commit it makes
    /// gets the date the next commit of a helper will get; use [`Repository::commit`] for commits one minute apart.
    ///
    /// # Panics
    ///
    /// When git fails, with the command and everything git printed, or prints something that is not UTF-8.
    #[must_use = "use `let _ =` for a command run only for what it does"]
    pub fn git(&self, arguments: &[&str]) -> String {
        self.environment.run_successfully(self.place(), arguments)
    }

    /// Writes files, making the folders they are in, stages them and commits them with a message, one minute after the
    /// commit before. With no files it makes an empty commit. Returns the full id of the new commit.
    ///
    /// The paths are relative to the working tree, with `/` between folders, and are given to git literally: a name
    /// such as `pages/[id].tsx` is that file, not a pattern.
    ///
    /// # Panics
    ///
    /// When a file cannot be written or git fails, for instance when the files change nothing.
    pub fn commit(&mut self, message: &str, files: &[(&str, &str)]) -> String {
        for (name, content) in files {
            self.write(name, content);
        }
        if files.is_empty() {
            let _ = self.git(&["commit", "--quiet", "--allow-empty", "-m", message]);
        } else {
            let mut arguments = vec![String::from("add"), String::from("--")];
            arguments.extend(files.iter().map(|(name, _)| format!(":(literal){name}")));
            let _ = self.environment.run_successfully(self.place(), &arguments);
            let _ = self.git(&["commit", "--quiet", "-m", message]);
        }
        self.date = self.date.next();
        self.git(&["rev-parse", "HEAD"]).trim_end().to_owned()
    }

    /// Makes a branch at what is checked out, without checking it out.
    ///
    /// # Panics
    ///
    /// When git fails, for instance when the branch exists or there is no commit yet.
    pub fn branch(&self, name: &str) {
        let _ = self.git(&["branch", name]);
    }

    /// Checks out a branch, a tag or a commit: a tag or a commit leaves `HEAD` detached.
    ///
    /// # Panics
    ///
    /// When git fails, for instance when there is no such branch or the working tree is in the way.
    pub fn checkout(&self, name: &str) {
        let _ = self.git(&["checkout", "--quiet", name, "--"]);
    }

    /// Makes a lightweight tag at what is checked out.
    ///
    /// # Panics
    ///
    /// When git fails, for instance when the tag exists.
    pub fn tag(&self, name: &str) {
        let _ = self.git(&["tag", name]);
    }

    /// Merges a branch into what is checked out with a merge commit, even when a fast-forward would do, and with the
    /// message git proposes.
    ///
    /// # Panics
    ///
    /// When git fails, for instance when the merge stops on a conflict.
    pub fn merge(&mut self, name: &str) {
        let _ = self.git(&["merge", "--quiet", "--no-ff", "--no-edit", name]);
        self.date = self.date.next();
    }

    /// Leaves a merge stopped on a conflict in one file, [`CONFLICT_FILE`]: it commits the file, makes the branch
    /// [`CONFLICT_BRANCH`] there, changes the file on both sides, one commit each, and merges that branch into what was
    /// checked out. Works in an empty repository too.
    ///
    /// # Panics
    ///
    /// When git fails, or when the merge does not stop on a conflict.
    pub fn conflict(&mut self) {
        let checked_out = self.git(&["symbolic-ref", "--short", "HEAD"]).trim_end().to_owned();
        let _ = self.commit("Add the file that will conflict", &[(CONFLICT_FILE, "base\n")]);
        self.branch(CONFLICT_BRANCH);
        let _ = self.commit(
            &format!("Change the file on {checked_out}"),
            &[(CONFLICT_FILE, "ours\n")],
        );
        self.checkout(CONFLICT_BRANCH);
        let _ = self.commit(
            &format!("Change the file on {CONFLICT_BRANCH}"),
            &[(CONFLICT_FILE, "theirs\n")],
        );
        self.checkout(&checked_out);
        let output = self
            .environment
            .run(self.place(), &["merge", "--no-ff", "--no-edit", CONFLICT_BRANCH]);
        let unmerged = self.git(&["diff", "--name-only", "--diff-filter=U"]);
        assert!(
            !output.status.success() && unmerged.trim_end() == CONFLICT_FILE,
            "the merge of {CONFLICT_BRANCH} into {checked_out} did not stop on a conflict in {CONFLICT_FILE}:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    /// The environment of the repository's git commands, for a remote beside it.
    pub(crate) fn environment(&self) -> &Environment {
        &self.environment
    }

    /// The folder that holds the repository, for a remote beside it.
    pub(crate) fn folder(&self) -> &Path {
        self.folder.path()
    }

    /// Where the repository's git commands run, with the date of the next commit.
    pub(crate) fn place(&self) -> Place<'_> {
        Place {
            directory: &self.path,
            date: self.date,
        }
    }

    /// Writes a file of the working tree, making the folders it is in.
    fn write(&self, name: &str, content: &str) {
        let path = self.path.join(name);
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            panic!("could not make the folder {}: {error}", parent.display());
        }
        if let Err(error) = fs::write(&path, content) {
            panic!("could not write {}: {error}", path.display());
        }
    }
}

impl Default for Repository {
    fn default() -> Repository {
        Repository::new()
    }
}
