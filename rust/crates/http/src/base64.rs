//! Base64, as RFC 4648 says it, for the user and the password of basic authentication.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bytes in base64, padded with `=`.
pub(crate) fn encode(bytes: &[u8]) -> String {
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

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn the_examples_of_the_rfc() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
            ("Aladdin:open sesame", "QWxhZGRpbjpvcGVuIHNlc2FtZQ=="),
        ];
        for (text, encoded) in cases {
            assert_eq!(encode(text.as_bytes()), encoded, "{text}");
        }
    }
}
