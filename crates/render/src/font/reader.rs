//! Reading the big-endian numbers of a font file, never past its end.

/// A part of a font file, read by offsets from its start. Every read that would go past its end gives nothing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reader<'data> {
    /// The bytes of the part.
    data: &'data [u8],
}

impl<'data> Reader<'data> {
    /// A reader over some bytes.
    pub(crate) fn new(data: &'data [u8]) -> Reader<'data> {
        Reader { data }
    }

    /// The bytes from an offset to the end, or nothing past the end.
    pub(crate) fn from(self, offset: usize) -> Option<Reader<'data>> {
        self.data.get(offset..).map(Reader::new)
    }

    /// A number of bytes from an offset, or nothing when they go past the end.
    pub(crate) fn slice(self, offset: usize, length: usize) -> Option<Reader<'data>> {
        let end = offset.checked_add(length)?;
        self.data.get(offset..end).map(Reader::new)
    }

    /// The bytes.
    pub(crate) fn bytes(self) -> &'data [u8] {
        self.data
    }

    /// The number of bytes.
    pub(crate) fn len(self) -> usize {
        self.data.len()
    }

    /// The unsigned byte at an offset.
    pub(crate) fn u8(self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    /// The signed byte at an offset.
    pub(crate) fn i8(self, offset: usize) -> Option<i8> {
        self.u8(offset).map(|byte| i8::from_be_bytes([byte]))
    }

    /// The unsigned 16-bit number at an offset.
    pub(crate) fn u16(self, offset: usize) -> Option<u16> {
        self.array(offset).map(u16::from_be_bytes)
    }

    /// The signed 16-bit number at an offset.
    pub(crate) fn i16(self, offset: usize) -> Option<i16> {
        self.array(offset).map(i16::from_be_bytes)
    }

    /// The unsigned 32-bit number at an offset.
    pub(crate) fn u32(self, offset: usize) -> Option<u32> {
        self.array(offset).map(u32::from_be_bytes)
    }

    /// The unsigned 16-bit number at an offset, as an index.
    pub(crate) fn offset16(self, offset: usize) -> Option<usize> {
        self.u16(offset).map(usize::from)
    }

    /// The unsigned 32-bit number at an offset, as an index.
    pub(crate) fn offset32(self, offset: usize) -> Option<usize> {
        self.u32(offset).and_then(|value| usize::try_from(value).ok())
    }

    /// The fixed-point number of 2 integer bits and 14 fraction bits at an offset, the scales of composite glyphs.
    pub(crate) fn f2dot14(self, offset: usize) -> Option<f32> {
        self.i16(offset).map(|value| f32::from(value) / 16384.0)
    }

    /// A number of bytes from an offset, as an array.
    fn array<const LENGTH: usize>(self, offset: usize) -> Option<[u8; LENGTH]> {
        let end = offset.checked_add(LENGTH)?;
        self.data.get(offset..end)?.try_into().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_big_endian_and_never_read_past_the_end() {
        let reader = Reader::new(&[0x01, 0x02, 0xFF, 0xFE, 0x40, 0x00]);
        assert_eq!(reader.u16(0), Some(0x0102));
        assert_eq!(reader.i16(2), Some(-2));
        assert_eq!(reader.u32(0), Some(0x0102_FFFE));
        assert_eq!(reader.u32(4), None);
        assert_eq!(reader.f2dot14(4), Some(1.0));
        assert_eq!(reader.i8(2), Some(-1));
        assert!(reader.slice(4, 3).is_none());
        assert_eq!(reader.from(6).map(Reader::len), Some(0));
    }
}
