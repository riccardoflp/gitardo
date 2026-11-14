//! The answers given to a feature checked against what its question accepts.

use model::contract::{Answer, Question, QuestionKind, Text};

use crate::error::EngineError;

/// Refuses an answer its question would not accept, so that a caller with no dialog, such as the command line, cannot
/// give one.
pub(super) fn check_answer(question: &Question, answer: &Answer) -> Result<(), EngineError> {
    let problem = match (&question.kind, answer) {
        (QuestionKind::Text { validation, .. } | QuestionKind::Secret { validation }, Answer::Text(text)) => {
            validation.problem(text)
        }
        (QuestionKind::Pick { choices } | QuestionKind::Search { choices }, Answer::Choice(picked)) => {
            (!choices.iter().any(|choice| choice.identifier == *picked))
                .then(|| Text::from(format!("{picked} is not one of the choices")))
        }
        (QuestionKind::Confirm { .. }, Answer::Confirmed)
        | (QuestionKind::PickMany { .. }, Answer::Choices(_))
        | (QuestionKind::OpenFile(_) | QuestionKind::SaveFile(_), Answer::Path(_)) => None,
        _ => Some(Text::from("This is not the kind of answer the question asks for")),
    };
    match problem {
        Some(problem) => Err(EngineError::AnswerRefused {
            question: question.title.clone(),
            problem,
        }),
        None => Ok(()),
    }
}
