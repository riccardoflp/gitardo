//! The GitHub token: read from the credential store, or from git's credential helper when the feature allows it
//! (`github.token`), and kept once a feature found or was given one (`github.load-pull-requests`); and the key of the
//! AI provider, with which its models are listed (`ai.choose-model`).

use std::time::{Duration, Instant};

use git::GitError;
use git::read::github::{TokenRequest, read_token, token_arguments};
use http::{Cancel, Http, Proxies, ProxySettings, Trust};
use model::ai::{AiModels, AiSettings, Key};
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

impl Engine {
    /// The models the AI provider lists, asked with the key typed, else with the one kept in the credential store.
    pub(super) fn ai_models(&mut self, settings: AiSettings, typed: Option<Key>) -> AiModels {
        let Some(provider) = settings.provider else {
            return AiModels::Failed(String::from("No AI provider is set up."));
        };
        let key = typed.or_else(|| match self.secrets.read(&provider.key_name()) {
            Ok(kept) => kept.map(Key),
            Err(error) => {
                // A store that cannot be read keeps no key Gitardo can use: the provider may need none, or one is asked.
                let line = format!("the credential store of the system could not be read: {error}");
                self.log.record_read(self.clock.now(), line, Duration::ZERO);
                None
            }
        });
        let http = match self.client() {
            Ok(http) => http,
            Err(error) => return AiModels::Failed(error.to_string()),
        };
        let token = self.cancel.clone();
        let cancel = Cancel::following(move || token.is_cancelled());
        let started = Instant::now();
        let listed = crate::ai::models_answer(
            crate::ai::Sending {
                http: &http,
                cancel: &cancel,
            },
            &crate::ai::Provider { settings, key },
        );
        let line = String::from("list the models of the AI provider");
        self.log.record_read(self.clock.now(), line, started.elapsed());
        listed
    }

    /// The client that sends requests: the one given, or one that trusts the certificates of the system and goes
    /// through the proxy of the environment.
    fn client(&mut self) -> Result<Http, http::Error> {
        if let Some(http) = &self.http {
            return Ok(http.clone());
        }
        let proxies = Proxies::from_settings(&ProxySettings::from_environment(None))?;
        let http = Http::new(Trust::system().map_err(http::Error::Tls)?, proxies);
        self.http = Some(http.clone());
        Ok(http)
    }

    /// Keeps the keys of the AI provider a plan that ran says to keep. One that cannot be kept is said in the log: it
    /// is asked for again the next time.
    pub(super) fn keep_ai_keys(&mut self, after: &[FollowUp]) {
        for follow_up in after {
            if let FollowUp::KeepAiKey(provider, key) = follow_up
                && let Err(error) = self.secrets.write(&provider.key_name(), &key.0)
            {
                let line = format!("The key of the AI provider could not be kept in the credential store: {error}");
                self.log.record_warning(self.clock.now(), line);
            }
        }
    }
}
