use super::*;
use crate::png::write::zlib_stored;

/// Writes bits as deflate packs them, to build test streams by hand from RFC 1951.
struct BitWriter {
    bytes: Vec<u8>,
    bit: u8,
}

impl BitWriter {
    fn new() -> BitWriter {
        BitWriter {
            bytes: Vec::new(),
            bit: 0,
        }
    }

    /// A number, its least significant bit first.
    fn bits(&mut self, value: u32, count: u32) {
        for index in 0..count {
            self.push((value >> index) & 1);
        }
    }

    /// A Huffman code, its most significant bit first.
    fn code(&mut self, code: u32, length: u8) {
        for index in (0..length).rev() {
            self.push((code >> index) & 1);
        }
    }

    /// Pads to the next byte, as before the length of a stored block.
    fn align(&mut self) {
        self.bit = 0;
    }

    fn push(&mut self, bit: u32) {
        if self.bit == 0 {
            self.bytes.push(0);
        }
        let last = self.bytes.last_mut().unwrap();
        *last |= u8::try_from(bit).unwrap() << self.bit;
        self.bit = (self.bit + 1) % 8;
    }
}

/// The codes of a canonical Huffman code from its lengths, as RFC 1951 section 3.2.2 computes them.
fn canonical_codes(lengths: &[u8]) -> Vec<u32> {
    let mut counts = [0_u32; 16];
    for length in lengths {
        counts[usize::from(*length)] += 1;
    }
    counts[0] = 0;
    let mut next = [0_u32; 16];
    let mut code = 0;
    for bits in 1..16 {
        code = (code + counts[bits - 1]) << 1;
        next[bits] = code;
    }
    let mut codes = Vec::new();
    for length in lengths {
        let length = usize::from(*length);
        codes.push(if length == 0 { 0 } else { next[length] });
        next[length] += 1;
    }
    codes
}

/// Writes a symbol with the code of its lengths.
fn write_symbol(writer: &mut BitWriter, lengths: &[u8], symbol: usize) {
    let codes = canonical_codes(lengths);
    writer.code(codes[symbol], lengths[symbol]);
}

/// The lengths of the fixed literal and length code, RFC 1951 section 3.2.6.
fn fixed_literal_lengths() -> Vec<u8> {
    (0..288)
        .map(|symbol| match symbol {
            0..=143 | 280..=287 => 8,
            144..=255 => 9,
            _ => 7,
        })
        .collect()
}

/// The lengths of the code that the code lengths of [`write_dynamic_header`] are written with: the symbols 18 and 3
/// in two bits, 16, 17, 2 and 1 in three.
fn code_length_lengths() -> [u8; 19] {
    let mut lengths = [0; 19];
    for (symbol, length) in [(18, 2), (3, 2), (16, 3), (17, 3), (2, 3), (1, 3)] {
        lengths[symbol] = length;
    }
    lengths
}

/// Writes the header of a last dynamic block, with code lengths written as symbols of the code length code and their
/// extra bits.
fn write_dynamic_header(writer: &mut BitWriter, counts: (u32, u32), written: &[(usize, u32, u32)]) {
    let (literal_count, distance_count) = counts;
    writer.bits(1, 1);
    writer.bits(2, 2);
    writer.bits(literal_count - 257, 5);
    writer.bits(distance_count - 1, 5);
    // The symbol 1 is the 18th in the order of the code length code.
    writer.bits(18 - 4, 4);
    let lengths = code_length_lengths();
    for symbol in CODE_LENGTH_ORDER.iter().take(18) {
        writer.bits(u32::from(lengths[*symbol]), 3);
    }
    for (symbol, extra, extra_bits) in written {
        write_symbol(writer, &lengths, *symbol);
        writer.bits(*extra, *extra_bits);
    }
}

/// A zlib stream around raw deflate data.
fn zlib(deflate: &[u8], content: &[u8]) -> Vec<u8> {
    let mut stream = vec![0x78, 0x01];
    stream.extend_from_slice(deflate);
    stream.extend_from_slice(&adler32(content).to_be_bytes());
    stream
}

fn deflate_error(deflate: &[u8]) -> Error {
    inflate(&mut BitReader::new(deflate)).expect_err("the deflate data is not valid")
}

#[test]
fn a_fixed_huffman_block_of_one_letter_built_by_hand_decompresses() {
    // The last block, fixed codes (1, then 01), the literal 0x61 as the 8-bit code 0x30 + 0x61 = 10010001, and the
    // end of the block as the 7-bit code 0000000, packed from the least significant bit: 4B 04 00.
    let stream = [0x78, 0x9C, 0x4B, 0x04, 0x00, 0x00, 0x62, 0x00, 0x62];
    assert_eq!(decompress(&stream).unwrap(), b"a");
}

#[test]
fn a_fixed_huffman_block_with_copies_and_extra_bits_decompresses() {
    let lengths = fixed_literal_lengths();
    let distance_lengths = [5_u8; 30];
    let mut writer = BitWriter::new();
    writer.bits(1, 1);
    writer.bits(1, 2);
    for literal in b"abc" {
        write_symbol(&mut writer, &lengths, usize::from(*literal));
    }
    // Length 6 is the symbol 260; distance 3 is the symbol 2.
    write_symbol(&mut writer, &lengths, 260);
    write_symbol(&mut writer, &distance_lengths, 2);
    // Length 12 is the symbol 265 with the extra bit 1; distance 5 is the symbol 4 with the extra bit 0.
    write_symbol(&mut writer, &lengths, 265);
    writer.bits(1, 1);
    write_symbol(&mut writer, &distance_lengths, 4);
    writer.bits(0, 1);
    write_symbol(&mut writer, &lengths, 256);
    let expected = b"abcabcabcbcabcbcabcbc";
    assert_eq!(decompress(&zlib(&writer.bytes, expected)).unwrap(), expected);
}

#[test]
fn a_dynamic_huffman_block_with_every_code_length_symbol_decompresses() {
    let mut literal_lengths = vec![0_u8; 258];
    for literal in b'a'..=b'd' {
        literal_lengths[usize::from(literal)] = 3;
    }
    literal_lengths[256] = 2;
    literal_lengths[257] = 2;
    let distance_lengths = [1_u8, 1];
    let mut writer = BitWriter::new();
    // 97 zeros, a 3 repeated three more times, 155 zeros, then 2, 2 for the end and length 3, and 1, 1 for the
    // distances 1 and 2.
    let written = [
        (18, 97 - 11, 7),
        (3, 0, 0),
        (16, 0, 2),
        (18, 138 - 11, 7),
        (17, 10 - 3, 3),
        (17, 7 - 3, 3),
    ];
    let written = [&written[..], &[(2, 0, 0), (2, 0, 0), (1, 0, 0), (1, 0, 0)]].concat();
    write_dynamic_header(&mut writer, (258, 2), &written);
    for literal in b"abcd" {
        write_symbol(&mut writer, &literal_lengths, usize::from(*literal));
    }
    write_symbol(&mut writer, &literal_lengths, 257);
    write_symbol(&mut writer, &distance_lengths, 1);
    write_symbol(&mut writer, &literal_lengths, usize::from(b'a'));
    write_symbol(&mut writer, &literal_lengths, 256);
    let expected = b"abcdcdca";
    assert_eq!(decompress(&zlib(&writer.bytes, expected)).unwrap(), expected);
}

#[test]
fn a_stored_block_then_a_fixed_block_decompress_one_after_the_other() {
    let lengths = fixed_literal_lengths();
    let mut writer = BitWriter::new();
    writer.bits(0, 1);
    writer.bits(0, 2);
    writer.align();
    writer.bytes.extend_from_slice(&[3, 0, 0xFC, 0xFF]);
    writer.bytes.extend_from_slice(b"xyz");
    writer.bits(1, 1);
    writer.bits(1, 2);
    write_symbol(&mut writer, &lengths, usize::from(b'!'));
    write_symbol(&mut writer, &lengths, 256);
    assert_eq!(decompress(&zlib(&writer.bytes, b"xyz!")).unwrap(), b"xyz!");
}

#[test]
fn stored_streams_of_every_size_decompress_to_what_they_hold() {
    for size in [0, 1, 65_534, 65_535, 65_536, 200_000] {
        let bytes: Vec<u8> = (0..size).map(|index: usize| index.to_le_bytes()[0] ^ 0x5A).collect();
        assert_eq!(decompress(&zlib_stored(&bytes)).unwrap(), bytes, "size {size}");
    }
}

#[test]
fn a_damaged_zlib_stream_is_an_error() {
    let good = zlib_stored(b"hello");
    let mut wrong_checksum = good.clone();
    *wrong_checksum.last_mut().unwrap() ^= 1;
    assert!(matches!(decompress(&wrong_checksum), Err(Error::AdlerChecksum { .. })));
    assert!(matches!(decompress(&[0x78, 0x02]), Err(Error::Zlib { .. })));
    assert!(matches!(decompress(&[0x79, 0x01]), Err(Error::Zlib { .. })));
    assert!(matches!(decompress(&[0x78, 0x20]), Err(Error::Zlib { .. })));
    assert!(matches!(decompress(&[0x78]), Err(Error::Truncated { .. })));
    assert!(matches!(
        decompress(&good[..good.len() - 2]),
        Err(Error::Truncated { .. })
    ));
    let mut trailing = good;
    trailing.push(0);
    assert!(matches!(decompress(&trailing), Err(Error::Zlib { .. })));
}

#[test]
fn damaged_deflate_data_is_an_error_not_a_panic() {
    let mut reserved = BitWriter::new();
    reserved.bits(1, 1);
    reserved.bits(3, 2);
    assert!(matches!(deflate_error(&reserved.bytes), Error::Deflate { .. }));
    assert!(matches!(
        deflate_error(&[0x01, 0x05, 0x00, 0x00, 0x00]),
        Error::Deflate { .. }
    ));
    assert!(matches!(
        deflate_error(&[0x01, 0x05, 0x00, 0xFA, 0xFF, b'a']),
        Error::Truncated { .. }
    ));
    assert!(matches!(deflate_error(&[0x4B, 0x04]), Error::Truncated { .. }));
    assert!(matches!(deflate_error(&[]), Error::Truncated { .. }));
    let lengths = fixed_literal_lengths();
    let mut too_far = BitWriter::new();
    too_far.bits(1, 1);
    too_far.bits(1, 2);
    write_symbol(&mut too_far, &lengths, usize::from(b'a'));
    write_symbol(&mut too_far, &lengths, 257);
    write_symbol(&mut too_far, &[5; 30], 1);
    write_symbol(&mut too_far, &lengths, 256);
    assert!(matches!(deflate_error(&too_far.bytes), Error::Deflate { .. }));
}

#[test]
fn damaged_dynamic_headers_are_errors() {
    let mut repeat_first = BitWriter::new();
    write_dynamic_header(&mut repeat_first, (257, 1), &[(16, 0, 2)]);
    assert!(matches!(deflate_error(&repeat_first.bytes), Error::Deflate { .. }));
    let mut no_end = BitWriter::new();
    write_dynamic_header(
        &mut no_end,
        (257, 2),
        &[(18, 127, 7), (18, 108, 7), (1, 0, 0), (1, 0, 0)],
    );
    assert!(matches!(deflate_error(&no_end.bytes), Error::Deflate { .. }));
    let mut past_the_end = BitWriter::new();
    write_dynamic_header(&mut past_the_end, (257, 1), &[(18, 127, 7), (18, 127, 7)]);
    assert!(matches!(deflate_error(&past_the_end.bytes), Error::Deflate { .. }));
    let mut too_many = BitWriter::new();
    too_many.bits(1, 1);
    too_many.bits(2, 2);
    too_many.bits(31, 5);
    too_many.bits(0, 9);
    assert!(matches!(deflate_error(&too_many.bytes), Error::Deflate { .. }));
}

#[test]
fn a_code_with_more_codes_than_its_lengths_allow_is_refused() {
    assert!(Huffman::from_lengths(&[1, 1, 1]).is_err());
    assert!(Huffman::from_lengths(&[16]).is_err());
    assert!(Huffman::from_lengths(&[1, 1]).is_ok());
    assert!(Huffman::from_lengths(&[1]).is_ok());
}
