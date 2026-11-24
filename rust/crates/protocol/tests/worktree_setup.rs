//! The JSON of the settings of new worktrees saved by the engine's plans under the extension (1.13, milestone M48):
//! what `feature/run` gives of them, the target the tab of what new worktrees get saves, and where a setting is written.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::contract::{SettingTarget, Target};
use model::worktree_setup::{SettingSource, SettingsPlace};
use protocol::feature::{FeatureRunParams, FollowUpJson, SettingTargetJson};
use serde_json::json;

#[test]
fn worktree_setup_save__the_tab_sends_what_it_saves_and_where_with_what_vs_code_sets_in_the_folder() {
    let params: FeatureRunParams = serde_json::from_value(json!({
        "repository": "/work/app",
        "feature": "worktree-setup.save",
        "target": {
            "kind": "setupSettings",
            "place": "repository",
            "link": ["node_modules/"],
            "copy": ["/.env"],
            "commands": ["npm ci"],
            "alsoVsCode": true
        },
        "setup": {
            "linkFolders": { "values": [" node_modules/ ", ""], "from": "workspace" },
            "copyFiles": { "values": [], "from": "none" },
            "setupCommands": { "values": ["npm ci"], "from": "user" },
            "inWorkspaceFolder": false
        }
    }))
    .unwrap();
    let Target::SetupSettings(save) = Target::try_from(params.target).unwrap() else {
        panic!("the settings of new worktrees");
    };
    assert_eq!(save.place, SettingsPlace::Repository);
    assert_eq!(save.patterns.link, ["node_modules/"]);
    assert_eq!(save.commands, ["npm ci"]);
    assert!(save.also_vs_code);
    let given = params.setup.unwrap();
    let settings = given.settings();
    assert_eq!(settings.link_folders.values, ["node_modules/"]);
    assert_eq!(settings.link_folders.source, SettingSource::Repository);
    assert_eq!(settings.copy_files.source, SettingSource::Nothing);
    assert_eq!(settings.setup_commands.source, SettingSource::User);
    // A repository in no folder of the workspace has its settings saved in the user's.
    assert_eq!(given.repository_target(), SettingTarget::User);
}

#[test]
fn a_setting_written_by_an_engine_of_1_12_goes_in_the_users_settings() {
    let older: FollowUpJson =
        serde_json::from_value(json!({ "kind": "writeSetting", "key": "ai.model", "value": "two" })).unwrap();
    assert_eq!(
        older,
        FollowUpJson::WriteSetting {
            key: String::from("ai.model"),
            value: Some(json!("two")),
            target: SettingTargetJson::User,
        }
    );
}
