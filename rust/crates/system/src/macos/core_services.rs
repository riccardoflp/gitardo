//! The functions, types and constants of `CoreServices` that Gitardo asks for itself, with the names Apple's
//! documentation gives them: the streams of `FSEvents`, which say what changed under a folder.

#![allow(
    non_upper_case_globals,
    reason = "the names of the constants are those of Apple's documentation"
)]

use core::ffi::c_void;

use super::core_foundation::{Boolean, CFAllocatorRef, CFArrayRef, CFIndex, CFStringRef, CFTimeInterval};
use super::libsystem::dispatch_queue_t;

/// A stream of the changes under some folders.
pub(crate) type FSEventStreamRef = *mut c_void;
/// A stream, as its callback is given it.
pub(crate) type ConstFSEventStreamRef = *const c_void;
/// The number of a change, in the order the system saw them.
pub(crate) type FSEventStreamEventId = u64;
/// What a stream is asked to say.
pub(crate) type FSEventStreamCreateFlags = u32;
/// What a change was.
pub(crate) type FSEventStreamEventFlags = u32;

/// What a stream calls with the changes it gathered: their count, their paths (an array of C strings, the stream not
/// being asked for objects of `CoreFoundation`), what each was and its number.
pub(crate) type FSEventStreamCallback = unsafe extern "C" fn(
    stream: ConstFSEventStreamRef,
    info: *mut c_void,
    count: usize,
    paths: *mut c_void,
    flags: *const FSEventStreamEventFlags,
    ids: *const FSEventStreamEventId,
);

/// What a stream keeps for its callback, with how to retain and release it.
#[repr(C)]
pub(crate) struct FSEventStreamContext {
    pub(crate) version: CFIndex,
    pub(crate) info: *mut c_void,
    pub(crate) retain: Option<unsafe extern "C" fn(info: *const c_void) -> *const c_void>,
    pub(crate) release: Option<unsafe extern "C" fn(info: *const c_void)>,
    pub(crate) copy_description: Option<unsafe extern "C" fn(info: *const c_void) -> CFStringRef>,
}

/// The changes from now on, none of those before.
pub(crate) const kFSEventStreamEventIdSinceNow: FSEventStreamEventId = u64::MAX;

/// A change alone after a quiet moment is said at once, not after the latency.
pub(crate) const kFSEventStreamCreateFlagNoDefer: FSEventStreamCreateFlags = 0x0000_0002;
/// The folder watched itself being moved or deleted is said.
pub(crate) const kFSEventStreamCreateFlagWatchRoot: FSEventStreamCreateFlags = 0x0000_0004;
/// Each file is said, not only the folder it is in.
pub(crate) const kFSEventStreamCreateFlagFileEvents: FSEventStreamCreateFlags = 0x0000_0010;

/// Something changed under the path that the stream could not say one by one: everything under it is to be read.
pub(crate) const kFSEventStreamEventFlagMustScanSubDirs: FSEventStreamEventFlags = 0x0000_0001;
/// Changes were lost by the program's side.
pub(crate) const kFSEventStreamEventFlagUserDropped: FSEventStreamEventFlags = 0x0000_0002;
/// Changes were lost by the system's side.
pub(crate) const kFSEventStreamEventFlagKernelDropped: FSEventStreamEventFlags = 0x0000_0004;
/// The changes before the stream started are all said: none are asked for, but it may come.
pub(crate) const kFSEventStreamEventFlagHistoryDone: FSEventStreamEventFlags = 0x0000_0010;
/// The folder watched, or a folder above it, was moved or deleted.
pub(crate) const kFSEventStreamEventFlagRootChanged: FSEventStreamEventFlags = 0x0000_0020;
/// The item was created.
pub(crate) const kFSEventStreamEventFlagItemCreated: FSEventStreamEventFlags = 0x0000_0100;
/// The item was deleted.
pub(crate) const kFSEventStreamEventFlagItemRemoved: FSEventStreamEventFlags = 0x0000_0200;
/// The item was renamed, from or to its path.
pub(crate) const kFSEventStreamEventFlagItemRenamed: FSEventStreamEventFlags = 0x0000_0800;
/// The item is a folder.
pub(crate) const kFSEventStreamEventFlagItemIsDir: FSEventStreamEventFlags = 0x0002_0000;

#[link(name = "CoreServices", kind = "framework")]
unsafe extern "C" {
    pub(crate) fn FSEventStreamCreate(
        allocator: CFAllocatorRef,
        callback: FSEventStreamCallback,
        context: *const FSEventStreamContext,
        paths_to_watch: CFArrayRef,
        since_when: FSEventStreamEventId,
        latency: CFTimeInterval,
        flags: FSEventStreamCreateFlags,
    ) -> FSEventStreamRef;
    pub(crate) fn FSEventStreamSetDispatchQueue(stream: FSEventStreamRef, queue: dispatch_queue_t);
    pub(crate) fn FSEventStreamStart(stream: FSEventStreamRef) -> Boolean;
    pub(crate) fn FSEventStreamStop(stream: FSEventStreamRef);
    pub(crate) fn FSEventStreamInvalidate(stream: FSEventStreamRef);
    pub(crate) fn FSEventStreamRelease(stream: FSEventStreamRef);
}
