//! Sending a request and reading its answer, following redirects.

use std::io::{BufReader, Read, Write};
use std::net::SocketAddr;

use tls::Trust;

use crate::body::{Framed, framing};
use crate::cancel::Cancel;
use crate::connection::{Route, Stream, connect, io_error, settled};
use crate::error::Error;
use crate::proxy::Proxies;
use crate::request::{Method, Request};
use crate::response::{Head, HeadError, Response, read_head};
use crate::url::Scheme;

/// How many redirects a request follows before it gives up.
const MOST_REDIRECTS: usize = 5;

/// What sends requests: with the root certificates it trusts and the proxies it goes through. One connection per
/// request, closed once its answer is read.
#[derive(Debug, Clone)]
pub struct Http {
    trust: Trust,
    proxies: Proxies,
    addresses: Vec<(String, u16, SocketAddr)>,
}

impl Http {
    /// Sends requests trusting the servers whose certificates lead to `trust`, through the proxies named.
    #[must_use]
    pub fn new(trust: Trust, proxies: Proxies) -> Http {
        Http {
            trust,
            proxies,
            addresses: Vec::new(),
        }
    }

    /// The same, reaching a host and port at an address given rather than at those its name resolves to: a server of
    /// the tests on 127.0.0.1 that answers for `api.github.com`. The name is still the one the certificate is checked
    /// against and the `Host` header says.
    #[must_use]
    pub fn with_address(mut self, host: &str, port: u16, address: SocketAddr) -> Http {
        self.addresses.push((host.to_ascii_lowercase(), port, address));
        self
    }

    /// Sends a request and reads its answer to its end. An answer that is not 2xx is still an answer: what it means is
    /// the caller's to say.
    ///
    /// # Errors
    ///
    /// When the request cannot be sent or its answer read, as [`Error`] lists; [`Error::Cancelled`] once `cancel` is.
    pub fn send(&self, request: &Request, cancel: &Cancel) -> Result<Response, Error> {
        let mut opened = self.open(request, cancel)?;
        let body = opened.body.read_all()?;
        Ok(Response {
            head: opened.head,
            body,
        })
    }

    /// Sends a request and gives its answer as soon as its head is read, with its body to read as it arrives: the
    /// server-sent events of a model that streams its answer (see [`crate::Events`]). Redirects are followed up to five
    /// times, to `https` only; a `303`, or a `301` or `302` of a `POST`, is followed with a `GET` without the body, and
    /// the secret headers are not sent on to another host.
    ///
    /// # Errors
    ///
    /// As [`Http::send`].
    pub fn open(&self, request: &Request, cancel: &Cancel) -> Result<Opened, Error> {
        let mut request = request.clone();
        let mut redirects = 0;
        loop {
            let opened = self.open_once(&request, cancel)?;
            let location = opened
                .head
                .header("location")
                .filter(|_| is_redirect(opened.head.status));
            let Some(location) = location else {
                return Ok(opened);
            };
            let host = request.url.host().to_owned();
            if redirects == MOST_REDIRECTS {
                return Err(Error::TooManyRedirects { host });
            }
            let next = request.url.join(location)?;
            if next.scheme() != Scheme::Https {
                return Err(Error::InsecureRedirect { host });
            }
            request = redirected(request, next, opened.head.status);
            redirects += 1;
        }
    }

    fn open_once(&self, request: &Request, cancel: &Cancel) -> Result<Opened, Error> {
        let route = Route {
            trust: &self.trust,
            proxies: &self.proxies,
            addresses: &self.addresses,
        };
        let host = request.url.host().to_owned();
        let mut stream = connect(&route, &request.url, request.timeouts, cancel)?;
        let sent = stream
            .write_all(&request.head())
            .and_then(|()| stream.write_all(&request.body))
            .and_then(|()| stream.flush());
        sent.map_err(|source| settled(io_error(&host, source), cancel))?;
        let mut reader = BufReader::new(stream);
        let head = loop {
            let head = read_head(&mut reader).map_err(|error| match error {
                HeadError::Io(source) => settled(io_error(&host, source), cancel),
                HeadError::Bad(reason) => Error::BadResponse {
                    host: host.clone(),
                    reason,
                },
            })?;
            // A `100 Continue`, or another answer that says more is coming, is not the answer.
            if !(100..200).contains(&head.status) {
                break head;
            }
        };
        let framing = framing(&head, true);
        Ok(Opened {
            head,
            body: Body {
                framed: Framed::new(reader, framing),
                cancel: cancel.clone(),
                host,
            },
        })
    }
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// The request a redirect asks for, to another address.
fn redirected(mut request: Request, next: crate::url::Url, status: u16) -> Request {
    if status == 303 || (matches!(status, 301 | 302) && request.method == Method::Post) {
        request.method = Method::Get;
        request.body.clear();
        request
            .headers
            .retain(|header| !header.name.eq_ignore_ascii_case("content-type"));
    }
    if !next.host().eq_ignore_ascii_case(request.url.host()) {
        request.headers.retain(|header| !header.is_secret);
    }
    request.url = next;
    request
}

/// An answer whose head is read and whose body is still to read.
#[derive(Debug)]
pub struct Opened {
    /// Its status and headers.
    pub head: Head,
    /// Its body, to read as it arrives.
    pub body: Body,
}

/// The body of an answer, read as it arrives, its chunks joined. Its connection is closed when it is dropped.
pub struct Body {
    framed: Framed<BufReader<Stream>>,
    cancel: Cancel,
    host: String,
}

impl std::fmt::Debug for Body {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Body")
            .field("host", &self.host)
            .finish_non_exhaustive()
    }
}

impl Body {
    /// Reads what is left of the body.
    ///
    /// # Errors
    ///
    /// When the connection fails or times out, the body is not framed as its head says, or the request is cancelled.
    pub fn read_all(&mut self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        let _length = self
            .framed
            .read_to_end(&mut bytes)
            .map_err(|source| self.error(source))?;
        Ok(bytes)
    }

    /// What a failure while reading the body is, in the words of [`Error`]: what [`Read`] gives as an
    /// [`std::io::Error`] is said this way.
    #[must_use]
    pub fn error(&self, source: std::io::Error) -> Error {
        if source.kind() == std::io::ErrorKind::InvalidData {
            return Error::BadResponse {
                host: self.host.clone(),
                reason: source.to_string(),
            };
        }
        settled(io_error(&self.host, source), &self.cancel)
    }
}

impl Read for Body {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.framed.read(buffer);
        if self.cancel.is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                Error::Cancelled,
            ));
        }
        read
    }
}
