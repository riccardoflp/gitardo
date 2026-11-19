//! `changes.ignore`: has git ignore a file or a folder by a pattern written where the user chooses, and offers to stop
//! tracking it too when git tracks it.

use model::changes::{IgnoreChoices, IgnoreInput, IgnorePattern, IgnorePlace, ignore_patterns, with_line};
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Query, Question, QuestionKind, Refusal,
    Step, Text, Validation,
};
use model::worktree_setup::{PathStatus, Tracked};

use super::files::plan;
use super::stop_tracking::{PATH_STATUS, one_path, stays_on_disk, untrack_steps};

/// The key of the pattern chosen: the place of a pattern offered in the list, or [`TYPED_CHOICE`].
pub const WHAT: AnswerKey = AnswerKey("ignore-what");
/// The key of the pattern typed.
pub const TYPED: AnswerKey = AnswerKey("ignore-typed");
/// The key of the place chosen, by its place in the list.
pub const WHERE: AnswerKey = AnswerKey("ignore-where");
/// The key of what is done with a path git tracks: [`STOP_TRACKING`] or [`ONLY_WRITE`].
pub const TRACKED: AnswerKey = AnswerKey("ignore-tracked");
/// The key of the file and the pattern that ignore the path already.
const IGNORED_BY: AnswerKey = AnswerKey("ignore-ignored-by");
/// The key of the exclude file of the repository.
const EXCLUDE: AnswerKey = AnswerKey("ignore-exclude-file");
/// The key of the text of the file the line goes in.
const FILE_TEXT: AnswerKey = AnswerKey("ignore-file-text");

/// The choice of a pattern typed.
pub const TYPED_CHOICE: &str = "typed";
/// The choice that stops tracking the path too.
pub const STOP_TRACKING: &str = "stop-tracking";
/// The choice that only writes the pattern.
pub const ONLY_WRITE: &str = "only-write";

/// What a plan of ignoring is made of, once every question is answered.
struct Ignoring<'a> {
    path: &'a str,
    pattern: IgnorePattern,
    place: IgnorePlace,
    status: PathStatus,
}

/// Writes the line of the pattern chosen at the end of the file chosen, as one `write` step; on a path git tracks, then
/// takes it out of the index when the user says so (`changes.stop-tracking`).
///
/// Feature: changes.ignore
#[must_use]
pub fn ignore(context: &Context, answers: &Answers) -> Outcome {
    let path = match one_path(context) {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let Some(ignored_by) = answers.names(IGNORED_BY) else {
        return Outcome::Read(IGNORED_BY, Query::IgnoredBy { path });
    };
    if let [file, pattern] = ignored_by {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{path} is ignored already, by {pattern} in {file}."
        )));
    }
    let Some(Answer::PathStatuses(statuses)) = answers.get(PATH_STATUS) else {
        return Outcome::Read(PATH_STATUS, Query::PathStatuses { paths: vec![path] });
    };
    let Some(Answer::Path(exclude)) = answers.get(EXCLUDE) else {
        return Outcome::Read(EXCLUDE, Query::ExcludeFile);
    };
    let status = statuses.first().copied().unwrap_or(PathStatus {
        is_folder: false,
        tracked: Tracked::Nothing,
        ignored: model::worktree_setup::Ignored::Nothing,
    });
    let choices = ignore_patterns(&IgnoreInput {
        path: path.clone(),
        is_folder: status.is_folder,
        exclude: exclude.clone(),
    });
    let (pattern, place) = match chosen(&path, &choices, answers) {
        Ok(chosen) => chosen,
        Err(outcome) => return *outcome,
    };
    if status.tracked != Tracked::Nothing && answers.choice(TRACKED).is_none() {
        return Outcome::Ask(tracked_question(&path));
    }
    written(
        context,
        answers,
        &Ignoring {
            path: &path,
            pattern,
            place,
            status,
        },
    )
}

/// The pattern and the place chosen, asking for each in turn.
fn chosen(
    path: &str,
    choices: &IgnoreChoices,
    answers: &Answers,
) -> Result<(IgnorePattern, IgnorePlace), Box<Outcome>> {
    let Some(what) = answers.choice(WHAT) else {
        return Err(Box::new(Outcome::Ask(what_question(path, choices))));
    };
    let pattern = if what == TYPED_CHOICE {
        let Some(typed) = answers.text(TYPED) else {
            return Err(Box::new(Outcome::Ask(typed_question())));
        };
        let typed = typed.trim_end();
        if typed.trim().is_empty() || typed.contains(['\n', '\r']) {
            return Err(Box::new(Outcome::Refuse(Refusal::saying(
                "Enter a pattern of one line.",
            ))));
        }
        IgnorePattern::typed(typed)
    } else {
        let offered = what.parse::<usize>().ok().and_then(|index| choices.patterns.get(index));
        let Some(offered) = offered else {
            return Err(Box::new(Outcome::Ask(what_question(path, choices))));
        };
        offered.clone()
    };
    let places: Vec<(usize, &IgnorePlace, String)> = choices
        .places
        .iter()
        .enumerate()
        .filter_map(|(index, place)| pattern.line_in(place).map(|line| (index, place, line)))
        .collect();
    let place = answers
        .choice(WHERE)
        .and_then(|chosen| places.iter().find(|(index, ..)| index.to_string() == chosen));
    let Some((_, place, _)) = place else {
        return Err(Box::new(Outcome::Ask(where_question(&places))));
    };
    Ok((pattern, (*place).clone()))
}

/// The plan, once the text of the file chosen is read.
fn written(context: &Context, answers: &Answers, ignoring: &Ignoring) -> Outcome {
    let folder = &context.repository.folder;
    let file = if ignoring.place.is_exclude {
        ignoring.place.file.clone()
    } else {
        folder.join(&ignoring.place.file)
    };
    let Some(texts) = answers.files(FILE_TEXT) else {
        return Outcome::Read(FILE_TEXT, Query::FileTexts { paths: vec![file] });
    };
    let text = texts
        .first()
        .cloned()
        .flatten()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    let line = ignoring.pattern.line_in(&ignoring.place).unwrap_or_default();
    let shown_file = ignoring.place.file.display().to_string().replace('\\', "/");
    let mut steps = Vec::new();
    if let Some(contents) = with_line(text.as_deref(), &line) {
        steps.push(Step::Write {
            path: file,
            contents: contents.into_bytes(),
        });
    }
    let stopping = ignoring.status.tracked != Tracked::Nothing && answers.choice(TRACKED) == Some(STOP_TRACKING);
    let mut danger = Danger::None;
    let mut on_failure = Vec::new();
    if stopping {
        let (step, offer) = untrack_steps(folder.clone(), ignoring.path, ignoring.status.is_folder);
        steps.push(step);
        on_failure.push(offer);
        danger = Danger::Confirm(Text::from(stays_on_disk(ignoring.path)));
    }
    if steps.is_empty() {
        return Outcome::Refuse(Refusal::saying(&format!("{shown_file} has {line} already.")));
    }
    let mut ignored = plan(
        format!("Ignore {}", ignoring.path),
        steps,
        danger,
        &format!("Not taken back by Undo: take the line {line} out of {shown_file}."),
    );
    ignored.after.push(FollowUp::Say(Text::from(format!(
        "{} is ignored by {line} in {shown_file}.",
        ignoring.path
    ))));
    ignored.on_failure = on_failure;
    Outcome::Run(ignored)
}

/// **Ignore what**: the patterns offered, the first by default, and a pattern typed.
fn what_question(path: &str, choices: &IgnoreChoices) -> Question {
    let mut offered: Vec<Choice> = choices
        .patterns
        .iter()
        .enumerate()
        .map(|(index, pattern)| choice(index.to_string(), &pattern.words, Some(&pattern.shown)))
        .collect();
    offered.push(choice(String::from(TYPED_CHOICE), "A pattern typed…", None));
    Question {
        key: WHAT,
        kind: QuestionKind::Pick { choices: offered },
        title: Text::from(format!("Ignore what of {path}")),
        detail: None,
        default: Some(Answer::Choice(String::from("0"))),
    }
}

/// The pattern typed.
fn typed_question() -> Question {
    Question {
        key: TYPED,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("tmp-*")),
            validation: Validation::Required(Text::from("Enter a pattern.")),
        },
        title: Text::from("The pattern, as a .gitignore holds it"),
        detail: None,
        default: None,
    }
}

/// **Where**: the places a line can go in, each with the line it writes there, the `.gitignore` of the repository by
/// default.
fn where_question(places: &[(usize, &IgnorePlace, String)]) -> Question {
    let choices = places
        .iter()
        .map(|(index, place, line)| {
            let description = if place.is_exclude {
                format!("{line} · Not committed, and not shared with anyone")
            } else {
                line.clone()
            };
            choice(index.to_string(), &place.words, Some(&description))
        })
        .collect();
    let first = places.first().map_or_else(String::new, |(index, ..)| index.to_string());
    Question {
        key: WHERE,
        kind: QuestionKind::Pick { choices },
        title: Text::from("Where to write it"),
        detail: None,
        default: Some(Answer::Choice(first)),
    }
}

/// The question asked when git tracks the path: stop tracking it too, or only write the pattern.
fn tracked_question(path: &str) -> Question {
    Question {
        key: TRACKED,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(
                    String::from(STOP_TRACKING),
                    "Stop tracking it, and keep it on disk",
                    None,
                ),
                choice(String::from(ONLY_WRITE), "Only write the pattern", None),
            ],
        },
        title: Text::from(format!(
            "Git tracks {path}: a pattern does not stop git from following it."
        )),
        detail: None,
        default: Some(Answer::Choice(String::from(STOP_TRACKING))),
    }
}

/// One choice of a pick.
fn choice(identifier: String, label: &str, description: Option<&str>) -> Choice {
    Choice {
        identifier,
        label: Text::from(label),
        description: description.map(Text::from),
    }
}

#[cfg(test)]
#[path = "ignore_tests.rs"]
mod tests;
