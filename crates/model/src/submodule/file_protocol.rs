//! Cloning a submodule from a folder on disk, which git refuses unless it is allowed (`submodule.allow-file-protocol`).

use crate::contract::{Danger, FollowUp, Plan, Step, Text};

/// What git says when it refuses to clone a submodule from a folder on disk.
pub const FILE_NOT_ALLOWED: &str = "transport 'file' not allowed";

/// The configuration that allows it, for one command.
const ALLOW: [&str; 2] = ["-c", "protocol.file.allow=always"];

/// Why the plan offered again is asked first, at the start of its note.
pub const WHY_ALLOWED: &str = "Git does not clone submodules from folders on disk by default, since a repository could \
point one at any folder of this computer. This allows it for these commands only.";

/// A plan whose step failed, and what git said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailedPlan<'a> {
    /// The plan.
    pub plan: &'a Plan,
    /// The place of the step that failed among its steps: 0 for the first.
    pub failed_step: usize,
    /// What git said.
    pub said: &'a str,
}

/// The plan to offer again when git refused to clone a submodule from a folder on disk: the same plan from the step
/// that failed, with `-c protocol.file.allow=always` before each of its git commands, its title followed by **from a
/// folder on disk**, asked first with the reason at the start of its note. Nothing is changed in the configuration of
/// git. A failure with any other words offers nothing.
///
/// Feature: submodule.allow-file-protocol
#[must_use]
pub fn allow_file_protocol(input: &FailedPlan<'_>) -> Option<Plan> {
    if !input.said.contains(FILE_NOT_ALLOWED) {
        return None;
    }
    let plan = input.plan;
    let steps = plan.steps.iter().skip(input.failed_step).map(allowed).collect();
    let danger = match &plan.danger {
        Danger::None => Danger::Confirm(Text::from(WHY_ALLOWED)),
        Danger::Confirm(note) => Danger::Confirm(Text::from(format!("{WHY_ALLOWED}\n\n{note}"))),
        Danger::Destructive(note) => Danger::Destructive(Text::from(format!("{WHY_ALLOWED}\n\n{note}"))),
    };
    Some(Plan {
        title: Text::from(format!("{} from a folder on disk", plan.title)),
        steps,
        danger,
        undo: plan.undo.clone(),
        after: plan.after.clone(),
        on_failure: plan
            .on_failure
            .iter()
            .filter(|follow_up| **follow_up != FollowUp::AllowFileProtocol)
            .cloned()
            .collect(),
    })
}

/// A step with the file protocol allowed, when it runs git.
fn allowed(step: &Step) -> Step {
    match step {
        Step::Git {
            arguments,
            directory,
            input,
        } => Step::Git {
            arguments: ALLOW.iter().map(Into::into).chain(arguments.iter().cloned()).collect(),
            directory: directory.clone(),
            input: input.clone(),
        },
        // Only git clones; the other steps run as they are.
        other => other.clone(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::{FILE_NOT_ALLOWED, FailedPlan, WHY_ALLOWED, allow_file_protocol};
    use crate::contract::{Danger, FollowUp, Plan, Step, Text, Undo};

    /// The plan of adding `../lib` as the submodule `lib`.
    fn adding() -> Plan {
        let folder = PathBuf::from("repository");
        Plan {
            title: Text::from("Add submodule lib"),
            steps: vec![
                Step::git(folder.clone(), ["submodule", "add", "--", "../lib", "lib"]),
                Step::git(folder, ["submodule", "update", "--init", "--recursive", "--", "lib"]),
            ],
            danger: Danger::None,
            undo: Undo::Impossible(Text::from("Remove it.")),
            after: Vec::new(),
            on_failure: vec![FollowUp::AllowFileProtocol],
        }
    }

    fn lines(plan: &Plan) -> Vec<String> {
        plan.steps.iter().map(Step::log_line).collect()
    }

    #[test]
    fn submodule_allow_file_protocol__the_refused_add_is_offered_again_with_the_protocol_allowed_on_both_commands() {
        let said = format!("Cloning into 'lib'...\nfatal: {FILE_NOT_ALLOWED}\nfatal: clone of '../lib' failed");
        let plan = adding();
        let failed = FailedPlan {
            plan: &plan,
            failed_step: 0,
            said: &said,
        };
        let offer = allow_file_protocol(&failed).unwrap();
        assert_eq!(
            lines(&offer),
            [
                "git -c protocol.file.allow=always submodule add -- ../lib lib",
                "git -c protocol.file.allow=always submodule update --init --recursive -- lib",
            ]
        );
        assert_eq!(offer.title.as_str(), "Add submodule lib from a folder on disk");
        assert_eq!(offer.danger, Danger::Confirm(Text::from(WHY_ALLOWED)));
        assert_eq!(offer.on_failure, []);
    }

    #[test]
    fn submodule_allow_file_protocol__nothing_is_changed_in_the_configuration_of_git() {
        let plan = adding();
        let said = format!("fatal: {FILE_NOT_ALLOWED}");
        let failed = FailedPlan {
            plan: &plan,
            failed_step: 1,
            said: &said,
        };
        let offer = allow_file_protocol(&failed).unwrap();
        assert_eq!(
            lines(&offer),
            ["git -c protocol.file.allow=always submodule update --init --recursive -- lib"],
            "only the commands from the one that failed, each allowed with -c, none writing the configuration"
        );
        assert!(lines(&offer).iter().all(|line| !line.contains("git config")));
    }

    #[test]
    fn submodule_allow_file_protocol__a_failure_with_any_other_message_offers_nothing() {
        let plan = adding();
        let failed = FailedPlan {
            plan: &plan,
            failed_step: 0,
            said: "fatal: repository '../lib' does not exist",
        };
        assert_eq!(allow_file_protocol(&failed), None);
    }
}
