//! Who wrote what (`docs/features/11-history-and-blame.md`): the switches of the blame written in the user's settings,
//! the blame of a file shown or hidden in its File page, and the commit of the line under the cursor opened as a page,
//! shown in the timeline, or the file blamed as it was before it.

mod every_file;
mod ignore_whitespace;
mod open_line_commit;
mod previous;
mod reveal_line_commit;
mod toggle_current_line;
mod toggle_file;

pub use every_file::every_file;
pub use ignore_whitespace::ignore_whitespace;
pub use open_line_commit::open_line_commit;
pub use previous::previous;
pub use reveal_line_commit::reveal_line_commit;
pub use toggle_current_line::toggle_current_line;
pub use toggle_file::toggle_file;

use std::sync::Arc;

use model::blame::{BlameCommit, BlameLine, BlameQuery, LineOfFile, ShownBlame};
use model::contract::{AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Refusal, Step, Text, Undo};

/// The key of the user's settings file, read before a switch is written in it.
pub const SETTINGS: AnswerKey = AnswerKey("settings file");
/// The key of the blame of the file, read when the page shows none.
pub const BLAME: AnswerKey = AnswerKey("blame");

/// Said with no File page of a file of a repository.
const PUT_THE_CURSOR: &str = "Put the cursor on a line of a file of a repository.";
/// Why the operations that only show something have nothing to undo.
const NOTHING_TO_UNDO: &str = "Showing who changed a line changes nothing to undo.";

/// A switch of the blame written in the user's settings file, set to the opposite of what it is, keeping the rest of
/// the file: `write <user settings.json>`. It is not taken back by Undo: the switch again turns it back.
fn switch(context: &Context, answers: &Answers, (key, is_on): (&str, bool)) -> Outcome {
    let Some(file) = context.settings.user_settings_file.clone() else {
        return Outcome::Refuse(Refusal::saying(
            "Gitardo has no settings file of the user to write the switch in.",
        ));
    };
    let Some(files) = answers.files(SETTINGS) else {
        return Outcome::Read(SETTINGS, Query::FileTexts { paths: vec![file] });
    };
    let text = files
        .first()
        .and_then(Option::as_deref)
        .map_or_default(String::from_utf8_lossy);
    let path = [json::Key::Member(key.to_owned())];
    let Ok(changed) = json::set_value(&text, &path, &json::Value::Bool(!is_on)) else {
        let said = format!("The user's settings file is not JSON with comments, so {key} cannot be written in it.");
        return Outcome::Refuse(Refusal::saying(&said));
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Set {key} to {}", !is_on)),
        steps: vec![Step::Write {
            path: file,
            contents: changed.into_bytes(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Switching it again turns it back.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The file and the line an operation of the line under the cursor was invoked on, or the refusal.
fn line_of_file(context: &Context) -> Result<&LineOfFile, Box<Outcome>> {
    match &context.target {
        model::contract::Target::LineOfFile(line) => Ok(line),
        _ => Err(Box::new(Outcome::Refuse(Refusal::saying(PUT_THE_CURSOR)))),
    }
}

/// The read of the blame of the file of a page, with the text the page has when it is not saved.
fn blame_query(context: &Context, line: &LineOfFile) -> Query {
    Query::Blame(Box::new(BlameQuery {
        path: line.path.clone(),
        revision: line.revision.clone(),
        unsaved: line.unsaved.clone(),
        ignores_whitespace: context.settings.blame.ignores_whitespace,
    }))
}

/// The commit that last changed the line under the cursor: from the blame the page shows, or read; refused for a line
/// not committed yet, and for a line git has no blame for.
fn line_commit(context: &Context, answers: &Answers) -> Result<Arc<BlameCommit>, Box<Outcome>> {
    let line = line_of_file(context)?;
    let blamed = match &line.shown {
        ShownBlame::Line(shown) => shown.clone(),
        ShownBlame::Hidden => match answers.blame(BLAME) {
            None => return Err(Box::new(Outcome::Read(BLAME, blame_query(context, line)))),
            Some(lines) => lines
                .and_then(|lines| lines.get(line.line.saturating_sub(1)))
                .cloned()
                .flatten(),
        },
    };
    match blamed {
        Some(BlameLine::Committed(commit)) => Ok(commit),
        Some(BlameLine::NotCommitted) => Err(Box::new(Outcome::Refuse(Refusal::saying(
            "This line is not committed yet.",
        )))),
        None => Err(Box::new(Outcome::Refuse(Refusal::saying("This line is in no commit.")))),
    }
}

/// A plan with no step that asks the window for something once it ran.
fn no_step(title: String, after: FollowUp) -> Outcome {
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOTHING_TO_UNDO)),
        after: vec![after],
        on_failure: Vec::new(),
    })
}

/// The name of a file without its folders.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
pub(crate) mod testing {
    use std::path::PathBuf;
    use std::sync::Arc;

    use model::Timestamp;
    use model::blame::{BlameCommit, BlameLine, LineOfFile, PreviousVersion, ShownBlame};
    use model::contract::{Context, Target};

    use crate::testing::{commit, context, state};

    /// A commit of the blame, by the digit of its hash, with the commit before it when it has one.
    pub(crate) fn blamed(digit: char, previous: Option<(char, &str)>) -> BlameLine {
        BlameLine::Committed(Arc::new(BlameCommit {
            id: commit(digit),
            author: String::from("Ada Lovelace"),
            author_time: Timestamp(0),
            summary: String::from("Fix the parser"),
            previous: previous.map(|(digit, path)| PreviousVersion {
                commit: commit(digit),
                path: path.to_owned(),
            }),
        }))
    }

    /// The context of an operation on line `line` of `src/app.rs`, with the blame its page shows.
    pub(crate) fn on_line(line: usize, shown: ShownBlame) -> Context {
        let mut context = context(
            state(),
            Target::LineOfFile(Box::new(LineOfFile {
                path: String::from("src/app.rs"),
                revision: None,
                line,
                unsaved: None,
                shown,
            })),
        );
        context.settings.user_settings_file = Some(PathBuf::from("settings.json"));
        context
    }
}
