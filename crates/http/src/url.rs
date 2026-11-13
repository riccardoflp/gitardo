//! The addresses a request is sent to: `https://api.github.com/graphql`.

use std::fmt;

use crate::error::Error;

/// Whether a connection is made secure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// `http`: in the clear. Used only for a proxy, and by tests on 127.0.0.1.
    Http,
    /// `https`: through TLS.
    Https,
}

impl Scheme {
    /// The port of the scheme when an address names none.
    #[must_use]
    pub fn default_port(self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
        }
    }
}

/// An address: its scheme, its host and port, and what is asked of the host (the path and the query).
#[derive(Clone, PartialEq, Eq)]
pub struct Url {
    scheme: Scheme,
    host: String,
    port: u16,
    target: String,
    user: Option<UserInfo>,
}

/// The user and the password an address carries before its host, decoded: only a proxy's address may.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct UserInfo {
    pub(crate) user: String,
    pub(crate) password: String,
}

impl fmt::Debug for Url {
    /// The address without its user and password, which may be secrets.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}://{}{}",
            self.scheme.name(),
            self.authority(),
            self.target
        )
    }
}

impl fmt::Display for Url {
    /// The address without its user and password, which may be secrets.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, formatter)
    }
}

impl Url {
    /// Reads an address: `http` or `https`, a host, a port when it is not the scheme's, then the path and the query;
    /// a fragment is left out. A user and a password before the host are kept apart, for a proxy.
    ///
    /// # Errors
    ///
    /// When the scheme is neither, the host is missing or the port is not a number.
    pub fn parse(text: &str) -> Result<Url, Error> {
        let text = text.trim();
        let bad = |reason: &str| Error::BadUrl {
            reason: reason.to_owned(),
        };
        let (scheme, rest) = text
            .split_once("://")
            .ok_or_else(|| bad("it does not start with http:// or https://"))?;
        let scheme = if scheme.eq_ignore_ascii_case("https") {
            Scheme::Https
        } else if scheme.eq_ignore_ascii_case("http") {
            Scheme::Http
        } else {
            return Err(bad("only http and https are spoken"));
        };
        let rest = rest.split_once('#').map_or(rest, |(before, _)| before);
        let split = rest.find(['/', '?']).unwrap_or(rest.len());
        let (authority, target) = rest.split_at(split);
        let (user, host_and_port) = match authority.rsplit_once('@') {
            Some((user, host)) => (Some(user_info(user)), host),
            None => (None, authority),
        };
        let (host, port) =
            host_and_port_of(host_and_port, scheme).ok_or_else(|| bad("it has no host, or a bad port"))?;
        let target = if target.starts_with('/') {
            target.to_owned()
        } else {
            format!("/{target}")
        };
        Ok(Url {
            scheme,
            host,
            port,
            target,
            user,
        })
    }

    /// Its scheme.
    #[must_use]
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// Its host, without the brackets of an IPv6 address: `api.github.com`.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Its port, the scheme's when it names none.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// What is asked of the host: the path and the query, `/graphql`.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The host as the `Host` header says it, with the port when it is not the scheme's.
    #[must_use]
    pub fn authority(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        if self.port == self.scheme.default_port() {
            host
        } else {
            format!("{host}:{}", self.port)
        }
    }

    pub(crate) fn user(&self) -> Option<&UserInfo> {
        self.user.as_ref()
    }

    /// Where a redirect sends a request sent here: an address in full, one without its scheme, a path from the top of
    /// the host, or a path beside this one.
    ///
    /// # Errors
    ///
    /// When the address in full cannot be read.
    pub fn join(&self, location: &str) -> Result<Url, Error> {
        let location = location.trim();
        if location.contains("://") {
            return Url::parse(location);
        }
        if let Some(rest) = location.strip_prefix("//") {
            return Url::parse(&format!("{}://{rest}", self.scheme.name()));
        }
        let target = if location.starts_with('/') {
            location.to_owned()
        } else {
            let path = self.target.split('?').next().unwrap_or_default();
            let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
            format!("{folder}/{location}")
        };
        Ok(Url {
            target,
            user: None,
            ..self.clone()
        })
    }
}

/// The host and the port of an authority: `example.test:8080`, `[::1]:8080`, `example.test`.
fn host_and_port_of(text: &str, scheme: Scheme) -> Option<(String, u16)> {
    let (host, port) = if let Some(rest) = text.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        (host, after.strip_prefix(':'))
    } else {
        match text.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (text, None),
        }
    };
    if host.is_empty() {
        return None;
    }
    let port = match port {
        Some(port) => port.parse().ok()?,
        None => scheme.default_port(),
    };
    Some((host.to_ascii_lowercase(), port))
}

fn user_info(text: &str) -> UserInfo {
    let (user, password) = text.split_once(':').unwrap_or((text, ""));
    UserInfo {
        user: percent_decoded(user),
        password: percent_decoded(password),
    }
}

/// A part of an address with its `%xx` turned back into the bytes they stand for.
fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let escaped = (byte == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        if let Some(value) = escaped {
            decoded.push(value);
            index += 3;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{Scheme, Url};

    #[test]
    fn an_address_is_read_into_its_parts() {
        let url = Url::parse("https://api.github.com/graphql").unwrap();
        assert_eq!(url.scheme(), Scheme::Https);
        assert_eq!(url.host(), "api.github.com");
        assert_eq!(url.port(), 443);
        assert_eq!(url.target(), "/graphql");
        assert_eq!(url.authority(), "api.github.com");
    }

    #[test]
    fn a_port_a_query_and_a_fragment() {
        let url = Url::parse("HTTP://Example.Test:8080?q=1#top").unwrap();
        assert_eq!(url.scheme(), Scheme::Http);
        assert_eq!(url.host(), "example.test");
        assert_eq!(url.target(), "/?q=1");
        assert_eq!(url.authority(), "example.test:8080");
        let url = Url::parse("http://[::1]:3128/").unwrap();
        assert_eq!((url.host(), url.port()), ("::1", 3128));
        assert_eq!(url.authority(), "[::1]:3128");
    }

    #[test]
    fn a_user_and_a_password_are_kept_apart_and_never_shown() {
        let url = Url::parse("http://ada%40work:p%3Ass@proxy.test:3128").unwrap();
        let user = url.user().unwrap();
        assert_eq!((user.user.as_str(), user.password.as_str()), ("ada@work", "p:ss"));
        assert_eq!(url.to_string(), "http://proxy.test:3128/");
        assert_eq!(format!("{url:?}"), "http://proxy.test:3128/");
    }

    #[test]
    fn what_is_not_an_address_is_refused() {
        for text in [
            "ftp://example.test/",
            "example.test",
            "https://",
            "https://host:port/",
            "https://:443/",
        ] {
            assert!(Url::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn a_redirect_is_read_against_the_address_it_came_from() {
        let base = Url::parse("https://example.test/a/b?x=1").unwrap();
        assert_eq!(base.join("/c").unwrap().to_string(), "https://example.test/c");
        assert_eq!(base.join("c").unwrap().to_string(), "https://example.test/a/c");
        assert_eq!(base.join("//other.test/d").unwrap().to_string(), "https://other.test/d");
        assert_eq!(base.join("http://plain.test/").unwrap().scheme(), Scheme::Http);
    }
}
