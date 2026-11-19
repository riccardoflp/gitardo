//! Ask Gitardo (`help.ask`): the two requests of a question sent to a language model, the first read whole, the
//! second streamed, and its answer read. Nothing of the repository is read or sent.

use model::help::{App, HelpAnswer, HelpInput, NO_PROVIDER, choose_cards, guide, question, read_answer, read_chosen};

use crate::ai::{AiError, LanguageModel, Purpose};

/// A question to Ask Gitardo: the app it is asked in, whose guide answers it, and the question with the conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelpRequest {
    /// The app the question is asked in.
    pub app: App,
    /// The question and the conversation before it.
    pub input: HelpInput,
}

/// Asks a question: the first request chooses the cards of the guide that answer it, and its answer is read whole; the
/// second asks the question with those cards, each piece of its answer given to `pieces` as it comes; the answer is
/// then read. A cancelled first request sends no second one.
///
/// # Errors
///
/// When either request fails, is refused or is cancelled, as [`LanguageModel::ask`].
///
/// Feature: help.ask
pub fn ask(
    model: &mut dyn LanguageModel,
    request: &HelpRequest,
    pieces: &mut dyn FnMut(&str),
) -> Result<HelpAnswer, AiError> {
    let guide = guide(request.app);
    let first = model.ask(Purpose::Help, &choose_cards(&request.input, &guide), &mut |_| {})?;
    let chosen = read_chosen(&first.text, &guide);
    let second = model.ask(Purpose::Help, &question(&request.input, &guide, &chosen), pieces)?;
    let mut answer = read_answer(&second.text, &guide);
    answer.model = second.model;
    Ok(answer)
}

/// What Ask Gitardo says when a question failed: that no provider is set up, or what the provider said.
#[must_use]
pub fn said(error: &AiError) -> String {
    match error {
        AiError::NotSetUp => NO_PROVIDER.to_owned(),
        error => error.to_string(),
    }
}

#[cfg(test)]
mod tests;
