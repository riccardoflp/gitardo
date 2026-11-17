//! The messages between the extension for VS Code and Gitardo's engine, `gitardo-engine --stdio`: JSON-RPC 2.0
//! ([`rpc`]), the parameters and answers of each method ([`messages`]), and the JSON form of the types of `model` and
//! `engine` they carry ([`state`]), converted from them here so that `model` stays without `serde` (rule R1.7,
//! decision D42). This crate is what `docs/transition/02-protocol.md` says, field by field.
//!
//! The version is [`messages::VERSION`]. A field or a message added raises its minor number; one changed or removed
//! raises its major number, which the extension checks at `initialize`.

#![forbid(unsafe_code)]

pub mod ai;
pub mod feature;
pub mod journal;
pub mod mermaid;
pub mod messages;
pub mod reads;
pub mod rpc;
pub mod state;
