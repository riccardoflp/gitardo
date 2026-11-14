//! Secrets in the keychain of the user, as generic passwords of the service `Gitardo Desktop` whose account is the name
//! of the secret, such as `github.com`.

use super::core_foundation::{CFTypeRef, kCFBooleanTrue};
use super::objects::{self, Owned};
use super::security as sec;
use crate::error::SystemError;

/// The service every secret of Gitardo is kept under.
const SERVICE: &str = "Gitardo Desktop";

/// The strings that name the item of a secret, kept alive while a query that points to them is.
struct Item {
    service: Owned,
    account: Owned,
}

impl Item {
    /// The item kept under a name.
    fn named(name: &str) -> Result<Item, SystemError> {
        Ok(Item {
            service: objects::string(SERVICE)?,
            account: objects::string(name)?,
        })
    }

    /// The pairs of a query that finds the item, with more pairs after them.
    fn query(&self, more: &[(CFTypeRef, CFTypeRef)]) -> Result<Owned, SystemError> {
        let mut pairs = vec![
            (sec::kSecClass, sec::kSecClassGenericPassword),
            (sec::kSecAttrService, self.service.get()),
            (sec::kSecAttrAccount, self.account.get()),
        ];
        pairs.extend_from_slice(more);
        objects::dictionary(&pairs)
    }
}

/// What the keychain said, as an error.
fn refused(call: &'static str, status: sec::OSStatus) -> SystemError {
    SystemError::System {
        call,
        details: format!("the keychain said {status}"),
    }
}

/// The secret kept under a name; none when there is none.
pub(crate) fn read_secret(name: &str) -> Result<Option<String>, SystemError> {
    let item = Item::named(name)?;
    let query = item.query(&[
        (sec::kSecReturnData, kCFBooleanTrue),
        (sec::kSecMatchLimit, sec::kSecMatchLimitOne),
    ])?;
    let mut found: CFTypeRef = core::ptr::null();
    // SAFETY: the query is a dictionary alive for the call; the keychain writes into `found` bytes it copied for the
    // caller, which `Owned` releases.
    let status = unsafe { sec::SecItemCopyMatching(query.get(), &raw mut found) };
    if status == sec::errSecItemNotFound {
        return Ok(None);
    }
    if status != sec::errSecSuccess {
        return Err(refused("SecItemCopyMatching", status));
    }
    let Some(found) = Owned::take(found) else {
        return Ok(None);
    };
    String::from_utf8(objects::bytes_of(&found))
        .map(Some)
        .map_err(|error| SystemError::system("reading a secret of the keychain", &error))
}

/// Keeps a secret under a name, in place of what was kept there.
pub(crate) fn write_secret(name: &str, secret: &str) -> Result<(), SystemError> {
    let item = Item::named(name)?;
    let value = objects::data(secret.as_bytes())?;
    let query = item.query(&[])?;
    let change = objects::dictionary(&[(sec::kSecValueData, value.get())])?;
    // SAFETY: both dictionaries are alive for the call.
    let status = unsafe { sec::SecItemUpdate(query.get(), change.get()) };
    if status == sec::errSecSuccess {
        return Ok(());
    }
    if status != sec::errSecItemNotFound {
        return Err(refused("SecItemUpdate", status));
    }
    let added = item.query(&[(sec::kSecValueData, value.get())])?;
    // SAFETY: the dictionary is alive for the call, and no result is asked for.
    let status = unsafe { sec::SecItemAdd(added.get(), core::ptr::null_mut()) };
    if status == sec::errSecSuccess {
        Ok(())
    } else {
        Err(refused("SecItemAdd", status))
    }
}

/// Takes the secret kept under a name out of the keychain; nothing happens when there is none.
pub(crate) fn delete_secret(name: &str) -> Result<(), SystemError> {
    let item = Item::named(name)?;
    let query = item.query(&[])?;
    // SAFETY: the dictionary is alive for the call.
    let status = unsafe { sec::SecItemDelete(query.get()) };
    if status == sec::errSecSuccess || status == sec::errSecItemNotFound {
        Ok(())
    } else {
        Err(refused("SecItemDelete", status))
    }
}
