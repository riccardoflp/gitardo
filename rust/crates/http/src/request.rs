//! A request: its method, its address, its headers and its body.

use std::fmt::{self, Write as _};
use std::time::Duration;

use crate::url::Url;

/// The methods Gitardo sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `GET`: read something.
    Get,
    /// `POST`: send something, and read the answer.
    Post,
}

impl Method {
    /// The word of the request line.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

/// A header of a request. A secret header (a token, a key) is sent like the others but never shown: not in `Debug`,
/// not in an error, not in the log.
#[derive(Clone, PartialEq, Eq)]
pub struct Header {
    /// Its name: `Content-Type`.
    pub name: String,
    /// Its value.
    pub value: String,
    /// Whether its value is a secret.
    pub is_secret: bool,
}

impl fmt::Debug for Header {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_secret {
            write!(formatter, "{}: <hidden>", self.name)
        } else {
            write!(formatter, "{}: {}", self.name, self.value)
        }
    }
}

/// How long a request waits before it gives up, when nothing comes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// To make the connection: 10 seconds.
    pub connect: Duration,
    /// Between two reads of the answer, or two writes of the request: 60 seconds, 300 while a model streams.
    pub idle: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Timeouts {
            connect: Duration::from_secs(10),
            idle: Duration::from_secs(60),
        }
    }
}

/// A request to send.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    /// Its method.
    pub method: Method,
    /// Where it goes.
    pub url: Url,
    /// Its headers, in order. `Host`, `Content-Length` and `Connection` are added when it is sent.
    pub headers: Vec<Header>,
    /// Its body; empty for none.
    pub body: Vec<u8>,
    /// How long it waits.
    pub timeouts: Timeouts,
}

impl fmt::Debug for Request {
    /// The request without its body and the values of its secret headers.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Request")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("body", &format_args!("{} bytes", self.body.len()))
            .finish_non_exhaustive()
    }
}

impl Request {
    /// A `GET` of an address.
    #[must_use]
    pub fn get(url: Url) -> Request {
        Request {
            method: Method::Get,
            url,
            headers: Vec::new(),
            body: Vec::new(),
            timeouts: Timeouts::default(),
        }
    }

    /// A `POST` of a body to an address.
    #[must_use]
    pub fn post(url: Url, body: Vec<u8>) -> Request {
        Request {
            method: Method::Post,
            body,
            ..Request::get(url)
        }
    }

    /// The same request with a header more.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Request {
        self.headers.push(Header {
            name: name.to_owned(),
            value: value.to_owned(),
            is_secret: false,
        });
        self
    }

    /// The same request with a header more whose value is a secret: `Authorization: Bearer <token>`.
    #[must_use]
    pub fn secret_header(mut self, name: &str, value: &str) -> Request {
        self.headers.push(Header {
            name: name.to_owned(),
            value: value.to_owned(),
            is_secret: true,
        });
        self
    }

    /// The same request waiting longer, or less, between two reads: 300 seconds while a model streams.
    #[must_use]
    pub fn idle_timeout(mut self, idle: Duration) -> Request {
        self.timeouts.idle = idle;
        self
    }

    /// The head of the request as it is sent straight to its host: the request line, `Host`, its headers,
    /// `Content-Length` when it has a body or is a `POST`, and `Connection: close`, one connection per request.
    pub(crate) fn head(&self) -> Vec<u8> {
        let mut head = String::new();
        let _ = write!(
            head,
            "{} {} HTTP/1.1\r\nHost: {}\r\n",
            self.method.word(),
            self.url.target(),
            self.url.authority()
        );
        for header in &self.headers {
            // A line break in a value would end the header there and start another, which nobody wrote.
            let _ = write!(head, "{}: {}\r\n", one_line(&header.name), one_line(&header.value));
        }
        if !self.body.is_empty() || self.method == Method::Post {
            let _ = write!(head, "Content-Length: {}\r\n", self.body.len());
        }
        head.push_str("Connection: close\r\n\r\n");
        head.into_bytes()
    }
}

/// A text with its line breaks turned into spaces.
pub(crate) fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}
