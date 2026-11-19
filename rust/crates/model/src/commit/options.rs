//! The options of the commit box (`commit.options`): how git's configuration signs commits (`commit.read-signing`),
//! what the user ticked for the next commit, and the arguments they add to `git commit` (`commit.option-arguments`).

use std::ffi::OsString;

/// The kind of signature git makes, from `gpg.format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SigningKind {
    /// `openpgp`, the kind when `gpg.format` is not set.
    #[default]
    OpenPgp,
    /// `x509`.
    X509,
    /// `ssh`.
    Ssh,
}

impl SigningKind {
    /// The kind a value of `gpg.format` names; `openpgp` for a value git does not know, as git refuses to sign with it.
    #[must_use]
    pub fn of_format(format: &str) -> SigningKind {
        match format.trim().to_ascii_lowercase().as_str() {
            "x509" => SigningKind::X509,
            "ssh" => SigningKind::Ssh,
            _ => SigningKind::OpenPgp,
        }
    }
}

/// How git's configuration signs commits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SigningConfig {
    /// Whether every commit is signed: `commit.gpgSign`.
    pub signs_every_commit: bool,
    /// The kind of signature: `gpg.format`.
    pub kind: SigningKind,
    /// The key it signs with: `user.signingKey`, a key's identifier, a path, or a public key itself.
    pub key: Option<String>,
}

impl SigningConfig {
    /// Whether git has a key to sign with: `openpgp` finds one from the email of the identity, SSH and X.509 need
    /// `user.signingKey` (D79).
    #[must_use]
    pub fn has_key(&self) -> bool {
        self.key.as_ref().is_some_and(|key| !key.trim().is_empty()) || self.kind == SigningKind::OpenPgp
    }
}

/// Whether a value of git's configuration is true, as git reads a boolean: `true`, `yes`, `on`, `1` and a key with no
/// value at all are true; anything else is false.
#[must_use]
pub fn is_true(value: Option<&str>) -> bool {
    match value {
        None => true,
        Some(value) => matches!(value.trim().to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"),
    }
}

/// A person named as a co-author of a commit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoAuthor {
    /// The name.
    pub name: String,
    /// The email.
    pub email: String,
}

impl CoAuthor {
    /// The line of the message that names them: `Co-authored-by: Ben <ben@example.com>`, the form GitHub reads (Q1095).
    #[must_use]
    pub fn trailer(&self) -> String {
        format!("Co-authored-by: {} <{}>", self.name, self.email)
    }

    /// A co-author from `Name <email>` as it is typed; none without a name, or without an email in angle brackets.
    #[must_use]
    pub fn parse(typed: &str) -> Option<CoAuthor> {
        let (name, rest) = typed.trim().split_once('<')?;
        let email = rest.strip_suffix('>')?.trim();
        let name = name.trim();
        (!name.is_empty() && email.contains('@') && !email.contains(['<', '>'])).then(|| CoAuthor {
            name: name.to_owned(),
            email: email.to_owned(),
        })
    }
}

/// What the user ticked in the menu of the commit box for the next commit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitOptions {
    /// **Sign this commit**: none while it is as git's configuration says.
    pub sign: Option<bool>,
    /// **Add Signed-off-by**.
    pub sign_off: bool,
    /// **Skip the hooks**.
    pub skip_hooks: bool,
    /// The co-authors chosen, in the order chosen.
    pub co_authors: Vec<CoAuthor>,
}

/// The arguments the options add to `git commit`, after `commit` and `--amend` and before `-m <message>` or
/// `--no-edit`: `--gpg-sign` or `--no-gpg-sign` when the tick differs from git's configuration, `--signoff`,
/// `--no-verify`, and a `--trailer` for each co-author the message does not name already.
///
/// Feature: commit.option-arguments
#[must_use]
pub fn option_arguments(options: &CommitOptions, signing: &SigningConfig, message: &str) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = Vec::new();
    match options.sign {
        Some(true) if !signing.signs_every_commit => arguments.push("--gpg-sign".into()),
        Some(false) if signing.signs_every_commit => arguments.push("--no-gpg-sign".into()),
        Some(_) | None => {}
    }
    if options.sign_off {
        arguments.push("--signoff".into());
    }
    if options.skip_hooks {
        arguments.push("--no-verify".into());
    }
    for co_author in &options.co_authors {
        let trailer = co_author.trailer();
        if !message.lines().any(|line| line.trim() == trailer) {
            arguments.push("--trailer".into());
            arguments.push(trailer.into());
        }
    }
    arguments
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;

    fn ben() -> CoAuthor {
        CoAuthor {
            name: String::from("Ben"),
            email: String::from("ben@example.com"),
        }
    }

    fn signing(signs: bool) -> SigningConfig {
        SigningConfig {
            signs_every_commit: signs,
            ..SigningConfig::default()
        }
    }

    fn words(arguments: &[OsString]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn commit_option_arguments__nothing_ticked_without_signing_gives_no_argument() {
        let arguments = option_arguments(&CommitOptions::default(), &signing(false), "Fix the parser");
        assert_eq!(arguments, Vec::<std::ffi::OsString>::new());
    }

    #[test]
    fn commit_option_arguments__sign_ticked_gives_gpg_sign_and_against_the_configuration_no_gpg_sign() {
        let ticked = CommitOptions {
            sign: Some(true),
            ..CommitOptions::default()
        };
        assert_eq!(words(&option_arguments(&ticked, &signing(false), "")), ["--gpg-sign"]);
        assert_eq!(
            option_arguments(&ticked, &signing(true), ""),
            Vec::<std::ffi::OsString>::new()
        );
        let unticked = CommitOptions {
            sign: Some(false),
            ..CommitOptions::default()
        };
        assert_eq!(
            words(&option_arguments(&unticked, &signing(true), "")),
            ["--no-gpg-sign"]
        );
    }

    #[test]
    fn commit_option_arguments__two_co_authors_give_two_trailers_in_order_and_one_named_already_none() {
        let cy = CoAuthor {
            name: String::from("Cy"),
            email: String::from("cy@example.com"),
        };
        let options = CommitOptions {
            co_authors: vec![ben(), cy],
            ..CommitOptions::default()
        };
        assert_eq!(
            words(&option_arguments(&options, &signing(false), "Fix the parser")),
            [
                "--trailer",
                "Co-authored-by: Ben <ben@example.com>",
                "--trailer",
                "Co-authored-by: Cy <cy@example.com>"
            ]
        );
        let named = "Fix the parser\n\nCo-authored-by: Ben <ben@example.com>";
        assert_eq!(
            words(&option_arguments(&options, &signing(false), named)),
            ["--trailer", "Co-authored-by: Cy <cy@example.com>"]
        );
    }

    #[test]
    fn commit_option_arguments__sign_off_and_skip_hooks_give_their_flags() {
        let options = CommitOptions {
            sign_off: true,
            skip_hooks: true,
            co_authors: vec![ben()],
            ..CommitOptions::default()
        };
        assert_eq!(
            words(&option_arguments(&options, &signing(false), "")),
            [
                "--signoff",
                "--no-verify",
                "--trailer",
                "Co-authored-by: Ben <ben@example.com>"
            ]
        );
    }

    #[test]
    fn commit_option_arguments__a_name_with_a_space_or_a_quote_is_one_argument() {
        let options = CommitOptions {
            co_authors: vec![CoAuthor {
                name: String::from("Dee \"D\" O'Neil"),
                email: String::from("dee@example.com"),
            }],
            ..CommitOptions::default()
        };
        let arguments = words(&option_arguments(&options, &signing(false), ""));
        assert_eq!(
            arguments,
            ["--trailer", "Co-authored-by: Dee \"D\" O'Neil <dee@example.com>"]
        );
    }

    #[test]
    fn a_co_author_is_parsed_from_a_name_and_an_email_only() {
        assert_eq!(CoAuthor::parse("Ben <ben@example.com>"), Some(ben()));
        assert_eq!(CoAuthor::parse("Ben"), None);
        assert_eq!(CoAuthor::parse("<ben@example.com>"), None);
    }

    #[test]
    fn a_boolean_of_git_is_read_as_git_reads_it() {
        for value in [None, Some("true"), Some("Yes"), Some("on"), Some("1")] {
            assert!(is_true(value), "{value:?}");
        }
        for value in [Some("false"), Some("no"), Some("0"), Some("")] {
            assert!(!is_true(value), "{value:?}");
        }
    }
}
