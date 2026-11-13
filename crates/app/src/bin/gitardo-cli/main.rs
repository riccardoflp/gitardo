//! `gitardo-cli`: the state of a repository, and its features run from a terminal, through the same engine as the
//! window. It is how the features are tried before there is a window, and how the integration tests run them.

#![forbid(unsafe_code)]

mod arguments;
mod asker;
mod show;

use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use engine::{Clock, CommandLog, Engine, Finished, LogLevel};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::contract::{CommandPreview, Context, FeatureSettings, FollowUp};
use model::state::RepositoryState;

use arguments::{Arguments, Command, FeatureRun, PlanChoice};
use asker::ArgumentAsker;

/// The program ran what it was asked.
const DONE: u8 = 0;
/// Git or a feature refused, or failed.
const FAILED: u8 = 1;
/// The command line did not say enough: an option, an answer or a confirmation is missing.
const NEEDS_MORE: u8 = 2;

/// The system's clock.
struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        Timestamp(i64::try_from(seconds).unwrap_or(i64::MAX))
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut out = io::stdout().lock();
    let mut errors = io::stderr().lock();
    let code = match arguments::parse(&arguments) {
        Ok(arguments) => run(&arguments, &mut out, &mut errors),
        Err(problem) => writeln!(errors, "{problem}\n\n{}", arguments::USAGE).map(|()| NEEDS_MORE),
    };
    // When the terminal is gone there is no one left to tell.
    ExitCode::from(code.unwrap_or(FAILED))
}

fn run(arguments: &Arguments, out: &mut dyn Write, errors: &mut dyn Write) -> io::Result<u8> {
    if arguments.command == Command::Features {
        for (feature, _operation) in features::OPERATIONS {
            writeln!(out, "{}", feature.0)?;
        }
        return Ok(DONE);
    }
    let git = Git::new(&arguments.repository);
    let state = match read_state(
        &git,
        &StateRequest {
            folder: arguments.repository.clone(),
        },
    ) {
        Ok(state) => state,
        Err(error) => {
            writeln!(errors, "{error}")?;
            return Ok(FAILED);
        }
    };
    match &arguments.command {
        Command::Feature(feature) => run_feature(state, feature, out, errors),
        Command::State | Command::Features => show::state(out, &state).map(|()| DONE),
    }
}

fn run_feature(
    state: RepositoryState,
    run: &FeatureRun,
    out: &mut dyn Write,
    errors: &mut dyn Write,
) -> io::Result<u8> {
    let Some((feature, operation)) = features::operation(&run.identifier) else {
        writeln!(
            errors,
            "{} is not a feature: gitardo-cli features lists them.",
            run.identifier
        )?;
        return Ok(NEEDS_MORE);
    };
    let mut settings = FeatureSettings::default();
    settings.worktrees.home = app::settings::home_folder().unwrap_or_default();
    if run.plan_choice == PlanChoice::ShowOnly {
        // Every plan is then shown before it runs, and the asker never confirms one.
        settings.command_preview = CommandPreview::All;
    }
    let context = Context {
        target: run.target.clone(),
        settings,
        now: SystemClock.now(),
        repository: Arc::new(state),
    };
    let git = Git::new(context.repository.folder.clone());
    let mut engine = Engine::new(git, CommandLog::new(LogLevel::Info), Box::new(SystemClock));
    engine.find_operations_with(features::operation);
    let mut asker = ArgumentAsker::new(run.answers.clone(), run.plan_choice);
    let finished = match engine.run_feature(feature, operation, &context, &mut asker) {
        Ok(finished) => finished,
        Err(error) => {
            writeln!(errors, "{error}")?;
            if let engine::EngineError::Step(failure) = &error {
                for follow_up in &failure.on_failure {
                    follow_up_line(out, follow_up)?;
                }
            }
            return Ok(FAILED);
        }
    };
    let code = report(&finished, &asker, run.plan_choice, out, errors)?;
    autostash_notice(&context, &finished, out)?;
    Ok(code)
}

/// Says, after a pull with `--autostash`, that the changes it set aside were left in a stash, when they were
/// (`stash.autostash-left`).
fn autostash_notice(context: &Context, finished: &Finished, out: &mut dyn Write) -> io::Result<()> {
    let Finished::Ran { plan, .. } = finished else {
        return Ok(());
    };
    if !plan.after.contains(&FollowUp::CheckAutostash) {
        return Ok(());
    }
    let request = StateRequest {
        folder: context.repository.folder.clone(),
    };
    let Ok(after) = read_state(&Git::new(&context.repository.folder), &request) else {
        return Ok(());
    };
    let outcome = model::stash::PullOutcome {
        had_autostash: true,
        succeeded: true,
        stashes_before: &context.repository.stashes,
        stashes_after: &after.stashes,
    };
    match model::stash::autostash_left(&outcome) {
        Some(notice) => writeln!(out, "{}", notice.message),
        None => Ok(()),
    }
}

/// The line `gitardo-cli` prints for what the window would do once a plan ran, if any.
fn follow_up_line(out: &mut dyn Write, follow_up: &FollowUp) -> io::Result<()> {
    match follow_up {
        FollowUp::Say(words) => writeln!(out, "{words}")?,
        FollowUp::Offer(feature, label) => writeln!(out, "Next: {label} ({})", feature.0)?,
        FollowUp::SelectCommit(commit) => writeln!(out, "Select {commit}")?,
        FollowUp::SelectBeforeHead(count) => writeln!(out, "Select HEAD~{count}")?,
        FollowUp::OpenCommitPage(commit) => writeln!(out, "Open the page of {commit}")?,
        FollowUp::CopyToClipboard(text) => writeln!(out, "Copy {text}")?,
        FollowUp::FillCommitMessage(message) => writeln!(out, "Commit message: {message}")?,
        FollowUp::OpenComparison(target) => {
            writeln!(out, "Compare {} with {}", target.first.name, target.second.name)?;
        }
        FollowUp::CloseMergePage(path) => writeln!(out, "Close the Merge page of {path}")?,
        FollowUp::CompareInPlace(sides) => {
            writeln!(out, "Compare {} with {}", sides.from.name, sides.to.name)?;
        }
        FollowUp::OpenReview(files) => writeln!(out, "Review {}", files.join(", "))?,
        FollowUp::OpenWindow(folder) => writeln!(out, "Open {} in a new window", folder.display())?,
        FollowUp::ShowHistory(Some(history)) => writeln!(out, "Show the history of {}", history.paths().join(", "))?,
        FollowUp::ShowHistory(None) => writeln!(out, "Show all commits")?,
        FollowUp::ShowBlame(blame) => writeln!(out, "Show the blame of {}", blame.path)?,
        FollowUp::HideBlame(path, _) => writeln!(out, "Hide the blame of {path}")?,
        FollowUp::OpenFile(file) => writeln!(out, "Open {} at line {}", file.blame.path, file.line)?,
        FollowUp::EditFile(path, line) => writeln!(out, "Open {path} at line {line}")?,
        FollowUp::OpenFileDiff(sides) => {
            writeln!(
                out,
                "Compare {} of {} with {}",
                sides.path, sides.right.name, sides.left.name
            )?;
        }
        FollowUp::ShowFolder(folder) => writeln!(out, "Show {} in this window", folder.display())?,
        FollowUp::OfferRun(offered) => writeln!(out, "Next: {} ({})", offered.label, offered.feature.0)?,
        FollowUp::KeepGitHubToken(_) => writeln!(out, "Keep the GitHub token in the credential store")?,
        FollowUp::KeepAiKey(..) => writeln!(out, "Keep the key of the AI provider in the credential store")?,
        FollowUp::SelectHead
        | FollowUp::DeleteWhenRebaseEnded(_)
        | FollowUp::EmptyCommitMessage
        | FollowUp::ReadPullRequests
        | FollowUp::CheckAutostash
        | FollowUp::Recover(_)
        | FollowUp::PreparedMessage
        | FollowUp::Cleanup(_)
        | FollowUp::AllowFileProtocol
        | FollowUp::OfferWhenFailed(_)
        | FollowUp::CheckBisect => {}
    }
    Ok(())
}

fn report(
    finished: &Finished,
    asker: &ArgumentAsker,
    plan_choice: PlanChoice,
    out: &mut dyn Write,
    errors: &mut dyn Write,
) -> io::Result<u8> {
    match finished {
        Finished::Ran {
            plan,
            steps,
            went_on_after,
            ..
        } => {
            show::plan(out, plan)?;
            // A step that failed and was got past, as a cherry-pick of a commit that changes nothing, ran more steps.
            for line in steps.iter().skip(plan.steps.len()) {
                writeln!(out, "  > {line}")?;
            }
            for follow_up in &plan.after {
                follow_up_line(out, follow_up)?;
            }
            // The links and copies of a worktree that failed while the others went on.
            let failed: Vec<String> = went_on_after.iter().map(|failure| failure.failed.clone()).collect();
            if let Some(warning) = model::worktree_setup::failed_setup_steps(&failed) {
                writeln!(errors, "{warning}")?;
            }
            Ok(DONE)
        }
        Finished::Refused(refusal) => {
            writeln!(errors, "{}", refusal.message)?;
            if let Some((feature, label)) = &refusal.offer {
                writeln!(errors, "Instead: {label} ({})", feature.0)?;
            }
            Ok(FAILED)
        }
        Finished::BackedOut => {
            if let Some(question) = &asker.unanswered {
                show::question(errors, question)?;
            }
            Ok(NEEDS_MORE)
        }
        Finished::NotConfirmed(plan) => {
            show::plan(out, plan)?;
            if plan_choice == PlanChoice::ShowOnly {
                return Ok(DONE);
            }
            writeln!(
                errors,
                "This plan is shown before it runs: run again with --yes to run it."
            )?;
            Ok(NEEDS_MORE)
        }
    }
}
