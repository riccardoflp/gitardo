//! `changes.save`: **Save all** on the Changes page, which writes the edits typed in Gitardo's editors and not saved
//! yet, so that git sees them.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Target, Text, Undo};

/// Said when the operation is given no file to save.
const NOTHING_TO_SAVE: &str = "No file has edits not saved.";
/// Why Undo does not take it back.
const NOT_UNDONE: &str = "Ctrl+Z in the page of a file takes back its edits; Undo does not take back the save.";

/// Writes each file given with the text of its page, in its own line endings and encoding: `write <file>` for each.
///
/// Feature: changes.save
#[must_use]
pub fn save(context: &Context, _answers: &Answers) -> Outcome {
    let Target::FileSaves(files) = &context.target else {
        return Outcome::Refuse(Refusal::saying(NOTHING_TO_SAVE));
    };
    if files.is_empty() {
        return Outcome::Refuse(Refusal::saying(NOTHING_TO_SAVE));
    }
    let steps = files
        .iter()
        .map(|file| Step::Write {
            path: crate::file_view::on_disk(&context.repository.folder, &file.path),
            contents: file.contents.clone(),
        })
        .collect();
    let title = match files.as_slice() {
        [file] => format!("Save {}", file.path.rsplit('/').next().unwrap_or(&file.path)),
        _ => format!("Save {} files", files.len()),
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_UNDONE)),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, Danger, Outcome, Step, Target};
    use model::file_view::FileSave;

    use super::save;
    use crate::testing::{context, lines, plan, state};

    fn file(path: &str, contents: &str) -> FileSave {
        FileSave {
            path: path.to_owned(),
            contents: contents.as_bytes().to_vec(),
            read: Vec::new(),
        }
    }

    #[test]
    fn changes_save__two_files_are_written_with_one_write_line_each() {
        let files = vec![file("src/app.rs", "fn main() {}\r\n"), file("notes.txt", "two\n")];
        let plan = plan(save(&context(state(), Target::FileSaves(files)), &Answers::new()));
        let at = |parts: &[&str]| {
            let mut path = std::path::PathBuf::from("repository");
            path.extend(parts);
            format!("write {}", path.display())
        };
        assert_eq!(lines(&plan), [at(&["src", "app.rs"]), at(&["notes.txt"])]);
        let Some(Step::Write { contents, .. }) = plan.steps.first() else {
            panic!("a write")
        };
        assert_eq!(contents, b"fn main() {}\r\n");
        assert_eq!(plan.title.as_str(), "Save 2 files");
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn changes_save__given_no_file_it_refuses() {
        let none = context(state(), Target::FileSaves(Vec::new()));
        assert!(matches!(save(&none, &Answers::new()), Outcome::Refuse(_)));
        let other = context(state(), Target::Head);
        assert!(matches!(save(&other, &Answers::new()), Outcome::Refuse(_)));
    }
}
