//! A single-probe LZ77 encoder with fixed DEFLATE codes. PDF streams contain
//! many repeated operators; this trades a little ratio for bounded encoding work.
use alloc::vec::Vec;

#[derive(Default)]
pub(super) struct Zip {
    heads: Vec<u32>,
    pub(super) output: Vec<u8>,
}
struct Bits<'a> {
    out: &'a mut Vec<u8>,
    value: u64,
    count: u32,
}
impl Bits<'_> {
    #[inline(always)]
    fn put(&mut self, value: u32, count: u32) {
        self.value |= (value as u64) << self.count;
        self.count += count;
        while self.count >= 8 {
            self.out.push(self.value as u8);
            self.value >>= 8;
            self.count -= 8;
        }
    }
    #[inline(always)]
    fn code(&mut self, symbol: u32) {
        let (code, bits) = match symbol {
            0..=143 => (symbol + 0x30, 8),
            144..=255 => (symbol - 144 + 0x190, 9),
            256..=279 => (symbol - 256, 7),
            _ => (symbol - 280 + 0xc0, 8),
        };
        self.put(code.reverse_bits() >> (32 - bits), bits);
    }
}
const BASE: [usize; 29] =
    [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const EXTRA: [u32; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const LENGTH: [(u32, u32, u32); 259] = {
    let mut table = [(0, 0, 0); 259];
    let mut n = 3;
    while n <= 258 {
        let mut i = 0;
        while i + 1 < BASE.len() && BASE[i + 1] <= n {
            i += 1;
        }
        table[n] = ((i + 257) as u32, (n - BASE[i]) as u32, EXTRA[i]);
        n += 1;
    }
    table
};
const DIST: [usize; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385,
    24577,
];
#[inline(always)]
fn hash(data: &[u8], at: usize) -> usize {
    let v = ((data[at] as u32) << 16) | ((data[at + 1] as u32) << 8) | data[at + 2] as u32;
    (v.wrapping_mul(0x9e3779b1) >> 20) as usize
}
impl Zip {
    pub(super) fn compress(&mut self, data: &[u8]) -> &[u8] {
        self.heads.resize(4096, 0);
        self.heads.fill(0);
        self.output.clear();
        self.output.extend_from_slice(&[0x78, 0x01]);
        let mut bits = Bits { out: &mut self.output, value: 0, count: 0 };
        bits.put(3, 3);
        let mut at = 0;
        while at + 3 <= data.len() {
            let slot = hash(data, at);
            let previous = self.heads[slot];
            self.heads[slot] = (at + 1) as u32;
            let mut len = 0;
            let mut distance = 0;
            if previous != 0 {
                let old = previous as usize - 1;
                distance = at - old;
                if distance <= 32768 && data[old..old + 3] == data[at..at + 3] {
                    len = 3;
                    let max = 258.min(data.len() - at);
                    while len < max && data[old + len] == data[at + len] {
                        len += 1;
                    }
                }
            }
            if len >= 3 {
                let (symbol, extra, count) = LENGTH[len];
                bits.code(symbol);
                bits.put(extra, count);
                let code = if distance <= 4 {
                    (distance - 1) as u32
                } else {
                    let n = (distance as u32 - 1).ilog2();
                    2 * n + (((distance as u32 - 1) >> (n - 1)) & 1)
                };
                bits.put(code.reverse_bits() >> 27, 5);
                if code >= 4 {
                    bits.put((distance - DIST[code as usize]) as u32, code / 2 - 1);
                }
                at += len;
                if at >= 2 && at + 1 < data.len() {
                    self.heads[hash(data, at - 2)] = (at - 1) as u32;
                }
                if at >= 1 && at + 2 < data.len() {
                    self.heads[hash(data, at - 1)] = at as u32;
                }
            } else {
                bits.code(data[at] as u32);
                at += 1;
            }
        }
        for byte in &data[at..] {
            bits.code(*byte as u32);
        }
        bits.code(256);
        if bits.count > 0 {
            bits.out.push(bits.value as u8);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for chunk in data.chunks(5552) {
            for byte in chunk {
                a += *byte as u32;
                b += a;
            }
            a %= 65521;
            b %= 65521;
        }
        self.output.extend_from_slice(&((b << 16) | a).to_be_bytes());
        &self.output
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrips_literals_matches_and_all_distances() {
        let mut zip = Zip::default();
        for len in [0usize, 1, 2, 3, 20, 258, 259, 4096, 32768, 65536] {
            let data: Vec<u8> = (0..len).map(|i| ((i * i + 17 * i) % 251) as u8).collect();
            let compressed = zip.compress(&data);
            assert_eq!(miniz_oxide::inflate::decompress_to_vec_zlib(compressed).unwrap(), data);
        }
        let repeated = alloc::vec![b'a';100000];
        assert_eq!(miniz_oxide::inflate::decompress_to_vec_zlib(zip.compress(&repeated)).unwrap(), repeated);
        assert!(zip.output.len() < 2000);
    }
}
