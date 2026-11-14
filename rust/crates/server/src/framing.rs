//! The framing of the messages, as the Language Server Protocol frames them: a `Content-Length: <bytes>` header, any
//! other header, an empty line, then the JSON in UTF-8 (`docs/transition/02-protocol.md`, Transport).

use std::io::{self, BufRead, Write};

/// The header that says how long the message is.
const LENGTH: &str = "content-length";

/// The bytes of the next message; none when the input ended before one started.
///
/// # Errors
///
/// When the input cannot be read, ends inside a message, or a message has no length.
pub fn read_frame(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut length = None;
    let mut is_first = true;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return if is_first {
                Ok(None)
            } else {
                Err(invalid("The input ended inside the headers of a message."))
            };
        }
        is_first = false;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case(LENGTH)
        {
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| invalid("The length of a message is not a number."))?,
            );
        }
    }
    let length = length.ok_or_else(|| invalid("A message has no Content-Length header."))?;
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

/// Writes one message with its header, and sends it at once.
///
/// # Errors
///
/// When the output cannot be written.
pub fn write_frame(output: &mut impl Write, body: &[u8]) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(body)?;
    output.flush()
}

/// An error of input that is not framed as it should be.
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{read_frame, write_frame};

    #[test]
    fn reads_back_what_it_writes_one_message_after_another() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, br#"{"jsonrpc":"2.0","method":"exit"}"#).unwrap();
        write_frame(&mut bytes, "{\"text\":\"è\"}".as_bytes()).unwrap();
        assert!(bytes.starts_with(b"Content-Length: 33\r\n\r\n{"));
        let mut input = Cursor::new(bytes);
        assert_eq!(
            read_frame(&mut input).unwrap().unwrap(),
            br#"{"jsonrpc":"2.0","method":"exit"}"#
        );
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), "{\"text\":\"è\"}".as_bytes());
        assert_eq!(read_frame(&mut input).unwrap(), None);
    }

    #[test]
    fn takes_the_length_in_any_case_and_leaves_other_headers() {
        let text = "content-length: 2\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{}";
        assert_eq!(read_frame(&mut Cursor::new(text)).unwrap().unwrap(), b"{}");
    }

    #[test]
    fn a_message_without_a_length_or_cut_short_is_an_error() {
        assert!(read_frame(&mut Cursor::new("Content-Type: x\r\n\r\n{}")).is_err());
        assert!(read_frame(&mut Cursor::new("Content-Length: 10\r\n\r\n{}")).is_err());
        assert!(read_frame(&mut Cursor::new("Content-Length: 2\r\n")).is_err());
    }
}
