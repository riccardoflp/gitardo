//! The GitHub token: read from the credential store, or from git's credential helper when the feature allows it
//! (`github.token`), and kept once a feature found or was given one (`github.load-pull-requests`); and the key of the
//! AI provider, with which its models are listed (`ai.choose-model`).

use std::time::{Duration, Instant};

use git::GitError;
use git::read::github::{TokenRequest, read_token, token_arguments};
use git::read::hosting::HostingTokenRequest;
use http::{Cancel, Http, Proxies, ProxySettings, Trust};
use model::ai::{AiModels, AiSettings, Key};
use model::contract::FollowUp;
use model::github::{HelperAsked, Token};
use model::hosting::{HostingRepo, HostingService, PullList};

use super::Engine;
use crate::secrets::{GITHUB_TOKEN, SecretStore};

/// What looking for the GitHub token came to.
pub(super) enum TokenFound {
    /// The token, or none.
    Found(Option<Token>),
    /// The host asked the user to sign in, and they did not.
    BackedOut,
}

impl Engine {
    /// Keeps the secrets of the features (the GitHub token) in another store than the system's: the memory of a test.
    pub fn keep_secrets_in(&mut self, secrets: Box<dyn SecretStore>) {
        self.secrets = secrets;
    }

    /// Asks the host it runs under for the GitHub token and the language models, in place of the credential store,
    /// git's credential helper and the AI provider: the extension's sign-in to GitHub and VS Code's models. A token or
    /// a key a plan says to keep is then the host's to keep, and the engine keeps none.
    pub fn ask_host_with(&mut self, host: Box<dyn crate::Host>) {
        self.host = Some(host);
    }

    /// The GitHub token: the one kept in the credential store, or the one git's credential helper gives when `helper`
    /// allows it. Running `git credential fill` is logged at the debug level, never what it printed. Under a host, the
    /// host's, which may ask the user to sign in when `helper` allows it: then none is the user backing out.
    pub(super) fn github_token(&mut self, helper: HelperAsked) -> Result<TokenFound, GitError> {
        if let Some(host) = &mut self.host {
            let ask = helper == HelperAsked::Allowed;
            return Ok(match host.github_token(ask) {
                None if ask => TokenFound::BackedOut,
                token => TokenFound::Found(token),
            });
        }
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
        Ok(TokenFound::Found(token))
    }

    /// The token of a GitLab for a host name: the one kept in the credential store for it, or the one git's credential
    /// helper gives when `helper` allows it (`hosting.token`). Under a host, the host's, which may ask the user for one
    /// when `helper` allows it: then none is the user backing out.
    pub(super) fn hosting_token(&mut self, host: &str, helper: HelperAsked) -> Result<TokenFound, GitError> {
        if let Some(host_of_engine) = &mut self.host {
            let ask = helper == HelperAsked::Allowed;
            return Ok(match host_of_engine.hosting_token(host, ask) {
                None if ask => TokenFound::BackedOut,
                token => TokenFound::Found(token),
            });
        }
        let kept = match self.secrets.read(host) {
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
        let request = HostingTokenRequest {
            host: host.to_owned(),
            kept,
            helper,
        };
        let token = git::read::hosting::read_token(&self.git, &request)?;
        if asks_git {
            let line = git::command_line(&token_arguments());
            self.log.record_read(self.clock.now(), line, started.elapsed());
        }
        Ok(TokenFound::Found(token))
    }

    /// Keeps the tokens a plan that ran says to keep, in the credential store. One that cannot be kept is said in the
    /// log: the pull requests are still read with it this time, and it is asked for again the next.
    pub(super) fn keep_tokens(&mut self, after: &[FollowUp]) {
        if self.host.is_some() {
            return; // The host keeps the token it gave.
        }
        for follow_up in after {
            let (name, token) = match follow_up {
                FollowUp::KeepGitHubToken(token) => (GITHUB_TOKEN, token),
                FollowUp::KeepHostingToken(host, token) => (host.as_str(), token),
                _ => continue,
            };
            if let Err(error) = self.secrets.write(name, &token.0) {
                let line = format!("The token of {name} could not be kept in the credential store: {error}");
                self.log.record_warning(self.clock.now(), line);
            }
        }
    }
}

impl Engine {
    /// The open pull requests of the main remote, read with the token kept for its service, asking nothing and running
    /// no credential helper (`hosting.read-pull-request-list`); why they were not, when they were not.
    pub(super) fn pull_request_list(&mut self, repository: HostingRepo) -> PullList {
        let found = match repository.service {
            HostingService::GitHub => self.github_token(HelperAsked::NotAllowed),
            HostingService::GitLab => self.hosting_token(&repository.host, HelperAsked::NotAllowed),
            _ => return PullList::Listed(Vec::new()),
        };
        let Ok(TokenFound::Found(Some(token))) = found else {
            let service = repository.service.name();
            return PullList::Unread(format!("Sign in to {service} to list the open pull requests."));
        };
        let http = match self.client() {
            Ok(http) => http,
            Err(error) => return PullList::Unread(error.to_string()),
        };
        let cancelled = self.cancel.clone();
        let cancel = Cancel::following(move || cancelled.is_cancelled());
        let service = repository.service.name();
        let request = crate::hosting::PullListRequest {
            repository,
            token,
            cancel,
        };
        let started = Instant::now();
        let listed = crate::hosting::read_pull_request_list(&http, &request);
        let line = format!("list the open pull requests of {service}");
        self.log.record_read(self.clock.now(), line, started.elapsed());
        match listed {
            Ok(entries) => PullList::Listed(entries),
            Err(error) => PullList::Unread(error.to_string()),
        }
    }

    /// The models the AI provider lists, asked with the key typed, else with the one kept in the credential store;
    /// under a host, the host's models, which need no key.
    pub(super) fn ai_models(&mut self, settings: AiSettings, typed: Option<Key>) -> AiModels {
        if let Some(host) = &mut self.host {
            let started = Instant::now();
            let listed = match host.language_models() {
                Ok(models) => AiModels::Listed(models),
                Err(error) => AiModels::Failed(error.to_string()),
            };
            let line = String::from("list the language models of the host");
            self.log.record_read(self.clock.now(), line, started.elapsed());
            return listed;
        }
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
        if self.host.is_some() {
            return; // The host's models need no key.
        }
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
