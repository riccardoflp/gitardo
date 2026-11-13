//! Comparisons: two branches, tags, commits, or one of them and the working tree, chosen and opened in a page or in
//! the inspector, and what the page does to the repository: a file taken from the second side, the changes copied as a
//! patch (`docs/features/10-compare.md`).

mod choose;
mod copy_patch;
mod in_place;
mod path_with;
mod take_file;
mod with;

pub use choose::choose;
pub use copy_patch::copy_patch;
pub use in_place::{selected_before, with_current};
pub use path_with::path_with;
pub use take_file::take_file;
pub use with::{change_side, with};

use model::CommitId;
use model::compare::{
    CompareSide, ComparisonPart, ComparisonTarget, PickItem, RecentComparison, Side, SideChoice, SideChoicesInput,
    SideInput, follow_sides, initial_mode, side_choices,
};
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal,
    Target, Text, Undo, Validation,
};
use model::refs::References;

/// The key of the comparisons opened before in the repository, which the window gives before `compare.choose` asks.
pub const RECENT: AnswerKey = AnswerKey("recent");
/// The key of the first side chosen.
pub const FIRST: AnswerKey = AnswerKey("first");
/// The key of the second side chosen.
pub const SECOND: AnswerKey = AnswerKey("second");

/// The prompt of the box where a revision is typed.
const REVISION_PROMPT: &str = "A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}";

/// The keys of the question of a side: what is picked, what is typed after **Enter a hash or a revision…**, and the
/// commit git says it stands for.
#[derive(Clone, Copy)]
struct SideKeys {
    /// What is picked.
    picked: AnswerKey,
    /// What is typed.
    typed: AnswerKey,
    /// The commit of what is typed.
    commit: AnswerKey,
}

impl SideKeys {
    /// The keys of the first side.
    const FIRST: SideKeys = SideKeys {
        picked: FIRST,
        typed: AnswerKey("first-revision"),
        commit: AnswerKey("first-commit"),
    };
    /// The keys of the second side.
    const SECOND: SideKeys = SideKeys {
        picked: SECOND,
        typed: AnswerKey("second-revision"),
        commit: AnswerKey("second-commit"),
    };
}

/// A question of a side: its title, its placeholder, the side chosen already, whether the working tree may be offered,
/// and the comparisons opened before when they are.
struct SideQuestion<'a> {
    /// The title.
    title: String,
    /// What the box says before anything is typed.
    placeholder: &'a str,
    /// The side chosen already, left out.
    chosen: Option<&'a CompareSide>,
    /// Whether the working tree may be offered.
    may_offer_working_tree: bool,
    /// The comparisons opened before, offered first.
    recent: Option<&'a [RecentComparison]>,
}

/// What a side question came to.
enum Picked {
    /// A side.
    Side(CompareSide),
    /// A comparison opened before, by its place in the list.
    Recent(usize),
}

/// The side picked or typed for a question, or what the feature does next: ask, read, or refuse.
fn side(
    context: &Context,
    answers: &Answers,
    keys: SideKeys,
    question: &SideQuestion<'_>,
) -> Result<Picked, Box<Outcome>> {
    let items = items(context, question);
    let Some(picked) = answers.choice(keys.picked) else {
        return Err(Box::new(Outcome::Ask(pick(keys.picked, question, &items))));
    };
    if picked == "working-tree" {
        return Ok(Picked::Side(CompareSide::working_tree()));
    }
    if let Some(place) = picked.strip_prefix("recent:").and_then(|place| place.parse().ok()) {
        return Ok(Picked::Recent(place));
    }
    if let Some(name) = picked.strip_prefix("ref:") {
        let commit = items.iter().find_map(|item| match &item.choice {
            SideChoice::Named { name: named, commit } if named == name => Some(commit),
            _ => None,
        });
        return match commit {
            Some(commit) => Ok(Picked::Side(named(name, commit))),
            None => Err(Box::new(Outcome::Refuse(Refusal::saying(&format!(
                "\"{name}\" is not a commit of this repository."
            ))))),
        };
    }
    typed(answers, keys, &question.title).map(Picked::Side)
}

/// The side of what is typed after **Enter a hash or a revision…**: named by the first seven characters of a hash, by
/// what was typed otherwise.
fn typed(answers: &Answers, keys: SideKeys, title: &str) -> Result<CompareSide, Box<Outcome>> {
    let Some(text) = answers.text(keys.typed).map(str::trim) else {
        return Err(Box::new(Outcome::Ask(Question {
            key: keys.typed,
            kind: QuestionKind::Text {
                placeholder: None,
                validation: Validation::Anything,
            },
            title: Text::from(title),
            detail: Some(Text::from(REVISION_PROMPT)),
            default: None,
        })));
    };
    let Some(commit) = answers.commit(keys.commit) else {
        return Err(Box::new(Outcome::Read(
            keys.commit,
            Query::Revision {
                revision: text.to_owned(),
            },
        )));
    };
    let Some(commit) = commit else {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(&format!(
            "\"{text}\" is not a commit of this repository."
        )))));
    };
    let is_hash = !text.is_empty() && commit.as_str().starts_with(&text.to_lowercase());
    let name = if is_hash { commit.short() } else { text };
    Ok(follow_sides(&SideInput {
        name,
        chosen: &Side::Commit(commit.clone()),
        is_followed: None,
        resolved: Some(commit),
    }))
}

/// A side named by a branch, a tag or what is checked out, at its commit: followed, as its name resolves to it.
fn named(name: &str, commit: &CommitId) -> CompareSide {
    follow_sides(&SideInput {
        name,
        chosen: &Side::Commit(commit.clone()),
        is_followed: None,
        resolved: Some(commit),
    })
}

/// The items of a side question.
fn items(context: &Context, question: &SideQuestion<'_>) -> Vec<PickItem> {
    let repository = &context.repository;
    let references = References::of(repository);
    side_choices(&SideChoicesInput {
        references: &references,
        head: repository.head.commit.as_ref(),
        branch: repository.checked_out_branch(),
        chosen: question.chosen,
        may_offer_working_tree: question.may_offer_working_tree,
        recent: question.recent,
    })
}

/// The pick of a side, each item by an identifier its choice can be told from.
fn pick(key: AnswerKey, question: &SideQuestion<'_>, items: &[PickItem]) -> Question {
    Question {
        key,
        kind: QuestionKind::Pick {
            choices: items
                .iter()
                .map(|item| Choice {
                    identifier: match &item.choice {
                        SideChoice::Recent(place) => format!("recent:{place}"),
                        SideChoice::EnterRevision => String::from("revision"),
                        SideChoice::Named { name, .. } => format!("ref:{name}"),
                        SideChoice::WorkingTree => String::from("working-tree"),
                    },
                    label: Text::from(item.label.as_str()),
                    description: (!item.description.is_empty()).then(|| Text::from(item.description.as_str())),
                })
                .collect(),
        },
        title: Text::from(question.title.as_str()),
        detail: Some(Text::from(question.placeholder)),
        default: None,
    }
}

/// The side a feature was invoked on: a commit by its short hash, fixed; a branch, a remote branch or a tag by its
/// name, followed.
fn side_of_target(context: &Context) -> Option<CompareSide> {
    let repository = &context.repository;
    let commit_of = |name: &str| -> Option<CommitId> {
        match &context.target {
            Target::Branch(_) => repository
                .branches
                .iter()
                .find(|branch| branch.name == name)
                .map(|branch| branch.commit.clone()),
            Target::RemoteBranch(_) => repository
                .remote_branches
                .iter()
                .find(|remote| remote.name == name)
                .map(|remote| remote.commit.clone()),
            Target::Tag(_) => repository
                .tags
                .iter()
                .find(|tag| tag.name == name)
                .map(|tag| tag.commit.clone()),
            _ => None,
        }
    };
    match &context.target {
        Target::Commit(commit) => Some(CompareSide {
            name: commit.short().to_owned(),
            side: Side::Commit(commit.clone()),
            is_followed: false,
            is_gone: false,
        }),
        Target::Branch(name) | Target::RemoteBranch(name) | Target::Tag(name) => {
            commit_of(name).map(|commit| named(name, &commit))
        }
        _ => None,
    }
}

/// The comparison of two sides, as it opens: in the mode `compare.initial-mode` gives, narrowed to a path or not.
fn comparison(first: CompareSide, second: CompareSide, path: Option<String>) -> ComparisonTarget {
    let mode = initial_mode(&(first.clone(), second.clone()));
    ComparisonTarget {
        first,
        second,
        mode,
        path,
        merge_base: None,
        part: ComparisonPart::Whole,
    }
}

/// A plan with no step, which changes nothing in the repository and does what follows.
fn no_step(title: String, after: Vec<FollowUp>) -> Plan {
    Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A comparison changes nothing to undo.")),
        after,
        on_failure: Vec::new(),
    }
}

/// The plan that opens the comparison page of two sides.
fn opens(target: ComparisonTarget) -> Outcome {
    let title = format!("Compare {} with {}", target.first.name, target.second.name);
    Outcome::Run(no_step(title, vec![FollowUp::OpenComparison(Box::new(target))]))
}

#[cfg(test)]
pub(crate) mod testing {
    use model::compare::{CompareMode, CompareSide, ComparisonPart, ComparisonTarget, Side};
    use model::contract::{Answer, Answers};

    use crate::testing::commit;

    /// Answers that pick some choices under some keys.
    pub(crate) fn picked(choices: &[(model::contract::AnswerKey, &str)]) -> Answers {
        choices.iter().fold(Answers::new(), |answers, (key, choice)| {
            answers.with(*key, Answer::Choice((*choice).to_owned()))
        })
    }

    /// A side named by a branch, at the commit of a digit, followed.
    pub(crate) fn branch(name: &str, digit: char) -> CompareSide {
        CompareSide {
            name: name.to_owned(),
            side: Side::Commit(commit(digit)),
            is_followed: true,
            is_gone: false,
        }
    }

    /// The comparison of two sides, in a mode, of one part.
    pub(crate) fn compared(first: CompareSide, second: CompareSide, part: ComparisonPart) -> ComparisonTarget {
        ComparisonTarget {
            first,
            second,
            mode: CompareMode::Everything,
            path: None,
            merge_base: None,
            part,
        }
    }
}
