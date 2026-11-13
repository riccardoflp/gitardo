//! What the command line asks for.

use std::collections::BTreeMap;
use std::path::PathBuf;

use model::CommitId;
use model::contract::Target;

/// What `gitardo-cli` says when it is not given what it needs.
pub const USAGE: &str = "Usage:
  gitardo-cli [--repository <folder>] state
  gitardo-cli features
  gitardo-cli [--repository <folder>] [--target <what>] [--plan] [--yes] <feature> [<key>=<answer>]...

  --repository <folder>   the repository, by any folder in it; the current folder when not given
  --target <what>         what the feature is invoked on: commit:<hash>, commits:<hash>,<hash> (newest first),
                          branch:<name>, remote-branch:<name>, tag:<name>, stash:<N>, remote:<name>,
                          files:<path>,<path>, submodule:<folder>; what is checked out when not given
  --plan                  show the plan and do not run it
  --yes                   run a plan that asks to be confirmed first
  <key>=<answer>          the answer of a question: the text typed, the identifier of a choice, or yes";

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print the state of the repository.
    State,
    /// List the features that can be run.
    Features,
    /// Run a feature.
    Feature(FeatureRun),
}

/// A feature to run, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureRun {
    /// Its identifier: `branch.create`.
    pub identifier: String,
    /// What it is invoked on.
    pub target: Target,
    /// The answers of its questions, by key.
    pub answers: BTreeMap<String, String>,
    /// What to do with its plan.
    pub plan_choice: PlanChoice,
}

/// What to do with the plan of a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanChoice {
    /// Run it, unless it asks to be confirmed first.
    RunUnlessConfirmed,
    /// Run it, confirming it when it asks.
    RunConfirmed,
    /// Show it and run nothing.
    ShowOnly,
}

/// The command line, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    /// A folder of the repository.
    pub repository: PathBuf,
    /// What to do.
    pub command: Command,
}

/// Reads the arguments after the name of the program; what is wrong with them, in a sentence, when they cannot be
/// read.
pub fn parse(arguments: &[String]) -> Result<Arguments, String> {
    let mut repository = PathBuf::from(".");
    let mut target = Target::Head;
    let mut plan_choice = PlanChoice::RunUnlessConfirmed;
    let mut words = Vec::new();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--repository" => repository = PathBuf::from(rest.next().ok_or("--repository needs a folder.")?),
            "--target" => target = parse_target(rest.next().ok_or("--target needs what the feature is on.")?)?,
            "--plan" => plan_choice = PlanChoice::ShowOnly,
            "--yes" if plan_choice != PlanChoice::ShowOnly => plan_choice = PlanChoice::RunConfirmed,
            "--yes" => {}
            option if option.starts_with("--") => return Err(format!("{option} is not an option of gitardo-cli.")),
            word => words.push(word.to_owned()),
        }
    }
    let command = command(&words, target, plan_choice)?;
    Ok(Arguments { repository, command })
}

fn command(words: &[String], target: Target, plan_choice: PlanChoice) -> Result<Command, String> {
    let Some((first, answers)) = words.split_first() else {
        return Err(String::from("Say what to do."));
    };
    match first.as_str() {
        "state" if answers.is_empty() => Ok(Command::State),
        "features" if answers.is_empty() => Ok(Command::Features),
        identifier => {
            let mut parsed = BTreeMap::new();
            for answer in answers {
                let (key, value) = answer
                    .split_once('=')
                    .ok_or_else(|| format!("{answer} is not an answer: write <key>=<answer>."))?;
                let _replaced = parsed.insert(key.to_owned(), value.to_owned());
            }
            Ok(Command::Feature(FeatureRun {
                identifier: identifier.to_owned(),
                target,
                answers: parsed,
                plan_choice,
            }))
        }
    }
}

fn parse_target(text: &str) -> Result<Target, String> {
    let (kind, name) = text
        .split_once(':')
        .ok_or_else(|| format!("{text} is not a target: write <kind>:<name>."))?;
    match kind {
        "commit" => CommitId::parse(name)
            .map(Target::Commit)
            .ok_or_else(|| format!("{name} is not a full commit hash.")),
        "branch" => Ok(Target::Branch(name.to_owned())),
        "remote-branch" => Ok(Target::RemoteBranch(name.to_owned())),
        "tag" => Ok(Target::Tag(name.to_owned())),
        "stash" => name
            .parse()
            .map(Target::Stash)
            .map_err(|_| format!("{name} is not the place of a stash: 0 for stash@{{0}}.")),
        "remote" => Ok(Target::Remote(name.to_owned())),
        "files" => Ok(Target::Files(name.split(',').map(str::to_owned).collect())),
        "file" => Ok(Target::File(name.to_owned())),
        "changes" => Ok(Target::Changes),
        "submodule" => Ok(Target::Submodule(name.to_owned())),
        "commits" => name
            .split(',')
            .map(|hash| CommitId::parse(hash).ok_or_else(|| format!("{hash} is not a full commit hash.")))
            .collect::<Result<Vec<_>, _>>()
            .map(Target::Commits),
        _ => Err(format!(
            "{kind} is not a kind of target: commit, commits, branch, remote-branch, tag, file, changes, stash, \
             remote, files or submodule."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, PlanChoice, parse};
    use model::contract::Target;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn reads_a_feature_with_its_answers_and_target() {
        let arguments = parse(&words(
            "--repository work --target branch:release --yes branch.create name=fix mode=only",
        ))
        .unwrap();
        assert_eq!(arguments.repository.to_string_lossy(), "work");
        let Command::Feature(run) = arguments.command else {
            panic!("a feature is run")
        };
        assert_eq!(run.identifier, "branch.create");
        assert_eq!(run.target, Target::Branch(String::from("release")));
        assert_eq!(run.answers.get("name").map(String::as_str), Some("fix"));
        assert_eq!(run.plan_choice, PlanChoice::RunConfirmed);
    }

    #[test]
    fn shows_only_when_asked_for_the_plan_even_with_yes() {
        let Command::Feature(run) = parse(&words("--yes --plan undo.last")).unwrap().command else {
            panic!()
        };
        assert_eq!(run.plan_choice, PlanChoice::ShowOnly);
    }

    #[test]
    fn says_what_is_wrong() {
        for line in [
            "",
            "--target nothing x",
            "--frobnicate state",
            "branch.create name",
            "--target commit:abc x",
        ] {
            assert!(parse(&words(line)).is_err(), "{line}");
        }
        assert_eq!(parse(&words("state")).unwrap().command, Command::State);
    }
}
