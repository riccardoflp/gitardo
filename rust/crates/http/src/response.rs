//! An answer: its status, its headers and its body.

use std::io::BufRead;

/// The most bytes the head of an answer may have: far more than any server sends.
const MOST_HEAD_BYTES: usize = 64 * 1024;

/// The status line and the headers of an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    /// The status: `200`.
    pub status: u16,
    /// Its reason, as the server wrote it: `OK`.
    pub reason: String,
    /// The headers, in order, with their names as sent.
    pub headers: Vec<(String, String)>,
}

impl Head {
    /// The value of the first header of a name, compared without regard to case.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(found, _)| found.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Whether the status is 2xx.
    #[must_use]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The status with its reason: `403 Forbidden`.
    #[must_use]
    pub fn status_line(&self) -> String {
        if self.reason.is_empty() {
            self.status.to_string()
        } else {
            format!("{} {}", self.status, self.reason)
        }
    }
}

/// An answer read to its end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Its status and headers.
    pub head: Head,
    /// Its body, chunks joined.
    pub body: Vec<u8>,
}

/// What is wrong with the head of an answer.
#[derive(Debug)]
pub(crate) enum HeadError {
    /// The connection failed, or ended before the head did.
    Io(std::io::Error),
    /// The head is not HTTP.
    Bad(String),
}

impl From<std::io::Error> for HeadError {
    fn from(error: std::io::Error) -> Self {
        HeadError::Io(error)
    }
}

/// Reads the head of an answer: the status line, then a header per line, up to an empty line. Lines may end with
/// `\r\n` or `\n`.
pub(crate) fn read_head(reader: &mut impl BufRead) -> Result<Head, HeadError> {
    let mut budget = MOST_HEAD_BYTES;
    let status_line = read_line(reader, &mut budget)?;
    let (status, reason) =
        parse_status_line(&status_line).ok_or_else(|| HeadError::Bad(String::from("no status line")))?;
    let mut headers = Vec::new();
    loop {
        let line = read_line(reader, &mut budget)?;
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| HeadError::Bad(String::from("a header without a colon")))?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    Ok(Head {
        status,
        reason,
        headers,
    })
}

/// `HTTP/1.1 200 OK` → 200 and `OK`.
fn parse_status_line(line: &str) -> Option<(u16, String)> {
    let rest = line.strip_prefix("HTTP/")?;
    let (_version, rest) = rest.split_once(' ')?;
    let (code, reason) = rest.split_once(' ').unwrap_or((rest, ""));
    let status = code.parse().ok().filter(|status| (100..1000).contains(status))?;
    Some((status, reason.trim().to_owned()))
}

/// One line, without its end, counted against what the head may still have.
pub(crate) fn read_line(reader: &mut impl BufRead, budget: &mut usize) -> Result<String, HeadError> {
    let mut line = Vec::new();
    let limit = u64::try_from(*budget).unwrap_or(u64::MAX);
    let read = std::io::Read::take(&mut *reader, limit).read_until(b'\n', &mut line)?;
    *budget = budget.saturating_sub(read);
    if line.last() != Some(&b'\n') {
        return Err(if read == 0 || *budget > 0 {
            HeadError::Io(std::io::ErrorKind::UnexpectedEof.into())
        } else {
            HeadError::Bad(String::from("the head is too long"))
        });
    }
    let _ = line.pop();
    if line.last() == Some(&b'\r') {
        let _ = line.pop();
    }
    String::from_utf8(line).map_err(|_| HeadError::Bad(String::from("a line that is not UTF-8")))
}

#[cfg(test)]
mod tests {
    use super::{HeadError, read_head};

    #[test]
    fn a_head_is_read_into_its_status_and_headers() {
        let mut text: &[u8] = b"HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nX-Two:  a: b \r\n\r\nbody";
        let head = read_head(&mut text).unwrap();
        assert_eq!(head.status, 403);
        assert_eq!(head.status_line(), "403 Forbidden");
        assert_eq!(head.header("content-type"), Some("application/json"));
        assert_eq!(head.header("x-two"), Some("a: b"));
        assert!(!head.is_success());
        assert_eq!(text, b"body");
    }

    #[test]
    fn lines_may_end_with_a_new_line_alone_and_the_reason_may_be_missing() {
        let mut text: &[u8] = b"HTTP/1.0 204\nA: 1\n\n";
        let head = read_head(&mut text).unwrap();
        assert_eq!((head.status, head.reason.as_str()), (204, ""));
        assert_eq!(head.status_line(), "204");
    }

    #[test]
    fn what_is_not_a_head_is_refused() {
        for text in [
            &b"SSH-2.0-OpenSSH\r\n\r\n"[..],
            b"HTTP/1.1 abc OK\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nno colon\r\n\r\n",
        ] {
            let mut reader = text;
            assert!(matches!(read_head(&mut reader), Err(HeadError::Bad(_))));
        }
        let mut cut: &[u8] = b"HTTP/1.1 200 OK\r\nA: 1";
        assert!(matches!(read_head(&mut cut), Err(HeadError::Io(_))));
    }
}
