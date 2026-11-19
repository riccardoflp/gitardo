//! The signature of a commit, as git says of it (`details.read-signature`): whether it holds, by whom, with which key.

/// What git says of the signature of a commit, from the letter of `%G?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureVerdict {
    /// A good signature (`G`).
    Good,
    /// A good signature from a key whose validity is not known (`U`).
    GoodUnknownValidity,
    /// A good signature that has expired (`X`).
    GoodExpired,
    /// A good signature made by a key that has expired (`Y`).
    GoodExpiredKey,
    /// A good signature made by a key that was revoked (`R`).
    GoodRevokedKey,
    /// A bad signature (`B`).
    Bad,
    /// A signature git cannot check, as when its key is missing (`E`).
    CannotCheck,
    /// A signature git says nothing of (`N`) though the commit carries one, as a commit signed with SSH while
    /// `gpg.ssh.allowedSignersFile` is not set.
    SignedNotChecked,
    /// No signature.
    NotSigned,
}

impl SignatureVerdict {
    /// The verdict of a letter of `%G?`; none for a letter git does not print.
    #[must_use]
    pub fn of_letter(letter: &str) -> Option<SignatureVerdict> {
        Some(match letter {
            "G" => SignatureVerdict::Good,
            "U" => SignatureVerdict::GoodUnknownValidity,
            "X" => SignatureVerdict::GoodExpired,
            "Y" => SignatureVerdict::GoodExpiredKey,
            "R" => SignatureVerdict::GoodRevokedKey,
            "B" => SignatureVerdict::Bad,
            "E" => SignatureVerdict::CannotCheck,
            "N" => SignatureVerdict::NotSigned,
            _ => return None,
        })
    }

    /// Whether git found the signature good, doubted or not.
    #[must_use]
    pub fn is_good(self) -> bool {
        matches!(
            self,
            SignatureVerdict::Good
                | SignatureVerdict::GoodUnknownValidity
                | SignatureVerdict::GoodExpired
                | SignatureVerdict::GoodExpiredKey
                | SignatureVerdict::GoodRevokedKey
        )
    }
}

/// The kind of a signature, from the first line of the header that carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureKind {
    /// `-----BEGIN PGP SIGNATURE-----`.
    OpenPgp,
    /// `-----BEGIN SIGNED MESSAGE-----`.
    X509,
    /// `-----BEGIN SSH SIGNATURE-----`.
    Ssh,
}

impl SignatureKind {
    /// The kind of the first line of a signature; none for a line git does not write.
    #[must_use]
    pub fn of_first_line(line: &str) -> Option<SignatureKind> {
        match line.trim() {
            "-----BEGIN PGP SIGNATURE-----" | "-----BEGIN PGP MESSAGE-----" => Some(SignatureKind::OpenPgp),
            "-----BEGIN SIGNED MESSAGE-----" => Some(SignatureKind::X509),
            "-----BEGIN SSH SIGNATURE-----" => Some(SignatureKind::Ssh),
            _ => None,
        }
    }
}

/// The signature of a commit, as git says of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSignature {
    /// Whether it holds.
    pub verdict: SignatureVerdict,
    /// Its kind, when the commit carries one.
    pub kind: Option<SignatureKind>,
    /// Who signed, as git names them (`%GS`): `Ada <ada@example.com>`, or empty when git cannot say.
    pub signer: String,
    /// The key it was made with (`%GK`), empty when git cannot say.
    pub key: String,
    /// The fingerprint of the key (`%GF`), empty when git cannot say.
    pub fingerprint: String,
    /// How far git trusts the key (`%GT`): `ultimate`, `fully`, `undefined`…, empty when git cannot say.
    pub trust: String,
}

impl CommitSignature {
    /// A commit with no signature.
    #[must_use]
    pub fn not_signed() -> CommitSignature {
        CommitSignature {
            verdict: SignatureVerdict::NotSigned,
            kind: None,
            signer: String::new(),
            key: String::new(),
            fingerprint: String::new(),
            trust: String::new(),
        }
    }
}
