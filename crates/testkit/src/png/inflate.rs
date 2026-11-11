//! Decompressing a zlib stream of deflate data: stored, fixed Huffman and dynamic Huffman blocks.
//!
//! Deutsch, RFC 1951, "DEFLATE Compressed Data Format Specification version 1.3", 1996, and Deutsch and Gailly,
//! RFC 1950, "ZLIB Compressed Data Format Specification version 3.3", 1996.

use super::checksum::adler32;
use crate::error::Error;

/// The longest Huffman code deflate uses, in bits.
const LONGEST_CODE: usize = 15;

/// The length that each length symbol from 257 to 285 starts from, RFC 1951 section 3.2.5.
const LENGTH_BASES: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258,
];

/// The number of extra bits after each length symbol from 257 to 285.
const LENGTH_EXTRA_BITS: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// The distance that each distance symbol from 0 to 29 starts from, RFC 1951 section 3.2.5.
const DISTANCE_BASES: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];

/// The number of extra bits after each distance symbol from 0 to 29.
const DISTANCE_EXTRA_BITS: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

/// The order in which a dynamic block gives the lengths of the code that its code lengths are written with,
/// RFC 1951 section 3.2.7.
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// The symbol that ends a block.
const END_OF_BLOCK: u16 = 256;

/// The bytes of a zlib stream, decompressed, with its header and its Adler-32 checked.
pub(crate) fn decompress(stream: &[u8]) -> Result<Vec<u8>, Error> {
    let Some(([method, flags], data)) = stream.split_first_chunk::<2>() else {
        return Err(Error::Truncated {
            part: "the zlib header",
        });
    };
    if method & 0x0F != 8 || method >> 4 > 7 {
        return Err(Error::Zlib {
            reason: "the compression method is not deflate with a window of 32 KiB or less",
        });
    }
    if ((u16::from(*method) << 8) | u16::from(*flags)) % 31 != 0 {
        return Err(Error::Zlib {
            reason: "the header check bits are wrong",
        });
    }
    if flags & 0x20 != 0 {
        return Err(Error::Zlib {
            reason: "the stream needs a preset dictionary",
        });
    }
    let mut reader = BitReader::new(data);
    let bytes = inflate(&mut reader)?;
    let trailer = data.get(reader.consumed()..).unwrap_or_default();
    let Some((stored, after)) = trailer.split_first_chunk::<4>() else {
        return Err(Error::Truncated {
            part: "the Adler-32 of the zlib stream",
        });
    };
    if !after.is_empty() {
        return Err(Error::Zlib {
            reason: "there is data after the end of the stream",
        });
    }
    let (stored, computed) = (u32::from_be_bytes(*stored), adler32(&bytes));
    if stored != computed {
        return Err(Error::AdlerChecksum { stored, computed });
    }
    Ok(bytes)
}

/// The bytes of deflate data, block after block until the last one.
pub(crate) fn inflate(reader: &mut BitReader<'_>) -> Result<Vec<u8>, Error> {
    let mut output = Vec::new();
    loop {
        let is_last = reader.bits(1)? == 1;
        let codes = match reader.bits(2)? {
            0 => None,
            1 => Some(Codes::fixed()?),
            2 => Some(Codes::dynamic(reader)?),
            _ => {
                return Err(Error::Deflate {
                    reason: "a block has the reserved type 3",
                });
            }
        };
        match codes {
            None => stored_block(reader, &mut output)?,
            Some(codes) => compressed_block(reader, &codes, &mut output)?,
        }
        if is_last {
            return Ok(output);
        }
    }
}

/// Reads the bits of deflate data, from the least significant bit of each byte to the most.
#[derive(Debug)]
pub(crate) struct BitReader<'a> {
    bytes: &'a [u8],
    /// The byte the next bit is in.
    position: usize,
    /// How many bits of that byte are read already.
    bit: u8,
}

impl<'a> BitReader<'a> {
    /// A reader at the first bit of the bytes.
    pub(crate) fn new(bytes: &'a [u8]) -> BitReader<'a> {
        BitReader {
            bytes,
            position: 0,
            bit: 0,
        }
    }

    /// The next bit.
    fn bit(&mut self) -> Result<u32, Error> {
        let byte = self.bytes.get(self.position).ok_or(Error::Truncated {
            part: "the deflate data",
        })?;
        let bit = (byte >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.position += 1;
        }
        Ok(u32::from(bit))
    }

    /// A number of up to 16 bits, its least significant bit first, as deflate writes everything but Huffman codes.
    fn bits(&mut self, count: u32) -> Result<u32, Error> {
        let mut value = 0;
        for index in 0..count {
            value |= self.bit()? << index;
        }
        Ok(value)
    }

    /// Bytes as they are, after skipping to the start of the next byte.
    fn whole_bytes(&mut self, count: usize) -> Result<&'a [u8], Error> {
        self.position = self.consumed();
        self.bit = 0;
        let bytes = self.bytes.get(self.position..).and_then(|rest| rest.get(..count));
        let bytes = bytes.ok_or(Error::Truncated { part: "a stored block" })?;
        self.position += count;
        Ok(bytes)
    }

    /// How many bytes are read, counting the one the next bit is in as read when some of its bits are.
    pub(crate) fn consumed(&self) -> usize {
        self.position + usize::from(self.bit != 0)
    }
}

/// A canonical Huffman code, RFC 1951 section 3.2.2: how many codes there are of each length, and the symbols in
/// the order of their codes.
#[derive(Debug)]
struct Huffman {
    /// How many codes there are of each length in bits, from 0 to 15; the count of length 0 is not used.
    counts: [u32; LONGEST_CODE + 1],
    /// The symbols that have a code, shortest code first, and in the order of the symbols among codes of a length.
    symbols: Vec<u16>,
}

impl Huffman {
    /// The code in which each symbol has the length in bits at its place, 0 for a symbol that does not occur. A code
    /// with fewer codes than its lengths could have is allowed, as RFC 1951 allows one distance code alone; a code
    /// with more than they can have is not.
    fn from_lengths(lengths: &[u8]) -> Result<Huffman, Error> {
        let mut used: Vec<(u8, u16)> = Vec::new();
        for (symbol, length) in (0_u16..).zip(lengths) {
            if usize::from(*length) > LONGEST_CODE {
                return Err(Error::Deflate {
                    reason: "a code is longer than 15 bits",
                });
            }
            if *length != 0 {
                used.push((*length, symbol));
            }
        }
        // The sort is stable: among codes of a length, the symbols stay in their order.
        used.sort_by_key(|(length, _)| *length);
        let mut counts = [0; LONGEST_CODE + 1];
        for (length, _) in &used {
            if let Some(count) = counts.get_mut(usize::from(*length)) {
                *count += 1;
            }
        }
        let mut available: u32 = 1;
        for count in counts.iter().skip(1) {
            available = (available * 2).checked_sub(*count).ok_or(Error::Deflate {
                reason: "a code is oversubscribed",
            })?;
        }
        Ok(Huffman {
            counts,
            symbols: used.into_iter().map(|(_, symbol)| symbol).collect(),
        })
    }

    /// The next symbol. A code is read one bit at a time, its most significant bit first; the codes of a length follow
    /// those of the length before, so the first code of each length tells whether the bits so far are one of them.
    fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, Error> {
        let (mut code, mut first, mut index) = (0_u32, 0_u32, 0_u32);
        for count in self.counts.iter().skip(1) {
            code |= reader.bit()?;
            if code < first + count {
                let position = usize::try_from(index + code - first).unwrap_or(usize::MAX);
                return self.symbols.get(position).copied().ok_or(Error::Deflate {
                    reason: "a code has no symbol",
                });
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(Error::Deflate {
            reason: "a code is not one of the block's codes",
        })
    }
}

/// The two codes of a compressed block: one for literals, lengths and the end of the block, one for distances.
#[derive(Debug)]
struct Codes {
    literals: Huffman,
    distances: Huffman,
}

impl Codes {
    /// The codes of a block compressed with fixed Huffman codes, RFC 1951 section 3.2.6.
    fn fixed() -> Result<Codes, Error> {
        let mut literals = [8_u8; 288];
        for (symbol, length) in (0_u16..).zip(literals.iter_mut()) {
            *length = match symbol {
                144..=255 => 9,
                256..=279 => 7,
                // Literals 0 to 143 and the symbols 280 to 287.
                _ => 8,
            };
        }
        Ok(Codes {
            literals: Huffman::from_lengths(&literals)?,
            distances: Huffman::from_lengths(&[5; 30])?,
        })
    }

    /// The codes of a block compressed with dynamic Huffman codes, read from its header, RFC 1951 section 3.2.7.
    fn dynamic(reader: &mut BitReader<'_>) -> Result<Codes, Error> {
        let literal_count = usize::try_from(reader.bits(5)? + 257).unwrap_or(usize::MAX);
        let distance_count = usize::try_from(reader.bits(5)? + 1).unwrap_or(usize::MAX);
        let code_length_count = usize::try_from(reader.bits(4)? + 4).unwrap_or(usize::MAX);
        if literal_count > 286 || distance_count > 30 {
            return Err(Error::Deflate {
                reason: "a block has more length or distance codes than there are",
            });
        }
        let mut code_lengths = [0_u8; 19];
        for symbol in CODE_LENGTH_ORDER.iter().take(code_length_count) {
            if let Some(length) = code_lengths.get_mut(*symbol) {
                *length = u8::try_from(reader.bits(3)?).unwrap_or(u8::MAX);
            }
        }
        let code_length_code = Huffman::from_lengths(&code_lengths)?;
        let lengths = code_lengths_of_block(reader, &code_length_code, literal_count + distance_count)?;
        let (literal_lengths, distance_lengths) = lengths.split_at(literal_count);
        if literal_lengths.get(usize::from(END_OF_BLOCK)).copied().unwrap_or(0) == 0 {
            return Err(Error::Deflate {
                reason: "a block has no code for its end",
            });
        }
        Ok(Codes {
            literals: Huffman::from_lengths(literal_lengths)?,
            distances: Huffman::from_lengths(distance_lengths)?,
        })
    }
}

/// The lengths of the literal and distance codes of a dynamic block, written with the code length code: a length
/// from 0 to 15 as it is, 16 for the length before repeated 3 to 6 times, 17 and 18 for 3 to 10 and 11 to 138 zeros.
fn code_lengths_of_block(reader: &mut BitReader<'_>, code: &Huffman, count: usize) -> Result<Vec<u8>, Error> {
    let mut lengths: Vec<u8> = Vec::with_capacity(count);
    while lengths.len() < count {
        let symbol = code.decode(reader)?;
        let (length, repeat) = match symbol {
            0..=15 => (u8::try_from(symbol).unwrap_or(0), 1),
            16 => {
                let previous = lengths.last().ok_or(Error::Deflate {
                    reason: "a length repeats one before the first",
                })?;
                (*previous, 3 + reader.bits(2)?)
            }
            17 => (0, 3 + reader.bits(3)?),
            18 => (0, 11 + reader.bits(7)?),
            _ => {
                return Err(Error::Deflate {
                    reason: "a code length symbol is larger than 18",
                });
            }
        };
        lengths.extend(std::iter::repeat_n(
            length,
            usize::try_from(repeat).unwrap_or(usize::MAX),
        ));
    }
    if lengths.len() > count {
        return Err(Error::Deflate {
            reason: "repeated code lengths go past the last code",
        });
    }
    Ok(lengths)
}

/// Copies the bytes of a stored block to the output, RFC 1951 section 3.2.4.
fn stored_block(reader: &mut BitReader<'_>, output: &mut Vec<u8>) -> Result<(), Error> {
    let header = reader.whole_bytes(4)?;
    let Some(([length_low, length_high], [complement_low, complement_high])) = header.split_first_chunk::<2>() else {
        return Err(Error::Truncated { part: "a stored block" });
    };
    let length = u16::from_le_bytes([*length_low, *length_high]);
    if !length != u16::from_le_bytes([*complement_low, *complement_high]) {
        return Err(Error::Deflate {
            reason: "the length of a stored block does not match its complement",
        });
    }
    output.extend_from_slice(reader.whole_bytes(usize::from(length))?);
    Ok(())
}

/// Decodes a block compressed with Huffman codes into the output, up to its end, RFC 1951 section 3.2.5.
fn compressed_block(reader: &mut BitReader<'_>, codes: &Codes, output: &mut Vec<u8>) -> Result<(), Error> {
    loop {
        let symbol = codes.literals.decode(reader)?;
        if let Ok(literal) = u8::try_from(symbol) {
            output.push(literal);
            continue;
        }
        if symbol == END_OF_BLOCK {
            return Ok(());
        }
        let length = with_extra_bits(reader, symbol - 257, &LENGTH_BASES, &LENGTH_EXTRA_BITS)?;
        let distance_symbol = codes.distances.decode(reader)?;
        let distance = with_extra_bits(reader, distance_symbol, &DISTANCE_BASES, &DISTANCE_EXTRA_BITS)?;
        copy_back(output, distance, length)?;
    }
}

/// A length or a distance: the base of its symbol, plus the number in the extra bits that follow the symbol.
fn with_extra_bits(reader: &mut BitReader<'_>, index: u16, bases: &[u16], extra_bits: &[u8]) -> Result<usize, Error> {
    let index = usize::from(index);
    let (Some(base), Some(extra_bits)) = (bases.get(index), extra_bits.get(index)) else {
        return Err(Error::Deflate {
            reason: "a length or distance symbol does not exist",
        });
    };
    let extra = reader.bits(u32::from(*extra_bits))?;
    Ok(usize::from(*base) + usize::try_from(extra).unwrap_or(0))
}

/// Appends to the output a copy of the bytes that start a distance back from its end. The copy may overlap what it
/// appends, which repeats the bytes.
fn copy_back(output: &mut Vec<u8>, distance: usize, length: usize) -> Result<(), Error> {
    let Some(start) = output.len().checked_sub(distance) else {
        return Err(Error::Deflate {
            reason: "a distance goes back before the start of the data",
        });
    };
    for position in start..start + length {
        let byte = output.get(position).copied().unwrap_or_default();
        output.push(byte);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
