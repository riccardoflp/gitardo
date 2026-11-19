//! Commits signed with an SSH key made for one test, in the test's own folder, never with a key of the person running
//! the tests.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use super::Repository;
use crate::environment::IDENTITY_EMAIL;

/// An SSH key made for a test, which git signs with.
#[derive(Debug, Clone)]
pub struct SigningKey {
    /// The private key, which `user.signingKey` names.
    pub private: PathBuf,
    /// The public key, as `ssh-keygen` wrote it: `ssh-ed25519 AAAA… comment`.
    pub public: String,
    /// The fingerprint of the key, as git says it: `SHA256:…`.
    pub fingerprint: String,
}

impl Repository {
    /// Makes an Ed25519 key with no passphrase in the test's folder with `ssh-keygen`, and sets the repository to sign
    /// with it: `gpg.format` `ssh`, `user.signingKey` the key, and `gpg.ssh.program` the `ssh-keygen` found.
    ///
    /// # Panics
    ///
    /// When `ssh-keygen` cannot be found or fails: the test needs it, as git does to sign with SSH.
    #[must_use]
    pub fn ssh_signing_key(&self) -> SigningKey {
        let program = ssh_keygen(self);
        let private = self.folder().join("signing_key");
        let output = self
            .command(&program)
            .args(["-q", "-t", "ed25519", "-N", "", "-C", IDENTITY_EMAIL, "-f"])
            .arg(&private)
            .output()
            .unwrap_or_else(|error| panic!("could not run {}: {error}", program.display()));
        assert!(
            output.status.success(),
            "ssh-keygen failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let public = fs::read_to_string(private.with_extension("pub"))
            .unwrap()
            .trim_end()
            .to_owned();
        let printed = self
            .command(&program)
            .args(["-l", "-E", "sha256", "-f"])
            .arg(private.with_extension("pub"))
            .output()
            .unwrap();
        let listed = String::from_utf8_lossy(&printed.stdout).into_owned();
        let fingerprint = listed.split(' ').nth(1).unwrap_or_default().to_owned();
        let private_text = slashed(&private);
        for (key, value) in [
            ("gpg.format", "ssh"),
            ("user.signingKey", private_text.as_str()),
            ("gpg.ssh.program", slashed(&program).as_str()),
        ] {
            let _ = self.git(&["config", key, value]);
        }
        SigningKey {
            private,
            public,
            fingerprint,
        }
    }

    /// Lists the key as the identity of the tests' in a file of allowed signers, and names it in
    /// `gpg.ssh.allowedSignersFile`, so that git can check what it signed.
    ///
    /// # Panics
    ///
    /// When the file cannot be written or git fails.
    pub fn allow_signer(&self, key: &SigningKey) {
        let file = self.folder().join("allowed_signers");
        fs::write(&file, format!("{IDENTITY_EMAIL} {}\n", key.public)).unwrap();
        let _ = self.git(&["config", "gpg.ssh.allowedSignersFile", &slashed(&file)]);
    }

    /// Makes an empty commit signed with the key the repository signs with, one minute after the commit before, and
    /// returns its full id.
    ///
    /// # Panics
    ///
    /// When git fails, as when no key is set.
    pub fn commit_signed(&mut self, message: &str) -> String {
        let _ = self.git(&["commit", "--quiet", "--allow-empty", "--gpg-sign", "-m", message]);
        self.date = self.date.next();
        self.git(&["rev-parse", "HEAD"]).trim_end().to_owned()
    }
}

/// `ssh-keygen` on the `PATH`, or else the one that comes with Git for Windows, two folders above git's own programs.
fn ssh_keygen(repository: &Repository) -> PathBuf {
    let found = repository.command("ssh-keygen").arg("-?").output().is_ok();
    if found {
        return PathBuf::from("ssh-keygen");
    }
    let exec_path = repository.git(&["--exec-path"]);
    let mut beside = PathBuf::from(exec_path.trim_end());
    for _ in 0..3 {
        let _ = beside.pop();
    }
    let program = beside.join("usr").join("bin").join("ssh-keygen.exe");
    assert!(
        program.is_file(),
        "ssh-keygen is needed to sign with SSH, and is not on the PATH"
    );
    program
}

/// A path with `/` between its folders, which git's configuration reads on every system.
fn slashed(path: &Path) -> String {
    OsString::from(path).to_string_lossy().replace('\\', "/")
}
