//! Making the connection of a request: to its host or through a proxy with `CONNECT`, then through TLS for `https`.

use std::io::{BufReader, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use tls::{TlsStream, Trust};

use crate::base64;
use crate::cancel::{Cancel, Patient};
use crate::error::Error;
use crate::proxy::Proxies;
use crate::request::{Timeouts, one_line};
use crate::response::{HeadError, read_head};
use crate::url::{Scheme, Url};

/// A connection a request is sent on: in the clear, or through TLS.
pub(crate) enum Stream {
    Plain(Patient),
    Tls(Box<TlsStream<Patient>>),
}

impl Read for Stream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(stream) => stream.read(buffer),
            Stream::Tls(stream) => stream.read(buffer),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(stream) => stream.write(bytes),
            Stream::Tls(stream) => stream.write(bytes),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Plain(stream) => stream.flush(),
            Stream::Tls(stream) => stream.flush(),
        }
    }
}

/// What a connection is made with: the roots to trust, the proxies, and the addresses given for some hosts.
pub(crate) struct Route<'a> {
    pub(crate) trust: &'a Trust,
    pub(crate) proxies: &'a Proxies,
    pub(crate) addresses: &'a [(String, u16, SocketAddr)],
}

/// Connects to the host of an address, through the proxy that the proxies name for it, and makes the connection
/// secure for `https`.
pub(crate) fn connect(route: &Route<'_>, url: &Url, timeouts: Timeouts, cancel: &Cancel) -> Result<Stream, Error> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let proxy = route.proxies.proxy_for(url);
    let (host, port) = proxy.map_or((url.host(), url.port()), |proxy| (proxy.host(), proxy.port()));
    let tcp = dial(route.addresses, host, port, timeouts)?;
    let failed = |source: std::io::Error| Error::Io {
        host: host.to_owned(),
        source,
    };
    let mut tcp = Patient::new(tcp, cancel, timeouts.idle).map_err(failed)?;
    if let Some(proxy) = proxy {
        tunnel(&mut tcp, proxy, url).map_err(|error| settled(error, cancel))?;
    }
    let stream = match url.scheme() {
        Scheme::Http => Stream::Plain(tcp),
        Scheme::Https => match TlsStream::connect(route.trust, url.host(), tcp) {
            Ok(stream) => Stream::Tls(Box::new(stream)),
            Err(tls::Error::Io { source, .. }) => return Err(settled(io_error(url.host(), source), cancel)),
            Err(error) => return Err(Error::Tls(error)),
        },
    };
    Ok(stream)
}

/// A TCP connection to a host, at the address given for it or at those its name resolves to, the first that answers.
fn dial(
    addresses: &[(String, u16, SocketAddr)],
    host: &str,
    port: u16,
    timeouts: Timeouts,
) -> Result<TcpStream, Error> {
    let given: Vec<SocketAddr> = addresses
        .iter()
        .filter(|(name, given_port, _)| name.eq_ignore_ascii_case(host) && *given_port == port)
        .map(|(_, _, address)| *address)
        .collect();
    let resolved = if given.is_empty() {
        (host, port)
            .to_socket_addrs()
            .map_err(|source| Error::Resolve {
                host: host.to_owned(),
                source,
            })?
            .collect()
    } else {
        given
    };
    let mut last = None;
    for address in resolved {
        match TcpStream::connect_timeout(&address, timeouts.connect) {
            Ok(tcp) => return Ok(tcp),
            Err(error) => last = Some(error),
        }
    }
    let source = last.unwrap_or_else(|| ErrorKind::AddrNotAvailable.into());
    Err(if is_timeout(&source) {
        Error::TimedOut { host: host.to_owned() }
    } else {
        Error::Connect {
            host: host.to_owned(),
            source,
        }
    })
}

/// Asks the proxy for a tunnel to the host of the address: `CONNECT host:port`, with basic authentication when the
/// proxy's address has a user.
fn tunnel(tcp: &mut Patient, proxy: &Url, url: &Url) -> Result<(), Error> {
    let proxy_name = proxy.authority();
    let target = format!("{}:{}", bracketed(url.host()), url.port());
    let mut head = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
    if let Some(user) = proxy.user() {
        let credentials = base64::encode(format!("{}:{}", user.user, user.password).as_bytes());
        head.push_str("Proxy-Authorization: Basic ");
        head.push_str(&one_line(&credentials));
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    tcp.write_all(head.as_bytes())
        .and_then(|()| tcp.flush())
        .map_err(|source| io_error(&proxy_name, source))?;
    // One byte at a time, so that nothing the host sends through the tunnel is read as part of the proxy's answer.
    let mut reader = BufReader::with_capacity(1, &mut *tcp);
    let answer = read_head(&mut reader).map_err(|error| match error {
        HeadError::Io(source) => io_error(&proxy_name, source),
        HeadError::Bad(reason) => Error::BadResponse {
            host: proxy_name.clone(),
            reason,
        },
    })?;
    if answer.is_success() {
        Ok(())
    } else {
        Err(Error::Proxy {
            proxy: proxy_name,
            status: answer.status_line(),
        })
    }
}

fn bracketed(host: &str) -> String {
    if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    }
}

/// What a failure of the connection is: a timeout, or the connection failing.
pub(crate) fn io_error(host: &str, source: std::io::Error) -> Error {
    if is_timeout(&source) {
        Error::TimedOut { host: host.to_owned() }
    } else {
        Error::Io {
            host: host.to_owned(),
            source,
        }
    }
}

/// A failure that came of the request being cancelled is said as that.
pub(crate) fn settled(error: Error, cancel: &Cancel) -> Error {
    if cancel.is_cancelled() { Error::Cancelled } else { error }
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}
