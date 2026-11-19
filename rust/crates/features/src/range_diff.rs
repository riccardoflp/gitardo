//! `range-diff.compare`, which opens what a rebase or an amend changed in a branch, commit by commit, what a force push
//! would replace, or what someone else's force push replaced (`docs/features/07-history-rewriting.md`).

use model::contract::{
    AnswerKey, Answers, Context, FeatureId, FollowUp, OfferedRun, Outcome, Query, Refusal, Target, Text,
};
use model::range_diff::{RangeDiffTarget, RangesInput, Rewrite, RewriteKind, Side, ranges};

/// The key of what the branch is compared with: its last rewrite, or [`WITH_UPSTREAM`].
pub const WITH: AnswerKey = AnswerKey("range-diff.with");

/// The answer under [`WITH`] that compares a local branch with its upstream: **Compare with `<upstream>` commit by
/// commit**.
pub const WITH_UPSTREAM: &str = "upstream";

/// The key of the last rewrite of the reference compared.
const LAST_REWRITE: AnswerKey = AnswerKey("range-diff.last-rewrite");

/// The feature itself, as it is offered.
const COMPARE: FeatureId = FeatureId("range-diff.compare");

/// The words of the button that opens what a rewrite changed, after it ended.
pub const SEE_WHAT_CHANGED: &str = "See what changed";

/// The words of the button of the warning of a force push.
pub const COMPARE_FIRST: &str = "Compare first";

/// Opens the page of what a rewrite changed: for a local branch, its last rebase or amend, or, with [`WITH_UPSTREAM`],
/// its upstream against it; for a remote branch, the last force push a fetch found. It changes nothing.
///
/// Feature: range-diff.compare
#[must_use]
pub fn compare(context: &Context, answers: &Answers) -> Outcome {
    let state = &context.repository;
    match &context.target {
        Target::RemoteBranch(name) => force_push(answers, name),
        Target::Branch(name) => local(context, answers, name),
        Target::Head => match state.checked_out_branch() {
            Some(name) => local(context, answers, name),
            None => refuse("Check out a branch to see what its last rewrite changed."),
        },
        _ => refuse("Choose a branch."),
    }
}

/// A local branch: its last rewrite, or its upstream against it.
fn local(context: &Context, answers: &Answers, name: &str) -> Outcome {
    let state = &context.repository;
    let Some(branch) = state.branches.iter().find(|branch| branch.name == name) else {
        return refuse(&format!("{name} is not a local branch."));
    };
    let rewrite = match last_rewrite(answers, &format!("refs/heads/{name}")) {
        Ok(rewrite) => rewrite,
        Err(read) => return *read,
    };
    if answers.choice(WITH) != Some(WITH_UPSTREAM) {
        return of_rewrite(rewrite, name, "has not been rebased or amended lately.");
    }
    let upstream = branch
        .upstream
        .as_ref()
        .filter(|upstream| !upstream.is_gone)
        .and_then(|upstream| upstream.reference.strip_prefix("refs/remotes/"))
        .and_then(|upstream| state.remote_branches.iter().find(|remote| remote.name == upstream));
    let Some(upstream) = upstream else {
        return refuse(&format!("{name} follows no remote branch."));
    };
    let input = RangesInput::Upstream {
        branch: Side {
            name,
            commit: &branch.commit,
        },
        upstream: Side {
            name: &upstream.name,
            commit: &upstream.commit,
        },
        last_rewrite: rewrite,
    };
    let Some(ranges) = ranges(&input) else {
        return refuse(&format!("{name} and {} are at the same commit.", upstream.name));
    };
    opens(RangeDiffTarget {
        title: format!("{name} and {}, commit by commit", upstream.name),
        branch: name.to_owned(),
        ranges,
    })
}

/// A remote branch: the last force push a fetch found.
fn force_push(answers: &Answers, name: &str) -> Outcome {
    match last_rewrite(answers, &format!("refs/remotes/{name}")) {
        Ok(rewrite) => of_rewrite(rewrite, name, "has not been force pushed lately."),
        Err(read) => *read,
    }
}

/// The page of a rewrite, or why there is none: `none` ends the sentence after the name of the branch.
fn of_rewrite(rewrite: Option<&Rewrite>, name: &str, none: &str) -> Outcome {
    let Some(rewrite) = rewrite else {
        return refuse(&format!("{name} {none}"));
    };
    let Some(ranges) = ranges(&RangesInput::Rewrite(rewrite)) else {
        return refuse(&format!(
            "The last amend of {name} was of its first commit, which has nothing before it to compare."
        ));
    };
    let what = match rewrite.kind {
        RewriteKind::Rebase { .. } => "rebase",
        RewriteKind::Amend { .. } => "amend",
        RewriteKind::ForcePush => "force push",
    };
    opens(RangeDiffTarget {
        title: format!("What the {what} of {name} changed"),
        branch: name.to_owned(),
        ranges,
    })
}

/// The last rewrite of a reference, read first.
fn last_rewrite<'a>(answers: &'a Answers, reference: &str) -> Result<Option<&'a Rewrite>, Box<Outcome>> {
    answers.last_rewrite(LAST_REWRITE).ok_or_else(|| {
        Box::new(Outcome::Read(
            LAST_REWRITE,
            Query::LastRewrite {
                reference: reference.to_owned(),
            },
        ))
    })
}

/// The plan with no step that opens the page.
fn opens(target: RangeDiffTarget) -> Outcome {
    let title = target.title.clone();
    Outcome::Run(crate::compare::no_step(
        title,
        vec![FollowUp::OpenRangeDiff(Box::new(target))],
    ))
}

/// A refusal saying why.
fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}

/// **See what changed** on what is said once a rewrite of a local branch ended, which opens what its last rewrite
/// changed.
#[must_use]
pub fn see_what_changed(branch: &str) -> FollowUp {
    FollowUp::OfferRun(Box::new(OfferedRun {
        label: Text::from(SEE_WHAT_CHANGED),
        feature: COMPARE,
        target: Target::Branch(branch.to_owned()),
        answers: Answers::new(),
        when_said: Vec::new(),
    }))
}

/// **See what changed** for the branch checked out, none when `HEAD` is detached.
#[must_use]
pub fn see_what_changed_here(context: &Context) -> Option<FollowUp> {
    context.repository.checked_out_branch().map(see_what_changed)
}

/// **Compare first** in the warning of a force push of a local branch, which compares its upstream with it instead.
#[must_use]
pub fn compare_first(branch: &str) -> FollowUp {
    FollowUp::OfferBeforeRun(Box::new(OfferedRun {
        label: Text::from(COMPARE_FIRST),
        feature: COMPARE,
        target: Target::Branch(branch.to_owned()),
        answers: Answers::new().with(WITH, model::contract::Answer::Choice(WITH_UPSTREAM.to_owned())),
        when_said: Vec::new(),
    }))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
