//! A TLS stream over a TCP connection: what is written is sent encrypted, what is read arrives decrypted.

use std::io::{Read, Write};

use rustls::pki_types::ServerName;
use rustls::{ClientConnection, StreamOwned};

use crate::error::Error;
use crate::trust::Trust;

/// A connection to a server made secure: its certificate was checked against the roots of a [`Trust`] and the name of
/// the host reached. It reads and writes like the connection under it, a TCP connection or what wraps one, whose
/// timeouts it keeps.
pub struct TlsStream<S: Read + Write> {
    inner: StreamOwned<ClientConnection, S>,
}

impl<S: Read + Write> std::fmt::Debug for TlsStream<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TlsStream")
    }
}

impl<S: Read + Write> TlsStream<S> {
    /// Makes a connection to a host secure, and checks its certificate before anything is sent: a server that is
    /// not trusted is an error here, never later. The connection may go through a proxy that a `CONNECT` opened; the
    /// host is the one at the end of it.
    ///
    /// # Errors
    ///
    /// When the host is not a name, its certificate is not trusted, the server speaks no TLS that Gitardo speaks, or
    /// the connection fails or times out.
    pub fn connect(trust: &Trust, host: &str, tcp: S) -> Result<TlsStream<S>, Error> {
        let name = ServerName::try_from(host.to_owned()).map_err(|_| Error::BadHostName { host: host.to_owned() })?;
        let mut connection = ClientConnection::new(trust.config.clone(), name).map_err(|error| Error::Handshake {
            host: host.to_owned(),
            reason: error.to_string(),
        })?;
        let mut tcp = tcp;
        while connection.is_handshaking() {
            let _exchanged = connection.complete_io(&mut tcp).map_err(|error| failure(host, error))?;
        }
        Ok(TlsStream {
            inner: StreamOwned::new(connection, tcp),
        })
    }
}

impl<S: Read + Write> Read for TlsStream<S> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buffer)
    }
}

impl<S: Read + Write> Write for TlsStream<S> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// What a failure during the handshake was: a certificate not trusted, a disagreement, or the connection itself.
pub(crate) fn failure(host: &str, error: std::io::Error) -> Error {
    let library = error.get_ref().and_then(|inner| inner.downcast_ref::<rustls::Error>());
    match library {
        Some(rustls::Error::InvalidCertificate(reason)) => Error::Untrusted {
            host: host.to_owned(),
            reason: format!("{reason:?}"),
        },
        Some(other) => Error::Handshake {
            host: host.to_owned(),
            reason: other.to_string(),
        },
        None => Error::Io {
            host: host.to_owned(),
            source: error,
        },
    }
}
