//! The four entries of the Files page that give a file or a folder to new worktrees, or take it from them:
//! `worktree-setup.link-into-new-worktrees`, `worktree-setup.copy-into-new-worktrees`, `worktree-setup.stop-linking`
//! and `worktree-setup.stop-copying`.

use json::Value;
use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FollowUp, Outcome, Plan, Query, Refusal, Step, Target,
    Text, Undo,
};
use model::worktree_setup::{
    ExplainInput, PathStatus, SettingsPlace, SetupAction, SetupPatterns, SetupPreview, TakeRequest, change_patterns,
    explain_change,
};

use super::files::{COPY_FILES, LINK_FOLDERS, NO_USER_FILE, SettingsFiles, edited, list_value, patterns_of, place_of};
use super::files::{host_target, host_target_words, place_words, read_files, setting_written, write_step};

/// The key of what git says of the paths chosen.
const PATH_STATUSES: AnswerKey = AnswerKey("worktree-setup-path-statuses");
/// The key of what the patterns take, before the change and after it.
const BEFORE_AND_AFTER: AnswerKey = AnswerKey("worktree-setup-before-and-after");

/// The entry that opens the page, offered once a choice is saved.
pub const CHOOSE_ON_THE_PAGE: &str = "Choose What New Worktrees Get…";

/// Has new worktrees link the folders chosen in the Files page, by the pattern of their path.
///
/// Feature: worktree-setup.link-into-new-worktrees
#[must_use]
pub fn link_into_new_worktrees(context: &Context, answers: &Answers) -> Outcome {
    change(context, answers, SetupAction::Link)
}

/// Has new worktrees get a copy of the files and folders chosen in the Files page, by the pattern of their path.
///
/// Feature: worktree-setup.copy-into-new-worktrees
#[must_use]
pub fn copy_into_new_worktrees(context: &Context, answers: &Answers) -> Outcome {
    change(context, answers, SetupAction::Copy)
}

/// Takes the link patterns of the paths of the folders chosen in the Files page out.
///
/// Feature: worktree-setup.stop-linking
#[must_use]
pub fn stop_linking(context: &Context, answers: &Answers) -> Outcome {
    change(context, answers, SetupAction::StopLinking)
}

/// Takes the copy patterns of the paths of the files and folders chosen in the Files page out.
///
/// Feature: worktree-setup.stop-copying
#[must_use]
pub fn stop_copying(context: &Context, answers: &Answers) -> Outcome {
    change(context, answers, SetupAction::StopCopying)
}

/// The paths chosen, relative to the repository, with `/`.
fn chosen_paths(target: &Target) -> Vec<String> {
    let paths = match target {
        Target::Files(paths) => paths.clone(),
        Target::File(path) => vec![path.clone()],
        _ => Vec::new(),
    };
    paths
        .into_iter()
        .map(|path| path.replace('\\', "/").trim_matches('/').to_owned())
        .collect()
}

/// A path that is not in the repository the operation runs in.
fn outside(path: &str) -> bool {
    path.is_empty() || path == ".." || path.starts_with("../") || path.contains(':') || path.starts_with('/')
}

/// The change for one action, on every path chosen.
fn change(context: &Context, answers: &Answers, action: SetupAction) -> Outcome {
    let paths = chosen_paths(&context.target);
    if let Some(path) = paths.iter().find(|path| outside(path)) {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{path} is in no repository of the workspace."
        )));
    }
    // Inside VS Code, which gives the settings, no file is read: VS Code writes them where they are set.
    let (files, settings) = match &context.settings.worktrees.setup {
        Some(given) => (None, given.clone()),
        None => match read_files(context, answers) {
            Ok(files) => {
                let settings = files.in_effect();
                (Some(files), settings)
            }
            Err(outcome) => return *outcome,
        },
    };
    let Some(Answer::PathStatuses(statuses)) = answers.get(PATH_STATUSES) else {
        return Outcome::Read(PATH_STATUSES, Query::PathStatuses { paths });
    };
    let chosen: Vec<(&String, &PathStatus)> = paths.iter().zip(statuses).collect();
    let before = patterns_of(&settings);
    let after = chosen.iter().fold(before.clone(), |patterns, (path, status)| {
        change_patterns(&patterns, (path, status.is_folder), action)
    });
    let Some(Answer::SetupTakes(takes)) = answers.get(BEFORE_AND_AFTER) else {
        let request = |patterns: SetupPatterns| TakeRequest {
            folder: context.repository.folder.clone(),
            patterns,
            commit: None,
        };
        return Outcome::Read(
            BEFORE_AND_AFTER,
            Query::SetupTakes(vec![request(before), request(after)]),
        );
    };
    let empty = SetupPreview::default();
    let (taken_before, taken_after) = (takes.first().unwrap_or(&empty), takes.get(1).unwrap_or(&empty));
    let mut sentences = Vec::new();
    let mut changed = before.clone();
    for (path, status) in &chosen {
        let explained = explain_change(&ExplainInput {
            path,
            action,
            tracked: status.tracked,
            ignored: status.ignored,
            before: taken_before,
            after: taken_after,
        });
        if explained.writes {
            changed = change_patterns(&changed, (path, status.is_folder), action);
        }
        sentences.push(explained.sentence);
    }
    let said = sentences.join(" ");
    let sources = (
        place_of(settings.link_folders.source),
        place_of(settings.copy_files.source),
    );
    saved_change(context, files, (&before, &changed), sources, (action, &said))
}

/// The plan of a change once the patterns are worked out: the steps that write Gitardo's files, or inside VS Code the
/// settings it writes; refused, saying what became of each path, when nothing changes.
fn saved_change(
    context: &Context,
    files: Option<SettingsFiles>,
    (before, changed): (&SetupPatterns, &SetupPatterns),
    sources: (SettingsPlace, SettingsPlace),
    (action, said): (SetupAction, &str),
) -> Outcome {
    let Some(files) = files else {
        let (settings, places) = host_settings(context, before, changed, sources);
        if settings.is_empty() {
            return Outcome::Refuse(Refusal::saying(said));
        }
        return Outcome::Run(saving_plan(
            action,
            Saved {
                steps: Vec::new(),
                settings,
                places,
            },
            said,
        ));
    };
    match write_steps(&files, before, changed, sources) {
        Ok((steps, places)) if !steps.is_empty() => {
            let places = places.into_iter().map(place_words).collect();
            Outcome::Run(saving_plan(
                action,
                Saved {
                    steps,
                    settings: Vec::new(),
                    places,
                },
                said,
            ))
        }
        Ok(_) => Outcome::Refuse(Refusal::saying(said)),
        Err(refusal) => Outcome::Refuse(refusal),
    }
}

/// What a change saves: the steps that write Gitardo's files, or the settings the host writes, and the words of the
/// places saved in.
struct Saved {
    /// The steps.
    steps: Vec<Step>,
    /// The settings the host writes (`FollowUp::WriteSetting`).
    settings: Vec<FollowUp>,
    /// The places, as **Saved in the settings of …** names them.
    places: Vec<&'static str>,
}

/// The settings the host writes for the lists that changed, each in the place of its settings where it is set, with
/// the words of the places.
fn host_settings(
    context: &Context,
    before: &SetupPatterns,
    changed: &SetupPatterns,
    (link_place, copy_place): (SettingsPlace, SettingsPlace),
) -> (Vec<FollowUp>, Vec<&'static str>) {
    let mut settings = Vec::new();
    let mut places = Vec::new();
    let lists = [
        (LINK_FOLDERS, link_place, &before.link, &changed.link),
        (COPY_FILES, copy_place, &before.copy, &changed.copy),
    ];
    for (key, place, was, is) in lists {
        if was == is {
            continue;
        }
        let target = host_target(context, place);
        settings.push(setting_written(key, Some(&list_value(is)), target));
        let words = host_target_words(target);
        if !places.contains(&words) {
            places.push(words);
        }
    }
    (settings, places)
}

/// The writes of the lists that changed, each in the file of the place it is set in, with the places written.
fn write_steps(
    files: &SettingsFiles,
    before: &SetupPatterns,
    changed: &SetupPatterns,
    (link_place, copy_place): (SettingsPlace, SettingsPlace),
) -> Result<(Vec<Step>, Vec<SettingsPlace>), Refusal> {
    let mut steps = Vec::new();
    let mut places = Vec::new();
    for place in [SettingsPlace::Repository, SettingsPlace::User] {
        let mut keys: Vec<(&str, Option<Value>)> = Vec::new();
        if link_place == place && changed.link != before.link {
            keys.push((LINK_FOLDERS, Some(list_value(&changed.link))));
        }
        if copy_place == place && changed.copy != before.copy {
            keys.push((COPY_FILES, Some(list_value(&changed.copy))));
        }
        if keys.is_empty() {
            continue;
        }
        let file = files.of(place).ok_or_else(|| Refusal::saying(NO_USER_FILE))?;
        if let Some(step) = write_step(file, edited(file, &keys)?) {
            steps.push(step);
            places.push(place);
        }
    }
    Ok((steps, places))
}

/// The plan that saves the change, and says what became of each path and where it was saved.
fn saving_plan(action: SetupAction, saved: Saved, said: &str) -> Plan {
    let title = match action {
        SetupAction::Link => "Link into new worktrees",
        SetupAction::Copy => "Copy into new worktrees",
        SetupAction::StopLinking => "Stop linking into new worktrees",
        SetupAction::StopCopying => "Stop copying into new worktrees",
    };
    let undo = match action {
        SetupAction::Link => "Not taken back by Undo: Stop Linking into New Worktrees takes it out.",
        SetupAction::Copy => "Not taken back by Undo: Stop Copying into New Worktrees takes it out.",
        SetupAction::StopLinking => "Not taken back by Undo: Link into New Worktrees puts it back.",
        SetupAction::StopCopying => "Not taken back by Undo: Copy into New Worktrees puts it back.",
    };
    let where_saved = saved.places.join(" and of ");
    let mut after = saved.settings;
    after.push(FollowUp::Say(Text::from(format!(
        "{said} Saved in the settings of {where_saved}."
    ))));
    after.push(FollowUp::Offer(
        FeatureId("worktree-setup.page"),
        Text::from(CHOOSE_ON_THE_PAGE),
    ));
    Plan {
        title: Text::from(title),
        steps: saved.steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(undo)),
        after,
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[path = "give_tests.rs"]
mod tests;
