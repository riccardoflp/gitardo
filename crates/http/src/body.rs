//! The body of an answer, as its head frames it: so many bytes (`Content-Length`), chunks (`Transfer-Encoding:
//! chunked`), or everything until the connection closes.

use std::io::{BufRead, ErrorKind, Read};

use crate::response::{Head, HeadError, read_line};

/// The most bytes the size line of a chunk, or a trailer, may have.
const MOST_LINE_BYTES: usize = 8 * 1024;

/// How the body of an answer ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Framing {
    /// After so many bytes more.
    Length(u64),
    /// Chunk by chunk, up to a chunk of size 0.
    Chunked(Chunk),
    /// When the connection closes.
    UntilClose,
}

/// Where a chunked body is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Chunk {
    /// Before the size line of a chunk.
    Size,
    /// In a chunk, with so many bytes of it left.
    Data(u64),
    /// After the bytes of a chunk, before the line end that closes it.
    End,
    /// After the last chunk: its trailers have been read.
    Done,
}

/// How the body of an answer to a request ends, from its head.
pub(crate) fn framing(head: &Head, has_body: bool) -> Framing {
    let is_chunked = head
        .header("transfer-encoding")
        .and_then(|codings| codings.rsplit(',').next())
        .is_some_and(|last| last.trim().eq_ignore_ascii_case("chunked"));
    if !has_body || head.status == 204 || head.status == 304 || (100..200).contains(&head.status) {
        Framing::Length(0)
    } else if is_chunked {
        Framing::Chunked(Chunk::Size)
    } else if let Some(length) = head
        .header("content-length")
        .and_then(|length| length.trim().parse().ok())
    {
        Framing::Length(length)
    } else {
        Framing::UntilClose
    }
}

/// A body read from what comes after the head, its framing taken off.
#[derive(Debug)]
pub(crate) struct Framed<R> {
    reader: R,
    framing: Framing,
}

impl<R: BufRead> Framed<R> {
    pub(crate) fn new(reader: R, framing: Framing) -> Self {
        Framed { reader, framing }
    }

    #[cfg(test)]
    pub(crate) fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    fn read_chunked(&mut self, chunk: Chunk, buffer: &mut [u8]) -> std::io::Result<usize> {
        let mut chunk = chunk;
        loop {
            match chunk {
                Chunk::Done => return Ok(0),
                Chunk::Size => {
                    let line = self.line()?;
                    let size = line.split(';').next().unwrap_or_default().trim();
                    let size = u64::from_str_radix(size, 16).map_err(|_| invalid("a chunk with no size"))?;
                    chunk = if size == 0 {
                        while !self.line()?.is_empty() {}
                        Chunk::Done
                    } else {
                        Chunk::Data(size)
                    };
                }
                Chunk::Data(left) => {
                    let wanted = usize::try_from(left).unwrap_or(usize::MAX).min(buffer.len());
                    let read = self.reader.read(buffer.get_mut(..wanted).unwrap_or_default())?;
                    if read == 0 && wanted > 0 {
                        return Err(ErrorKind::UnexpectedEof.into());
                    }
                    let left = left - read as u64;
                    self.framing = Framing::Chunked(if left == 0 { Chunk::End } else { Chunk::Data(left) });
                    return Ok(read);
                }
                Chunk::End => {
                    if !self.line()?.is_empty() {
                        return Err(invalid("a chunk longer than its size"));
                    }
                    chunk = Chunk::Size;
                }
            }
            self.framing = Framing::Chunked(chunk);
        }
    }

    fn line(&mut self) -> std::io::Result<String> {
        let mut budget = MOST_LINE_BYTES;
        read_line(&mut self.reader, &mut budget).map_err(|error| match error {
            HeadError::Io(error) => error,
            HeadError::Bad(reason) => invalid(&reason),
        })
    }
}

impl<R: BufRead> Read for Framed<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self.framing {
            Framing::Length(0) => Ok(0),
            Framing::Length(left) => {
                let wanted = usize::try_from(left).unwrap_or(usize::MAX).min(buffer.len());
                let read = self.reader.read(buffer.get_mut(..wanted).unwrap_or_default())?;
                if read == 0 && wanted > 0 {
                    return Err(ErrorKind::UnexpectedEof.into());
                }
                self.framing = Framing::Length(left - read as u64);
                Ok(read)
            }
            Framing::Chunked(chunk) => self.read_chunked(chunk, buffer),
            Framing::UntilClose => match self.reader.read(buffer) {
                // A server that closes a TLS connection without saying so first ends the body all the same: nothing
                // tells a body cut short from one that ended there.
                Err(error) if error.kind() == ErrorKind::UnexpectedEof => Ok(0),
                other => other,
            },
        }
    }
}

fn invalid(reason: &str) -> std::io::Error {
    std::io::Error::new(ErrorKind::InvalidData, reason.to_owned())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::{Chunk, Framed, Framing, framing};
    use crate::response::Head;

    fn head(headers: &[(&str, &str)]) -> Head {
        Head {
            status: 200,
            reason: String::from("OK"),
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    fn body(bytes: &[u8], framing: Framing) -> std::io::Result<Vec<u8>> {
        let mut read = Vec::new();
        let _ = Framed::new(bytes, framing).read_to_end(&mut read)?;
        Ok(read)
    }

    #[test]
    fn the_head_says_how_the_body_ends() {
        assert_eq!(framing(&head(&[("Content-Length", "12")]), true), Framing::Length(12));
        assert_eq!(
            framing(
                &head(&[("Transfer-Encoding", "gzip, Chunked"), ("Content-Length", "3")]),
                true
            ),
            Framing::Chunked(Chunk::Size)
        );
        assert_eq!(framing(&head(&[]), true), Framing::UntilClose);
        assert_eq!(framing(&head(&[("Content-Length", "12")]), false), Framing::Length(0));
        let mut no_content = head(&[("Content-Length", "12")]);
        no_content.status = 204;
        assert_eq!(framing(&no_content, true), Framing::Length(0));
    }

    #[test]
    fn a_body_of_a_length_stops_there() {
        assert_eq!(body(b"hello, and more", Framing::Length(5)).unwrap(), b"hello");
        assert!(body(b"hel", Framing::Length(5)).is_err());
    }

    #[test]
    fn chunks_are_joined_and_their_trailers_left_out() {
        let chunked = b"5;name=value\r\nhello\r\n7\r\n, world\r\n0\r\nTrailer: yes\r\n\r\nafter";
        assert_eq!(body(chunked, Framing::Chunked(Chunk::Size)).unwrap(), b"hello, world");
        let mut framed = Framed::new(&chunked[..], Framing::Chunked(Chunk::Size));
        let mut read = Vec::new();
        let _ = framed.read_to_end(&mut read).unwrap();
        assert_eq!(*framed.reader_mut(), b"after");
    }

    #[test]
    fn chunks_with_new_lines_alone_and_large_sizes() {
        let text = "a".repeat(300);
        let chunked = format!("12C\n{text}\n0\n\n");
        assert_eq!(
            body(chunked.as_bytes(), Framing::Chunked(Chunk::Size)).unwrap(),
            text.as_bytes()
        );
    }

    #[test]
    fn a_chunk_cut_short_or_with_no_size_is_an_error() {
        assert!(body(b"5\r\nhel", Framing::Chunked(Chunk::Size)).is_err());
        assert!(body(b"zz\r\nhello\r\n0\r\n\r\n", Framing::Chunked(Chunk::Size)).is_err());
        assert!(body(b"2\r\nhello\r\n0\r\n\r\n", Framing::Chunked(Chunk::Size)).is_err());
    }

    #[test]
    fn a_body_until_the_close_is_everything_left() {
        assert_eq!(body(b"all of it", Framing::UntilClose).unwrap(), b"all of it");
    }
}
