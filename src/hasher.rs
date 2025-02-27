//! A speedy, non-cryptographic hashing algorithm used by `rustc`.
//!
//! Copied from `rustc_hash` to avoid a dependency and non-needed code.

use core::default::Default;
use core::hash::{BuildHasher, Hasher};
use std::collections::HashMap;

pub(crate) type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;

/// A speedy hash algorithm for use within rustc. The hashmap in liballoc
/// by default uses SipHash which isn't quite as speedy as we want. In the
/// compiler we're not really worried about DOS attempts, so we use a fast
/// non-cryptographic hash.
///
/// The current implementation is a fast polynomial hash with a single
/// bit rotation as a finishing step designed by Orson Peters.
#[derive(Clone)]
pub(crate) struct FxHasher {
    hash: usize,
}

// One might view a polynomial hash
//    m[0] * k    + m[1] * k^2  + m[2] * k^3  + ...
// as a multilinear hash with keystream k[..]
//    m[0] * k[0] + m[1] * k[1] + m[2] * k[2] + ...
// where keystream k just happens to be generated using a multiplicative
// congruential pseudorandom number generator (MCG). For that reason we chose a
// constant that was found to be good for a MCG in:
//     "Computationally Easy, Spectrally Good Multipliers for Congruential
//     Pseudorandom Number Generators" by Guy Steele and Sebastiano Vigna.
#[cfg(target_pointer_width = "64")]
const K: usize = 0xf1357aea2e62a9c5;
#[cfg(target_pointer_width = "32")]
const K: usize = 0x93d765dd;

impl FxHasher {
    /// Creates a default `fx` hasher.
    pub(crate) const fn default() -> FxHasher {
        FxHasher { hash: 0 }
    }
}

impl Default for FxHasher {
    #[inline]
    fn default() -> FxHasher {
        Self::default()
    }
}

impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, i: usize) {
        self.hash = self.hash.wrapping_add(i).wrapping_mul(K);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn finish(&self) -> u64 {
        // Since we used a multiplicative hash our top bits have the most
        // entropy (with the top bit having the most, decreasing as you go).
        // As most hash table implementations (including hashbrown) compute
        // the bucket index from the bottom bits we want to move bits from the
        // top to the bottom. Ideally we'd rotate left by exactly the hash table
        // size, but as we don't know this we'll choose 20 bits, giving decent
        // entropy up until 2^20 table sizes. On 32-bit hosts we'll dial it
        // back down a bit to 15 bits.

        #[cfg(target_pointer_width = "64")]
        const ROTATE: u32 = 20;
        #[cfg(target_pointer_width = "32")]
        const ROTATE: u32 = 15;

        self.hash.rotate_left(ROTATE) as u64

        // A bit reversal would be even better, except hashbrown also expects
        // good entropy in the top 7 bits and a bit reverse would fill those
        // bits with low entropy. More importantly, bit reversals are very slow
        // on x86-64. A byte reversal is relatively fast, but still has a 2
        // cycle latency on x86-64 compared to the 1 cycle latency of a rotate.
        // It also suffers from the hashbrown-top-7-bit-issue.
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // Compress the byte string to a single u64 and add to our hash.
        self.write_u64(hash_bytes(bytes));
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i as usize);
        #[cfg(target_pointer_width = "32")]
        self.add_to_hash((i >> 32) as usize);
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.add_to_hash(i as usize);
        #[cfg(target_pointer_width = "32")]
        self.add_to_hash((i >> 32) as usize);
        self.add_to_hash((i >> 64) as usize);
        #[cfg(target_pointer_width = "32")]
        self.add_to_hash((i >> 96) as usize);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i);
    }
}

// Nothing special, digits of pi.
const SEED1: u64 = 0x243f6a8885a308d3;
const SEED2: u64 = 0x13198a2e03707344;
const PREVENT_TRIVIAL_ZERO_COLLAPSE: u64 = 0xa4093822299f31d0;

#[inline]
fn multiply_mix(x: u64, y: u64) -> u64 {
    #[cfg(target_pointer_width = "64")]
    {
        // We compute the full u64 x u64 -> u128 product, this is a single mul
        // instruction on x86-64, one mul plus one mulhi on ARM64.
        let full = (x as u128) * (y as u128);
        let lo = full as u64;
        let hi = (full >> 64) as u64;

        // The middle bits of the full product fluctuate the most with small
        // changes in the input. This is the top bits of lo and the bottom bits
        // of hi. We can thus make the entire output fluctuate with small
        // changes to the input by XOR'ing these two halves.
        lo ^ hi

        // Unfortunately both 2^64 + 1 and 2^64 - 1 have small prime factors,
        // otherwise combining with + or - could result in a really strong hash, as:
        //     x * y = 2^64 * hi + lo = (-1) * hi + lo = lo - hi,   (mod 2^64 + 1)
        //     x * y = 2^64 * hi + lo =    1 * hi + lo = lo + hi,   (mod 2^64 - 1)
        // Multiplicative hashing is universal in a field (like mod p).
    }

    #[cfg(target_pointer_width = "32")]
    {
        // u64 x u64 -> u128 product is prohibitively expensive on 32-bit.
        // Decompose into 32-bit parts.
        let lx = x as u32;
        let ly = y as u32;
        let hx = (x >> 32) as u32;
        let hy = (y >> 32) as u32;

        // u32 x u32 -> u64 the low bits of one with the high bits of the other.
        let afull = (lx as u64) * (hy as u64);
        let bfull = (hx as u64) * (ly as u64);

        // Combine, swapping low/high of one of them so the upper bits of the
        // product of one combine with the lower bits of the other.
        afull ^ bfull.rotate_right(32)
    }
}

/// A wyhash-inspired non-collision-resistant hash for strings/slices designed
/// by Orson Peters, with a focus on small strings and small codesize.
///
/// The 64-bit version of this hash passes the SMHasher3 test suite on the full
/// 64-bit output, that is, f(hash_bytes(b) ^ f(seed)) for some good avalanching
/// permutation f() passed all tests with zero failures. When using the 32-bit
/// version of multiply_mix this hash has a few non-catastrophic failures where
/// there are a handful more collisions than an optimal hash would give.
///
/// We don't bother avalanching here as we'll feed this hash into a
/// multiplication after which we take the high bits, which avalanches for us.
#[inline]
fn hash_bytes(bytes: &[u8]) -> u64 {
    let len = bytes.len();
    let mut s0 = SEED1;
    let mut s1 = SEED2;

    if len <= 16 {
        // XOR the input into s0, s1.
        if len >= 8 {
            s0 ^= u64::from_le_bytes(bytes[0..8].try_into().unwrap());
            s1 ^= u64::from_le_bytes(bytes[len - 8..].try_into().unwrap());
        } else if len >= 4 {
            s0 ^= u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as u64;
            s1 ^= u32::from_le_bytes(bytes[len - 4..].try_into().unwrap()) as u64;
        } else if len > 0 {
            let lo = bytes[0];
            let mid = bytes[len / 2];
            let hi = bytes[len - 1];
            s0 ^= lo as u64;
            s1 ^= ((hi as u64) << 8) | mid as u64;
        }
    } else {
        // Handle bulk (can partially overlap with suffix).
        let mut off = 0;
        while off < len - 16 {
            let x = u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap());
            let y = u64::from_le_bytes(bytes[off + 8..off + 16].try_into().unwrap());

            // Replace s1 with a mix of s0, x, and y, and s0 with s1.
            // This ensures the compiler can unroll this loop into two
            // independent streams, one operating on s0, the other on s1.
            //
            // Since zeroes are a common input we prevent an immediate trivial
            // collapse of the hash function by XOR'ing a constant with y.
            let t = multiply_mix(s0 ^ x, PREVENT_TRIVIAL_ZERO_COLLAPSE ^ y);
            s0 = s1;
            s1 = t;
            off += 16;
        }

        let suffix = &bytes[len - 16..];
        s0 ^= u64::from_le_bytes(suffix[0..8].try_into().unwrap());
        s1 ^= u64::from_le_bytes(suffix[8..16].try_into().unwrap());
    }

    multiply_mix(s0, s1) ^ (len as u64)
}

/// An implementation of [`BuildHasher`] that produces [`FxHasher`]s.
#[derive(Copy, Clone, Default)]
pub(crate) struct FxBuildHasher;

impl BuildHasher for FxBuildHasher {
    type Hasher = FxHasher;
    fn build_hasher(&self) -> FxHasher {
        FxHasher::default()
    }
}