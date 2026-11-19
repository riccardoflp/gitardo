//! The signature of a commit (`details.read-signature`): whether its object carries one, and what git says of it.

use std::ffi::OsString;

use model::CommitId;
use model::details::{CommitSignature, SignatureKind, SignatureVerdict};

use crate::read::fields::{arguments, unexpected};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the signature of a commit";

/// The headers of a commit object that carry its signature: with SHA-1, and with SHA-256 in a repository of either.
const SIGNATURE_HEADERS: [&str; 2] = ["gpgsig ", "gpgsig-sha256 "];

/// Which commit's signature to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureRequest {
    /// The commit.
    pub commit: CommitId,
}

/// `git cat-file commit <hash>`: the object of the commit, whose headers say whether it carries a signature.
#[must_use]
pub fn object_arguments(request: &SignatureRequest) -> Vec<OsString> {
    arguments(&["cat-file", "commit", request.commit.as_str()])
}

/// `git show --no-patch --no-show-signature --format=%G?%x1f%GS%x1f%GK%x1f%GF%x1f%GT <hash>`: what git says of the
/// signature, checked with the program its configuration names, with the signer, the key, its fingerprint and its
/// trust. `--no-show-signature` keeps `log.showSignature` from printing what the program said before the format.
#[must_use]
pub fn check_arguments(request: &SignatureRequest) -> Vec<OsString> {
    let object = format!("{}^{{commit}}", request.commit);
    arguments(&[
        "show",
        "--no-patch",
        "--no-show-signature",
        "--format=%G?%x1f%GS%x1f%GK%x1f%GF%x1f%GT",
        &object,
        "--",
    ])
}

/// The kind of the signature a commit object carries, from the first line of its header; none when it carries none.
/// A header git does not know the first line of is of the kind git signs with when nothing else is set, `openpgp`.
#[must_use]
pub fn parse_object(output: &[u8]) -> Option<SignatureKind> {
    let text = String::from_utf8_lossy(output);
    let headers = text.split("\n\n").next().unwrap_or_default();
    headers.lines().find_map(|line| {
        SIGNATURE_HEADERS
            .iter()
            .find_map(|header| line.strip_prefix(header))
            .map(|first| SignatureKind::of_first_line(first).unwrap_or(SignatureKind::OpenPgp))
    })
}

/// What git said of a signature with the format of [`check_arguments`], for a commit that carries one of a kind.
///
/// # Errors
///
/// When git printed a letter it does not print for a signature, or fewer fields than asked.
pub fn parse_check(output: &[u8], kind: SignatureKind) -> Result<CommitSignature, GitError> {
    let text = String::from_utf8_lossy(output);
    let line = text.trim_end_matches(['\n', '\r']);
    let fields: Vec<&str> = line.split('\x1f').collect();
    let [letter, signer, key, fingerprint, trust] = fields.as_slice() else {
        return Err(unexpected(WHAT, line));
    };
    let verdict = match SignatureVerdict::of_letter(letter) {
        Some(SignatureVerdict::NotSigned) => SignatureVerdict::SignedNotChecked,
        Some(verdict) => verdict,
        None => return Err(unexpected(WHAT, line)),
    };
    Ok(CommitSignature {
        verdict,
        kind: Some(kind),
        signer: (*signer).to_owned(),
        key: (*key).to_owned(),
        fingerprint: (*fingerprint).to_owned(),
        trust: (*trust).to_owned(),
    })
}

/// Whether a commit is signed, and what git says of its signature. The object is read first: a commit that carries no
/// signature is **Not signed** without asking the program that checks signatures. One that carries a signature git
/// says nothing of, as a commit signed with SSH while `gpg.ssh.allowedSignersFile` is not set, is **Signed, not
/// checked**.
///
/// Feature: details.read-signature
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_signature(git: &Git, request: &SignatureRequest) -> Result<CommitSignature, GitError> {
    let Some(kind) = parse_object(&git.run_read(&object_arguments(request))?) else {
        return Ok(CommitSignature::not_signed());
    };
    parse_check(&git.run_read(&check_arguments(request))?, kind)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;

    #[test]
    fn details_read_signature__a_commit_signed_with_openpgp_by_a_key_not_in_the_keyring_cannot_be_checked() {
        let printed = b"E\x1f\x1f3AA5C34371567BD2\x1f\x1fundefined\n";
        let signature = parse_check(printed, SignatureKind::OpenPgp).unwrap();
        assert_eq!(signature.verdict, SignatureVerdict::CannotCheck);
        assert_eq!(signature.key, "3AA5C34371567BD2");
        assert_eq!(signature.kind, Some(SignatureKind::OpenPgp));
    }

    #[test]
    fn details_read_signature__the_kind_is_the_first_line_of_the_header() {
        let object = b"tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\nauthor A <a@example.com> 1 +0000\n\
            committer A <a@example.com> 1 +0000\ngpgsig -----BEGIN SSH SIGNATURE-----\n U1NIU0lH\n \
            -----END SSH SIGNATURE-----\n\nA message\n\ngpgsig in the message\n";
        assert_eq!(parse_object(object), Some(SignatureKind::Ssh));
        let unsigned = b"tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\n\ngpgsig -----BEGIN SSH SIGNATURE-----\n";
        assert_eq!(parse_object(unsigned), None);
    }
}
