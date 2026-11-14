//! The two files as read, and how a value is taken from them: the repository's first for a setting it may set, a
//! value of the wrong kind left for the default and noted as a problem.

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::Path;

use json::Value;

use crate::keys::Scope;
use crate::{ProblemReason, SettingProblem, SettingsFile, SettingsFiles};

/// The two files as read, and the problems found in them so far.
#[derive(Clone, Debug, Default)]
pub struct Lookup {
    /// The user's file, when it was read.
    user: Option<Value>,
    /// The repository's file, when it was read.
    repository: Option<Value>,
    /// The values not taken.
    problems: Vec<SettingProblem>,
}

impl Lookup {
    /// The files of a window, the user's left out when the system does not say where it is. A file that is not there
    /// sets nothing; one that cannot be read or parsed sets nothing and is noted as a problem.
    #[must_use]
    pub fn read(files: &SettingsFiles) -> Lookup {
        let mut problems = Vec::new();
        let user = files
            .user
            .as_deref()
            .and_then(|file| file_text(file, SettingsFile::User, &mut problems));
        let repository = files
            .repository
            .as_deref()
            .and_then(|file| file_text(file, SettingsFile::Repository, &mut problems));
        let mut lookup = Lookup::parse(user.as_deref(), repository.as_deref());
        problems.append(&mut lookup.problems);
        lookup.problems = problems;
        lookup
    }

    /// The texts of the two files, without reading anything.
    #[must_use]
    pub fn parse(user: Option<&str>, repository: Option<&str>) -> Lookup {
        let (user, user_problem) = document(user, SettingsFile::User);
        let (repository, repository_problem) = document(repository, SettingsFile::Repository);
        Lookup {
            user,
            repository,
            problems: user_problem.into_iter().chain(repository_problem).collect(),
        }
    }

    /// The values not taken so far, in the order they were found.
    #[must_use]
    pub fn into_problems(self) -> Vec<SettingProblem> {
        self.problems
    }

    /// Every value the two files set, by key, the repository's first.
    #[must_use]
    pub fn written_values(&self) -> BTreeMap<String, Vec<(SettingsFile, Value)>> {
        let mut values: BTreeMap<String, Vec<(SettingsFile, Value)>> = BTreeMap::new();
        let files = [
            (SettingsFile::Repository, self.repository.as_ref()),
            (SettingsFile::User, self.user.as_ref()),
        ];
        for (file, document) in files {
            for member in document.and_then(Value::as_object).unwrap_or_default() {
                values
                    .entry(member.name.clone())
                    .or_default()
                    .push((file, member.value.clone()));
            }
        }
        values
    }

    /// The values of a key, in the order they are tried: the repository's first for a setting it may set, then the
    /// user's. A setting the repository may not set and does is a problem.
    fn values(&mut self, key: &str, scope: Scope) -> Vec<(Value, SettingsFile)> {
        let mut found = Vec::new();
        if let Some(value) = self.repository.as_ref().and_then(|document| document.get(key)) {
            match scope {
                Scope::UserAndRepository => found.push((value.clone(), SettingsFile::Repository)),
                Scope::User => self.problem(key, SettingsFile::Repository, ProblemReason::UserOnly),
            }
        }
        if let Some(value) = self.user.as_ref().and_then(|document| document.get(key)) {
            found.push((value.clone(), SettingsFile::User));
        }
        found
    }

    /// Notes a value not taken.
    fn problem(&mut self, key: &str, file: SettingsFile, reason: ProblemReason) {
        self.problems.push(SettingProblem {
            key: key.to_owned(),
            file,
            reason,
        });
    }

    /// The first value of a key that a test takes, the others noted as problems with the reason it gives.
    pub fn first<T>(
        &mut self,
        (key, scope): (&str, Scope),
        take: impl Fn(&Value) -> Result<T, ProblemReason>,
    ) -> Option<T> {
        for (value, file) in self.values(key, scope) {
            match take(&value) {
                Ok(taken) => return Some(taken),
                Err(reason) => self.problem(key, file, reason),
            }
        }
        None
    }

    /// A string.
    pub fn string(&mut self, key: &str, scope: Scope) -> Option<String> {
        self.first((key, scope), |value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(ProblemReason::WrongType { expected: "a string" })
        })
    }

    /// One of some choices, which the user's file sets.
    pub fn choice(&mut self, key: &str, choices: &'static [&'static str]) -> Option<&'static str> {
        self.first((key, Scope::User), |value| {
            let text = value
                .as_str()
                .ok_or(ProblemReason::WrongType { expected: "a string" })?;
            choices
                .iter()
                .find(|choice| **choice == text)
                .copied()
                .ok_or(ProblemReason::NotAChoice { choices })
        })
    }

    /// A boolean.
    pub fn boolean(&mut self, key: &str, scope: Scope) -> Option<bool> {
        self.first((key, scope), |value| {
            value.as_bool().ok_or(ProblemReason::WrongType {
                expected: "true or false",
            })
        })
    }

    /// A number.
    pub fn number(&mut self, key: &str, scope: Scope) -> Option<f64> {
        self.first((key, scope), |value| {
            value
                .as_number()
                .and_then(json::Number::as_f64)
                .ok_or(ProblemReason::WrongType { expected: "a number" })
        })
    }
}

/// The text of a settings file; none when it is not there, or cannot be read, which is a problem.
fn file_text(path: &Path, file: SettingsFile, problems: &mut Vec<SettingProblem>) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            problems.push(SettingProblem {
                key: String::new(),
                file,
                reason: ProblemReason::Unreadable(error.to_string()),
            });
            None
        }
    }
}

/// The value of a file's text, or the problem of a text that is not JSON with comments.
fn document(text: Option<&str>, file: SettingsFile) -> (Option<Value>, Option<SettingProblem>) {
    match text.map(json::parse_with_comments) {
        None => (None, None),
        Some(Ok(value)) => (Some(value), None),
        Some(Err(error)) => (
            None,
            Some(SettingProblem {
                key: String::new(),
                file,
                reason: ProblemReason::Unreadable(error.to_string()),
            }),
        ),
    }
}
