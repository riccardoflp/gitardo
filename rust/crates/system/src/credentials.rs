//! Secrets kept in the credential store of the system, by name: the GitHub token, the key of the AI provider.
//! Windows keeps them as generic credentials named `Gitardo Desktop/<name>`; Linux, until Gitardo speaks to the
//! Secret Service, in a file of the user's configuration folder only the user may read; macOS as generic passwords of
//! the keychain, of the service `Gitardo Desktop` (`docs/desktop/05-platform.md`, Credentials).

use crate::error::SystemError;

/// The secret kept under a name, such as `github.com`; none when nothing is kept there.
///
/// # Errors
///
/// When the store cannot be read, or the system has none Gitardo speaks to yet.
pub fn read_secret(name: &str) -> Result<Option<String>, SystemError> {
    #[cfg(windows)]
    return crate::windows::read_secret(name);
    #[cfg(target_os = "linux")]
    return crate::linux::read_secret(name);
    #[cfg(target_os = "macos")]
    return crate::macos::read_secret(name);
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    return Err(unsupported(name.is_empty()));
}

/// Keeps a secret under a name, in place of what was kept there.
///
/// # Errors
///
/// When the store refuses it, or the system has none Gitardo speaks to yet.
pub fn write_secret(name: &str, secret: &str) -> Result<(), SystemError> {
    #[cfg(windows)]
    return crate::windows::write_secret(name, secret);
    #[cfg(target_os = "linux")]
    return crate::linux::write_secret(name, secret);
    #[cfg(target_os = "macos")]
    return crate::macos::write_secret(name, secret);
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    return Err(unsupported(name.is_empty() || secret.is_empty()));
}

/// Takes the secret kept under a name out of the store; nothing happens when none is kept there.
///
/// # Errors
///
/// When the store refuses it, or the system has none Gitardo speaks to yet.
pub fn delete_secret(name: &str) -> Result<(), SystemError> {
    #[cfg(windows)]
    return crate::windows::delete_secret(name);
    #[cfg(target_os = "linux")]
    return crate::linux::delete_secret(name);
    #[cfg(target_os = "macos")]
    return crate::macos::delete_secret(name);
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    return Err(unsupported(name.is_empty()));
}

/// What a system without a credential store Gitardo speaks to says; what was asked does not change it.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn unsupported(_asked: bool) -> SystemError {
    SystemError::Unsupported {
        what: "keep secrets in its credential store yet",
    }
}

#[cfg(test)]
mod tests {
    use super::{delete_secret, read_secret, write_secret};

    /// Writes a secret of its own into the credential store of the user, reads it back and takes it out again: run by
    /// hand, so that `cargo test` does not touch the user's store.
    #[test]
    #[ignore = "writes to the credential store of the system"]
    fn a_secret_kept_is_read_back_and_taken_out() {
        let name = format!("test-{}", std::process::id());
        assert_eq!(read_secret(&name).unwrap(), None);
        write_secret(&name, "first").unwrap();
        write_secret(&name, "second, kept in its place").unwrap();
        assert_eq!(
            read_secret(&name).unwrap().as_deref(),
            Some("second, kept in its place")
        );
        delete_secret(&name).unwrap();
        assert_eq!(read_secret(&name).unwrap(), None);
        delete_secret(&name).unwrap();
    }
}
