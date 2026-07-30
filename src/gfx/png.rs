//! A minimal, dependency-free PNG writer.
//!
//! Used by `pop --shot` and `pop --tty-shot`, which render a frame off-screen and
//! write it out — handy for looking at the artwork without a terminal, and for the
//! screenshots in the README.
//!
//! Includes a small deflate compressor (fixed Huffman codes plus greedy LZ77) so
//! the output is a normal, reasonably sized PNG rather than a stored-block blob,
//! without pulling in a compression crate.

use crate::gfx::color::Rgb;

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    let mut with_kind = Vec::with_capacity(4 + body.len());
    with_kind.extend_from_slice(kind);
    with_kind.extend_from_slice(body);
    out.extend_from_slice(&with_kind);
    out.extend_from_slice(&crc32(&with_kind).to_be_bytes());
}

// ---------------------------------------------------------------- deflate

/// LSB-first bit writer, as deflate expects.
struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn new() -> Bits {
        Bits {
            out: Vec::new(),
            acc: 0,
            n: 0,
        }
    }
    /// Write `n` bits of `v`, least-significant bit first.
    fn put(&mut self, v: u32, n: u32) {
        self.acc |= v << self.n;
        self.n += n;
        while self.n >= 8 {
            self.out.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// Huffman codes travel most-significant bit first, so reverse them.
    fn put_code(&mut self, code: u32, n: u32) {
        let mut r = 0u32;
        for i in 0..n {
            r |= ((code >> i) & 1) << (n - 1 - i);
        }
        self.put(r, n);
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push((self.acc & 0xFF) as u8);
        }
        self.out
    }
}

/// Fixed Huffman code for a literal or length symbol (RFC 1951 §3.2.6).
fn fixed_lit(sym: u32) -> (u32, u32) {
    match sym {
        0..=143 => (0x30 + sym, 8),
        144..=255 => (0x190 + sym - 144, 9),
        256..=279 => (sym - 256, 7),
        _ => (0xC0 + sym - 280, 8),
    }
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

const WINDOW: usize = 32768;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
/// How far back to look along a hash chain. Small keeps it fast; screenshots
/// have long flat runs, which the first candidate usually already covers.
const CHAIN: usize = 24;

/// Deflate with fixed Huffman codes and greedy LZ77 matching. Small, dependency
/// free, and easily good enough for screenshots: flat walls and gradients
/// compress by roughly an order of magnitude.
fn deflate_fixed(raw: &[u8]) -> Vec<u8> {
    let mut b = Bits::new();
    b.put(1, 1); // BFINAL
    b.put(1, 2); // BTYPE = 01, fixed Huffman

    let n = raw.len();
    // hash of three bytes -> most recent position; `prev` chains older ones.
    let mut head = vec![u32::MAX; 1 << 15];
    let mut prev = vec![u32::MAX; n.max(1)];
    let hash = |a: u8, b2: u8, c: u8| -> usize {
        (((a as usize) << 10) ^ ((b2 as usize) << 5) ^ (c as usize)) & ((1 << 15) - 1)
    };

    let mut i = 0usize;
    while i < n {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        if i + MIN_MATCH <= n {
            let h = hash(raw[i], raw[i + 1], raw[i + 2]);
            let mut cand = head[h];
            let mut tries = 0;
            while cand != u32::MAX && tries < CHAIN {
                let c = cand as usize;
                let dist = i - c;
                if dist == 0 || dist > WINDOW {
                    break;
                }
                let max = (n - i).min(MAX_MATCH);
                let mut l = 0usize;
                while l < max && raw[c + l] == raw[i + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = dist;
                    if l == max {
                        break;
                    }
                }
                cand = prev[c];
                tries += 1;
            }
            prev[i] = head[h];
            head[h] = i as u32;
        }

        if best_len >= MIN_MATCH {
            // Length symbol.
            let mut li = 0usize;
            while li + 1 < LEN_BASE.len() && (LEN_BASE[li + 1] as usize) <= best_len {
                li += 1;
            }
            let (code, nb) = fixed_lit(257 + li as u32);
            b.put_code(code, nb);
            let extra = LEN_EXTRA[li] as u32;
            if extra > 0 {
                b.put((best_len - LEN_BASE[li] as usize) as u32, extra);
            }
            // Distance symbol (5-bit fixed code).
            let mut di = 0usize;
            while di + 1 < DIST_BASE.len() && (DIST_BASE[di + 1] as usize) <= best_dist {
                di += 1;
            }
            b.put_code(di as u32, 5);
            let dextra = DIST_EXTRA[di] as u32;
            if dextra > 0 {
                b.put((best_dist - DIST_BASE[di] as usize) as u32, dextra);
            }
            // Register the positions we skipped over so later matches find them.
            for k in 1..best_len {
                let p = i + k;
                if p + MIN_MATCH <= n {
                    let h = hash(raw[p], raw[p + 1], raw[p + 2]);
                    prev[p] = head[h];
                    head[h] = p as u32;
                }
            }
            i += best_len;
        } else {
            let (code, nb) = fixed_lit(raw[i] as u32);
            b.put_code(code, nb);
            i += 1;
        }
    }
    let (code, nb) = fixed_lit(256); // end of block
    b.put_code(code, nb);
    b.finish()
}

/// Wrap deflate output in a zlib stream.
fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut z = Vec::new();
    z.push(0x78); // CM = 8, CINFO = 7
    z.push(0x01); // FCHECK so that (0x78 << 8 | 0x01) % 31 == 0
    z.extend_from_slice(&deflate_fixed(raw));
    z.extend_from_slice(&adler32(raw).to_be_bytes());
    z
}

/// Encode an RGB image, optionally magnified by an integer factor.
pub fn encode(px: &[Rgb], w: i32, h: i32, zoom: i32) -> Vec<u8> {
    let zoom = zoom.max(1);
    let ow = w * zoom;
    let oh = h * zoom;
    let mut raw = Vec::with_capacity(((ow * 3 + 1) * oh) as usize);
    let mut line = vec![0u8; (ow * 3) as usize];
    let mut prev_line = vec![0u8; (ow * 3) as usize];
    for y in 0..oh {
        let sy = y / zoom;
        for x in 0..ow {
            let sx = x / zoom;
            let c = px[(sy * w + sx) as usize];
            let o = (x * 3) as usize;
            line[o] = c.r;
            line[o + 1] = c.g;
            line[o + 2] = c.b;
        }
        // Filter 2 ("Up"): store the difference from the row above. Adjacent
        // scanlines of a rendered frame are nearly identical, so this leaves long
        // runs of zeros for the compressor.
        raw.push(2);
        for i in 0..line.len() {
            raw.push(line[i].wrapping_sub(prev_line[i]));
        }
        prev_line.copy_from_slice(&line);
    }
    let mut out = Vec::with_capacity(raw.len() + 128);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(ow as u32).to_be_bytes());
    ihdr.extend_from_slice(&(oh as u32).to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // colour type: truecolour
    ihdr.push(0); // deflate
    ihdr.push(0); // adaptive filtering
    ihdr.push(0); // no interlace
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}
