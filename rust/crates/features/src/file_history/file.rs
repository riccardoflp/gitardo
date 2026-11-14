//! `file-history.file`: the timeline narrowed to the commits that changed a file, under every name it had.

use model::contract::{AnswerKey, Answers, Context, Outcome, Query, Refusal, Target};
use model::file_history::HistoryOf;

use super::{path_in_repository, shows};

/// The key of the names the file had.
pub const NAMES: AnswerKey = AnswerKey("names");

/// Said of a file outside the repository, or of no file.
const OPEN_A_FILE: &str = "Open a file of this repository to see its history.";

/// Narrows the timeline to the commits that changed a file of the repository, under its name and every name it had
/// before (`file-history.read-names`), on the branches the timeline shows.
///
/// Feature: file-history.file
#[must_use]
pub fn file(context: &Context, answers: &Answers) -> Outcome {
    let Target::Path(on_disk) = &context.target else {
        return Outcome::Refuse(Refusal::saying(OPEN_A_FILE));
    };
    let Some(path) = path_in_repository(context, on_disk).filter(|path| !path.is_empty()) else {
        return Outcome::Refuse(Refusal::saying(OPEN_A_FILE));
    };
    let Some(names) = answers.names(NAMES) else {
        return Outcome::Read(NAMES, Query::FileNames { path });
    };
    let earlier = names.iter().filter(|name| **name != path).cloned().collect();
    let title = format!("Show the history of {path}");
    Outcome::Run(shows(title, Some(HistoryOf::File { path, earlier })))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Outcome, Query, Target};
    use model::file_history::HistoryOf;

    use super::{NAMES, file};
    use crate::file_history::testing::{on_disk, shown};
    use crate::testing::{context, state};

    #[test]
    fn file_history_file__reads_the_names_of_the_file_then_shows_the_commits_of_each() {
        let context = context(state(), Target::Path(on_disk("src/b.rs")));
        let read = file(&context, &Answers::new());
        assert_eq!(
            read,
            Outcome::Read(
                NAMES,
                Query::FileNames {
                    path: String::from("src/b.rs")
                }
            )
        );
        let names = vec![String::from("src/b.rs"), String::from("src/a.rs")];
        let answers = Answers::new().with(NAMES, Answer::Names(names));
        assert_eq!(
            shown(file(&context, &answers)),
            Some(HistoryOf::File {
                path: String::from("src/b.rs"),
                earlier: vec![String::from("src/a.rs")],
            })
        );
    }

    #[test]
    fn file_history_file__refuses_a_file_outside_the_repository() {
        let outside = context(state(), Target::Path(std::path::PathBuf::from("elsewhere/b.rs")));
        let Outcome::Refuse(refusal) = file(&outside, &Answers::new()) else {
            panic!("refuses a file outside the repository");
        };
        assert_eq!(
            refusal.message.as_str(),
            "Open a file of this repository to see its history."
        );
        let top = context(state(), Target::Path(on_disk("")));
        assert!(matches!(file(&top, &Answers::new()), Outcome::Refuse(_)));
        let nothing = context(state(), Target::Head);
        assert!(matches!(file(&nothing, &Answers::new()), Outcome::Refuse(_)));
    }
}
