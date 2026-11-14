//! What can go wrong with a request, in words the user can act on. No message carries a header's value, the body of
//! a request or the password of a proxy (`docs/05-platform.md`, Secrets).

use std::fmt;

/// What went wrong.
#[derive(Debug)]
pub enum Error {
    /// An address that is not one Gitardo can reach: not `http` or `https`, or with no host.
    BadUrl {
        /// What is wrong with it.
        reason: String,
    },
    /// The name of the host could not be turned into an address.
    Resolve {
        /// The host.
        host: String,
        /// What the system said.
        source: std::io::Error,
    },
    /// No connection could be made to the host, or to the proxy.
    Connect {
        /// The host or the proxy.
        host: String,
        /// What the system said.
        source: std::io::Error,
    },
    /// The host, or the proxy, did not answer in time.
    TimedOut {
        /// The host or the proxy.
        host: String,
    },
    /// The connection could not be made secure.
    Tls(tls::Error),
    /// The proxy refused to open a tunnel to the host.
    Proxy {
        /// The proxy.
        proxy: String,
        /// The status it answered, and its reason: `407 Proxy Authentication Required`.
        status: String,
    },
    /// The connection failed while the request was sent or the answer read.
    Io {
        /// The host.
        host: String,
        /// What the system said.
        source: std::io::Error,
    },
    /// The answer is not HTTP that Gitardo can read.
    BadResponse {
        /// The host.
        host: String,
        /// What is wrong with it.
        reason: String,
    },
    /// The answer sent the request elsewhere more than five times.
    TooManyRedirects {
        /// The host that sent it the last time.
        host: String,
    },
    /// The answer sent the request to an address that is not `https`, where it would not be secure.
    InsecureRedirect {
        /// The host that sent it there.
        host: String,
    },
    /// The request was cancelled: nobody wants its answer any more.
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadUrl { reason } => write!(formatter, "The address cannot be reached: {reason}"),
            Error::Resolve { host, source } => write!(formatter, "Could not find {host}: {source}"),
            Error::Connect { host, source } => write!(formatter, "Could not connect to {host}: {source}"),
            Error::TimedOut { host } => write!(formatter, "{host} did not answer in time."),
            Error::Tls(error) => error.fmt(formatter),
            Error::Proxy { proxy, status } => {
                write!(
                    formatter,
                    "The proxy {proxy} refused the connection: it answered {status}."
                )
            }
            Error::Io { host, source } => write!(formatter, "The connection to {host} failed: {source}"),
            Error::BadResponse { host, reason } => write!(formatter, "{host} answered something unreadable: {reason}"),
            Error::TooManyRedirects { host } => {
                write!(formatter, "{host} sent the request elsewhere more than five times.")
            }
            Error::InsecureRedirect { host } => write!(
                formatter,
                "{host} sent the request to an address that is not secure (not https), so it was not followed."
            ),
            Error::Cancelled => formatter.write_str("The request was cancelled."),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Resolve { source, .. } | Error::Connect { source, .. } | Error::Io { source, .. } => Some(source),
            Error::Tls(error) => Some(error),
            Error::BadUrl { .. }
            | Error::TimedOut { .. }
            | Error::Proxy { .. }
            | Error::BadResponse { .. }
            | Error::TooManyRedirects { .. }
            | Error::InsecureRedirect { .. }
            | Error::Cancelled => None,
        }
    }
}
