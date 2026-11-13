//! `branch.name-from-format`: what is asked, in order, to fill the format of branch names in, with the values proposed;
//! the name it makes is then shown, to change or keep, by the feature that asked.

use std::collections::BTreeMap;

use crate::CommitId;
use crate::contract::{
    Answer, AnswerKey, Answers, Choice, Outcome, Query, Question, QuestionKind, Refusal, Target, Text, Validation,
};
use crate::state::RepositoryState;

use super::format::{has_base, input_ids};
use super::{BranchInput, FormatValues, InputKind, format_name, format_problem, local_name};

/// Where the value of each input comes back, by the place of the input among those of the format.
pub const INPUTS: [AnswerKey; 10] = [
    AnswerKey("input1"),
    AnswerKey("input2"),
    AnswerKey("input3"),
    AnswerKey("input4"),
    AnswerKey("input5"),
    AnswerKey("input6"),
    AnswerKey("input7"),
    AnswerKey("input8"),
    AnswerKey("input9"),
    AnswerKey("input10"),
];

/// Where the value typed after **Type another value…** comes back, by the place of the input.
pub const TYPED: [AnswerKey; 10] = [
    AnswerKey("typed1"),
    AnswerKey("typed2"),
    AnswerKey("typed3"),
    AnswerKey("typed4"),
    AnswerKey("typed5"),
    AnswerKey("typed6"),
    AnswerKey("typed7"),
    AnswerKey("typed8"),
    AnswerKey("typed9"),
    AnswerKey("typed10"),
];

/// What `${base}` is, picked at a commit.
pub const BASE: AnswerKey = AnswerKey("base");

/// The branches that have the commit `${base}` is asked at, read.
pub const CONTAINING: AnswerKey = AnswerKey("containing");

/// The choice of a pick that types another value: a colon, which starts no option written in the settings.
pub const TYPE_ANOTHER: &str = ":type";

/// How many branches the pick of `${base}` lists at most.
const MOST_BASES: usize = 50;

/// What the questions of a format are made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatAsking<'a> {
    /// `branches.format`.
    pub format: &'a str,
    /// `branches.inputs`.
    pub inputs: &'a [BranchInput],
    /// The values given last in this workspace, by the id of their input.
    pub used: &'a BTreeMap<String, String>,
    /// What `${base}` stands for, or the commit it is asked at.
    pub base: BaseOf,
    /// What `${repo}` and `${date}` stand for, and the remotes `${base}` loses.
    pub values: FormatValues,
    /// The title of the feature that asks: **Create branch**.
    pub title: &'a str,
}

/// What `${base}` stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseOf {
    /// A name known without asking: the branch checked out, a branch, a remote branch with its remote, a tag.
    Known(String),
    /// A commit, where it is asked.
    Commit {
        /// The commit.
        commit: CommitId,
        /// The branches at the commit, local ones first, the remote ones without their remote.
        at: Vec<String>,
    },
}

/// The name a format made, and the values given for its inputs, which are remembered for the next time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatName {
    /// The name.
    pub name: String,
    /// The value given for each input, by its id.
    pub inputs: BTreeMap<String, String>,
}

/// What `${base}` is where a feature was invoked: from the header and Quick search the branch checked out; on a branch,
/// a remote branch or a tag, that one; at a commit, or on a detached `HEAD`, the commit, to ask about.
#[must_use]
pub fn base_of(state: &RepositoryState, target: &Target) -> BaseOf {
    let at_commit = |commit: &CommitId| BaseOf::Commit {
        commit: commit.clone(),
        at: names_at(state, commit),
    };
    match target {
        Target::Commit(commit) => at_commit(commit),
        Target::Branch(name) | Target::RemoteBranch(name) | Target::Tag(name) => BaseOf::Known(name.clone()),
        // Anything else is no place to start a branch: what is checked out is.
        _ => match (state.checked_out_branch(), &state.head.commit) {
            (Some(branch), _) => BaseOf::Known(branch.to_owned()),
            (None, Some(commit)) => at_commit(commit),
            (None, None) => BaseOf::Known(String::new()),
        },
    }
}

/// The branches at a commit: the local ones, then the remote ones without their remote.
fn names_at(state: &RepositoryState, commit: &CommitId) -> Vec<String> {
    let local = state
        .branches
        .iter()
        .filter(|branch| &branch.commit == commit)
        .map(|branch| branch.name.clone());
    let remote = state
        .remote_branches
        .iter()
        .filter(|branch| &branch.commit == commit)
        .map(|branch| local_name(&branch.name, &state.remotes).to_owned());
    local.chain(remote).collect()
}

/// The name the format makes once every part of it is known, with the values given; until then, what to do next: the
/// next question, the read of the branches that have a commit, or why the format is refused.
///
/// The inputs are asked in the order they first appear, each once, then `${base}` when it is at a commit. A format
/// with a problem, one with more than ten inputs, and a pattern that is not a regular expression are refused before
/// any question.
///
/// Feature: branch.name-from-format
///
/// # Errors
///
/// What to do before the name is known, as an [`Outcome`] the feature that asked returns.
pub fn name_from_format(input: &FormatAsking<'_>, answers: &Answers) -> Result<FormatName, Box<Outcome>> {
    if let Some(problem) = format_problem(input.format, input.inputs) {
        return Err(Box::new(Outcome::Refuse(Refusal {
            message: problem,
            offer: None,
        })));
    }
    let asked = asked_inputs(input)?;
    let mut values = input.values.clone();
    for (place, branch_input) in asked.iter().enumerate() {
        let value = input_value(input, (place, branch_input), answers)?;
        let _given = values.inputs.insert(branch_input.id.clone(), value);
    }
    if has_base(input.format) {
        values.base = base_value(input, answers)?;
    }
    Ok(FormatName {
        name: format_name(input.format, &values),
        inputs: values.inputs,
    })
}

/// The inputs of the format, in the order they first appear; refused when there are more than ten, or when a pattern
/// is not a regular expression.
fn asked_inputs<'a>(input: &FormatAsking<'a>) -> Result<Vec<&'a BranchInput>, Box<Outcome>> {
    let refuse = |message: String| Box::new(Outcome::Refuse(Refusal::saying(&message)));
    let ids = input_ids(input.format);
    if ids.len() > INPUTS.len() {
        return Err(refuse(format!(
            "The format uses more than {} inputs. Change it in the settings.",
            INPUTS.len()
        )));
    }
    let asked: Vec<&BranchInput> = ids
        .iter()
        .filter_map(|id| input.inputs.iter().find(|branch_input| branch_input.id == *id))
        .collect();
    for branch_input in &asked {
        if let Some(pattern) = &branch_input.pattern
            && regex::Regex::new(pattern).is_err()
        {
            return Err(refuse(format!(
                "The pattern of the input `{}` in `branches.inputs` is not a valid regular expression.",
                branch_input.id
            )));
        }
    }
    Ok(asked)
}

/// The value given for an input, or the question that asks it.
fn input_value(
    input: &FormatAsking<'_>,
    (place, branch_input): (usize, &BranchInput),
    answers: &Answers,
) -> Result<String, Box<Outcome>> {
    let (Some(&key), Some(&typed_key)) = (INPUTS.get(place), TYPED.get(place)) else {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(
            "The format uses too many inputs.",
        ))));
    };
    let proposed = input.used.get(&branch_input.id).or(branch_input.default.as_ref());
    let ask = |question: Question| Err(Box::new(Outcome::Ask(question)));
    match branch_input.kind {
        InputKind::PromptString => match answers.text(key) {
            Some(value) => Ok(value.to_owned()),
            None => ask(prompt_question(input, branch_input, (key, proposed))),
        },
        InputKind::PickString => match answers.choice(key) {
            Some(TYPE_ANOTHER) => match answers.text(typed_key) {
                Some(value) => Ok(value.to_owned()),
                None => ask(typed_question(input, branch_input, typed_key)),
            },
            Some(value) => Ok(value.to_owned()),
            None => ask(pick_question(input, branch_input, (key, proposed))),
        },
    }
}

/// The prompt of an input: its description, or its id when it has none.
fn prompt(branch_input: &BranchInput) -> Text {
    Text::from(branch_input.description.as_deref().unwrap_or(&branch_input.id))
}

/// The input box of a `promptString`, filled with the value proposed; what is typed must match its pattern.
fn prompt_question(
    input: &FormatAsking<'_>,
    branch_input: &BranchInput,
    (key, proposed): (AnswerKey, Option<&String>),
) -> Question {
    let validation = branch_input.pattern.clone().map_or_else(
        || Validation::Required(Text::from("Enter a value.")),
        Validation::Pattern,
    );
    Question {
        key,
        kind: QuestionKind::Text {
            placeholder: None,
            validation,
        },
        title: Text::from(input.title),
        detail: Some(prompt(branch_input)),
        default: proposed.map(|value| Answer::Text(value.clone())),
    }
}

/// The pick of a `pickString`: the value proposed first, described **Used last** when it is not among the options and
/// the input takes another; then the options; then **Type another value…** when it does.
fn pick_question(
    input: &FormatAsking<'_>,
    branch_input: &BranchInput,
    (key, proposed): (AnswerKey, Option<&String>),
) -> Question {
    let option = |value: &String, description: Option<&str>| Choice {
        identifier: value.clone(),
        label: Text::from(value.as_str()),
        description: description.map(Text::from),
    };
    let mut choices = Vec::new();
    if let Some(value) = proposed {
        if branch_input.options.contains(value) {
            choices.push(option(value, None));
        } else if branch_input.allow_custom {
            choices.push(option(value, Some("Used last")));
        }
    }
    let rest: Vec<Choice> = branch_input
        .options
        .iter()
        .filter(|value| Some(*value) != proposed)
        .map(|value| option(value, None))
        .collect();
    choices.extend(rest);
    if branch_input.allow_custom {
        choices.push(Choice {
            identifier: TYPE_ANOTHER.to_owned(),
            label: Text::from("Type another value\u{2026}"),
            description: None,
        });
    }
    let default = choices.first().map(|choice| Answer::Choice(choice.identifier.clone()));
    Question {
        key,
        kind: QuestionKind::Pick { choices },
        title: Text::from(input.title),
        detail: Some(prompt(branch_input)),
        default,
    }
}

/// The input box of **Type another value…**, which cannot be left empty.
fn typed_question(input: &FormatAsking<'_>, branch_input: &BranchInput, key: AnswerKey) -> Question {
    Question {
        key,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Required(Text::from("Enter a value.")),
        },
        title: Text::from(input.title),
        detail: Some(prompt(branch_input)),
        default: None,
    }
}

/// What `${base}` is: the name known, or the one picked at a commit once the branches that have it are read.
fn base_value(input: &FormatAsking<'_>, answers: &Answers) -> Result<String, Box<Outcome>> {
    let BaseOf::Commit { commit, at } = &input.base else {
        let BaseOf::Known(name) = &input.base else {
            return Ok(String::new());
        };
        return Ok(name.clone());
    };
    if let Some(picked) = answers.choice(BASE) {
        return Ok(picked.to_owned());
    }
    let Some(containing) = answers.names(CONTAINING) else {
        return Err(Box::new(Outcome::Read(
            CONTAINING,
            Query::Containing { commit: commit.clone() },
        )));
    };
    Err(Box::new(Outcome::Ask(base_question(input, (commit, at), containing))))
}

/// The pick of what `${base}` is at a commit: the branches at it, the commit itself, then the other branches that have
/// it; a remote branch without its remote, each name once, no more than 50 branches.
fn base_question(input: &FormatAsking<'_>, (commit, at): (&CommitId, &[String]), containing: &[String]) -> Question {
    let remotes = &input.values.remotes;
    let mut seen: Vec<String> = Vec::new();
    let mut branch = |name: String, description: &str| {
        let is_new = !seen.contains(&name) && seen.len() < MOST_BASES;
        is_new.then(|| {
            seen.push(name.clone());
            Choice {
                identifier: name.clone(),
                label: Text::from(name),
                description: Some(Text::from(description)),
            }
        })
    };
    let mut choices: Vec<Choice> = at
        .iter()
        .filter_map(|name| branch(name.clone(), "Branch at this commit"))
        .collect();
    let others: Vec<Choice> = containing
        .iter()
        .filter_map(|full| {
            full.strip_prefix("refs/heads/").map(str::to_owned).or_else(|| {
                full.strip_prefix("refs/remotes/")
                    .map(|name| local_name(name, remotes).to_owned())
            })
        })
        .filter_map(|name| branch(name, "Branch that has this commit"))
        .collect();
    choices.push(Choice {
        identifier: commit.short().to_owned(),
        label: Text::from(commit.short()),
        description: Some(Text::from("This commit")),
    });
    choices.extend(others);
    let default = choices.first().map(|choice| Answer::Choice(choice.identifier.clone()));
    Question {
        key: BASE,
        kind: QuestionKind::Pick { choices },
        title: Text::from(format!("{}: what ${{base}} is", input.title)),
        detail: None,
        default,
    }
}

#[cfg(test)]
mod tests;
