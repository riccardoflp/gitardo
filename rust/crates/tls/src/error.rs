//! What can go wrong opening or using a TLS stream, in words the user can act on.

use std::fmt;

/// What went wrong.
#[derive(Debug)]
pub enum Error {
    /// The system gave no root certificate at all, so no server could be trusted.
    NoRootCertificates {
        /// What the system said while they were read, when it said something.
        reason: String,
    },
    /// A file of root certificates held none that could be read.
    BadCertificates {
        /// What was wrong with it.
        reason: String,
    },
    /// The name of the host cannot be checked against a certificate: it is empty or not a name.
    BadHostName {
        /// The name, as given.
        host: String,
    },
    /// The server's certificate is not one the system trusts, or not one for the host reached.
    Untrusted {
        /// The host reached.
        host: String,
        /// Why, as the library said it.
        reason: String,
    },
    /// The server and Gitardo could not agree on a secure connection.
    Handshake {
        /// The host reached.
        host: String,
        /// Why, as the library said it.
        reason: String,
    },
    /// The connection failed while it was being made secure.
    Io {
        /// The host reached.
        host: String,
        /// What the system said.
        source: std::io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoRootCertificates { reason } => write!(
                formatter,
                "The system gave no root certificates, so no server can be trusted. {reason}"
            ),
            Error::BadCertificates { reason } => write!(formatter, "The certificates could not be read: {reason}"),
            Error::BadHostName { host } => write!(formatter, "\"{host}\" is not the name of a host."),
            Error::Untrusted { host, reason } => write!(
                formatter,
                "The certificate of {host} is not trusted by this system ({reason}). Check the date of the computer, \
                 or ask whoever runs the network whether a proxy replaces certificates."
            ),
            Error::Handshake { host, reason } => {
                write!(formatter, "Could not make a secure connection to {host}: {reason}")
            }
            Error::Io { host, source } => write!(formatter, "The connection to {host} failed: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::NoRootCertificates { .. }
            | Error::BadCertificates { .. }
            | Error::BadHostName { .. }
            | Error::Untrusted { .. }
            | Error::Handshake { .. } => None,
        }
    }
}
