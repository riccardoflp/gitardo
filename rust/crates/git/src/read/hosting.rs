//! What a hosting service other than GitHub needs of git: the default branch of the main remote, the target of a new
//! pull request on Gitea and Forgejo (`hosting.web-links`), and the token of a GitLab, the one Gitardo keeps or the one
//! git's credential helper knows for its host name (`hosting.token`).

use std::ffi::OsString;

use model::github::{HelperAsked, Token};

use crate::read::fields::{arguments, read_or_nothing};
use crate::read::github::{parse_token, token_arguments};
use crate::{Git, GitError};

/// `git symbolic-ref --quiet --short refs/remotes/<remote>/HEAD`: the branch the remote's `HEAD` names, as the clone
/// left it or `git remote set-head` set it.
#[must_use]
pub fn default_branch_arguments(remote: &str) -> Vec<OsString> {
    let mut arguments = arguments(&["symbolic-ref", "--quiet", "--short"]);
    arguments.push(OsString::from(format!("refs/remotes/{remote}/HEAD")));
    arguments
}

/// The branch of the remote that `git symbolic-ref --short` printed, without the remote: `main` for `origin/main`;
/// none when it printed nothing.
#[must_use]
pub fn parse_default_branch(output: &[u8], remote: &str) -> Option<String> {
    let text = String::from_utf8_lossy(output);
    let name = text.trim();
    let name = name
        .strip_prefix(remote)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(name);
    Some(name.to_owned()).filter(|name| !name.is_empty())
}

/// The default branch of a remote; none when the repository does not have `refs/remotes/<remote>/HEAD`.
///
/// # Errors
///
/// When git cannot be started, or fails otherwise.
pub fn read_default_branch(git: &Git, remote: &str) -> Result<Option<String>, GitError> {
    let output = read_or_nothing(git, &default_branch_arguments(remote))?;
    Ok(output.and_then(|output| parse_default_branch(&output, remote)))
}

/// Where the token of a GitLab is looked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostingTokenRequest {
    /// The host name it is for: `gitlab.com`.
    pub host: String,
    /// The token Gitardo keeps in the credential store of the system for that host name, as read from the store; none
    /// when none is kept.
    pub kept: Option<Token>,
    /// Whether `git credential fill` may run when none is kept.
    pub helper: HelperAsked,
}

/// What `git credential fill` reads on its standard input for a host name: `protocol=https`, `host=<host>`, then an
/// empty line.
#[must_use]
pub fn token_question(host: &str) -> Vec<u8> {
    format!("protocol=https\nhost={host}\n\n").into_bytes()
}

/// The token the API of a GitLab is asked with, without asking the user anything, as `github.token` finds GitHub's:
/// the one kept for its host name, or, when the request allows it, the password git's credential helper gives for that
/// host name. Git runs so that nothing is ever asked; nothing printed, or an exit code other than 0, is no token. What
/// git printed is never kept or said.
///
/// Feature: hosting.token
///
/// # Errors
///
/// When git cannot be started, or is cancelled.
pub fn read_token(git: &Git, request: &HostingTokenRequest) -> Result<Option<Token>, GitError> {
    if let Some(kept) = &request.kept {
        return Ok(Some(kept.clone()));
    }
    if request.helper == HelperAsked::NotAllowed || request.host.is_empty() {
        return Ok(None);
    }
    let quiet = git
        .clone()
        .with_variable("GIT_TERMINAL_PROMPT", "0")
        .with_variable("GIT_ASKPASS", "")
        .with_variable("SSH_ASKPASS", "")
        .with_variable("GCM_INTERACTIVE", "never");
    let output = quiet.read_output_with_input(&token_arguments(), &token_question(&request.host))?;
    if !output.is_success() {
        return Ok(None);
    }
    Ok(parse_token(&output.stdout))
}
