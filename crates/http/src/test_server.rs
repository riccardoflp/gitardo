//! A server on 127.0.0.1 for the tests of this crate and of those that send requests with it: it answers each
//! connection with the next of the answers it was given, in the clear or over TLS with the certificate of the tests,
//! and keeps what it received. Nothing reaches the network. Built only with the feature `test-server`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use tls::Trust;

/// What the server does with a connection.
pub enum Answer {
    /// Reads the request, writes these bytes and closes the connection.
    Bytes(Vec<u8>),
    /// Reads the request, writes the first bytes, waits until the test says to go on, then writes the others.
    Parts(Vec<u8>, Receiver<()>, Vec<u8>),
    /// Reads the request and answers nothing, until the server is dropped.
    Silence,
    /// As a proxy: reads a `CONNECT`, answers `200`, then answers through the tunnel over TLS as the host would.
    Tunnel(Box<Answer>),
}

/// A request the server received: its head as sent, and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    /// The request line and the headers, each line ending with `\r\n`.
    pub head: String,
    /// The body, as long as its `Content-Length` says.
    pub body: Vec<u8>,
}

impl Received {
    /// The value of a header, compared without regard to case.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<String> {
        self.head.lines().skip(1).find_map(|line| {
            let (found, value) = line.split_once(':')?;
            found.eq_ignore_ascii_case(name).then(|| value.trim().to_owned())
        })
    }

    /// The request line: `GET /path HTTP/1.1`.
    #[must_use]
    pub fn request_line(&self) -> String {
        self.head.lines().next().unwrap_or_default().to_owned()
    }
}

/// A server of the tests, listening on 127.0.0.1 on a port of its own.
pub struct TestServer {
    address: SocketAddr,
    received: Arc<Mutex<Vec<Received>>>,
    is_stopped: Arc<AtomicBool>,
}

/// The connections a server answers, and what it keeps.
struct Serving {
    received: Arc<Mutex<Vec<Received>>>,
    is_stopped: Arc<AtomicBool>,
    is_secure: bool,
}

impl TestServer {
    /// A server that speaks HTTP in the clear, answering each connection with the next answer.
    ///
    /// # Panics
    ///
    /// When no port can be had on 127.0.0.1.
    #[must_use]
    pub fn plain(answers: Vec<Answer>) -> TestServer {
        TestServer::start(answers, false)
    }

    /// A server that speaks HTTPS, with the certificate of the tests, for `localhost`, `127.0.0.1`, `api.github.com`
    /// and `example.test`.
    ///
    /// # Panics
    ///
    /// When no port can be had on 127.0.0.1.
    #[must_use]
    pub fn secure(answers: Vec<Answer>) -> TestServer {
        TestServer::start(answers, true)
    }

    #[allow(
        clippy::expect_used,
        reason = "a test server that cannot listen leaves the test nothing to do"
    )]
    fn start(answers: Vec<Answer>, is_secure: bool) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let address = listener.local_addr().expect("the address listened on");
        let serving = Serving {
            received: Arc::default(),
            is_stopped: Arc::default(),
            is_secure,
        };
        let server = TestServer {
            address,
            received: Arc::clone(&serving.received),
            is_stopped: Arc::clone(&serving.is_stopped),
        };
        let _thread = thread::spawn(move || {
            for answer in answers {
                let Ok((tcp, _)) = listener.accept() else {
                    return;
                };
                serving.serve(tcp, answer);
            }
        });
        server
    }

    /// The port it listens on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.address.port()
    }

    /// The address it listens on.
    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// The requests received so far, in order; for a tunnel, the `CONNECT` then the request through it.
    #[must_use]
    pub fn received(&self) -> Vec<Received> {
        self.received.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The roots that trust the servers of the tests, and no other.
    ///
    /// # Panics
    ///
    /// Never: the certificate of the tests is part of the crate `tls`.
    #[must_use]
    #[allow(clippy::expect_used, reason = "the certificate of the tests is part of the crate")]
    pub fn trust() -> Trust {
        Trust::from_pem(tls::test_server::AUTHORITY).expect("the authority of the tests")
    }

    /// Roots that do not trust the servers of the tests.
    ///
    /// # Panics
    ///
    /// Never: the certificate is part of the crate `tls`.
    #[must_use]
    #[allow(clippy::expect_used, reason = "the certificate of the tests is part of the crate")]
    pub fn other_trust() -> Trust {
        Trust::from_pem(tls::test_server::OTHER_AUTHORITY).expect("another authority")
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.is_stopped.store(true, Ordering::SeqCst);
    }
}

impl Serving {
    fn serve(&self, mut tcp: TcpStream, answer: Answer) {
        match answer {
            Answer::Tunnel(inner) => {
                let Some(request) = read_request(&mut tcp) else {
                    return;
                };
                self.received
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(request);
                if tcp
                    .write_all(
                        b"HTTP/1.1 200 Connection established

",
                    )
                    .is_ok()
                    && let Ok(secure) = tls::test_server::accept(tcp)
                {
                    self.answer(secure, *inner);
                }
            }
            // A client that does not trust the certificate gives up in the handshake; the test checks what it says.
            answer if self.is_secure => {
                if let Ok(stream) = tls::test_server::accept(tcp) {
                    self.answer(stream, answer);
                }
            }
            answer => self.answer(tcp, answer),
        }
    }

    fn answer<S: Read + Write>(&self, mut stream: S, answer: Answer) {
        let Some(request) = read_request(&mut stream) else {
            return;
        };
        self.received
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request);
        // The client may be gone when the server writes, as one that was cancelled is: there is nobody to tell.
        match answer {
            Answer::Bytes(bytes) => {
                let _ = stream.write_all(&bytes).and_then(|()| stream.flush());
            }
            Answer::Parts(first, go_on, rest) => {
                let _ = stream.write_all(&first).and_then(|()| stream.flush());
                let _ = go_on.recv_timeout(Duration::from_secs(30));
                let _ = stream.write_all(&rest).and_then(|()| stream.flush());
            }
            Answer::Silence => {
                let started = Instant::now();
                while !self.is_stopped.load(Ordering::SeqCst) && started.elapsed() < Duration::from_secs(30) {
                    thread::sleep(Duration::from_millis(10));
                }
            }
            // A tunnel is opened by the server itself, before any answer through it.
            Answer::Tunnel(_) => {}
        }
    }
}

/// The head of a request, read a byte at a time up to its empty line, then its body.
fn read_request(stream: &mut impl Read) -> Option<Received> {
    let mut head = Vec::new();
    let mut byte = [0_u8];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        head.extend_from_slice(&byte);
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let mut received = Received { head, body: Vec::new() };
    let length = received
        .header("content-length")
        .and_then(|length| length.parse::<usize>().ok())
        .unwrap_or_default();
    received.body = vec![0; length];
    stream.read_exact(&mut received.body).ok()?;
    Some(received)
}

/// An answer in full: its status line, its headers and its body, with its `Content-Length`.
#[must_use]
pub fn answer(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut bytes = format!("HTTP/1.1 {status}\r\n").into_bytes();
    for (name, value) in headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    bytes.extend_from_slice(body);
    bytes
}
