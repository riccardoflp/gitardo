//! What the settings page and the picker of the themes ask (`settings.edit`, `settings.theme`): a value written in the
//! file chosen, keeping the rest of its text; the window takes it when the watcher reads the file again. And which
//! changes of the settings ask for a read of the repository (`repo.watch-changes`).

use views::Intent;
use views::settings::{SettingChange, SettingValue, SettingsTarget};

use super::message::Effect;
use super::state::{Dialog, State};
use crate::settings::{Settings, repository_settings_file};

/// The settings under which a change asks for a read of the repository: they change what is read.
const READ_SETTINGS: [&str; 5] = ["conflicts.", "pull.", "worktrees.", "branches.", "graph."];

/// An intent of the settings page or of the picker of the themes.
pub fn reduce_intent(state: &mut State, intent: &Intent) -> Vec<Effect> {
    match intent {
        Intent::PickTheme => {
            state.dialog = Some(Dialog::ThemePicker);
            Vec::new()
        }
        Intent::CloseThemePicker => {
            if state.dialog == Some(Dialog::ThemePicker) {
                state.dialog = None;
            }
            Vec::new()
        }
        Intent::SetSetting(change) => set_setting(state, *change),
        Intent::ChooseSettingsFile(target) => {
            state.settings_target = *target;
            Vec::new()
        }
        Intent::Settings(asked) => {
            state.settings_page.apply(*asked);
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Writes a value in the user's file, or in `.gitardo/settings.json` of the repository shown; nothing when that file
/// has no place.
fn set_setting(state: &State, change: SettingChange) -> Vec<Effect> {
    let file = match change.target {
        SettingsTarget::User => state.user_settings_file.clone(),
        SettingsTarget::Repository => state.shown_folder().map(|folder| repository_settings_file(folder)),
    };
    let value = match change.value {
        SettingValue::Toggle(is_on) => json::Value::Bool(is_on),
        SettingValue::Choice(word) => json::Value::String(word.to_owned()),
    };
    file.map(|file| Effect::WriteSetting {
        file,
        key: change.key,
        value,
    })
    .into_iter()
    .collect()
}

/// Whether a setting under `conflicts`, `pull`, `worktrees`, `branches` or `graph` changed between two readings of
/// the files.
#[must_use]
pub fn changes_reads(before: &Settings, after: &Settings) -> bool {
    let of_reads = |settings: &Settings| {
        settings
            .values
            .iter()
            .filter(|(key, _)| READ_SETTINGS.iter().any(|prefix| key.starts_with(prefix)))
            .map(|(key, values)| (key.clone(), values.clone()))
            .collect::<Vec<_>>()
    };
    of_reads(before) != of_reads(after)
}
