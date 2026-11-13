//! Which servers a stream trusts: those whose certificate leads to one of a set of root certificates.

use std::sync::Arc;

use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use rustls::{ClientConfig, RootCertStore};

use crate::error::Error;

/// The root certificates a stream checks the server's certificate against, with the rest of what a connection needs:
/// TLS 1.2 or 1.3, with the cryptography of `ring`. Made once and shared by every connection.
#[derive(Clone)]
pub struct Trust {
    pub(crate) config: Arc<ClientConfig>,
}

impl std::fmt::Debug for Trust {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Trust")
    }
}

impl Trust {
    /// The root certificates the system trusts: the certificate store of Windows, the keychain of macOS, the
    /// certificate files of the Linux distribution, so that a company's own certificate authority is trusted as it is
    /// by the browser and by git. A certificate the system holds that cannot be read is left out.
    ///
    /// # Errors
    ///
    /// When the system gives no certificate that can be used.
    pub fn system() -> Result<Trust, Error> {
        let found = rustls_native_certs::load_native_certs();
        let mut roots = RootCertStore::empty();
        let (_added, _left_out) = roots.add_parsable_certificates(found.certs);
        if roots.is_empty() {
            let reason = found
                .errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Error::NoRootCertificates { reason });
        }
        Trust::with_roots(roots)
    }

    /// Only the root certificates of a PEM file's text, and none of the system's: for a test server, or a
    /// certificate authority the user names.
    ///
    /// # Errors
    ///
    /// When the text holds no certificate, or one that cannot be read.
    pub fn from_pem(pem: &[u8]) -> Result<Trust, Error> {
        let mut roots = RootCertStore::empty();
        for certificate in CertificateDer::pem_slice_iter(pem) {
            let certificate = certificate.map_err(|error| Error::BadCertificates {
                reason: error.to_string(),
            })?;
            roots.add(certificate).map_err(|error| Error::BadCertificates {
                reason: error.to_string(),
            })?;
        }
        if roots.is_empty() {
            return Err(Error::BadCertificates {
                reason: String::from("there is no certificate in it"),
            });
        }
        Trust::with_roots(roots)
    }

    fn with_roots(roots: RootCertStore) -> Result<Trust, Error> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|error| Error::Handshake {
                host: String::new(),
                reason: error.to_string(),
            })?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Trust {
            config: Arc::new(config),
        })
    }
}
