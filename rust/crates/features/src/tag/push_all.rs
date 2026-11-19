//! `tag.push-all`: sends to a remote the tags it does not have, every one or those ticked, in one push.

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, FeatureId, FollowUp, Outcome, Plan, Query, Question, QuestionKind,
    Refusal, Step, Text, Undo,
};
use model::tag::{TagComparison, TagsOnRemote, compare_remote};

use crate::branch::target::refuse;

/// The remote the tags are pushed to, when the repository has several.
pub const REMOTE: AnswerKey = AnswerKey("remote");

/// The tags ticked, by their names.
pub const TAGS: AnswerKey = AnswerKey("tags");

/// What the remote has of the tags, read before the pick: the remote is asked over the network.
const REMOTE_TAGS: AnswerKey = AnswerKey("remote-tags");

/// The commits of the tags the remote lacks, for the short hash and the subject beside each.
const COMMITS: AnswerKey = AnswerKey("commits");

/// Asks which remote with several, the one of `remote.default` first; asks the remote for its tags
/// (`tag.read-remote-tags`) and sorts those of the repository against them (`tag.compare-remote`); then picks among
/// those it lacks, all ticked. Every one ticked is `git push <remote> --tags`, which sends only what the remote lacks;
/// some, or any while the remote has a tag of the same name under another object, are each named in full, since
/// `--tags` would send that one too and the remote would refuse the push (Q1115, Q1116).
///
/// Feature: tag.push-all
#[must_use]
pub fn push_all(context: &Context, answers: &Answers) -> Outcome {
    let remotes = &context.repository.remotes;
    let Some(default) = model::remote::default(remotes) else {
        return Outcome::Refuse(Refusal {
            message: Text::from("This repository has no remote."),
            offer: Some((FeatureId("remote.add"), Text::from("Add remote…"))),
        });
    };
    if context.repository.tags.is_empty() {
        return refuse("This repository has no tags.");
    }
    let remote = match (remotes.len(), answers.choice(REMOTE)) {
        (_, Some(remote)) => remote.to_owned(),
        (1, None) => default,
        (_, None) => return Outcome::Ask(remote_question(remotes, &default)),
    };
    let Some(Answer::RemoteTags(read)) = answers.get(REMOTE_TAGS) else {
        return Outcome::Read(REMOTE_TAGS, Query::RemoteTags { remote });
    };
    let compared = compare_remote(&read.here, read);
    let missing = newest_first(context, &compared.not_pushed());
    if missing.is_empty() {
        return refuse(&format!("{remote} has every tag of this repository."));
    }
    let Some(Answer::Commits(commits)) = answers.get(COMMITS) else {
        let revisions = missing.iter().map(|tag| tag.commit.as_str().to_owned()).collect();
        return Outcome::Read(COMMITS, Query::Commits { revisions });
    };
    let Some(ticked) = answers.choices(TAGS) else {
        return Outcome::Ask(tags_question(&remote, &missing, commits, &compared));
    };
    if ticked.is_empty() {
        return refuse("No tags selected.");
    }
    Outcome::Run(plan(
        context,
        &remote,
        ticked,
        missing.len() == ticked.len() && compared.different_there().is_empty(),
    ))
}

/// The tags of the repository among those named, the newest first: by the date of the commit, or of an annotated tag.
fn newest_first<'a>(context: &'a Context, names: &[&str]) -> Vec<&'a model::state::Tag> {
    let mut tags: Vec<&model::state::Tag> = context
        .repository
        .tags
        .iter()
        .filter(|tag| names.contains(&tag.name.as_str()))
        .collect();
    tags.sort_by(|left, right| right.date.cmp(&left.date).then_with(|| left.name.cmp(&right.name)));
    tags
}

/// The push: `--tags` when it sends what was ticked and nothing else, else each tag named in full.
fn plan(context: &Context, remote: &str, ticked: &[String], is_every_tag: bool) -> Plan {
    let mut arguments = vec![String::from("push"), remote.to_owned()];
    if is_every_tag {
        arguments.push(String::from("--tags"));
    } else {
        arguments.extend(ticked.iter().map(|name| format!("refs/tags/{name}")));
    }
    let title = match ticked {
        [one] => format!("Push tag {one} to {remote}"),
        _ => format!("Push {} tags to {remote}", ticked.len()),
    };
    Plan {
        title: Text::from(title),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: model::contract::Danger::None,
        undo: Undo::Impossible(Text::from(
            "A push changes the remote, which Undo does not take back. Delete from the remote takes a tag away from it.",
        )),
        after: vec![FollowUp::TagsOnRemote(Box::new(TagsOnRemote::pushed(
            remote,
            ticked.to_vec(),
        )))],
        on_failure: Vec::new(),
    }
}

/// The pick of the remote, the one of `remote.default` first.
fn remote_question(remotes: &[String], default: &str) -> Question {
    let mut ordered: Vec<&String> = remotes.iter().collect();
    ordered.sort_by_key(|remote| remote.as_str() != default);
    Question {
        key: REMOTE,
        kind: QuestionKind::Pick {
            choices: ordered
                .into_iter()
                .map(|remote| Choice {
                    identifier: remote.clone(),
                    label: Text::from(remote.as_str()),
                    description: None,
                })
                .collect(),
        },
        title: Text::from("Push tags to"),
        detail: None,
        default: Some(Answer::Choice(default.to_owned())),
    }
}

/// The pick of the tags the remote lacks, all ticked, each with the short hash and the subject of its commit. A pick
/// of several has no entry that cannot be ticked, so the tags the remote has under another object are said in its
/// detail rather than listed, and are never pushed (Q1116).
fn tags_question(
    remote: &str,
    missing: &[&model::state::Tag],
    commits: &[model::commit::Commit],
    compared: &TagComparison,
) -> Question {
    let choices = missing
        .iter()
        .map(|tag| {
            let subject = commits
                .iter()
                .find(|commit| commit.id == tag.commit)
                .map(|commit| commit.subject.as_str())
                .unwrap_or_default();
            Choice {
                identifier: tag.name.clone(),
                label: Text::from(tag.name.as_str()),
                description: Some(Text::from(format!("{} {subject}", tag.commit.short()).trim_end())),
            }
        })
        .collect();
    let different = compared.different_there();
    let detail = (!different.is_empty()).then(|| {
        Text::from(format!(
            "{remote} has another tag of this name, which is not pushed: {}.",
            different.join(", ")
        ))
    });
    Question {
        key: TAGS,
        kind: QuestionKind::PickMany { choices },
        title: Text::from(format!("Tags not on {remote}")),
        detail,
        default: Some(Answer::Choices(missing.iter().map(|tag| tag.name.clone()).collect())),
    }
}

#[cfg(test)]
mod tests;
