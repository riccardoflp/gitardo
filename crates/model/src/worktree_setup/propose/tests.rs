#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{CommandsInput, Gets, Proposal, ProposeInput, Section, propose, propose_commands};

fn proposed(path: &str, is_folder: bool) -> Proposal {
    propose(&ProposeInput { path, is_folder })
}

fn proposal(section: Section, proposed: Gets, pattern: &str) -> Proposal {
    Proposal {
        section,
        proposed,
        pattern: pattern.to_owned(),
    }
}

#[test]
fn worktree_setup_propose__a_node_modules_anywhere_is_a_dependency_linked_by_its_name() {
    assert_eq!(
        proposed("packages/web/node_modules", true),
        proposal(Section::Dependencies, Gets::Link, "node_modules/")
    );
    assert_eq!(
        proposed(".yarn/cache", true),
        proposal(Section::Dependencies, Gets::Link, "/.yarn/cache/")
    );
}

#[test]
fn worktree_setup_propose__local_settings_are_secrets_copied_by_their_path() {
    assert_eq!(
        proposed(".env.local", false),
        proposal(Section::Secrets, Gets::Copy, "/.env.local")
    );
    assert_eq!(
        proposed("config/app.local.json", false),
        proposal(Section::Secrets, Gets::Copy, "/config/app.local.json")
    );
    assert_eq!(proposed("API/.ENV", false).section, Section::Secrets);
    assert_eq!(proposed("settings.local", false).section, Section::Secrets);
    assert_eq!(proposed(".local", false).section, Section::NotKnown);
}

#[test]
fn worktree_setup_propose__build_output_and_logs_are_proposed_nothing() {
    assert_eq!(
        proposed("build", true),
        proposal(Section::BuildOutput, Gets::Nothing, "/build/")
    );
    assert_eq!(
        proposed("debug.log", false),
        proposal(Section::Logs, Gets::Nothing, "/debug.log")
    );
    assert_eq!(proposed("Thumbs.DB", false).section, Section::Logs);
    assert_eq!(proposed("notes.txt~", false).section, Section::Logs);
    assert_eq!(proposed(".worktrees", true).section, Section::OtherWorktrees);
}

#[test]
fn worktree_setup_propose__a_vscode_folder_is_copied_and_a_vscode_file_is_not_known() {
    assert_eq!(
        proposed(".vscode", true),
        proposal(Section::Editor, Gets::Copy, "/.vscode/")
    );
    assert_eq!(
        proposed(".vscode", false),
        proposal(Section::NotKnown, Gets::Nothing, "/.vscode")
    );
}

#[test]
fn worktree_setup_propose__a_name_gitardo_does_not_know_is_proposed_nothing() {
    assert_eq!(
        proposed("notes", true),
        proposal(Section::NotKnown, Gets::Nothing, "/notes/")
    );
    assert_eq!(Section::ALL.first(), Some(&Section::NotKnown));
}

fn commands(committed: &[&str], package_manager: Option<&str>) -> Vec<String> {
    let committed: Vec<String> = committed.iter().map(|file| (*file).to_owned()).collect();
    propose_commands(&CommandsInput {
        committed: &committed,
        package_manager,
    })
    .into_iter()
    .map(|proposal| proposal.command)
    .collect()
}

#[test]
fn worktree_setup_propose_commands__pnpm_comes_before_npm() {
    assert_eq!(
        commands(&["package-lock.json", "pnpm-lock.yaml"], None),
        ["pnpm install --frozen-lockfile"]
    );
}

#[test]
fn worktree_setup_propose_commands__the_package_manager_named_comes_first() {
    assert_eq!(
        commands(&["package-lock.json", "pnpm-lock.yaml"], Some("npm@10.2.0")),
        ["npm ci"]
    );
    // A manager named without its lock file gives way to the order of the card.
    assert_eq!(
        commands(&["pnpm-lock.yaml"], Some("npm@10.2.0")),
        ["pnpm install --frozen-lockfile"]
    );
}

#[test]
fn worktree_setup_propose_commands__yarn_2_or_later_installs_immutable() {
    assert_eq!(
        commands(&["yarn.lock"], Some("yarn@4.1.0")),
        ["yarn install --immutable"]
    );
    assert_eq!(
        commands(&["yarn.lock", ".yarnrc.yml"], None),
        ["yarn install --immutable"]
    );
    assert_eq!(
        commands(&["yarn.lock"], Some("yarn@1.22.19")),
        ["yarn install --frozen-lockfile"]
    );
}

#[test]
fn worktree_setup_propose_commands__uv_comes_before_poetry_and_cargo_fetches() {
    let committed = ["uv.lock", "poetry.lock", "Cargo.lock"];
    assert_eq!(commands(&committed, None), ["uv sync --locked", "cargo fetch"]);
    let all = propose_commands(&CommandsInput {
        committed: &["go.mod".to_owned(), "Gemfile.lock".to_owned()],
        package_manager: None,
    });
    let files: Vec<(&str, &str)> = all
        .iter()
        .map(|proposal| (proposal.command.as_str(), proposal.file.as_str()))
        .collect();
    assert_eq!(
        files,
        [("go mod download", "go.mod"), ("bundle install", "Gemfile.lock")]
    );
}

#[test]
fn worktree_setup_propose_commands__a_lock_file_in_a_subfolder_and_requirements_propose_nothing() {
    assert_eq!(
        commands(&["web/package-lock.json", "requirements.txt"], None),
        Vec::<String>::new()
    );
}
