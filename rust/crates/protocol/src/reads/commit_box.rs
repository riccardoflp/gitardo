//! The reads of the signature of a commit and of the authors of the history, which version 1.11 adds for milestone
//! M34: `read/details.read-signature`, for the line **Signature** of the details beside the graph and of the tab of a
//! commit, and `read/commit.read-authors`, for the co-authors of the commit box of **Gitardo Changes**.

use model::commit::Author;
use model::details::{CommitSignature, SignatureKind, SignatureVerdict};
use serde::{Deserialize, Serialize};

use crate::state::SigningKindJson;

/// The parameters of `read/details.read-signature`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureParams {
    /// The commit, by its full id.
    pub commit: String,
}

/// What git says of the signature of a commit, from the letter of `%G?`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerdictJson {
    /// `G`.
    Good,
    /// `U`.
    GoodUnknownValidity,
    /// `X`.
    GoodExpired,
    /// `Y`.
    GoodExpiredKey,
    /// `R`.
    GoodRevokedKey,
    /// `B`.
    Bad,
    /// `E`.
    CannotCheck,
    /// `N` of a commit whose object carries a signature.
    SignedNotChecked,
    /// No signature.
    NotSigned,
}

/// The signature of a commit, as `details.read-signature` gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSignatureJson {
    /// Whether it holds.
    pub verdict: VerdictJson,
    /// Its kind, when the commit carries one.
    pub kind: Option<SigningKindJson>,
    /// Who signed, `Ada <ada@example.com>`, empty when git cannot say.
    pub signer: String,
    /// The key it was made with, empty when git cannot say.
    pub key: String,
    /// The fingerprint of the key, empty when git cannot say.
    pub fingerprint: String,
    /// How far git trusts the key, empty when git cannot say.
    pub trust: String,
}

impl From<&CommitSignature> for CommitSignatureJson {
    fn from(signature: &CommitSignature) -> CommitSignatureJson {
        CommitSignatureJson {
            verdict: match signature.verdict {
                SignatureVerdict::Good => VerdictJson::Good,
                SignatureVerdict::GoodUnknownValidity => VerdictJson::GoodUnknownValidity,
                SignatureVerdict::GoodExpired => VerdictJson::GoodExpired,
                SignatureVerdict::GoodExpiredKey => VerdictJson::GoodExpiredKey,
                SignatureVerdict::GoodRevokedKey => VerdictJson::GoodRevokedKey,
                SignatureVerdict::Bad => VerdictJson::Bad,
                SignatureVerdict::CannotCheck => VerdictJson::CannotCheck,
                SignatureVerdict::SignedNotChecked => VerdictJson::SignedNotChecked,
                SignatureVerdict::NotSigned => VerdictJson::NotSigned,
            },
            kind: signature.kind.map(|kind| match kind {
                SignatureKind::OpenPgp => SigningKindJson::Openpgp,
                SignatureKind::X509 => SigningKindJson::X509,
                SignatureKind::Ssh => SigningKindJson::Ssh,
            }),
            signer: signature.signer.clone(),
            key: signature.key.clone(),
            fingerprint: signature.fingerprint.clone(),
            trust: signature.trust.clone(),
        }
    }
}

/// Someone who made commits in the repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorJson {
    /// The name of the latest of their commits.
    pub name: String,
    /// The email, as their latest commit has it.
    pub email: String,
    /// How many commits they made, among those read.
    pub commits: usize,
}

/// The answer of `read/commit.read-authors`: the one with the most commits first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorsResult {
    /// The authors.
    pub authors: Vec<AuthorJson>,
}

impl From<&[Author]> for AuthorsResult {
    fn from(authors: &[Author]) -> AuthorsResult {
        AuthorsResult {
            authors: authors
                .iter()
                .map(|author| AuthorJson {
                    name: author.name.clone(),
                    email: author.email.clone(),
                    commits: author.commits,
                })
                .collect(),
        }
    }
}
