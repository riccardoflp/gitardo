//! `conflicts.take-side`: resolves a whole file for one side, when that side is simply right.

use std::ffi::OsString;

use model::changes::pathspec;
use model::conflicts::{ConflictTexts, FileStages, SideNamesInput, SideRole, side_names};
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, Outcome, Plan, Query, Question, QuestionKind, Step, Text, Undo,
    UndoRecord,
};

use super::{refuse, target_file};

/// Which side: `current` or `incoming`.
pub const SIDE: AnswerKey = AnswerKey("side");

/// The versions the index keeps of the file.
pub const STAGES: AnswerKey = AnswerKey("stages");

/// The files in conflict with their sides, to tell whether lines were typed in the file.
const TEXTS: AnswerKey = AnswerKey("texts");

/// How the lines of git's markers start.
const MARKERS: [&str; 4] = ["<<<<<<<", "|||||||", "=======", ">>>>>>>"];

/// The two sides, as the choices of [`SIDE`] name them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Taken {
    Current,
    Incoming,
}

impl Taken {
    /// The words of the side: `current`, `incoming`.
    fn words(self) -> &'static str {
        match self {
            Taken::Current => "current",
            Taken::Incoming => "incoming",
        }
    }
}

/// Makes a file in conflict what one side has: `git checkout --ours -- <file>` (or `--theirs`) then `git add`, or
/// `git rm` when that side deleted it. It asks first, since the changes the other side made to it are left out, and
/// warns when the file has lines that neither side nor the base has, which were typed in it and are lost too. Undo
/// marks its conflicts again when git still had both sides.
///
/// Feature: conflicts.take-side
#[must_use]
pub fn take_side(context: &Context, answers: &Answers) -> Outcome {
    let Some(path) = target_file(context) else {
        return refuse("Choose a file in conflict first.");
    };
    let taken = match answers.choice(SIDE) {
        Some("current") => Taken::Current,
        Some("incoming") => Taken::Incoming,
        _ => return Outcome::Ask(side_question(path)),
    };
    let stages = match answers.stages(STAGES) {
        None => return Outcome::Read(STAGES, Query::ConflictStages { path: path.to_owned() }),
        Some(None) => return refuse(&format!("{path} is not in conflict.")),
        Some(Some(stages)) => stages,
    };
    let Some(texts) = answers.conflict_texts(TEXTS) else {
        return Outcome::Read(TEXTS, Query::ConflictTexts);
    };
    let typed = texts.iter().find(|text| text.path == path).is_some_and(has_typed_lines);
    Outcome::Run(plan(context, (path, stages, typed), taken))
}

/// Whether the file on disk has a line that is in none of the base and the two sides and is not one of git's markers:
/// a line typed in it since git stopped.
fn has_typed_lines(texts: &ConflictTexts) -> bool {
    let lines_of = |bytes: Option<&[u8]>| -> Vec<String> {
        bytes.map_or_else(Vec::new, |bytes| {
            model::review::split_lines(&String::from_utf8_lossy(bytes))
                .into_iter()
                .map(|(line, _)| line.to_owned())
                .collect()
        })
    };
    let known: std::collections::HashSet<String> = [&texts.base, &texts.current, &texts.incoming]
        .into_iter()
        .flat_map(|side| lines_of(side.as_deref()))
        .collect();
    lines_of(texts.file.as_deref())
        .iter()
        .any(|line| !known.contains(line) && !MARKERS.iter().any(|marker| line.starts_with(marker)))
}

/// Which side to take, when the button did not say.
fn side_question(path: &str) -> Question {
    let choice = |identifier: &str, label: &str| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: None,
    };
    Question {
        key: SIDE,
        kind: QuestionKind::Pick {
            choices: vec![
                choice("current", "Take the current side for the whole file"),
                choice("incoming", "Take the incoming side for the whole file"),
            ],
        },
        title: Text::from(format!("Resolve {path}")),
        detail: None,
        default: None,
    }
}

/// The plan of a side taken.
fn plan(context: &Context, (path, stages, typed): (&str, &FileStages, bool), taken: Taken) -> Plan {
    let folder = context.repository.folder.clone();
    let words = |list: &[&str]| -> Vec<OsString> {
        let mut arguments: Vec<OsString> = list.iter().map(OsString::from).collect();
        arguments.push(pathspec(path));
        arguments
    };
    let has_file = match taken {
        Taken::Current => stages.has_current,
        Taken::Incoming => stages.has_incoming,
    };
    let side = taken.words();
    let role = lower_first(role(context, taken).words());
    let (title, steps, reason) = if has_file {
        let flag = if taken == Taken::Current { "--ours" } else { "--theirs" };
        (
            format!("Take the {side} side of {path}"),
            vec![
                Step::git(folder.clone(), words(&["checkout", flag, "--"])),
                Step::git(folder, words(&["add", "--"])),
            ],
            format!(
                "The {side} side is {role}. The whole file becomes what that side has: the changes the other side \
                 made to it are left out."
            ),
        )
    } else {
        (
            format!("Delete {path}, as the {side} side did"),
            vec![Step::git(folder, words(&["rm", "--"]))],
            format!(
                "The {side} side is {role}, which has no such file. The changes the other side made to it are left \
                 out."
            ),
        )
    };
    let undo = if stages.has_both_sides() {
        Undo::Journal(UndoRecord::ConflictsResolved {
            files: vec![path.to_owned()],
        })
    } else {
        Undo::Impossible(Text::from(format!(
            "{path} was deleted by one side, so git cannot mark its conflicts again."
        )))
    };
    let danger = if typed {
        Danger::Destructive(Text::from(format!(
            "{reason} {path} has lines that neither side has, typed in it since git stopped: they are lost too."
        )))
    } else {
        Danger::Confirm(Text::from(reason))
    };
    Plan {
        title: Text::from(title),
        steps,
        danger,
        undo,
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// What the side taken is in the operation in progress (`conflicts.side-names`).
fn role(context: &Context, taken: Taken) -> SideRole {
    let names = side_names(&SideNamesInput {
        operation: context.repository.operation,
        checked_out: context.repository.checked_out_branch(),
        ..SideNamesInput::default()
    });
    match taken {
        Taken::Current => names.current.role,
        Taken::Incoming => names.incoming.role,
    }
}

/// A sentence put inside another: its first letter in lower case.
fn lower_first(words: &str) -> String {
    let mut characters = words.chars();
    characters
        .next()
        .map_or_else(String::new, |first| first.to_lowercase().chain(characters).collect())
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
