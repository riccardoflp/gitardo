//! The settings of the features from what the extension sends: the object of `initialize` and `settings/changed`,
//! whose keys are those of Gitardo Desktop's `settings.json`, read with the code that reads that file (crate
//! `settings`), as `docs/transition/02-protocol.md` says under Settings.

use model::contract::FeatureSettings;
use model::trust::Trust;
use model::worktree_setup::{InEffect, SettingSource, SetupSettings};
use serde_json::{Map, Value};

/// The settings of the features, with the offset of the local time the extension says (`utcOffsetMinutes`). A value of
/// the wrong kind is left for its default, as in the desktop's file. With `host` `"vscode"`, what new worktrees get is
/// the extension's settings, `worktrees.linkFolders`, `worktrees.copyFiles` and `worktrees.setupCommands` as it
/// resolved them, and no settings file of the desktop is read or written; with `trust` `"vscode"`, the folder counts
/// as trusted, since the extension runs the setup commands in VS Code's terminals only in a workspace VS Code trusts.
/// The home folder, for a `~` in `worktrees.folder`, is the user's.
pub fn feature_settings(settings: &Map<String, Value>) -> FeatureSettings {
    let text = serde_json::to_string(settings).unwrap_or_default();
    let mut lookup = ::settings::Lookup::parse(Some(&text), None);
    let mut features = ::settings::feature_settings(&mut lookup);
    features.utc_offset = settings
        .get("utcOffsetMinutes")
        .and_then(Value::as_i64)
        .map_or(0, |minutes| minutes.saturating_mul(60));
    features.worktrees.home = ::settings::home_folder().unwrap_or_default();
    if settings.get("host").and_then(Value::as_str) == Some("vscode") {
        features.worktrees.setup = Some(SetupSettings {
            link_folders: in_effect(settings, "worktrees.linkFolders"),
            copy_files: in_effect(settings, "worktrees.copyFiles"),
            setup_commands: in_effect(settings, "worktrees.setupCommands"),
        });
    }
    if settings.get("trust").and_then(Value::as_str) == Some("vscode") {
        features.trust = Trust::Trusted;
    }
    features
}

/// A list of what new worktrees get as the extension sent it, the entries that are text, trimmed and not empty; it
/// counts as set where the extension sent it, even empty.
fn in_effect(settings: &Map<String, Value>, key: &str) -> InEffect {
    match settings.get(key).and_then(Value::as_array) {
        Some(items) => InEffect {
            values: items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect(),
            source: SettingSource::Repository,
        },
        None => InEffect {
            values: Vec::new(),
            source: SettingSource::Nothing,
        },
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::feature_settings;
    use model::contract::CommandPreview;
    use serde_json::json;

    #[test]
    fn the_settings_of_the_extension_are_read_by_the_desktops_names() {
        let Some(settings) = json!({
            "branches.format": "${input:ticket}-${base}",
            "submodules.updateOnCheckout": false,
            "commandPreview": "all",
            "host": "vscode",
            "ai.provider": "vscode",
            "utcOffsetMinutes": 120
        })
        .as_object()
        .cloned() else {
            panic!("an object")
        };
        let features = feature_settings(&settings);
        assert_eq!(features.branch_format, "${input:ticket}-${base}");
        assert!(!features.update_submodules_on_checkout);
        assert_eq!(features.command_preview, CommandPreview::All);
        assert_eq!(features.utc_offset, 7_200);
    }

    #[test]
    fn worktree_setup_settings_in_effect__under_vs_code_the_settings_sent_are_what_new_worktrees_get() {
        let Some(settings) = json!({
            "host": "vscode",
            "trust": "vscode",
            "worktrees.linkFolders": ["node_modules/", " "],
            "worktrees.setupCommands": ["npm ci"]
        })
        .as_object()
        .cloned() else {
            panic!("an object")
        };
        let features = feature_settings(&settings);
        let Some(setup) = features.worktrees.setup else {
            panic!("the settings of new worktrees come from the extension");
        };
        assert_eq!(setup.link_folders.values, ["node_modules/"]);
        assert_eq!(setup.copy_files.values, Vec::<String>::new());
        assert_eq!(setup.setup_commands.values, ["npm ci"]);
        assert_eq!(features.trust, model::trust::Trust::Trusted);
        let desktop = feature_settings(&serde_json::Map::new());
        assert_eq!(
            desktop.worktrees.setup, None,
            "without the host, the settings files are read"
        );
        assert_eq!(desktop.trust, model::trust::Trust::NotTrusted);
    }

    #[test]
    fn no_settings_are_the_defaults() {
        let features = feature_settings(&serde_json::Map::new());
        assert!(features.update_submodules_on_checkout);
        assert_eq!(features.branch_format, "");
        assert_eq!(features.utc_offset, 0);
    }
}
