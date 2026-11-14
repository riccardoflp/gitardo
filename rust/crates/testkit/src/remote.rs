//! A remote for a throwaway repository: a bare repository beside it, as a server would hold it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::environment::{CommitDate, Environment, Place};
use crate::repository::Repository;

/// A bare repository in `remote.git` beside a [`Repository`], added to it as `origin`. It is deleted with the
/// repository's folder, so it lasts as long as the repository does.
#[derive(Debug)]
pub struct Remote {
    path: PathBuf,
    environment: Environment,
}

impl Remote {
    /// Makes the bare repository, adds it to the repository as `origin`, and pushes `main` to it, with `main` set to
    /// follow `origin/main` as after a clone.
    ///
    /// # Panics
    ///
    /// When git fails, for instance when `main` has no commit yet or the repository has a remote already.
    #[must_use]
    pub fn new(repository: &Repository) -> Remote {
        let path = repository.folder().join("remote.git");
        let environment = repository.environment().clone();
        let place = Place {
            directory: repository.folder(),
            date: CommitDate::first(),
        };
        let _ = environment.run_successfully(place, &["init", "--quiet", "--bare", "-b", "main", "remote.git"]);
        let add = [
            OsStr::new("remote"),
            OsStr::new("add"),
            OsStr::new("origin"),
            path.as_os_str(),
        ];
        let _ = environment.run_successfully(repository.place(), &add);
        let _ = environment.run_successfully(
            repository.place(),
            &["push", "--quiet", "--set-upstream", "origin", "main"],
        );
        Remote { path, environment }
    }

    /// The folder of the bare repository.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs any git command in the bare repository and returns what it printed on its standard output.
    ///
    /// # Panics
    ///
    /// When git fails, with the command and everything git printed, or prints something that is not UTF-8.
    #[must_use = "use `let _ =` for a command run only for what it does"]
    pub fn git(&self, arguments: &[&str]) -> String {
        let place = Place {
            directory: &self.path,
            date: CommitDate::first(),
        };
        self.environment.run_successfully(place, arguments)
    }
}
