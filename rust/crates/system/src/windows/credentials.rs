//! Secrets in the credential store of Windows, as generic credentials named `Gitardo Desktop/<name>`, kept for this
//! user on this computer.

use core::ffi::c_void;

use super::advapi32;
use crate::error::SystemError;

/// What every credential of Gitardo is named with, before its own name.
const PREFIX: &str = "Gitardo Desktop/";

/// A credential the store gave, freed when dropped.
struct Read(*mut advapi32::CREDENTIALW);

impl Drop for Read {
    fn drop(&mut self) {
        // SAFETY: the pointer was given by `CredReadW` for this value, which owns it and frees it once.
        unsafe { advapi32::CredFree(self.0.cast::<c_void>()) };
    }
}

/// The name of a credential, in UTF-16 ended by a NUL.
fn target(name: &str) -> Vec<u16> {
    PREFIX
        .encode_utf16()
        .chain(name.encode_utf16())
        .chain(Some(0))
        .collect()
}

/// The secret kept under a name; none when there is none.
pub(crate) fn read_secret(name: &str) -> Result<Option<String>, SystemError> {
    let target = target(name);
    let mut credential = core::ptr::null_mut();
    // SAFETY: the name is ended by a NUL and lives through the call; the store writes a pointer it allocated into
    // `credential`, which `Read` frees.
    let read = unsafe { advapi32::CredReadW(target.as_ptr(), advapi32::CRED_TYPE_GENERIC, 0, &raw mut credential) };
    if read == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(advapi32::ERROR_NOT_FOUND) {
            return Ok(None);
        }
        return Err(SystemError::system("CredReadW", &error));
    }
    let credential = Read(credential);
    // SAFETY: the read succeeded, so the pointer is a credential the store allocated, valid until it is freed, and its
    // blob is `CredentialBlobSize` bytes long, or null when that is 0.
    let bytes = unsafe {
        let read = &*credential.0;
        if read.CredentialBlob.is_null() {
            Vec::new()
        } else {
            core::slice::from_raw_parts(read.CredentialBlob, read.CredentialBlobSize as usize).to_vec()
        }
    };
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| SystemError::system("CredReadW", &error))
}

/// Keeps a secret under a name, in place of what was kept there.
pub(crate) fn write_secret(name: &str, secret: &str) -> Result<(), SystemError> {
    let mut target = target(name);
    let mut user: Vec<u16> = "Gitardo Desktop".encode_utf16().chain(Some(0)).collect();
    let mut blob = secret.as_bytes().to_vec();
    let size = u32::try_from(blob.len()).map_err(|error| SystemError::system("CredWriteW", &error))?;
    let credential = advapi32::CREDENTIALW {
        Flags: 0,
        Type: advapi32::CRED_TYPE_GENERIC,
        TargetName: target.as_mut_ptr(),
        Comment: core::ptr::null_mut(),
        LastWritten: advapi32::FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        },
        CredentialBlobSize: size,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: advapi32::CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: core::ptr::null_mut(),
        TargetAlias: core::ptr::null_mut(),
        UserName: user.as_mut_ptr(),
    };
    // SAFETY: every pointer of the credential points into a buffer that lives through the call, the names are ended by
    // a NUL, and the blob is as long as its size says; the store copies what it keeps.
    let written = unsafe { advapi32::CredWriteW(&raw const credential, 0) };
    if written == 0 {
        return Err(SystemError::system("CredWriteW", &std::io::Error::last_os_error()));
    }
    Ok(())
}

/// Takes the secret kept under a name out of the store; nothing happens when there is none.
pub(crate) fn delete_secret(name: &str) -> Result<(), SystemError> {
    let target = target(name);
    // SAFETY: the name is ended by a NUL and lives through the call.
    let deleted = unsafe { advapi32::CredDeleteW(target.as_ptr(), advapi32::CRED_TYPE_GENERIC, 0) };
    if deleted == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(advapi32::ERROR_NOT_FOUND) {
            return Err(SystemError::system("CredDeleteW", &error));
        }
    }
    Ok(())
}
