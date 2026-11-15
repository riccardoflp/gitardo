//! What the engine asks of the program it runs under when that program is not Gitardo Desktop: the extension, which
//! has VS Code's language models and its sign-in to GitHub (`docs/transition/02-protocol.md`, Host questions). Gitardo
//! Desktop gives none, and the engine then reads the token from the credential store and git's credential helper, and
//! lists the models of the AI provider over HTTP, as before.

use model::ai::AiModel;
use model::github::Token;

use crate::ai::AiError;

/// The host of the engine, asked while a feature runs: in the server, one request to the extension for each question.
pub trait Host: Send {
    /// The token of GitHub the host keeps; `ask` lets it ask the user to sign in first. None when it has none, or the
    /// user backed out of signing in.
    fn github_token(&mut self, ask: bool) -> Option<Token>;

    /// The language models of the host, in its order.
    ///
    /// # Errors
    ///
    /// When the host could not list them, or the question was cancelled.
    fn language_models(&mut self) -> Result<Vec<AiModel>, AiError>;
}
