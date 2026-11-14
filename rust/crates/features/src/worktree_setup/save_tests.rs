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
