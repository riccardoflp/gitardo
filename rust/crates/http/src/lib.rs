//! HTTP/1.1 for Gitardo Desktop: requests, chunked bodies, server-sent events and proxies.
//!
//! Gitardo speaks HTTPS itself only to the API of GitHub and to the AI provider (decision D13); git does its own
//! networking. A request goes in, an answer or an [`Error`] comes out, and a [`Cancel`] stops it:
//!
//! - [`Http`] sends a [`Request`] with [`Http::send`], which reads the whole [`Response`], or with [`Http::open`],
//!   which gives the head at once and the [`Body`] to read as it arrives, for the [`Events`] of a model that streams.
//! - One connection per request, made secure with the crate `tls` for `https`; timeouts for connecting (10 s) and
//!   between two reads (60 s, more for a model that streams: [`Request::idle_timeout`]).
//! - `Content-Length`, chunked bodies, and bodies that end with the connection; gzip is never asked for.
//! - Redirects followed up to five times, to `https` only.
//! - Proxies ([`Proxies`]): `HTTPS_PROXY` or git's `http.proxy`, reached with `CONNECT`, with basic authentication;
//!   `NO_PROXY` for the hosts reached directly.
//! - Secrets ([`Request::secret_header`]) go in headers only, and are never shown in `Debug` or in an error.

#![forbid(unsafe_code)]

mod base64;
mod body;
mod cancel;
mod client;
mod connection;
mod error;
mod events;
mod proxy;
mod request;
mod response;
#[cfg(any(test, feature = "test-server"))]
pub mod test_server;
mod url;

pub use cancel::Cancel;
pub use client::{Body, Http, Opened};
pub use error::Error;
pub use events::{Event, Events};
pub use proxy::{Proxies, ProxySettings};
pub use request::{Header, Method, Request, Timeouts};
pub use response::{Head, Response};
pub use tls::Trust;
pub use url::{Scheme, Url};

#[cfg(test)]
mod tests;
