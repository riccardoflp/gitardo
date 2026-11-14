//! The GitHub token: the one Gitardo keeps, or the one git's credential helper knows for `github.com`
//! (`github.token`, decision D11).

use std::ffi::OsString;

pub use model::github::HelperAsked;
use model::github::Token;

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// What `git credential fill` reads on its standard input: the host whose credentials are asked for, then an empty
/// line.
pub const TOKEN_QUESTION: &[u8] = b"protocol=https\nhost=github.com\n\n";

/// Where the token is looked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRequest {
    /// The token Gitardo keeps in the credential store of the system for `github.com`, as read from the store; none
    /// when none is kept.
    pub kept: Option<Token>,
    /// Whether `git credential fill` may run when none is kept.
    pub helper: HelperAsked,
}

/// The token `api.github.com` is asked with, without asking the user anything: the one kept, or, when the request
/// allows it, the password git's credential helper gives for `github.com`. Git runs with `GIT_TERMINAL_PROMPT=0`, no
/// program to ask a password with (`GIT_ASKPASS` empty) and the Git Credential Manager told not to open a window, so
/// that nothing is ever asked; nothing printed, or an exit code other than 0, is no token. What git printed is never
/// kept or said.
///
/// Feature: github.token
///
/// # Errors
///
/// When git cannot be started, or is cancelled.
pub fn read_token(git: &Git, request: &TokenRequest) -> Result<Option<Token>, GitError> {
    if let Some(kept) = &request.kept {
        return Ok(Some(kept.clone()));
    }
    if request.helper == HelperAsked::NotAllowed {
        return Ok(None);
    }
    let quiet = git
        .clone()
        .with_variable("GIT_TERMINAL_PROMPT", "0")
        .with_variable("GIT_ASKPASS", "")
        .with_variable("SSH_ASKPASS", "")
        .with_variable("GCM_INTERACTIVE", "never");
    let output = quiet.read_output_with_input(&token_arguments(), TOKEN_QUESTION)?;
    if !output.is_success() {
        return Ok(None);
    }
    Ok(parse_token(&output.stdout))
}

/// `git config --get http.proxy`: the proxy git goes through, which requests to GitHub go through too when
/// `HTTPS_PROXY` is not set (`docs/05-platform.md`, Proxies).
#[must_use]
pub fn proxy_arguments() -> Vec<OsString> {
    arguments(&["config", "--get", "http.proxy"])
}

/// The proxy `git config --get http.proxy` printed; none when it printed nothing.
#[must_use]
pub fn parse_proxy(output: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(output);
    Some(text.trim().to_owned()).filter(|proxy| !proxy.is_empty())
}

/// The proxy of git's `http.proxy`; none when it is not set, which git says with the exit code 1.
///
/// # Errors
///
/// When git cannot be started, or is cancelled.
pub fn read_http_proxy(git: &Git) -> Result<Option<String>, GitError> {
    let output = git.read_output(&proxy_arguments())?;
    if !output.is_success() {
        return Ok(None);
    }
    Ok(parse_proxy(&output.stdout))
}

/// `git credential fill`.
#[must_use]
pub fn token_arguments() -> Vec<OsString> {
    arguments(&["credential", "fill"])
}

/// The value of `password=` in what `git credential fill` printed; none when it printed none, or an empty one.
#[must_use]
pub fn parse_token(output: &[u8]) -> Option<Token> {
    let text = String::from_utf8_lossy(output);
    text.lines()
        .find_map(|line| line.strip_prefix("password="))
        .map(|password| password.trim_end_matches('\r'))
        .filter(|password| !password.is_empty())
        .map(|password| Token(password.to_owned()))
}
