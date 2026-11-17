//! What the settings page and the picker of the themes ask (`settings.edit`, `settings.theme`): a value written in the
//! file chosen, keeping the rest of its text; the window takes it when the watcher reads the file again. And which
//! changes of the settings ask for a read of the repository (`repo.watch-changes`).

use views::Intent;
use views::settings::{Identity, SettingChange, SettingValue, SettingsTarget};

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
        Intent::SetSetting(change) => set_setting(state, change.clone()),
        Intent::ChooseSettingsFile(target) => {
            state.settings_target = *target;
            Vec::new()
        }
        Intent::Settings(asked) => {
            state.settings_page.apply(asked.clone());
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// The tab of the file the settings page shows: **User** while no repository is shown, whose tab is not there.
pub(super) fn shown_target(state: &State) -> SettingsTarget {
    if state.shown_folder().is_some() {
        state.settings_target
    } else {
        SettingsTarget::User
    }
}

/// Writes a value in the user's file, or in `.gitardo/settings.json` of the repository shown; nothing when that file
/// has no place.
fn set_setting(state: &State, change: SettingChange) -> Vec<Effect> {
    let file = match change.target {
        SettingsTarget::User => state.user_settings_file.clone(),
        SettingsTarget::Repository => state.shown_folder().map(|folder| repository_settings_file(folder)),
    };
    let Some(value) = json_of(change.value) else {
        return Vec::new();
    };
    file.map(|file| Effect::WriteSetting {
        file,
        key: change.key,
        value,
    })
    .into_iter()
    .collect()
}

/// A value of the page as JSON: nothing for text typed as JSON that is not, which is not written.
fn json_of(value: SettingValue) -> Option<json::Value> {
    Some(match value {
        SettingValue::Toggle(is_on) => json::Value::Bool(is_on),
        SettingValue::Choice(word) => json::Value::from(word),
        SettingValue::Number(number) => json::Value::from(number),
        SettingValue::Text(text) => json::Value::String(text),
        SettingValue::Strings(list) => json::Value::Array(list.into_iter().map(json::Value::String).collect()),
        SettingValue::Identities(list) => json::Value::Array(list.into_iter().map(identity_json).collect()),
        SettingValue::Json(text) => json::parse(&text).ok()?,
    })
}

/// An identity as `identities` writes it: `{ "name", "email", "signingKey" }`, without a signing key when it has none.
fn identity_json(identity: Identity) -> json::Value {
    let member = |name: &str, text: String| json::Member {
        name: name.to_owned(),
        value: json::Value::String(text),
    };
    let mut members = vec![member("name", identity.name), member("email", identity.email)];
    if !identity.signing_key.is_empty() {
        members.push(member("signingKey", identity.signing_key));
    }
    json::Value::Object(members)
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
