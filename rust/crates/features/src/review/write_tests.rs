#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::ai::Confidence;
use model::contract::{Answer, Answers, CommandPreview, Danger, Outcome, Step, Target, Undo, UndoRecord};
use model::review::{ChoiceKind, FileLine, FileResolutions, Proposal, ProposalSource, Resolutions, RuleCase};

use super::{FILES, WRITE, write};
use crate::testing::{context, plan, state};

/// A file of two conflicts, `<<<<<<< HEAD / X / ======= / Y / >>>>>>> f`, each with an unchanged line around it.
const TEXT: &str = "a\n<<<<<<< HEAD\nX\n=======\nY\n>>>>>>> f\nb\n<<<<<<< HEAD\nX\n=======\nY\n>>>>>>> f\nc\n";

/// A proposal from somewhere of `R` in place of the conflict at lines `start..start + 5`.
fn proposal(source: ProposalSource, start: usize) -> Proposal {
    Proposal {
        source,
        replaced: start..start + 5,
        lines: vec![FileLine::new("R", "\n")],
        new_lines: 0,
    }
}

const RULES: ProposalSource = ProposalSource::Rules(RuleCase::NextToEachOther);

/// A proposal of the AI provider.
fn ai() -> ProposalSource {
    ProposalSource::Ai {
        model: String::from("claude"),
        reason: String::new(),
        confidence: Confidence::High,
    }
}

/// A file of [`TEXT`] with some proposals accepted.
fn file(path: &str, accepted: Vec<Proposal>) -> FileResolutions {
    FileResolutions {
        path: path.to_owned(),
        text: String::from(TEXT),
        conflicts: 2,
        accepted,
    }
}

/// The resolutions of `a.rs`, both by the rules, and of `b.rs`, its first by some source.
fn two_files(second: ProposalSource) -> Resolutions {
    Resolutions {
        files: vec![
            file("a.rs", vec![proposal(RULES, 1), proposal(RULES, 7)]),
            file("b.rs", vec![proposal(second, 1)]),
        ],
    }
}

/// What `write` does with the files on disk as they were read.
fn written(resolutions: &Resolutions) -> model::contract::Plan {
    let context = context(state(), Target::Resolutions(Box::new(resolutions.clone())));
    let read = write(&context, &Answers::new());
    assert!(matches!(read, Outcome::Read(FILES, _)), "{read:?}");
    let on_disk = resolutions
        .files
        .iter()
        .filter(|file| !file.accepted.is_empty())
        .map(|file| Some(file.text.clone().into_bytes()))
        .collect();
    plan(write(&context, &Answers::new().with(FILES, Answer::Files(on_disk))))
}

/// The lines of the steps that change something: the `write` lines by the file's name, and the git commands.
fn changing(plan: &model::contract::Plan) -> Vec<String> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            Step::Write { path, .. } => Some(format!(
                "write {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            )),
            Step::Warn { .. } => None,
            other => Some(other.log_line()),
        })
        .collect()
}

/// The sentences the plan says of what it does.
fn detail(plan: &model::contract::Plan) -> Vec<String> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            Step::Warn { words } => Some(words.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn review_write__writes_both_files_and_stages_the_one_left_with_no_conflict() {
    let plan = written(&two_files(RULES));
    assert_eq!(changing(&plan), ["write a.rs", "write b.rs", "git add -- a.rs"]);
    let second = plan.steps.iter().find_map(|step| match step {
        Step::Write { path, contents } if path.ends_with("b.rs") => {
            Some(String::from_utf8_lossy(contents).into_owned())
        }
        _ => None,
    });
    let second = second.unwrap_or_default();
    assert!(second.starts_with("a\nR\nb\n<<<<<<< HEAD\n"), "{second}");
    assert_eq!(plan.title.as_str(), "Write the resolutions of 3 conflicts");
    assert_eq!(
        plan.undo,
        Undo::Journal(UndoRecord::ConflictsResolved {
            files: vec![String::from("a.rs"), String::from("b.rs")]
        })
    );
}

#[test]
fn review_write__the_detail_says_by_whom_and_warns_of_the_language_model() {
    let plan = written(&two_files(ai()));
    assert_eq!(
        detail(&plan),
        [
            "a.rs: 2 of 2 conflicts (by the rules)",
            "b.rs: 1 of 2 conflicts (by the AI)",
            "1 conflict will be left to resolve by hand, with the markers as they are.",
            "1 of them were written by a language model: look at them before you commit.",
        ]
    );
}

#[test]
fn review_write__a_file_changed_on_disk_since_it_was_read_writes_nothing() {
    let resolutions = two_files(RULES);
    let context = context(state(), Target::Resolutions(Box::new(resolutions)));
    let on_disk = vec![Some(b"a\nedited\n".to_vec()), Some(TEXT.as_bytes().to_vec())];
    let outcome = write(&context, &Answers::new().with(FILES, Answer::Files(on_disk)));
    let Outcome::Refuse(refusal) = outcome else {
        panic!("refused: {outcome:?}")
    };
    assert_eq!(
        refusal.message.as_str(),
        "a.rs has changed since it was read, so nothing was written. Try again."
    );
}

#[test]
fn review_write__two_proposals_on_the_same_lines_are_refused() {
    let resolutions = Resolutions {
        files: vec![file("a.rs", vec![proposal(RULES, 1), proposal(RULES, 3)])],
    };
    let context = context(state(), Target::Resolutions(Box::new(resolutions)));
    assert!(matches!(write(&context, &Answers::new()), Outcome::Refuse(_)));
}

#[test]
fn review_write__nothing_accepted_is_refused() {
    let resolutions = Resolutions {
        files: vec![file("a.rs", Vec::new())],
    };
    let context = context(state(), Target::Resolutions(Box::new(resolutions)));
    let Outcome::Refuse(refusal) = write(&context, &Answers::new()) else {
        panic!("refused")
    };
    assert_eq!(refusal.message.as_str(), "No resolution is accepted.");
}

#[test]
fn review_choose__a_choice_written_counts_as_chosen_by_you() {
    let chosen = ProposalSource::Choice(ChoiceKind::CurrentOnly);
    let alone = Resolutions {
        files: vec![file("a.rs", vec![proposal(chosen.clone(), 1)])],
    };
    assert_eq!(
        detail(&written(&alone)).first().map(String::as_str),
        Some("a.rs: 1 of 2 conflicts (chosen by you)")
    );
    let among = Resolutions {
        files: vec![file("a.rs", vec![proposal(RULES, 1), proposal(chosen, 7)])],
    };
    assert_eq!(
        detail(&written(&among)).first().map(String::as_str),
        Some("a.rs: 2 of 2 conflicts (1 by the rules, 1 chosen by you)")
    );
}

#[test]
fn review_write__with_every_plan_shown_it_asks_first_then_reads_the_files_again_and_shows_nothing_more() {
    let resolutions = two_files(RULES);
    let mut context = context(state(), Target::Resolutions(Box::new(resolutions.clone())));
    context.settings.command_preview = CommandPreview::All;
    let Outcome::Ask(question) = write(&context, &Answers::new()) else {
        panic!("asked first")
    };
    assert_eq!(question.key, WRITE);
    assert_eq!(question.title.as_str(), "Write the resolutions of 3 conflicts?");
    let detail = question
        .detail
        .as_ref()
        .map(model::contract::Text::as_str)
        .unwrap_or_default();
    assert!(detail.starts_with("a.rs: 2 of 2 conflicts (by the rules)"), "{detail}");
    assert!(detail.ends_with("git add -- a.rs"), "{detail}");
    let confirmed = Answers::new().with(WRITE, Answer::Confirmed);
    assert!(matches!(write(&context, &confirmed), Outcome::Read(FILES, _)));
    let on_disk = vec![Some(TEXT.as_bytes().to_vec()), Some(TEXT.as_bytes().to_vec())];
    let ran = plan(write(&context, &confirmed.clone().with(FILES, Answer::Files(on_disk))));
    assert_eq!(ran.danger, Danger::Agreed);
}

#[test]
fn review_write__a_file_saved_while_its_question_was_open_is_not_written_over() {
    let resolutions = two_files(RULES);
    let mut context = context(state(), Target::Resolutions(Box::new(resolutions.clone())));
    context.settings.command_preview = CommandPreview::All;
    let saved = vec![Some(TEXT.as_bytes().to_vec()), Some(b"saved meanwhile\n".to_vec())];
    let answers = Answers::new()
        .with(WRITE, Answer::Confirmed)
        .with(FILES, Answer::Files(saved));
    let Outcome::Refuse(refusal) = write(&context, &answers) else {
        panic!("refused")
    };
    assert_eq!(
        refusal.message.as_str(),
        "b.rs has changed since it was read, so nothing was written. Try again."
    );
}
