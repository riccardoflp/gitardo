//! The objects of `CoreFoundation` that Gitardo makes or is given, each owned by one value that releases it when
//! dropped: strings, bytes, arrays and dictionaries.

use core::ffi::c_void;

use super::core_foundation as cf;
use crate::error::SystemError;

/// An object of `CoreFoundation` this value owns and releases once.
pub struct Owned(cf::CFTypeRef);

impl Owned {
    /// Takes an object the system made or copied for the caller; none when it gave nothing.
    pub fn take(object: cf::CFTypeRef) -> Option<Owned> {
        (!object.is_null()).then_some(Owned(object))
    }

    /// The object, for a call that borrows it while this value is alive.
    pub fn get(&self) -> cf::CFTypeRef {
        self.0
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the object is not null, was made or copied for this value, which releases it once.
        unsafe { cf::CFRelease(self.0) };
    }
}

/// The length of some bytes or values, as `CoreFoundation` counts them.
fn length(count: usize) -> Result<cf::CFIndex, SystemError> {
    cf::CFIndex::try_from(count).map_err(|error| SystemError::system("CoreFoundation", &error))
}

/// What `CoreFoundation` says when it could not make an object.
fn refused(call: &'static str) -> SystemError {
    SystemError::System {
        call,
        details: String::from("no object was made"),
    }
}

/// A string of `CoreFoundation` with the text.
pub fn string(text: &str) -> Result<Owned, SystemError> {
    let count = length(text.len())?;
    let (bytes, utf8) = (text.as_ptr(), cf::kCFStringEncodingUTF8);
    // SAFETY: the bytes are alive for the call, and the count says how many there are; the string copies them.
    let made = unsafe { cf::CFStringCreateWithBytes(core::ptr::null(), bytes, count, utf8, 0) };
    Owned::take(made).ok_or_else(|| refused("CFStringCreateWithBytes"))
}

/// Bytes kept by `CoreFoundation`, copied from these.
pub fn data(bytes: &[u8]) -> Result<Owned, SystemError> {
    let count = length(bytes.len())?;
    // SAFETY: the bytes are alive for the call, and the count says how many there are; the object copies them.
    let made = unsafe { cf::CFDataCreate(core::ptr::null(), bytes.as_ptr(), count) };
    Owned::take(made).ok_or_else(|| refused("CFDataCreate"))
}

/// The bytes of an object of bytes, copied out of it.
pub fn bytes_of(data: &Owned) -> Vec<u8> {
    // SAFETY: the object is alive while `data` is, and is bytes, as the caller asked the keychain for.
    let (start, count) = unsafe { (cf::CFDataGetBytePtr(data.get()), cf::CFDataGetLength(data.get())) };
    let Ok(count) = usize::try_from(count) else {
        return Vec::new();
    };
    if start.is_null() || count == 0 {
        return Vec::new();
    }
    // SAFETY: `CoreFoundation` says the object holds `count` bytes from `start`, alive while `data` is; they are copied
    // before it is released.
    unsafe { core::slice::from_raw_parts(start, count) }.to_vec()
}

/// An array of objects, which it retains.
pub fn array(values: &[&Owned]) -> Result<Owned, SystemError> {
    let pointers: Vec<*const c_void> = values.iter().map(|value| value.get()).collect();
    let count = length(pointers.len())?;
    // SAFETY: the pointers are objects alive for the call, as many as the count says; the callbacks are those of
    // `CoreFoundation`, which retain them in the array.
    let made = unsafe {
        cf::CFArrayCreate(
            core::ptr::null(),
            pointers.as_ptr(),
            count,
            &raw const cf::kCFTypeArrayCallBacks,
        )
    };
    Owned::take(made).ok_or_else(|| refused("CFArrayCreate"))
}

/// A dictionary of keys and values, which it retains.
pub fn dictionary(pairs: &[(cf::CFTypeRef, cf::CFTypeRef)]) -> Result<Owned, SystemError> {
    let keys: Vec<*const c_void> = pairs.iter().map(|(key, _)| *key).collect();
    let values: Vec<*const c_void> = pairs.iter().map(|(_, value)| *value).collect();
    let count = length(pairs.len())?;
    // SAFETY: the keys and values are objects alive for the call, as many as the count says; the callbacks are those
    // of `CoreFoundation`, which retain them in the dictionary.
    let made = unsafe {
        cf::CFDictionaryCreate(
            core::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            count,
            &raw const cf::kCFTypeDictionaryKeyCallBacks,
            &raw const cf::kCFTypeDictionaryValueCallBacks,
        )
    };
    Owned::take(made).ok_or_else(|| refused("CFDictionaryCreate"))
}
