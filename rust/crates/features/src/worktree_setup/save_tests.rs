#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, Answers, Context, Outcome, Plan, Step, Target};
use model::worktree_setup::{SettingsPlace, SetupPatterns, SetupSave};

use super::save;
use crate::testing::{context, lines, plan, state};
use crate::worktree_setup::files::SETTINGS_FILES;
use crate::worktree_setup::remove_from;

/// The context of the page saving in a place, with the user's file in `user/settings.json`.
pub fn saving(save: SetupSave) -> Context {
    let mut context = context(state(), Target::SetupSettings(Box::new(save)));
    context.settings.user_settings_file = Some(PathBuf::from("user").join("settings.json"));
    context
}

/// The answers with the texts of the repository's file, VS Code's and the user's.
pub fn files(repository: Option<&str>, vs_code: Option<&str>, user: Option<&str>) -> Answers {
    let bytes = |text: Option<&str>| text.map(|text| text.as_bytes().to_vec());
    Answers::new().with(
        SETTINGS_FILES,
        Answer::Files(vec![bytes(repository), bytes(vs_code), bytes(user)]),
    )
}

/// The text a plan writes in a file.
pub fn written(plan: &Plan, file: &PathBuf) -> Option<String> {
    plan.steps.iter().find_map(|step| match step {
        Step::Write { path, contents } if path == file => Some(String::from_utf8_lossy(contents).into_owned()),
        _ => None,
    })
}

fn patterns(link: &[&str], copy: &[&str]) -> SetupPatterns {
    SetupPatterns {
        link: link.iter().map(|pattern| (*pattern).to_owned()).collect(),
        copy: copy.iter().map(|pattern| (*pattern).to_owned()).collect(),
    }
}

fn repository_file() -> PathBuf {
    PathBuf::from("repository").join(".gitardo").join("settings.json")
}

fn vs_code_file() -> PathBuf {
    PathBuf::from("repository").join(".vscode").join("settings.json")
}

#[test]
fn worktree_setup_save__the_repository_file_gets_the_three_keys_and_keeps_its_comment() {
    let context = saving(SetupSave {
        place: SettingsPlace::Repository,
        patterns: patterns(&["node_modules/"], &["/.env"]),
        commands: vec![String::from("npm ci")],
        also_vs_code: false,
    });
    let before = "{\n  // Shared by the team.\n  \"graph.pageSize\": 500\n}\n";
    let plan = plan(save(&context, &files(Some(before), None, None)));
    let text = written(&plan, &repository_file()).unwrap();
    assert!(text.contains("// Shared by the team."), "{text}");
    let value = json::parse_with_comments(&text).unwrap();
    let list = |key: &str| value.get(key).map(json::to_compact_text);
    assert_eq!(list("worktrees.linkFolders").as_deref(), Some(r#"["node_modules/"]"#));
    assert_eq!(list("worktrees.copyFiles").as_deref(), Some(r#"["/.env"]"#));
    assert_eq!(list("worktrees.setupCommands").as_deref(), Some(r#"["npm ci"]"#));
    assert_eq!(list("graph.pageSize").as_deref(), Some("500"));
}

#[test]
fn worktree_setup_save__no_commands_anywhere_writes_no_commands() {
    let context = saving(SetupSave {
        place: SettingsPlace::User,
        patterns: patterns(&["node_modules/"], &[]),
        ..SetupSave::default()
    });
    let plan = plan(save(&context, &files(None, None, None)));
    let text = written(&plan, &PathBuf::from("user").join("settings.json")).unwrap();
    assert!(!text.contains("worktrees.setupCommands"), "{text}");
    // With commands set in the repository, the user's file gets its empty list, which then applies.
    let set = files(Some(r#"{"worktrees.setupCommands": ["make"]}"#), None, None);
    let plan = crate::testing::plan(save(&context, &set));
    let text = written(&plan, &PathBuf::from("user").join("settings.json")).unwrap();
    assert!(text.contains("\"worktrees.setupCommands\": []"), "{text}");
}

#[test]
fn worktree_setup_save__the_box_gives_vs_code_the_same_patterns_and_nothing_else() {
    let context = saving(SetupSave {
        place: SettingsPlace::Repository,
        patterns: patterns(&["node_modules/"], &["/.env"]),
        commands: Vec::new(),
        also_vs_code: true,
    });
    let vs_code = "{\n  \"editor.tabSize\": 2 // Two spaces.\n}\n";
    let plan = plan(save(&context, &files(None, Some(vs_code), None)));
    let text = written(&plan, &vs_code_file()).unwrap();
    assert!(
        text.starts_with("{\n  \"editor.tabSize\": 2, // Two spaces.\n"),
        "{text}"
    );
    let value = json::parse_with_comments(&text).unwrap();
    let list = |key: &str| value.get(key).map(json::to_compact_text);
    assert_eq!(
        list("git.worktreeSymlinkFolders").as_deref(),
        Some(r#"["node_modules/"]"#)
    );
    assert_eq!(list("git.worktreeIncludeFiles").as_deref(), Some(r#"["/.env"]"#));
    assert_eq!(list("worktrees.linkFolders"), None);
}

#[test]
fn worktree_setup_save__the_log_has_a_write_line_for_each_file() {
    let context = saving(SetupSave {
        place: SettingsPlace::Repository,
        patterns: patterns(&["node_modules/"], &[]),
        commands: Vec::new(),
        also_vs_code: true,
    });
    let plan = plan(save(&context, &files(None, None, None)));
    assert_eq!(
        lines(&plan),
        [
            format!("write {}", repository_file().display()),
            format!("write {}", vs_code_file().display()),
        ]
    );
    // What is saved there already is not written again.
    let saved = r#"{"worktrees.linkFolders": ["node_modules/"], "worktrees.copyFiles": []}"#;
    let context = saving(SetupSave {
        place: SettingsPlace::Repository,
        patterns: patterns(&["node_modules/"], &[]),
        ..SetupSave::default()
    });
    assert!(matches!(
        save(&context, &files(Some(saved), None, None)),
        Outcome::Refuse(_)
    ));
}

#[test]
fn worktree_setup_remove_from__the_users_patterns_apply_again() {
    let context = saving(SetupSave {
        place: SettingsPlace::Repository,
        ..SetupSave::default()
    });
    let repository = "{\n  \"worktrees.linkFolders\": [],\n  \"graph.pageSize\": 500\n}\n";
    let user = r#"{"worktrees.linkFolders": ["node_modules/"]}"#;
    let answers = files(Some(repository), None, Some(user));
    let plan = plan(remove_from(&context, &answers));
    let text = written(&plan, &repository_file()).unwrap();
    assert_eq!(text, "{\n  \"graph.pageSize\": 500\n}\n");
    let after = files(Some(&text), None, Some(user));
    let settings = crate::worktree_setup::files::read_files(&context, &after)
        .unwrap()
        .in_effect();
    assert_eq!(settings.link_folders.values, ["node_modules/"]);
}

#[test]
fn worktree_setup_remove_from__vs_code_s_setting_stays() {
    let context = saving(SetupSave {
        place: SettingsPlace::Repository,
        ..SetupSave::default()
    });
    let repository = r#"{"worktrees.copyFiles": ["/.env"]}"#;
    let vs_code = r#"{"git.worktreeSymlinkFolders": ["node_modules/"]}"#;
    let plan = plan(remove_from(&context, &files(Some(repository), Some(vs_code), None)));
    assert_eq!(written(&plan, &vs_code_file()), None);
    assert_eq!(lines(&plan), [format!("write {}", repository_file().display())]);
}

/// The settings a plan has the host write, each with its value and its place.
fn host_writes(plan: &Plan) -> Vec<(String, Option<String>, model::contract::SettingTarget)> {
    plan.after
        .iter()
        .filter_map(|follow_up| match follow_up {
            model::contract::FollowUp::WriteSetting(written) => {
                Some((written.key.clone(), written.value.clone(), written.target))
            }
            _ => None,
        })
        .collect()
}

/// A list VS Code gives, and where it is set.
fn given(values: &[&str], source: model::worktree_setup::SettingSource) -> model::worktree_setup::InEffect {
    model::worktree_setup::InEffect {
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        source,
    }
}

/// The context of the page saving inside VS Code, which gives the settings: nothing set but the user's commands.
fn saving_in_vs_code(save: SetupSave) -> Context {
    use model::worktree_setup::{SettingSource, SetupSettings};
    let mut context = saving(save);
    context.settings.worktrees.setup = Some(SetupSettings {
        link_folders: given(&[], SettingSource::Nothing),
        copy_files: given(&[], SettingSource::Nothing),
        setup_commands: given(&["npm ci"], SettingSource::User),
    });
    context
}

#[test]
fn worktree_setup_save__inside_vs_code_the_host_writes_the_three_settings_in_the_workspace_folder_and_no_file() {
    use model::contract::SettingTarget;
    let context = saving_in_vs_code(SetupSave {
        place: SettingsPlace::Repository,
        patterns: patterns(&["node_modules/"], &["/.env", "*.missing"]),
        commands: Vec::new(),
        also_vs_code: true,
    });
    let plan = plan(save(&context, &Answers::new()));
    assert_eq!(plan.steps, []);
    let folder = SettingTarget::WorkspaceFolder;
    let list = |text: &str| Some(String::from(text));
    assert_eq!(
        host_writes(&plan),
        [
            (
                String::from("worktrees.linkFolders"),
                list("[\"node_modules/\"]"),
                folder
            ),
            (
                String::from("worktrees.copyFiles"),
                list("[\"/.env\",\"*.missing\"]"),
                folder
            ),
            // The user sets commands: the empty list is written, so that none run for this repository.
            (String::from("worktrees.setupCommands"), list("[]"), folder),
            (
                String::from("git.worktreeSymlinkFolders"),
                list("[\"node_modules/\"]"),
                folder
            ),
            (
                String::from("git.worktreeIncludeFiles"),
                list("[\"/.env\",\"*.missing\"]"),
                folder
            ),
        ]
    );
}

#[test]
fn worktree_setup_save__inside_vs_code_a_repository_in_no_folder_of_the_workspace_saves_in_the_users_settings() {
    use model::contract::SettingTarget;
    let mut context = saving_in_vs_code(SetupSave {
        place: SettingsPlace::Repository,
        patterns: patterns(&["node_modules/"], &[]),
        ..SetupSave::default()
    });
    context.settings.worktrees.repository_target = SettingTarget::User;
    let plan = plan(save(&context, &Answers::new()));
    assert!(
        host_writes(&plan)
            .iter()
            .all(|(_, _, target)| *target == SettingTarget::User)
    );
}

#[test]
fn worktree_setup_save__inside_vs_code_what_the_place_sets_already_is_refused() {
    use model::worktree_setup::{SettingSource, SetupSettings};
    let mut context = saving(SetupSave {
        place: SettingsPlace::User,
        patterns: patterns(&["node_modules/"], &[]),
        ..SetupSave::default()
    });
    context.settings.worktrees.setup = Some(SetupSettings {
        link_folders: given(&["node_modules/"], SettingSource::User),
        copy_files: given(&[], SettingSource::User),
        setup_commands: given(&[], SettingSource::Nothing),
    });
    assert!(matches!(save(&context, &Answers::new()), Outcome::Refuse(_)));
}

#[test]
fn worktree_setup_remove_from__inside_vs_code_the_host_removes_the_three_settings_from_the_place() {
    use model::contract::SettingTarget;
    let context = saving_in_vs_code(SetupSave {
        place: SettingsPlace::User,
        ..SetupSave::default()
    });
    let plan = plan(remove_from(&context, &Answers::new()));
    assert_eq!(plan.steps, []);
    let removed = |key: &str| (String::from(key), None, SettingTarget::User);
    assert_eq!(
        host_writes(&plan),
        [
            removed("worktrees.linkFolders"),
            removed("worktrees.copyFiles"),
            removed("worktrees.setupCommands")
        ]
    );
}
