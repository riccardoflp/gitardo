//! `file-history.folder`: the timeline narrowed to the commits that changed anything in a folder.

use model::contract::{Answers, Context, Outcome, Refusal, Target};
use model::file_history::HistoryOf;

use super::{path_in_repository, shows};

/// Said of a folder outside the repository, or of no folder.
const CHOOSE_A_FOLDER: &str = "Choose a folder of this repository to see its history.";

/// Narrows the timeline to the commits that changed anything under a folder of the repository; the top folder of the
/// repository, whose history is all of it, shows the timeline with no narrowing. Renames are not followed: git follows
/// files only, so the history of a folder starts where it got its name.
///
/// Feature: file-history.folder
#[must_use]
pub fn folder(context: &Context, _answers: &Answers) -> Outcome {
    let Target::Path(on_disk) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FOLDER));
    };
    let Some(path) = path_in_repository(context, on_disk) else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FOLDER));
    };
    if path.is_empty() {
        return Outcome::Run(shows(String::from("Show all commits"), None));
    }
    let title = format!("Show the history of the folder {path}");
    Outcome::Run(shows(title, Some(HistoryOf::Folder { path })))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answers, Outcome, Target};
    use model::file_history::HistoryOf;

    use super::folder;
    use crate::file_history::testing::{on_disk, shown};
    use crate::testing::{context, state};

    #[test]
    fn file_history_folder__narrows_the_timeline_to_the_folder() {
        let context = context(state(), Target::Path(on_disk("src/ui")));
        assert_eq!(
            shown(folder(&context, &Answers::new())),
            Some(HistoryOf::Folder {
                path: String::from("src/ui")
            })
        );
    }

    #[test]
    fn file_history_folder__shows_the_top_folder_with_no_narrowing_and_refuses_a_folder_outside() {
        let top = context(state(), Target::Path(on_disk("")));
        assert_eq!(shown(folder(&top, &Answers::new())), None);
        let outside = context(state(), Target::Path(PathBuf::from("elsewhere/src")));
        let Outcome::Refuse(refusal) = folder(&outside, &Answers::new()) else {
            panic!("refuses a folder outside");
        };
        assert_eq!(
            refusal.message.as_str(),
            "Choose a folder of this repository to see its history."
        );
    }
}
