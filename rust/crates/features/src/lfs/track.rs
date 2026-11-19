//! `lfs.track`: has Git LFS store the files a pattern takes from their next commit on, the line written by Git LFS in
//! the `.gitattributes` at the top of the repository.

use std::fmt::Write as _;

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FeatureId, FollowUp, OfferedRun, Outcome, Plan, Query,
    Question, QuestionKind, Refusal, Step, Target, Text, Undo, Validation,
};

/// How the files are named: `extension`, `file`, `folder` or `typed`.
pub const HOW: AnswerKey = AnswerKey("how");
/// The pattern typed.
pub const PATTERN: AnswerKey = AnswerKey("pattern");
/// The files git tracks that the pattern takes and that Git LFS does not store yet, read before the plan.
pub const TO_STORE: AnswerKey = AnswerKey("to-store");
/// The title of the questions.
const TITLE: &str = "Store with Git LFS";
/// Why Undo has nothing to take back.
const NOT_TAKEN_BACK: &str = "Not taken back by Undo: Stop storing with Git LFS on the Large files page does.";

/// Runs `git lfs track <pattern>`, or `git lfs track --filename <path>` for **This file**, at the top of the
/// repository, asking first how the files are named: by the extension of the file, the file alone, its folder, or a
/// pattern typed. Once it ran it says how many files committed before the pattern takes, and offers to store them
/// (`lfs.store-committed`). Refuses without Git LFS, and when it is not set up, with **Set up**.
///
/// Feature: lfs.track
#[must_use]
pub fn track(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = super::not_installed(context) {
        return Outcome::Refuse(refusal);
    }
    if !context.repository.lfs.is_set_up {
        return Outcome::Refuse(Refusal {
            message: Text::from("Git LFS is not set up in this repository."),
            offer: Some((FeatureId("lfs.set-up"), Text::from("Set up"))),
        });
    }
    let file = match &context.target {
        Target::File(path) => Some(path.clone()),
        Target::Path(on_disk) => from_top(&context.repository.folder, on_disk),
        _ => None,
    };
    let file = file.as_deref();
    let how = match (file, answers.choice(HOW)) {
        (Some(_), Some(how)) => how,
        (Some(path), None) => return Outcome::Ask(pick(path)),
        (None, _) => "typed",
    };
    let Some(named) = named(how, file, answers.text(PATTERN)) else {
        return Outcome::Ask(typed());
    };
    let Some(to_store) = answers.names(TO_STORE) else {
        return Outcome::Read(
            TO_STORE,
            Query::LfsToStore {
                pathspecs: vec![named.pathspec.clone()],
            },
        );
    };
    let mut arguments = vec!["lfs", "track"];
    if named.is_file {
        arguments.push("--filename");
    }
    arguments.push(&named.pattern);
    Outcome::Run(Plan {
        title: Text::from(format!("Store {} with Git LFS", named.pattern)),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_TAKEN_BACK)),
        after: after(&named.pattern, to_store),
        on_failure: Vec::new(),
    })
}

/// A pattern named to Git LFS, and the pathspec that takes the files it takes.
struct Named {
    /// The pattern, or the path of **This file**.
    pattern: String,
    /// Whether it is a path that `--filename` names as it is.
    is_file: bool,
    /// The pathspec of git that takes the same files.
    pathspec: String,
}

/// The pattern an answer names; none while a pattern is to be typed.
fn named(how: &str, file: Option<&str>, typed: Option<&str>) -> Option<Named> {
    let pattern = |pattern: String| {
        Some(Named {
            pathspec: pathspec_of(&pattern),
            pattern,
            is_file: false,
        })
    };
    match (how, file) {
        ("extension", Some(path)) => pattern(format!("*.{}", extension(path)?)),
        ("folder", Some(path)) => pattern(format!("{}/**", folder(path)?)),
        ("file", Some(path)) => Some(Named {
            pattern: path.to_owned(),
            is_file: true,
            pathspec: format!(":(literal){path}"),
        }),
        _ => pattern(typed.map(str::trim).filter(|typed| !typed.is_empty())?.to_owned()),
    }
}

/// The pathspec that takes the files a pattern of a `.gitattributes` at the top takes: a pattern with no `/` but at its
/// end matches the name of a file in any folder, any other matches from the top.
fn pathspec_of(pattern: &str) -> String {
    if let Some(from_top) = pattern.strip_prefix('/') {
        format!(":(glob){from_top}")
    } else if pattern.trim_end_matches('/').contains('/') {
        format!(":(glob){pattern}")
    } else {
        format!(":(glob)**/{pattern}")
    }
}

/// The path of a file on disk from the top of the working tree, with `/`, as the Files page gives it.
fn from_top(top: &std::path::Path, on_disk: &std::path::Path) -> Option<String> {
    let inside = on_disk.strip_prefix(top).ok()?;
    let names: Vec<String> = inside
        .components()
        .map(|name| name.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!names.is_empty()).then(|| names.join("/"))
}

/// The extension of the name of a file: `psd` of `art/logo.psd`; none for `Makefile` or `.env`.
fn extension(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next()?;
    name.rsplit_once('.')
        .filter(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
        .map(|(_, extension)| extension)
}

/// The folder of a file: `art` of `art/logo.psd`; none at the top.
fn folder(path: &str) -> Option<&str> {
    path.rsplit_once('/').map(|(folder, _)| folder)
}

/// The question of how the files are named, for a file.
fn pick(path: &str) -> Question {
    let mut choices = Vec::new();
    if let Some(extension) = extension(path) {
        choices.push(choice(
            "extension",
            format!("All .{extension} files"),
            format!("*.{extension}"),
        ));
    }
    choices.push(choice("file", String::from("This file"), path.to_owned()));
    if let Some(folder) = folder(path) {
        choices.push(choice(
            "folder",
            format!("Everything in {folder}"),
            format!("{folder}/**"),
        ));
    }
    choices.push(choice(
        "typed",
        String::from("A pattern…"),
        String::from("Type a pattern"),
    ));
    let default = choices.first().map(|first| Answer::Choice(first.identifier.clone()));
    Question {
        key: HOW,
        kind: QuestionKind::Pick { choices },
        title: Text::from(TITLE),
        detail: None,
        default,
    }
}

/// One choice of the pick.
fn choice(identifier: &str, label: String, description: String) -> Choice {
    Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: Some(Text::from(description)),
    }
}

/// The question of the pattern typed.
fn typed() -> Question {
    Question {
        key: PATTERN,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("*.psd")),
            validation: Validation::Required(Text::from("Type a pattern, such as *.psd or art/**.")),
        },
        title: Text::from(TITLE),
        detail: Some(Text::from(
            "The files this pattern takes are stored with Git LFS from their next commit.",
        )),
        default: None,
    }
}

/// What is said once it ran, and the offer to store the files committed before.
fn after(pattern: &str, to_store: &[String]) -> Vec<FollowUp> {
    let mut said = format!(
        "Files that {pattern} takes are stored with Git LFS from their next commit. Commit .gitattributes with them."
    );
    if to_store.is_empty() {
        return vec![FollowUp::Say(Text::from(said))];
    }
    match to_store.len() {
        1 => said.push_str(" 1 file committed before is not stored with Git LFS yet."),
        count => {
            let _ = write!(said, " {count} files committed before are not stored with Git LFS yet.");
        }
    }
    vec![
        FollowUp::Say(Text::from(said)),
        FollowUp::OfferRun(Box::new(OfferedRun {
            label: Text::from(super::store_committed::OFFER),
            feature: FeatureId("lfs.store-committed"),
            target: Target::Head,
            answers: Answers::new().with(super::store_committed::FILES, Answer::Names(to_store.to_vec())),
            when_said: Vec::new(),
        })),
    ]
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
