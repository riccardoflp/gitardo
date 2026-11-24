//! When a plan is shown to the user before it runs (`docs/desktop/04-feature-contract.md`, "Danger and confirmation").

use model::contract::{CommandPreview, Danger, FollowUp, Plan};

/// Whether a plan is shown and confirmed before it runs, with `commandPreview` set so, by its danger.
///
/// | `commandPreview` | None | Confirm | Destructive |
/// | --- | --- | --- | --- |
/// | `off` | runs | asks | asks, with the warning |
/// | `all` | asks | asks | asks, with the warning |
///
/// A plan the user agreed to in the feature's own question (`Agreed`) runs without being shown again, either way. A
/// plan with no danger that changes nothing itself, with no step and no setting for the host to write, such as opening
/// a page of GitHub or reading the pull requests, has nothing to show and is never asked, under `all` too (G6).
#[must_use]
pub fn needs_confirmation(plan: &Plan, preview: CommandPreview) -> bool {
    match (&plan.danger, preview) {
        (Danger::None, CommandPreview::Off) | (Danger::Agreed, _) => false,
        (Danger::None, CommandPreview::All) => changes_something(plan),
        (Danger::Confirm(_) | Danger::Destructive(_), _) => true,
    }
}

/// Whether a plan has something to show: a step, or a setting the host writes in place of a step
/// (`FollowUp::WriteSetting`).
fn changes_something(plan: &Plan) -> bool {
    !plan.steps.is_empty()
        || plan
            .after
            .iter()
            .any(|follow_up| matches!(follow_up, FollowUp::WriteSetting(_)))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::needs_confirmation;
    use model::contract::{CommandPreview, Danger, FollowUp, Plan, SettingWritten, Step, Text, Undo};

    fn plan(danger: Danger, steps: Vec<Step>, after: Vec<FollowUp>) -> Plan {
        Plan {
            title: Text::from("A plan"),
            steps,
            danger,
            undo: Undo::Impossible(Text::from("Not taken back.")),
            after,
            on_failure: Vec::new(),
        }
    }

    fn a_step() -> Vec<Step> {
        vec![Step::git(std::path::PathBuf::from("repository"), ["status"])]
    }

    #[test]
    fn follows_the_table_of_the_contract() {
        let reason = Text::from("It moves a branch.");
        let cases = [
            (Danger::None, CommandPreview::Off, false),
            (Danger::None, CommandPreview::All, true),
            (Danger::Agreed, CommandPreview::Off, false),
            (Danger::Agreed, CommandPreview::All, false),
            (Danger::Confirm(reason.clone()), CommandPreview::Off, true),
            (Danger::Confirm(reason.clone()), CommandPreview::All, true),
            (Danger::Destructive(reason.clone()), CommandPreview::Off, true),
            (Danger::Destructive(reason), CommandPreview::All, true),
        ];
        for (danger, preview, expected) in cases {
            let shown = plan(danger.clone(), a_step(), Vec::new());
            assert_eq!(needs_confirmation(&shown, preview), expected, "{danger:?} {preview:?}");
        }
    }

    #[test]
    fn preview_confirm__with_command_preview_all_open_on_github_and_loading_the_pull_requests_ask_nothing() {
        // The plans of `compare.open-on-github` and `github.load-pull-requests`: no step, no danger.
        let open = plan(
            Danger::None,
            Vec::new(),
            vec![FollowUp::OpenUrl(String::from("https://github.com"))],
        );
        let load = plan(Danger::None, Vec::new(), vec![FollowUp::ReadPullRequests]);
        for nothing in [open, load] {
            assert!(!needs_confirmation(&nothing, CommandPreview::All));
            assert!(!needs_confirmation(&nothing, CommandPreview::Off));
        }
        let warned = plan(Danger::Confirm(Text::from("Sure?")), Vec::new(), Vec::new());
        assert!(needs_confirmation(&warned, CommandPreview::All));
    }

    #[test]
    fn a_setting_the_host_writes_is_shown_under_all_as_a_step_is() {
        let written = FollowUp::WriteSetting(Box::new(SettingWritten {
            key: String::from("worktrees.linkFolders"),
            value: Some(String::from("[]")),
        }));
        let setting = plan(Danger::None, Vec::new(), vec![written]);
        assert!(needs_confirmation(&setting, CommandPreview::All));
        assert!(!needs_confirmation(&setting, CommandPreview::Off));
    }
}
