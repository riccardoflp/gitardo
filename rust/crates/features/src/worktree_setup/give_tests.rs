#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::contract::{Answer, Answers, Context, FollowUp, Outcome, Query, Step, Target};
use model::worktree_setup::{Ignored, PathStatus, PatternTake, SetupPreview, Tracked};

use super::{
    BEFORE_AND_AFTER, PATH_STATUSES, copy_into_new_worktrees, link_into_new_worktrees, stop_copying, stop_linking,
};
use crate::testing::{context, plan, state};
use crate::worktree_setup::files::SETTINGS_FILES;

/// What git says of a folder or a file it ignores as a whole.
const IGNORED_FOLDER: PathStatus = PathStatus {
    is_folder: true,
    tracked: Tracked::Nothing,
    ignored: Ignored::Whole,
};
const IGNORED_FILE: PathStatus = PathStatus {
    is_folder: false,
    tracked: Tracked::Nothing,
    ignored: Ignored::Whole,
};

/// What a set of patterns takes: the paths, and the pattern that takes each.
fn taken(link: &[(&str, &str)], copy: &[(&str, &str)]) -> SetupPreview {
    let take = |pairs: &[(&str, &str)]| -> Vec<PatternTake> {
        pairs
            .iter()
            .map(|(pattern, path)| PatternTake {
                pattern: (*pattern).to_owned(),
                count: 1,
                paths: vec![(*path).to_owned()],
            })
            .collect()
    };
    SetupPreview {
        linked: link.iter().map(|(_, path)| (*path).to_owned()).collect(),
        copied: copy.iter().map(|(_, path)| (*path).to_owned()).collect(),
        link_patterns: take(link),
        copy_patterns: take(copy),
        already: Vec::new(),
    }
}

/// The answers for paths, with the repository's settings, VS Code's, and what the patterns take before and after.
fn answers(settings: (&str, &str), statuses: &[PathStatus], before: SetupPreview, after: SetupPreview) -> Answers {
    Answers::new()
        .with(
            SETTINGS_FILES,
            Answer::Files(vec![
                Some(settings.0.as_bytes().to_vec()),
                Some(settings.1.as_bytes().to_vec()),
            ]),
        )
        .with(PATH_STATUSES, Answer::PathStatuses(statuses.to_vec()))
        .with(BEFORE_AND_AFTER, Answer::SetupTakes(vec![before, after]))
}

fn on(paths: &[&str]) -> Context {
    context(
        state(),
        Target::Files(paths.iter().map(|path| (*path).to_owned()).collect()),
    )
}

fn repository_file(folder: &str) -> PathBuf {
    PathBuf::from(folder).join(".gitardo").join("settings.json")
}

/// The list a plan writes for a key in a file.
fn written_list(outcome: Outcome, file: &PathBuf, key: &str) -> (Option<String>, String) {
    let plan = plan(outcome);
    let list = plan.steps.iter().find_map(|step| match step {
        Step::Write { path, contents } if path == file => {
            let value = json::parse_with_comments(&String::from_utf8_lossy(contents)).ok()?;
            value.get(key).map(json::to_compact_text)
        }
        _ => None,
    });
    let said = plan.after.iter().find_map(|follow_up| match follow_up {
        FollowUp::Say(words) => Some(words.as_str().to_owned()),
        _ => None,
    });
    (list, said.unwrap_or_default())
}

fn refused(outcome: Outcome) -> String {
    match outcome {
        Outcome::Refuse(refusal) => refusal.message.as_str().to_owned(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn worktree_setup_link_into_new_worktrees__an_ignored_folder_gets_the_pattern_of_its_path() {
    let path = "packages/api/node_modules";
    let after = taken(&[("/packages/api/node_modules/", path)], &[]);
    let answers = answers(("{}", "{}"), &[IGNORED_FOLDER], SetupPreview::default(), after);
    let (list, said) = written_list(
        link_into_new_worktrees(&on(&[path]), &answers),
        &repository_file("repository"),
        "worktrees.linkFolders",
    );
    assert_eq!(list.as_deref(), Some(r#"["/packages/api/node_modules/"]"#));
    assert_eq!(
        said,
        "packages/api/node_modules will be linked into new worktrees. Saved in the settings of the repository."
    );
}

#[test]
fn worktree_setup_link_into_new_worktrees__a_folder_git_tracks_files_in_writes_nothing() {
    let status = PathStatus {
        is_folder: true,
        tracked: Tracked::FilesIn,
        ignored: Ignored::Part,
    };
    let answers = answers(
        ("{}", "{}"),
        &[status],
        SetupPreview::default(),
        SetupPreview::default(),
    );
    assert_eq!(
        refused(link_into_new_worktrees(&on(&["src"]), &answers)),
        "Git tracks files in src: each worktree has them from its commit."
    );
}

#[test]
fn worktree_setup_link_into_new_worktrees__a_folder_linked_already_writes_nothing() {
    let settings = r#"{"worktrees.linkFolders": ["node_modules/"]}"#;
    let before = taken(&[("node_modules/", "node_modules")], &[]);
    let after = taken(
        &[("node_modules/", "node_modules"), ("/node_modules/", "node_modules")],
        &[],
    );
    let answers = answers((settings, "{}"), &[IGNORED_FOLDER], before, after);
    assert_eq!(
        refused(link_into_new_worktrees(&on(&["node_modules"]), &answers)),
        "node_modules is linked into new worktrees already, by the pattern node_modules/."
    );
}

#[test]
fn worktree_setup_link_into_new_worktrees__vs_code_s_patterns_go_to_the_repository_file_with_the_new_one() {
    let vs_code = r#"{"git.worktreeSymlinkFolders": ["node_modules/"]}"#;
    let before = taken(&[("node_modules/", "node_modules")], &[]);
    let after = taken(&[("node_modules/", "node_modules"), ("/.venv/", ".venv")], &[]);
    let outcome = link_into_new_worktrees(
        &on(&[".venv"]),
        &answers(("{}", vs_code), &[IGNORED_FOLDER], before, after),
    );
    let Outcome::Run(plan) = outcome.clone() else {
        panic!("expected a plan, got {outcome:?}");
    };
    let written: Vec<_> = plan
        .steps
        .iter()
        .filter_map(|step| {
            if let Step::Write { path, .. } = step {
                Some(path.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(written, [repository_file("repository")]);
    let (list, _) = written_list(outcome, &repository_file("repository"), "worktrees.linkFolders");
    assert_eq!(list.as_deref(), Some(r#"["node_modules/","/.venv/"]"#));
}

#[test]
fn worktree_setup_link_into_new_worktrees__each_repository_saves_in_its_own_settings() {
    for folder in ["first", "second"] {
        let mut repository = state();
        repository.folder = PathBuf::from(folder);
        let context = context(repository, Target::Files(vec![String::from("node_modules")]));
        let first = link_into_new_worktrees(&context, &Answers::new());
        let Outcome::Read(_, Query::FileTexts { paths }) = first else {
            panic!("expected a read of the settings files, got {first:?}");
        };
        assert_eq!(paths.first(), Some(&repository_file(folder)));
        let after = taken(&[("/node_modules/", "node_modules")], &[]);
        let answers = answers(("{}", "{}"), &[IGNORED_FOLDER], SetupPreview::default(), after);
        let (list, _) = written_list(
            link_into_new_worktrees(&context, &answers),
            &repository_file(folder),
            "worktrees.linkFolders",
        );
        assert_eq!(list.as_deref(), Some(r#"["/node_modules/"]"#));
    }
}

#[test]
fn worktree_setup_copy_into_new_worktrees__an_ignored_file_gets_the_pattern_of_its_path() {
    let after = taken(&[], &[("/.env", ".env")]);
    let answers = answers(("{}", "{}"), &[IGNORED_FILE], SetupPreview::default(), after);
    let (list, said) = written_list(
        copy_into_new_worktrees(&on(&[".env"]), &answers),
        &repository_file("repository"),
        "worktrees.copyFiles",
    );
    assert_eq!(list.as_deref(), Some(r#"["/.env"]"#));
    assert!(said.starts_with(".env will be copied into new worktrees."), "{said}");
}

#[test]
fn worktree_setup_copy_into_new_worktrees__of_a_folder_git_tracks_part_of_what_it_ignores_is_copied() {
    let status = PathStatus {
        is_folder: true,
        tracked: Tracked::FilesIn,
        ignored: Ignored::Part,
    };
    let after = taken(&[], &[("/config/", "config/local.json")]);
    let answers = answers(("{}", "{}"), &[status], SetupPreview::default(), after);
    let (list, said) = written_list(
        copy_into_new_worktrees(&on(&["config"]), &answers),
        &repository_file("repository"),
        "worktrees.copyFiles",
    );
    assert_eq!(list.as_deref(), Some(r#"["/config/"]"#));
    assert!(
        said.starts_with("What git ignores in config will be copied into new worktrees."),
        "{said}"
    );
}

#[test]
fn worktree_setup_copy_into_new_worktrees__a_file_in_a_linked_folder_writes_nothing() {
    let settings = r#"{"worktrees.linkFolders": ["node_modules/"]}"#;
    let before = taken(&[("node_modules/", "node_modules")], &[]);
    let answers = answers((settings, "{}"), &[IGNORED_FILE], before.clone(), before);
    assert_eq!(
        refused(copy_into_new_worktrees(&on(&["node_modules/.bin/tool"]), &answers)),
        "node_modules/.bin/tool is in node_modules, which the pattern node_modules/ links into new worktrees: nothing \
         in it is copied."
    );
}

#[test]
fn worktree_setup_copy_into_new_worktrees__a_file_git_does_not_ignore_writes_nothing() {
    let status = PathStatus {
        is_folder: false,
        tracked: Tracked::Nothing,
        ignored: Ignored::Nothing,
    };
    let answers = answers(
        ("{}", "{}"),
        &[status],
        SetupPreview::default(),
        SetupPreview::default(),
    );
    assert_eq!(
        refused(copy_into_new_worktrees(&on(&["notes.txt"]), &answers)),
        "Git does not ignore notes.txt: only what it ignores is linked or copied into new worktrees."
    );
}

#[test]
fn worktree_setup_stop_linking__another_pattern_still_links_it() {
    let settings = r#"{"worktrees.linkFolders": ["/node_modules/", "node_modules/"]}"#;
    let both = taken(
        &[("/node_modules/", "node_modules"), ("node_modules/", "node_modules")],
        &[],
    );
    let after = taken(&[("node_modules/", "node_modules")], &[]);
    let answers = answers((settings, "{}"), &[IGNORED_FOLDER], both, after);
    let (list, said) = written_list(
        stop_linking(&on(&["node_modules"]), &answers),
        &repository_file("repository"),
        "worktrees.linkFolders",
    );
    assert_eq!(list.as_deref(), Some(r#"["node_modules/"]"#));
    assert!(
        said.starts_with("node_modules is still linked into new worktrees, by the pattern node_modules/."),
        "{said}"
    );
}

#[test]
fn worktree_setup_stop_linking__the_only_pattern_leaves_the_list_empty() {
    let settings = r#"{"worktrees.linkFolders": ["/node_modules/"]}"#;
    let before = taken(&[("/node_modules/", "node_modules")], &[]);
    let answers = answers((settings, "{}"), &[IGNORED_FOLDER], before, SetupPreview::default());
    let (list, said) = written_list(
        stop_linking(&on(&["node_modules"]), &answers),
        &repository_file("repository"),
        "worktrees.linkFolders",
    );
    assert_eq!(list.as_deref(), Some("[]"));
    assert!(
        said.starts_with("New worktrees will no longer get node_modules."),
        "{said}"
    );
}

#[test]
fn worktree_setup_stop_copying__the_only_pattern_leaves_the_list_empty() {
    let settings = r#"{"worktrees.copyFiles": ["/.env"]}"#;
    let before = taken(&[], &[("/.env", ".env")]);
    let answers = answers((settings, "{}"), &[IGNORED_FILE], before, SetupPreview::default());
    let (list, said) = written_list(
        stop_copying(&on(&[".env"]), &answers),
        &repository_file("repository"),
        "worktrees.copyFiles",
    );
    assert_eq!(list.as_deref(), Some("[]"));
    assert!(said.starts_with("New worktrees will no longer get .env."), "{said}");
}

#[test]
fn worktree_setup_stop_copying__another_pattern_still_copies_it() {
    let settings = r#"{"worktrees.copyFiles": ["/.env", ".env"]}"#;
    let both = taken(&[], &[("/.env", ".env"), (".env", ".env")]);
    let after = taken(&[], &[(".env", ".env")]);
    let answers = answers((settings, "{}"), &[IGNORED_FILE], both, after);
    let (_, said) = written_list(
        stop_copying(&on(&[".env"]), &answers),
        &repository_file("repository"),
        "worktrees.copyFiles",
    );
    assert!(
        said.starts_with(".env is still copied into new worktrees, by the pattern .env."),
        "{said}"
    );
}

/// The context of an entry inside VS Code, which gives the settings: `node_modules/` linked by the user's settings,
/// and nothing copied.
fn in_vs_code(paths: &[&str]) -> Context {
    use model::worktree_setup::{InEffect, SettingSource, SetupSettings};
    let mut context = on(paths);
    let list = |values: &[&str], source| InEffect {
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        source,
    };
    context.settings.worktrees.setup = Some(SetupSettings {
        link_folders: list(&["node_modules/"], SettingSource::User),
        copy_files: list(&[], SettingSource::Nothing),
        setup_commands: list(&[], SettingSource::Nothing),
    });
    context
}

/// The answers of an entry inside VS Code: no settings file is read.
fn answers_in_vs_code(statuses: &[PathStatus], before: SetupPreview, after: SetupPreview) -> Answers {
    Answers::new()
        .with(PATH_STATUSES, Answer::PathStatuses(statuses.to_vec()))
        .with(BEFORE_AND_AFTER, Answer::SetupTakes(vec![before, after]))
}

/// Each setting a plan has the host write: its key, its value and where.
type Written = Vec<(String, Option<String>, model::contract::SettingTarget)>;

/// The settings a plan has the host write, its steps and what it says.
fn host_writes(outcome: Outcome) -> (Written, Vec<Step>, String) {
    let plan = plan(outcome);
    let written = plan
        .after
        .iter()
        .filter_map(|follow_up| match follow_up {
            FollowUp::WriteSetting(written) => Some((written.key.clone(), written.value.clone(), written.target)),
            _ => None,
        })
        .collect();
    let said = plan.after.iter().find_map(|follow_up| match follow_up {
        FollowUp::Say(words) => Some(words.as_str().to_owned()),
        _ => None,
    });
    (written, plan.steps, said.unwrap_or_default())
}

#[test]
fn worktree_setup_copy_into_new_worktrees__inside_vs_code_the_host_writes_the_setting_in_the_workspace_folder() {
    use model::contract::SettingTarget;
    let after = taken(&[("node_modules/", "node_modules")], &[("/.env", ".env")]);
    let before = taken(&[("node_modules/", "node_modules")], &[]);
    let answers = answers_in_vs_code(&[IGNORED_FILE], before, after);
    let (written, steps, said) = host_writes(copy_into_new_worktrees(&in_vs_code(&[".env"]), &answers));
    assert_eq!(steps, []);
    assert_eq!(
        written,
        [(
            String::from("worktrees.copyFiles"),
            Some(String::from(r#"["/.env"]"#)),
            SettingTarget::WorkspaceFolder
        )]
    );
    assert_eq!(
        said,
        ".env will be copied into new worktrees. Saved in the settings of the workspace folder."
    );
}

#[test]
fn worktree_setup_link_into_new_worktrees__inside_vs_code_a_list_set_by_the_user_is_written_there() {
    use model::contract::SettingTarget;
    let path = "packages/api/node_modules";
    let before = taken(&[("node_modules/", "node_modules")], &[]);
    let after = taken(
        &[("node_modules/", "node_modules"), ("/packages/api/node_modules/", path)],
        &[],
    );
    let answers = answers_in_vs_code(&[IGNORED_FOLDER], before, after);
    let (written, _, said) = host_writes(link_into_new_worktrees(&in_vs_code(&[path]), &answers));
    assert_eq!(
        written,
        [(
            String::from("worktrees.linkFolders"),
            Some(String::from(r#"["node_modules/","/packages/api/node_modules/"]"#)),
            SettingTarget::User
        )]
    );
    assert!(said.ends_with("Saved in the settings of the user."), "{said}");
}

#[test]
fn worktree_setup_stop_linking__inside_vs_code_with_nothing_to_change_it_is_refused_with_the_reason() {
    let before = taken(&[("node_modules/", "node_modules")], &[]);
    let answers = answers_in_vs_code(&[IGNORED_FOLDER], before.clone(), before);
    let said = refused(stop_linking(&in_vs_code(&["node_modules"]), &answers));
    assert!(said.contains("node_modules"), "{said}");
}
