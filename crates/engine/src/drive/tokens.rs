//! The GitHub token: read from the credential store, or from git's credential helper when the feature allows it
//! (`github.token`), and kept once a feature found or was given one (`github.load-pull-requests`).

use std::time::{Duration, Instant};

use git::GitError;
use git::read::github::{TokenRequest, read_token, token_arguments};
use model::contract::FollowUp;
use model::github::{HelperAsked, Token};

use super::Engine;
use crate::secrets::GITHUB_TOKEN;

impl Engine {
    /// The GitHub token: the one kept in the credential store, or the one git's credential helper gives when `helper`
    /// allows it. Running `git credential fill` is logged at the debug level, never what it printed.
    pub(super) fn github_token(&mut self, helper: HelperAsked) -> Result<Option<Token>, GitError> {
        let kept = match self.secrets.read(GITHUB_TOKEN) {
            Ok(kept) => kept.map(Token),
            Err(error) => {
                // A store that cannot be read keeps no token Gitardo can use; git's helper may still know one.
                let line = format!("the credential store of the system could not be read: {error}");
                self.log.record_read(self.clock.now(), line, Duration::ZERO);
                None
            }
        };
        let asks_git = kept.is_none() && helper == HelperAsked::Allowed;
        let started = Instant::now();
        let token = read_token(&self.git, &TokenRequest { kept, helper })?;
        if asks_git {
            let line = git::command_line(&token_arguments());
            self.log.record_read(self.clock.now(), line, started.elapsed());
        }
        Ok(token)
    }

    /// Keeps the tokens a plan that ran says to keep, in the credential store. One that cannot be kept is said in the
    /// log: the pull requests are still read with it this time, and it is asked for again the next.
    pub(super) fn keep_tokens(&mut self, after: &[FollowUp]) {
        for follow_up in after {
            if let FollowUp::KeepGitHubToken(token) = follow_up
                && let Err(error) = self.secrets.write(GITHUB_TOKEN, &token.0)
            {
                let line = format!("The GitHub token could not be kept in the credential store: {error}");
                self.log.record_warning(self.clock.now(), line);
            }
        }
    }
}
