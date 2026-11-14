//! SHA-1, which the fingerprint of a conflict is made with. It only tells conflicts apart: nothing relies on it being
//! hard to forge.

/// The SHA-1 digest of some bytes.
pub(super) fn sha1(bytes: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    let bit_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX).wrapping_mul(8);
    let mut message = bytes.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    for block in message.as_chunks::<64>().0 {
        compress(&mut state, block);
    }
    let mut digest = [0; 20];
    for (chunk, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *chunk = word.to_be_bytes();
    }
    digest
}

/// Mixes one block of 64 bytes into the state.
fn compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut words = [0_u32; 80];
    for (word, bytes) in words.iter_mut().zip(block.as_chunks::<4>().0) {
        *word = u32::from_be_bytes(*bytes);
    }
    for index in 16..80 {
        let mixed = [index - 3, index - 8, index - 14, index - 16]
            .iter()
            .map(|earlier| words.get(*earlier).copied().unwrap_or(0))
            .fold(0, |mixed, word| mixed ^ word);
        if let Some(word) = words.get_mut(index) {
            *word = mixed.rotate_left(1);
        }
    }
    let [mut first, mut second, mut third, mut fourth, mut fifth] = *state;
    for (index, word) in words.iter().enumerate() {
        let (mixed, constant) = match index {
            0..20 => ((second & third) | (!second & fourth), 0x5A82_7999),
            20..40 => (second ^ third ^ fourth, 0x6ED9_EBA1),
            40..60 => ((second & third) | (second & fourth) | (third & fourth), 0x8F1B_BCDC),
            _ => (second ^ third ^ fourth, 0xCA62_C1D6),
        };
        let next = first
            .rotate_left(5)
            .wrapping_add(mixed)
            .wrapping_add(fifth)
            .wrapping_add(constant)
            .wrapping_add(*word);
        fifth = fourth;
        fourth = third;
        third = second.rotate_left(30);
        second = first;
        first = next;
    }
    for (kept, added) in state.iter_mut().zip([first, second, third, fourth, fifth]) {
        *kept = kept.wrapping_add(added);
    }
}

#[cfg(test)]
mod tests {
    use super::sha1;

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }

    #[test]
    fn the_digests_of_the_standard() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(&sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hex(&sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        assert_eq!(hex(&sha1(&[b'a'; 1000])), "291e9a6c66994949b57ba5e650361e98fc36b1ba");
    }
}
