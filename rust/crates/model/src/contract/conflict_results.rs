//! The results of the queries of the files in conflict, as a feature reads them from its answers.

use super::{Answer, AnswerKey, Answers};
use crate::conflicts::{ConflictTexts, FileStages};

impl Answers {
    /// The versions of a file in conflict read under a key: `None` when it was not read yet, `Some(None)` when the
    /// file is not in conflict.
    #[must_use]
    pub fn stages(&self, key: AnswerKey) -> Option<Option<&FileStages>> {
        match self.get(key)? {
            Answer::Stages(stages) => Some(stages.as_ref()),
            _ => None, // Any other kind of answer is no stages.
        }
    }

    /// The texts of the files in conflict read under a key.
    #[must_use]
    pub fn conflict_texts(&self, key: AnswerKey) -> Option<&[ConflictTexts]> {
        match self.get(key)? {
            Answer::ConflictTexts(texts) => Some(texts),
            _ => None, // Any other kind of answer is no texts.
        }
    }
}
