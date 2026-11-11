//! The operating system, as Gitardo Desktop sees it: windows, input, clipboard, timers, file watching, dialogs, credentials, opening and revealing.
//!
//! This is the one crate allowed to use `unsafe` (rule R5.1): every call into the system is wrapped here in a safe
//! function, so that the crates above see none of it.

#![allow(unsafe_code)]
