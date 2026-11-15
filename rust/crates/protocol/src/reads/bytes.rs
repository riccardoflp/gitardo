//! Bytes in the JSON: `{"base64": "…"}`, as RFC 4648 writes them, for the contents of files, which may not be UTF-8.

use serde::{Deserialize, Serialize};

/// The letters of base64, in the order of their values.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bytes, as base64 padded with `=`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BytesJson {
    /// The bytes in base64.
    pub base64: String,
}

/// Text that is not base64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotBase64;

impl std::fmt::Display for NotBase64 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("The bytes are not written in base64.")
    }
}

impl BytesJson {
    /// The JSON of some bytes.
    #[must_use]
    pub fn of(bytes: &[u8]) -> BytesJson {
        BytesJson { base64: encode(bytes) }
    }

    /// The bytes it holds.
    ///
    /// # Errors
    ///
    /// When its text is not base64.
    pub fn bytes(&self) -> Result<Vec<u8>, NotBase64> {
        decode(&self.base64)
    }
}

/// Bytes in base64, padded with `=`.
fn encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let letter = |value: u32| char::from(ALPHABET.get((value & 63) as usize).copied().unwrap_or(b'='));
    for group in bytes.chunks(3) {
        let first = group.first().copied().map_or(0, u32::from);
        let second = group.get(1).copied().map_or(0, u32::from);
        let third = group.get(2).copied().map_or(0, u32::from);
        let value = (first << 16) | (second << 8) | third;
        encoded.push(letter(value >> 18));
        encoded.push(letter(value >> 12));
        encoded.push(if group.len() > 1 { letter(value >> 6) } else { '=' });
        encoded.push(if group.len() > 2 { letter(value) } else { '=' });
    }
    encoded
}

/// The bytes of base64, padded or not; spaces and line breaks are left out.
fn decode(text: &str) -> Result<Vec<u8>, NotBase64> {
    let mut bytes = Vec::with_capacity(text.len() / 4 * 3);
    let mut value: u32 = 0;
    let mut bits = 0;
    for letter in text.bytes().filter(|letter| !letter.is_ascii_whitespace()) {
        if letter == b'=' {
            break;
        }
        let digit = ALPHABET.iter().position(|known| *known == letter).ok_or(NotBase64)?;
        value = (value << 6) | u32::try_from(digit).map_err(|_| NotBase64)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push(u8::try_from((value >> bits) & 0xff).map_err(|_| NotBase64)?);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{BytesJson, decode, encode};

    #[test]
    fn the_examples_of_the_rfc_both_ways() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (text, encoded) in cases {
            assert_eq!(encode(text.as_bytes()), encoded, "{text}");
            assert_eq!(decode(encoded).unwrap(), text.as_bytes(), "{encoded}");
        }
    }

    #[test]
    fn bytes_that_are_not_utf8_come_back_as_they_were() {
        let bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(BytesJson::of(&bytes).bytes().unwrap(), bytes);
        assert!(decode("not base64!").is_err());
    }
}
