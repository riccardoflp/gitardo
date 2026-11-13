//! `file-view.save`: writes the text of a File page to the disk, after checking that no other program changed the file
//! since the page read it.

use model::contract::{
    AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step, Target, Text,
    Undo,
};
use model::file_view::FileSave;

/// The key under which the bytes on disk come back.
pub const ON_DISK: AnswerKey = AnswerKey("on disk");
/// The key of **Overwrite**, when the file changed on disk.
pub const OVERWRITE: AnswerKey = AnswerKey("overwrite");

/// Said when the operation is given no file to save.
const CHOOSE_A_FILE: &str = "Open a file to save it.";
/// Why Undo does not take it back.
const NOT_UNDONE: &str = "Ctrl+Z in the page takes back the edits; Undo does not take back the save.";

/// Reads the file on disk; when it is what the page read, or **Overwrite** is answered, writes the text of the page in
/// its own line endings and encoding: `write <file>`.
///
/// Feature: file-view.save
#[must_use]
pub fn save(context: &Context, answers: &Answers) -> Outcome {
    let Target::FileSave(file) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FILE));
    };
    let path = context.repository.folder.join(&file.path);
    let Some(on_disk) = answers.contents(ON_DISK) else {
        return Outcome::Read(ON_DISK, Query::FileContents { path });
    };
    if on_disk != file.read.as_slice() && !answers.is_confirmed(OVERWRITE) {
        return Outcome::Ask(changed_question(file));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Save {}", name_of(&file.path))),
        steps: vec![Step::Write {
            path,
            contents: file.contents.clone(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_UNDONE)),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// **app.rs changed on disk since it was opened.**, with **Overwrite**; **Cancel** writes nothing.
fn changed_question(file: &FileSave) -> Question {
    Question {
        key: OVERWRITE,
        kind: QuestionKind::Confirm {
            accept: Text::from("Overwrite"),
        },
        title: Text::from(format!("{} changed on disk since it was opened.", name_of(&file.path))),
        detail: None,
        default: None,
    }
}

/// The name of a file, without its folders.
fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::Answer;

    use super::*;
    use crate::testing::{context, lines, plan, question, state};

    fn file(contents: &str, read: &str) -> Context {
        let file = FileSave {
            path: String::from("src/app.rs"),
            contents: contents.as_bytes().to_vec(),
            read: read.as_bytes().to_vec(),
        };
        context(state(), Target::FileSave(Box::new(file)))
    }

    #[test]
    fn file_view_save__the_file_unchanged_on_disk_is_written_with_one_write_line() {
        let context = file("fn main() {}\r\n", "fn main(){}\r\n");
        assert!(matches!(save(&context, &Answers::new()), Outcome::Read(ON_DISK, _)));
        let read = Answers::new().with(ON_DISK, Answer::Contents(b"fn main(){}\r\n".to_vec()));
        let plan = plan(save(&context, &read));
        assert_eq!(
            lines(&plan),
            [format!(
                "write {}",
                std::path::Path::new("repository").join("src/app.rs").display()
            )]
        );
        let Some(Step::Write { contents, .. }) = plan.steps.first() else {
            panic!("a write")
        };
        assert_eq!(contents, b"fn main() {}\r\n");
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn file_view_save__a_file_changed_on_disk_asks_and_overwrite_writes_it() {
        let context = file("mine\n", "before\n");
        let read = Answers::new().with(ON_DISK, Answer::Contents(b"theirs\n".to_vec()));
        let asked = question(save(&context, &read));
        assert_eq!(asked.title.as_str(), "app.rs changed on disk since it was opened.");
        let overwrite = read.with(OVERWRITE, Answer::Confirmed);
        assert_eq!(plan(save(&context, &overwrite)).steps.len(), 1);
    }

    #[test]
    fn file_view_save__given_no_file_it_refuses() {
        let context = context(state(), Target::Head);
        assert!(matches!(save(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
