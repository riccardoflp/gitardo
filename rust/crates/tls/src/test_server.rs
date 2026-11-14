//! The server side of TLS, with the certificate of the tests, for the tests of this crate and of those above it: a
//! server on 127.0.0.1 that speaks HTTPS without reaching the network. Built only with the feature `test-server`.
//!
//! The certificate is for `localhost`, `127.0.0.1`, `api.github.com` and `example.test`, signed by
//! [`AUTHORITY`], a certificate authority made for these tests only and trusted nowhere else.

use std::io::{Read, Write};
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

use crate::error::Error;
use crate::stream::failure;

/// The certificate authority of the tests, as a PEM file: what a [`crate::Trust::from_pem`] that trusts the test
/// server is made of.
pub const AUTHORITY: &[u8] = include_bytes!("../fixtures/authority.pem");

/// Another certificate authority, which did not sign the test server's certificate.
pub const OTHER_AUTHORITY: &[u8] = include_bytes!("../fixtures/other-authority.pem");

const CERTIFICATE: &[u8] = include_bytes!("../fixtures/server.pem");
const KEY: &[u8] = include_bytes!("../fixtures/server.key");

/// A connection a test server accepted, made secure: it reads what the client sent and writes what it answers, and
/// says it is closing when dropped.
pub struct ServerStream<S: Read + Write> {
    inner: StreamOwned<ServerConnection, S>,
}

/// Makes a connection the test server accepted secure, with the certificate of the tests: a TCP connection, or the
/// tunnel a proxy of the tests opened.
///
/// # Errors
///
/// When the client gives up, as one that does not trust the certificate does.
pub fn accept<S: Read + Write>(tcp: S) -> Result<ServerStream<S>, Error> {
    let host = String::from("the client");
    let handshake = |reason: String| Error::Handshake {
        host: host.clone(),
        reason,
    };
    let certificates = CertificateDer::pem_slice_iter(CERTIFICATE)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| handshake(error.to_string()))?;
    let key = PrivateKeyDer::from_pem_slice(KEY).map_err(|error| handshake(error.to_string()))?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| handshake(error.to_string()))?
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map_err(|error| handshake(error.to_string()))?;
    let mut connection = ServerConnection::new(Arc::new(config)).map_err(|error| handshake(error.to_string()))?;
    let mut tcp = tcp;
    while connection.is_handshaking() {
        let _exchanged = connection
            .complete_io(&mut tcp)
            .map_err(|error| failure(&host, error))?;
    }
    Ok(ServerStream {
        inner: StreamOwned::new(connection, tcp),
    })
}

impl<S: Read + Write> Read for ServerStream<S> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buffer)
    }
}

impl<S: Read + Write> Write for ServerStream<S> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<S: Read + Write> Drop for ServerStream<S> {
    fn drop(&mut self) {
        self.inner.conn.send_close_notify();
        // The client may be gone already; a test server has nobody to tell.
        let _ = self.inner.conn.complete_io(&mut self.inner.sock);
    }
}
