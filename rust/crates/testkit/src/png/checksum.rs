//! The two checksums of a PNG file: the CRC-32 of each chunk and the Adler-32 of the zlib stream.

/// The polynomial of the CRC-32, in the reversed bit order in which it is applied.
const CRC_POLYNOMIAL: u32 = 0xEDB8_8320;

/// The CRC-32 of each value of a byte: the remainder of the byte divided by the polynomial.
fn crc_table() -> [u32; 256] {
    std::array::from_fn(|byte| {
        let mut remainder = u32::try_from(byte).unwrap_or_default();
        for _ in 0..8 {
            remainder = if remainder & 1 == 1 {
                CRC_POLYNOMIAL ^ (remainder >> 1)
            } else {
                remainder >> 1
            };
        }
        remainder
    })
}

/// The CRC-32 of bytes, as each chunk of a PNG file carries it over its type and its data.
///
/// The PNG specification, "Portable Network Graphics (PNG) Specification (Second Edition)", 2003, annex D: the
/// CRC of ISO 3309 and ITU-T V.42, with the polynomial x³² + x²⁶ + x²³ + x²² + x¹⁶ + x¹² + x¹¹ + x¹⁰ + x⁸ + x⁷ + x⁵ +
/// x⁴ + x² + x + 1, starting from all ones and inverted at the end.
pub(crate) fn crc32(parts: &[&[u8]]) -> u32 {
    let table = crc_table();
    let mut crc = u32::MAX;
    for part in parts {
        for byte in *part {
            let index = usize::from(crc.to_le_bytes()[0] ^ byte);
            // The index is a byte, so it is always one of the 256 entries.
            let entry = table.get(index).copied().unwrap_or_default();
            crc = entry ^ (crc >> 8);
        }
    }
    !crc
}

/// The prime that both sums of Adler-32 are taken modulo.
const ADLER_MODULUS: u32 = 65_521;

/// How many bytes can be added before the sums must be reduced, so that they never overflow 32 bits.
const ADLER_BYTES_BEFORE_REDUCING: usize = 5_552;

/// The Adler-32 of bytes, as a zlib stream carries it after the data.
///
/// RFC 1950, "ZLIB Compressed Data Format Specification version 3.3", 1996, section 8.2: two sums modulo 65521, of
/// the bytes and of the first sum after each byte, the second in the high 16 bits.
pub(crate) fn adler32(bytes: &[u8]) -> u32 {
    let (mut first, mut second) = (1_u32, 0_u32);
    for block in bytes.chunks(ADLER_BYTES_BEFORE_REDUCING) {
        for byte in block {
            first += u32::from(*byte);
            second += first;
        }
        first %= ADLER_MODULUS;
        second %= ADLER_MODULUS;
    }
    (second << 16) | first
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crc_32_of_the_check_string_is_its_known_value() {
        assert_eq!(crc32(&[b"123456789"]), 0xCBF4_3926);
        assert_eq!(crc32(&[b"1234", b"56789"]), 0xCBF4_3926);
        assert_eq!(crc32(&[]), 0);
    }

    #[test]
    fn the_crc_32_of_an_iend_chunk_is_the_one_every_png_file_ends_with() {
        assert_eq!(crc32(&[b"IEND"]), 0xAE42_6082);
    }

    #[test]
    fn the_adler_32_of_wikipedia_is_its_known_value() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"a"), 0x0062_0062);
    }

    #[test]
    fn the_adler_32_of_many_bytes_is_reduced_without_overflowing() {
        let bytes = vec![255_u8; 100_000];
        let (mut first, mut second) = (1_u64, 0_u64);
        for byte in &bytes {
            first = (first + u64::from(*byte)) % 65_521;
            second = (second + first) % 65_521;
        }
        assert_eq!(u64::from(adler32(&bytes)), (second << 16) | first);
    }
}
