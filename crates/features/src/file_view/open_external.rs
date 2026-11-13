//! `file-view.open-external`: opens the file of a File page in the editor the user works in, outside Gitardo, with the
//! command of `editor.external`, or with the program the system has for the file.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Target, Text, Undo};

/// Said when the operation is given no file of the working tree.
const CHOOSE_A_FILE: &str = "Open a file of the working tree to open it in another editor.";
/// Why there is nothing to undo.
const NOTHING_TO_UNDO: &str = "Opening a file in another editor changes nothing in the repository.";

/// Puts the absolute path of the file and the line of the cursor in the command of `editor.external`, the path after
/// it when the command does not say where, and runs it in the folder of the repository: `run <command>`. With
/// `editor.external` empty, the file is opened with the program the system has for it.
///
/// Feature: file-view.open-external
#[must_use]
pub fn open_external(context: &Context, _answers: &Answers) -> Outcome {
    let Target::FileAt(file) = &context.target else {
        return Outcome::Refuse(Refusal::saying(CHOOSE_A_FILE));
    };
    let folder = &context.repository.folder;
    let path = quoted(&super::on_disk(folder, &file.path).display().to_string());
    let template = context.settings.external_editor.trim();
    let command = if template.is_empty() {
        system_opener(&path)
    } else if template.contains("${file}") {
        template
            .replace("${file}", &path)
            .replace("${line}", &file.line.to_string())
    } else {
        format!("{} {path}", template.replace("${line}", &file.line.to_string()))
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Open {} in the external editor", file.path)),
        steps: vec![Step::Run {
            command,
            directory: folder.clone(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOTHING_TO_UNDO)),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// A path as the shell takes it as one word: in double quotes when it has a space or a character the shell reads.
fn quoted(path: &str) -> String {
    let is_plain = path
        .chars()
        .all(|character| character.is_alphanumeric() || r"/\:._-+~".contains(character));
    if is_plain {
        path.to_owned()
    } else {
        format!("\"{path}\"")
    }
}

/// The command that opens a file with the program the system has for its type.
fn system_opener(path: &str) -> String {
    if cfg!(windows) {
        format!("start \"\" {path}")
    } else if cfg!(target_os = "macos") {
        format!("open {path}")
    } else {
        format!("xdg-open {path}")
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::Path;

    use model::file_view::FileAt;

    use super::*;
    use crate::testing::{context, lines, plan, state};

    fn at_line(external: &str) -> Context {
        let mut context = context(
            state(),
            Target::FileAt(FileAt {
                path: String::from("src/app.rs"),
                line: 42,
            }),
        );
        context.settings.external_editor = external.to_owned();
        context
    }

    fn path() -> String {
        Path::new("repository").join("src").join("app.rs").display().to_string()
    }

    #[test]
    fn file_view_open_external__the_command_of_the_setting_is_given_the_path_and_the_line() {
        let plan = plan(open_external(&at_line("code --goto ${file}:${line}"), &Answers::new()));
        assert_eq!(lines(&plan), [format!("run code --goto {}:42", path())]);
        assert_eq!(plan.danger, Danger::None);
        let plan = plan_of("subl");
        assert_eq!(lines(&plan), [format!("run subl {}", path())]);
    }

    fn plan_of(external: &str) -> Plan {
        plan(open_external(&at_line(external), &Answers::new()))
    }

    #[test]
    fn file_view_open_external__with_the_setting_empty_the_system_opens_the_file() {
        let expected = if cfg!(windows) {
            format!("run start \"\" {}", path())
        } else if cfg!(target_os = "macos") {
            format!("run open {}", path())
        } else {
            format!("run xdg-open {}", path())
        };
        assert_eq!(lines(&plan_of("")), [expected]);
    }

    #[test]
    fn a_path_with_a_space_is_quoted() {
        assert_eq!(quoted(r"C:\My work\app.rs"), r#""C:\My work\app.rs""#);
        assert_eq!(quoted("/home/ada/app.rs"), "/home/ada/app.rs");
    }
}
