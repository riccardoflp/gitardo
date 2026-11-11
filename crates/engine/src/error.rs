//! What can keep the engine from carrying a feature to its end.

use std::fmt;

use git::GitError;
use model::contract::{FeatureId, Text};

use crate::steps::StepFailure;

/// Why a feature did not run to its end.
#[derive(Debug)]
pub enum EngineError {
    /// A query of the feature could not be read.
    Query(GitError),
    /// A step of the plan failed: the plan stopped there.
    Step(StepFailure),
    /// An answer was given that its question does not accept.
    AnswerRefused {
        /// The title of the question.
        question: Text,
        /// What is wrong with the answer.
        problem: Text,
    },
    /// The feature kept asking without deciding: a mistake in the feature, never in what the user did.
    EndlessFeature(FeatureId),
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Query(error) => write!(formatter, "{error}"),
            EngineError::Step(failure) => write!(formatter, "{failure}"),
            EngineError::AnswerRefused { question, problem } => write!(formatter, "{question}: {problem}"),
            EngineError::EndlessFeature(feature) => {
                write!(
                    formatter,
                    "{} asked again and again without deciding what to do.",
                    feature.0
                )
            }
        }
    }
}

impl std::error::Error for EngineError {}
