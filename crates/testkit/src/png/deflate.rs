//! Compressing bytes as a zlib stream, so that the reference images of the snapshots stay small in the repository:
//! copies of what came before, found through chains of hashes, written with the fixed Huffman codes of deflate in one
//! block.
//!
//! Deutsch, RFC 1951, sections 3.2.5 and 3.2.6, and Deutsch and Gailly, RFC 1950.

use super::checksum::adler32;
use super::inflate::{DISTANCE_BASES, DISTANCE_EXTRA_BITS, LENGTH_BASES, LENGTH_EXTRA_BITS};

/// How far back a copy may reach.
const WINDOW: usize = 32_768;

/// The shortest copy deflate can write.
const SHORTEST_COPY: usize = 3;

/// The longest copy deflate can write.
const LONGEST_COPY: usize = 258;

/// The bits of the hash of three bytes, which picks the chain of the places they were seen.
const HASH_BITS: u32 = 15;

/// How many earlier places with the same hash are tried for each copy: enough for images, which repeat a lot.
const LONGEST_CHAIN: usize = 64;

/// The end of a chain.
const NOWHERE: usize = usize::MAX;

/// The symbol that ends a block.
const END_OF_BLOCK: u16 = 256;

/// The bytes in a zlib stream of one deflate block with fixed Huffman codes.
pub(crate) fn zlib_compressed(bytes: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    // A 32 KiB window and the default level of compression, which makes the header a multiple of 31.
    writer.bytes.extend_from_slice(&[0x78, 0x9C]);
    // The first bit says this is the last block; the two after it, 1 then 0, say that it uses the fixed codes.
    writer.bits(1, 1);
    writer.bits(1, 2);
    let mut finder = CopyFinder::new(bytes);
    let mut position = 0;
    while let Some(byte) = bytes.get(position) {
        let taken = if let Some(copy) = finder.longest_copy(position) {
            write_copy(&mut writer, copy);
            copy.length
        } else {
            write_symbol(&mut writer, u16::from(*byte));
            1
        };
        for place in position..position + taken {
            finder.insert(place);
        }
        position += taken;
    }
    write_symbol(&mut writer, END_OF_BLOCK);
    writer.flush();
    writer.bytes.extend_from_slice(&adler32(bytes).to_be_bytes());
    writer.bytes
}

/// A copy of bytes that came before.
#[derive(Clone, Copy, Debug)]
struct Copy {
    length: usize,
    distance: usize,
}

/// The places where each hash of three bytes was seen, the newest first, within the window.
struct CopyFinder<'a> {
    bytes: &'a [u8],
    /// The newest place of each hash.
    heads: Vec<usize>,
    /// For each place in the window, the place before it with the same hash.
    previous: Vec<usize>,
}

impl<'a> CopyFinder<'a> {
    fn new(bytes: &'a [u8]) -> CopyFinder<'a> {
        CopyFinder {
            bytes,
            heads: vec![NOWHERE; 1 << HASH_BITS],
            previous: vec![NOWHERE; WINDOW],
        }
    }

    /// The hash of the three bytes at a place, when there are three.
    fn hash(&self, position: usize) -> Option<usize> {
        let three = self.bytes.get(position..position + SHORTEST_COPY)?;
        let value = three.iter().fold(0_u32, |value, byte| (value << 8) | u32::from(*byte));
        usize::try_from(value.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)).ok()
    }

    /// Notes that the bytes at a place were seen.
    fn insert(&mut self, position: usize) {
        let Some(hash) = self.hash(position) else {
            return;
        };
        let Some(head) = self.heads.get_mut(hash) else {
            return;
        };
        if let Some(slot) = self.previous.get_mut(position % WINDOW) {
            *slot = *head;
        }
        *head = position;
    }

    /// The longest copy of what starts at a place, from the places seen before it.
    fn longest_copy(&self, position: usize) -> Option<Copy> {
        let hash = self.hash(position)?;
        let mut candidate = self.heads.get(hash).copied().unwrap_or(NOWHERE);
        let mut best: Option<Copy> = None;
        for _tried in 0..LONGEST_CHAIN {
            if candidate == NOWHERE || position - candidate > WINDOW {
                break;
            }
            let length = self.common_length(candidate, position);
            if length >= SHORTEST_COPY && best.is_none_or(|best| length > best.length) {
                best = Some(Copy {
                    length,
                    distance: position - candidate,
                });
                if length == LONGEST_COPY {
                    break;
                }
            }
            let next = self.previous.get(candidate % WINDOW).copied().unwrap_or(NOWHERE);
            // A place of the window written again by a newer one would lead forward: the chain ends there.
            if next >= candidate {
                break;
            }
            candidate = next;
        }
        best
    }

    /// How many bytes from two places are the same, up to the longest copy.
    fn common_length(&self, earlier: usize, position: usize) -> usize {
        let (Some(from), Some(to)) = (self.bytes.get(earlier..), self.bytes.get(position..)) else {
            return 0;
        };
        from.iter()
            .zip(to)
            .take(LONGEST_COPY)
            .take_while(|(left, right)| left == right)
            .count()
    }
}

/// Writes a copy: its length symbol and extra bits, then its distance code and extra bits.
fn write_copy(writer: &mut BitWriter, copy: Copy) {
    let length_index = LENGTH_BASES
        .iter()
        .rposition(|base| usize::from(*base) <= copy.length)
        .unwrap_or(0);
    let distance_index = DISTANCE_BASES
        .iter()
        .rposition(|base| usize::from(*base) <= copy.distance)
        .unwrap_or(0);
    let length_symbol = u16::try_from(257 + length_index).unwrap_or(END_OF_BLOCK);
    write_symbol(writer, length_symbol);
    write_extra(writer, copy.length, &LENGTH_BASES, &LENGTH_EXTRA_BITS, length_index);
    writer.code(u32::try_from(distance_index).unwrap_or(0), 5);
    write_extra(
        writer,
        copy.distance,
        &DISTANCE_BASES,
        &DISTANCE_EXTRA_BITS,
        distance_index,
    );
}

/// Writes how far a length or a distance is past the base of its symbol.
fn write_extra(writer: &mut BitWriter, value: usize, bases: &[u16], extra_bits: &[u8], index: usize) {
    let base = bases.get(index).map_or(0, |base| usize::from(*base));
    let count = extra_bits.get(index).map_or(0, |count| u32::from(*count));
    if count > 0 {
        writer.bits(u32::try_from(value - base).unwrap_or(0), count);
    }
}

/// Writes a symbol of the literals and lengths with its fixed code, RFC 1951 section 3.2.6.
fn write_symbol(writer: &mut BitWriter, symbol: u16) {
    let (code, length) = match symbol {
        0..=143 => (0x30 + symbol, 8),
        144..=255 => (0x190 + symbol - 144, 9),
        256..=279 => (symbol - 256, 7),
        _ => (0xC0 + symbol - 280, 8),
    };
    writer.code(u32::from(code), length);
}

/// Bits written from the lowest bit of each byte, as deflate packs them.
#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    pending: u64,
    count: u32,
}

impl BitWriter {
    /// Writes the lowest bits of a value, the lowest first: how deflate writes everything but Huffman codes.
    fn bits(&mut self, value: u32, count: u32) {
        self.pending |= u64::from(value) << self.count;
        self.count += count;
        while self.count >= 8 {
            self.bytes
                .push(self.pending.to_le_bytes().first().copied().unwrap_or(0));
            self.pending >>= 8;
            self.count -= 8;
        }
    }

    /// Writes a Huffman code, its highest bit first.
    fn code(&mut self, code: u32, length: u32) {
        self.bits(code.reverse_bits() >> (32 - length), length);
    }

    /// Writes the bits left over, padded to a whole byte.
    fn flush(&mut self) {
        if self.count > 0 {
            self.bits(0, 8 - self.count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::zlib_compressed;
    use crate::Random;
    use crate::png::inflate::decompress;

    fn round_trip(bytes: &[u8]) {
        assert_eq!(
            decompress(&zlib_compressed(bytes)).unwrap(),
            bytes,
            "{} bytes",
            bytes.len()
        );
    }

    #[test]
    fn nothing_and_short_inputs_come_back_as_they_were() {
        round_trip(b"");
        round_trip(b"a");
        round_trip(b"ab");
        round_trip(b"abcabcabcabc");
    }

    #[test]
    fn long_runs_and_far_copies_come_back_and_shrink() {
        let mut bytes = vec![7_u8; 100_000];
        bytes.extend((0..70_000_u32).map(|value| u8::try_from(value % 251).unwrap()));
        bytes.extend(vec![9_u8; 1000]);
        let compressed = zlib_compressed(&bytes);
        assert!(compressed.len() < bytes.len() / 20, "{} bytes", compressed.len());
        round_trip(&bytes);
    }

    #[test]
    fn random_bytes_come_back() {
        let mut random = Random::new(7);
        for size in [1, 3, 300, 40_000, 70_000] {
            // Few different bytes, so that copies of every length and distance are found.
            let bytes: Vec<u8> = (0..size).map(|_| u8::try_from(random.below(4)).unwrap()).collect();
            round_trip(&bytes);
        }
    }
}
