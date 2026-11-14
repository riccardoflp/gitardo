//! `worktree-setup.edit-settings-file`: the settings file of the place chosen opened in a File page, to edit the
//! patterns and the commands by hand.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Refusal, Step, Target, Text, Undo};
use model::worktree_setup::SettingsPlace;

use super::files::{LINK_FOLDERS, NO_USER_FILE, SettingsFile, read_files};

/// What Undo says of the file it made.
const NOT_UNDONE: &str = "A settings file made is not taken back by Undo.";

/// Opens the settings file of the place chosen in a File page, at `worktrees.linkFolders` when it is set: the
/// repository's `.gitardo/settings.json`, made first with an empty object when it is not there, or the user's file.
///
/// Feature: worktree-setup.edit-settings-file
#[must_use]
pub fn edit_settings_file(context: &Context, answers: &Answers) -> Outcome {
    let place = match &context.target {
        Target::SetupSettings(save) => save.place,
        _ => SettingsPlace::Repository,
    };
    let files = match read_files(context, answers) {
        Ok(files) => files,
        Err(outcome) => return *outcome,
    };
    let Some(file) = files.of(place) else {
        return Outcome::Refuse(Refusal::saying(NO_USER_FILE));
    };
    let steps = match (place, &file.text) {
        (SettingsPlace::Repository, None) => vec![Step::Write {
            path: file.path.clone(),
            contents: b"{}\n".to_vec(),
        }],
        _ => Vec::new(),
    };
    let path = match place {
        SettingsPlace::Repository => String::from(".gitardo/settings.json"),
        SettingsPlace::User => file.path.display().to_string(),
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Edit {path}")),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_UNDONE)),
        after: vec![FollowUp::EditFile(path, line_of_link_folders(file))],
        on_failure: Vec::new(),
    })
}

/// The line, from 1, where `worktrees.linkFolders` is set in a file; the first line when it is not.
fn line_of_link_folders(file: &SettingsFile) -> usize {
    let quoted = format!("\"{LINK_FOLDERS}\"");
    file.text
        .as_deref()
        .and_then(|text| text.lines().position(|line| line.contains(&quoted)))
        .map_or(1, |index| index + 1)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{FollowUp, Step};
    use model::worktree_setup::{SettingsPlace, SetupSave};

    use super::edit_settings_file;
    use crate::testing::plan;
    use crate::worktree_setup::save::tests::{files, saving};

    fn at(place: SettingsPlace) -> model::contract::Context {
        saving(SetupSave {
            place,
            ..SetupSave::default()
        })
    }

    #[test]
    fn worktree_setup_edit_settings_file__without_the_repository_file_it_makes_it_with_an_empty_object_and_opens_it() {
        let plan = plan(edit_settings_file(
            &at(SettingsPlace::Repository),
            &files(None, None, None),
        ));
        let [Step::Write { path, contents }] = plan.steps.as_slice() else {
            panic!("one file written: {plan:?}")
        };
        assert_eq!(
            path,
            &PathBuf::from("repository").join(".gitardo").join("settings.json")
        );
        assert_eq!(contents.as_slice(), b"{}\n");
        assert_eq!(
            plan.after,
            [FollowUp::EditFile(String::from(".gitardo/settings.json"), 1)]
        );
    }

    #[test]
    fn worktree_setup_edit_settings_file__with_user_chosen_the_users_file_opens_at_link_folders() {
        let user = "{\n  \"graph.pageSize\": 500,\n  \"worktrees.linkFolders\": [\"node_modules/\"]\n}\n";
        let plan = plan(edit_settings_file(
            &at(SettingsPlace::User),
            &files(Some("{}"), None, Some(user)),
        ));
        assert_eq!(plan.steps, []);
        let whole = PathBuf::from("user").join("settings.json").display().to_string();
        assert_eq!(plan.after, [FollowUp::EditFile(whole, 3)]);
    }
}
