//! The functions, types and constants of `CoreFoundation` that Gitardo Desktop asks for itself, with the names Apple's
//! documentation gives them: the strings, arrays, dictionaries and bytes that the keychain and the watching of
//! folders are given and give back, and releasing them.

#![allow(
    non_upper_case_globals,
    reason = "the names of the constants are those of Apple's documentation"
)]

use core::ffi::c_void;

/// Any object of `CoreFoundation`.
pub(crate) type CFTypeRef = *const c_void;
/// A string of `CoreFoundation`.
pub(crate) type CFStringRef = *const c_void;
/// An array of `CoreFoundation`.
pub(crate) type CFArrayRef = *const c_void;
/// A dictionary of `CoreFoundation`.
pub(crate) type CFDictionaryRef = *const c_void;
/// Bytes kept by `CoreFoundation`.
pub(crate) type CFDataRef = *const c_void;
/// Who allocates an object: null is the default allocator.
pub(crate) type CFAllocatorRef = *const c_void;
/// A count or an index.
pub(crate) type CFIndex = isize;
/// A boolean of `CoreFoundation`: 0 is false.
pub(crate) type Boolean = u8;
/// How text is encoded.
pub(crate) type CFStringEncoding = u32;
/// A length of time, in seconds.
pub(crate) type CFTimeInterval = f64;

/// UTF-8.
pub(crate) const kCFStringEncodingUTF8: CFStringEncoding = 0x0800_0100;

/// How an array keeps its values: only its address is used, so its fields are left out.
#[repr(C)]
pub(crate) struct CFArrayCallBacks {
    _private: [u8; 0],
}

/// How a dictionary keeps its keys: only its address is used.
#[repr(C)]
pub(crate) struct CFDictionaryKeyCallBacks {
    _private: [u8; 0],
}

/// How a dictionary keeps its values: only its address is used.
#[repr(C)]
pub(crate) struct CFDictionaryValueCallBacks {
    _private: [u8; 0],
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Arrays that retain and release the objects they hold.
    pub(crate) static kCFTypeArrayCallBacks: CFArrayCallBacks;
    /// Dictionaries whose keys are objects, retained and released.
    pub(crate) static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
    /// Dictionaries whose values are objects, retained and released.
    pub(crate) static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;
    /// The boolean true, as an object.
    pub(crate) safe static kCFBooleanTrue: CFTypeRef;

    pub(crate) fn CFRelease(object: CFTypeRef);
    pub(crate) fn CFStringCreateWithBytes(
        allocator: CFAllocatorRef,
        bytes: *const u8,
        length: CFIndex,
        encoding: CFStringEncoding,
        is_external_representation: Boolean,
    ) -> CFStringRef;
    pub(crate) fn CFArrayCreate(
        allocator: CFAllocatorRef,
        values: *const *const c_void,
        count: CFIndex,
        callbacks: *const CFArrayCallBacks,
    ) -> CFArrayRef;
    pub(crate) fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: CFIndex,
        key_callbacks: *const CFDictionaryKeyCallBacks,
        value_callbacks: *const CFDictionaryValueCallBacks,
    ) -> CFDictionaryRef;
    pub(crate) fn CFDataCreate(allocator: CFAllocatorRef, bytes: *const u8, length: CFIndex) -> CFDataRef;
    pub(crate) fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
    pub(crate) fn CFDataGetLength(data: CFDataRef) -> CFIndex;
}
