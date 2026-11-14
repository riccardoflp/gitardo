//! `worktree.open-in-editor`: opens a worktree's folder in the editor of `editor.external`.

use model::contract::{Answers, Context, Outcome, Refusal};

use super::{label, opening, refuse_missing, target_worktree};

/// Said when `editor.external` is not set.
const NO_EDITOR: &str = "Set editor.external to the command of your editor to open a worktree in it.";

/// Starts the editor of `editor.external` on the folder of the worktree invoked on, or of the worktree that has the
/// branch invoked on: `run <command>`. When the editor cannot be started, what the system said is shown.
///
/// Feature: worktree.open-in-editor
#[must_use]
pub fn open_in_editor(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if let Some(refusal) = refuse_missing(worktree) {
        return refusal;
    }
    let Some(command) = opening::editor_command(context, &worktree.folder) else {
        return Outcome::Refuse(Refusal::saying(NO_EDITOR));
    };
    Outcome::Run(opening::in_editor(
        command,
        &worktree.folder,
        &label(context, &worktree.folder),
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, Outcome, Target};

    use super::open_in_editor;
    use crate::testing::{lines, plan};
    use crate::worktree::testing::{LOGIN, with_worktrees};

    fn folder() -> String {
        if cfg!(windows) {
            LOGIN.replace('/', "\\")
        } else {
            LOGIN.to_owned()
        }
    }

    #[test]
    fn worktree_open_in_editor__the_editor_is_started_with_the_worktrees_folder() {
        let mut context = with_worktrees(Target::Worktree(LOGIN.into()));
        context.settings.external_editor = String::from("code --goto ${file}:${line}");
        let set = plan(open_in_editor(&context, &Answers::new()));
        assert_eq!(lines(&set), [format!("run code {}", folder())]);
        context.settings.external_editor = String::from("\"C:/Program Files/Zed/zed.exe\" --new ${folder}");
        let with_folder = plan(open_in_editor(&context, &Answers::new()));
        assert_eq!(
            lines(&with_folder),
            [format!("run \"C:/Program Files/Zed/zed.exe\" --new {}", folder())]
        );
    }

    #[test]
    fn without_an_editor_set_it_is_refused() {
        let context = with_worktrees(Target::Worktree(LOGIN.into()));
        assert!(matches!(open_in_editor(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
