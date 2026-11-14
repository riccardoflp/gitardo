//! The functions and constants of the `Security` framework that Gitardo asks for itself, with the names Apple's
//! documentation gives them: generic passwords in the keychain of the user.

#![allow(
    non_upper_case_globals,
    reason = "the names of the constants are those of Apple's documentation"
)]

use super::core_foundation::{CFDictionaryRef, CFStringRef, CFTypeRef};

/// What a call of `Security` says: 0 when it did what was asked.
pub(crate) type OSStatus = i32;

/// The call did what was asked.
pub(crate) const errSecSuccess: OSStatus = 0;
/// No item of the keychain matches the query.
pub(crate) const errSecItemNotFound: OSStatus = -25_300;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    /// The key of a query that says the class of the items.
    pub(crate) safe static kSecClass: CFStringRef;
    /// The class of the passwords a program keeps for itself.
    pub(crate) safe static kSecClassGenericPassword: CFStringRef;
    /// The key of the service an item belongs to: Gitardo's name.
    pub(crate) safe static kSecAttrService: CFStringRef;
    /// The key of the account of an item: the name of the secret.
    pub(crate) safe static kSecAttrAccount: CFStringRef;
    /// The key of the secret itself, as bytes.
    pub(crate) safe static kSecValueData: CFStringRef;
    /// The key that asks for the secret back.
    pub(crate) safe static kSecReturnData: CFStringRef;
    /// The key that says how many items a query may give.
    pub(crate) safe static kSecMatchLimit: CFStringRef;
    /// One item at most.
    pub(crate) safe static kSecMatchLimitOne: CFStringRef;

    pub(crate) fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    pub(crate) fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    pub(crate) fn SecItemUpdate(query: CFDictionaryRef, attributes_to_update: CFDictionaryRef) -> OSStatus;
    pub(crate) fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
}
