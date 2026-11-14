//! The functions, types and constants of `CoreFoundation` that Gitardo asks for itself, with the names Apple's
//! documentation gives them: the strings, arrays, dictionaries and bytes that the keychain and the watching of
//! folders are given and give back, and releasing them.

#![allow(
    non_upper_case_globals,
    reason = "the names of the constants are those of Apple's documentation"
)]

use core::ffi::c_void;

/// Any object of `CoreFoundation`.
pub type CFTypeRef = *const c_void;
/// A string of `CoreFoundation`.
pub type CFStringRef = *const c_void;
/// An array of `CoreFoundation`.
pub type CFArrayRef = *const c_void;
/// A dictionary of `CoreFoundation`.
pub type CFDictionaryRef = *const c_void;
/// Bytes kept by `CoreFoundation`.
pub type CFDataRef = *const c_void;
/// Who allocates an object: null is the default allocator.
pub type CFAllocatorRef = *const c_void;
/// A count or an index.
pub type CFIndex = isize;
/// A boolean of `CoreFoundation`: 0 is false.
pub type Boolean = u8;
/// How text is encoded.
pub type CFStringEncoding = u32;
/// A length of time, in seconds.
pub type CFTimeInterval = f64;

/// UTF-8.
pub const kCFStringEncodingUTF8: CFStringEncoding = 0x0800_0100;

/// How an array keeps its values: only its address is used, so its fields are left out.
#[repr(C)]
pub struct CFArrayCallBacks {
    _private: [u8; 0],
}

/// How a dictionary keeps its keys: only its address is used.
#[repr(C)]
pub struct CFDictionaryKeyCallBacks {
    _private: [u8; 0],
}

/// How a dictionary keeps its values: only its address is used.
#[repr(C)]
pub struct CFDictionaryValueCallBacks {
    _private: [u8; 0],
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Arrays that retain and release the objects they hold.
    pub static kCFTypeArrayCallBacks: CFArrayCallBacks;
    /// Dictionaries whose keys are objects, retained and released.
    pub static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
    /// Dictionaries whose values are objects, retained and released.
    pub static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;
    /// The boolean true, as an object.
    pub safe static kCFBooleanTrue: CFTypeRef;

    pub fn CFRelease(object: CFTypeRef);
    pub fn CFStringCreateWithBytes(
        allocator: CFAllocatorRef,
        bytes: *const u8,
        length: CFIndex,
        encoding: CFStringEncoding,
        is_external_representation: Boolean,
    ) -> CFStringRef;
    pub fn CFArrayCreate(
        allocator: CFAllocatorRef,
        values: *const *const c_void,
        count: CFIndex,
        callbacks: *const CFArrayCallBacks,
    ) -> CFArrayRef;
    pub fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: CFIndex,
        key_callbacks: *const CFDictionaryKeyCallBacks,
        value_callbacks: *const CFDictionaryValueCallBacks,
    ) -> CFDictionaryRef;
    pub fn CFDataCreate(allocator: CFAllocatorRef, bytes: *const u8, length: CFIndex) -> CFDataRef;
    pub fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
    pub fn CFDataGetLength(data: CFDataRef) -> CFIndex;
}
