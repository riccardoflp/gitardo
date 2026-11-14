//! A TLS stream over TCP that trusts the certificates of the system; the one crate with a library from outside.
//!
//! Gitardo speaks HTTPS itself only to the API of GitHub and to the AI provider (decision D13). This crate gives the
//! rest of the program a stream that reads and writes, opened over a TCP connection with the name of the host it
//! reaches, checked against the root certificates a [`Trust`] holds: those the system trusts, read by
//! `rustls-native-certs`, or those of a file. No type of `rustls` appears in its interface (rule R1.5), so that
//! replacing the library changes this crate only.

#![forbid(unsafe_code)]

mod error;
mod stream;
#[cfg(any(test, feature = "test-server"))]
pub mod test_server;
mod trust;

pub use error::Error;
pub use stream::TlsStream;
pub use trust::Trust;

#[cfg(test)]
mod tests;
