//! The secrets Gitardo keeps, by name, in the credential store of the system: the GitHub token under `github.com`.
//! The engine keeps them through a [`SecretStore`], so that its tests keep them in memory and never in the user's
//! store.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use platform::PlatformError;

/// The name the GitHub token is kept under.
pub const GITHUB_TOKEN: &str = "github.com";

/// Where secrets are kept.
pub trait SecretStore: Send {
    /// The secret kept under a name; none when none is.
    ///
    /// # Errors
    ///
    /// When the store cannot be read.
    fn read(&self, name: &str) -> Result<Option<String>, PlatformError>;

    /// Keeps a secret under a name, in place of what was kept there.
    ///
    /// # Errors
    ///
    /// When the store refuses it.
    fn write(&self, name: &str, secret: &str) -> Result<(), PlatformError>;

    /// Takes the secret kept under a name out of the store.
    ///
    /// # Errors
    ///
    /// When the store refuses it.
    fn delete(&self, name: &str) -> Result<(), PlatformError>;
}

/// The credential store of the system.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemSecrets;

impl SecretStore for SystemSecrets {
    fn read(&self, name: &str) -> Result<Option<String>, PlatformError> {
        platform::read_secret(name)
    }

    fn write(&self, name: &str, secret: &str) -> Result<(), PlatformError> {
        platform::write_secret(name, secret)
    }

    fn delete(&self, name: &str) -> Result<(), PlatformError> {
        platform::delete_secret(name)
    }
}

/// Secrets kept in memory, shared by its clones: for tests, which never touch the user's store.
#[derive(Debug, Clone, Default)]
pub struct MemorySecrets(Arc<Mutex<BTreeMap<String, String>>>);

impl MemorySecrets {
    /// The secrets it keeps, by name.
    #[must_use]
    pub fn kept(&self) -> BTreeMap<String, String> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl SecretStore for MemorySecrets {
    fn read(&self, name: &str) -> Result<Option<String>, PlatformError> {
        Ok(self.0.lock().unwrap_or_else(PoisonError::into_inner).get(name).cloned())
    }

    fn write(&self, name: &str, secret: &str) -> Result<(), PlatformError> {
        let _before = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), PlatformError> {
        let _before = self.0.lock().unwrap_or_else(PoisonError::into_inner).remove(name);
        Ok(())
    }
}
