#![cfg_attr(
    target_os = "cuda",
    no_std,
    feature(register_attr),
    register_attr(nvvm_internal)
)]
#![no_std]

extern crate alloc;

use core::{fmt::Display, ops::Add};

use alloc::fmt;
use cuda_std::prelude::*;

trait JavaShift {
    fn jshr3(self, amount: u32) -> Self;
}

impl JavaShift for i64 {
    #[inline]
    fn jshr3(self, amount: u32) -> Self {
        (self as u64 >> amount) as i64
    }
}

trait JavaHash {
    fn jhash(&self) -> i32;
}

impl JavaHash for String {
    fn jhash(&self) -> i32 {
        let s: Vec<char> = self.chars().collect();
        let len = s.len();
        let mut r = 0;
        for i in 0..len {
            r = (((r << 5) as u32).wrapping_sub(r)).wrapping_add(s[i] as u32)
        }
        r as i32
    }
}

#[inline]
fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

#[inline]
fn reverse_lerp(value: f32, start: f32, end: f32) -> f32 {
    (value - start) / (end - start)
}

#[inline]
fn map(value: f32, old_start: f32, old_end: f32, new_start: f32, new_end: f32) -> f32 {
    lerp(reverse_lerp(value, old_start, old_end), new_start, new_end)
}

fn block_hash(block: &BlockPos) -> i64 {
    let x = block.0.wrapping_mul(3129871) as i64;
    let z = (block.2 as i64).wrapping_mul(116129781);
    let l = x ^ z ^ (block.1 as i64);
    let l = l
        .wrapping_mul(l)
        .wrapping_mul(42317861_i64)
        .wrapping_add(l.wrapping_mul(11_i64));
    let l = l >> 16;
    return l;
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BlockPos(i32, i32, i32);

impl Display for BlockPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("BlockPos({}, {}, {})", self.0, self.1, self.2))
    }
}

impl Add for BlockPos {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        BlockPos(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

trait Random {
    type Splitter: RandomSplitter;

    fn from_long(seed: i64) -> Self;

    fn next(&mut self, bits: u32) -> i64;

    fn next_long(&mut self) -> i64;

    fn next_float(&mut self) -> f32 {
        self.next(f32::MANTISSA_DIGITS) as f32 * 5.9604645E-8
    }

    fn next_splitter(&mut self) -> Self::Splitter;
}

trait RandomSplitter {
    type Random: Random;

    fn split(&self, block: &BlockPos) -> Self::Random;

    fn split_string(&self, seed: String) -> Self::Random;
}

pub struct CheckedRandom {
    seed: i64,
}

impl CheckedRandom {
    const INT_BITS: u32 = 48;
    const SEED_MASK: i64 = 281474976710655;
    const MULTIPLIER: i64 = 25214903917;
    const INCREMENT: i64 = 11;

    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed ^ Self::MULTIPLIER) & Self::SEED_MASK,
        }
    }
}

impl Random for CheckedRandom {
    type Splitter = CheckedRandomSplitter;

    fn from_long(seed: i64) -> Self {
        Self::new(seed)
    }

    fn next_long(&mut self) -> i64 {
        let i = self.next(32);
        let j = self.next(32);
        (i << 32).wrapping_add(j)
    }

    fn next(&mut self, bits: u32) -> i64 {
        let m = self.seed;
        let m = m
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        let m = m & Self::SEED_MASK;
        self.seed = m;
        (m >> Self::INT_BITS - bits) as i32 as i64
    }

    fn next_splitter(&mut self) -> Self::Splitter {
        Self::Splitter {
            seed: self.next_long(),
        }
    }
}

pub struct Xoroshiro128PlusPlus {
    seed_lo: i64,
    seed_hi: i64,
}

impl Xoroshiro128PlusPlus {
    fn new(mut seed_lo: i64, mut seed_hi: i64) -> Self {
        if seed_lo | seed_hi == 0 {
            seed_lo = -7046029254386353131_i64;
            seed_hi = 7640891576956012809_i64;
        }
        Self { seed_lo, seed_hi }
    }

    fn _next(&mut self) -> i64 {
        let l = self.seed_lo;
        let mut m = self.seed_hi;
        let n = l.wrapping_add(m).rotate_left(17).wrapping_add(l);
        m ^= l;
        self.seed_lo = l.rotate_left(49) ^ m ^ (m << 21);
        self.seed_hi = m.rotate_left(28);
        n
    }
}

impl Random for Xoroshiro128PlusPlus {
    type Splitter = XoroSplitter;

    fn from_long(seed: i64) -> Xoroshiro128PlusPlus {
        fn mix_stafford13(mut n: i64) -> i64 {
            n = (n ^ n.jshr3(30)).wrapping_mul(-4658895280553007687_i64);
            n = (n ^ n.jshr3(27)).wrapping_mul(-7723592293110705685_i64);
            n ^ n.jshr3(31)
        }
        let l = seed ^ 7640891576956012809_i64;
        let m = l.wrapping_add(-7046029254386353131_i64);
        Self::new(mix_stafford13(l), mix_stafford13(m))
    }

    fn next_long(&mut self) -> i64 {
        self._next()
    }

    fn next(&mut self, bits: u32) -> i64 {
        self._next().jshr3(64 - bits)
    }

    fn next_splitter(&mut self) -> Self::Splitter {
        let seed_lo = self._next();
        let seed_hi = self._next();
        Self::Splitter { seed_lo, seed_hi }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CheckedRandomSplitter {
    seed: i64,
}

impl RandomSplitter for CheckedRandomSplitter {
    type Random = CheckedRandom;

    fn split(&self, block: &BlockPos) -> Self::Random {
        let l = block_hash(block);
        let m = l ^ self.seed;
        Self::Random::new(m)
    }

    fn split_string(&self, seed: String) -> Self::Random {
        let i = seed.jhash() as i64;
        Self::Random::new(i ^ self.seed)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct XoroSplitter {
    seed_lo: i64,
    seed_hi: i64,
}

impl RandomSplitter for XoroSplitter {
    type Random = Xoroshiro128PlusPlus;

    fn split(&self, pos: &BlockPos) -> Self::Random {
        let l = block_hash(pos);
        let m = l ^ self.seed_lo;
        Self::Random::new(m, self.seed_hi)
    }

    fn split_string(&self, _seed: String) -> Self::Random {
        panic!("tried to split_string")
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub enum MinecraftRandomSplitter {
    Xoroshiro128PlusPlus(XoroSplitter),
    CheckedRandom(CheckedRandomSplitter),
}

impl MinecraftRandomSplitter {
    pub fn split(&self, block: &BlockPos) -> MinecraftRandom {
        match self {
            MinecraftRandomSplitter::Xoroshiro128PlusPlus(x) => {
                MinecraftRandom::Xoroshiro128PlusPlus(x.split(block))
            }
            MinecraftRandomSplitter::CheckedRandom(x) => {
                MinecraftRandom::CheckedRandom(x.split(block))
            }
        }
    }

    pub fn split_string(&self, seed: String) -> MinecraftRandom {
        match self {
            MinecraftRandomSplitter::Xoroshiro128PlusPlus(x) => {
                MinecraftRandom::Xoroshiro128PlusPlus(x.split_string(seed))
            }
            MinecraftRandomSplitter::CheckedRandom(x) => {
                MinecraftRandom::CheckedRandom(x.split_string(seed))
            }
        }
    }
}

pub enum MinecraftRandom {
    Xoroshiro128PlusPlus(Xoroshiro128PlusPlus),
    CheckedRandom(CheckedRandom),
}

impl MinecraftRandom {
    pub fn next(&mut self, bits: u32) -> i64 {
        match self {
            MinecraftRandom::Xoroshiro128PlusPlus(x) => x.next(bits),
            MinecraftRandom::CheckedRandom(x) => x.next(bits),
        }
    }

    pub fn next_long(&mut self) -> i64 {
        match self {
            MinecraftRandom::Xoroshiro128PlusPlus(x) => x.next_long(),
            MinecraftRandom::CheckedRandom(x) => x.next_long(),
        }
    }

    pub fn next_splitter(&mut self) -> MinecraftRandomSplitter {
        match self {
            MinecraftRandom::Xoroshiro128PlusPlus(x) => {
                MinecraftRandomSplitter::Xoroshiro128PlusPlus(x.next_splitter())
            }
            MinecraftRandom::CheckedRandom(x) => {
                MinecraftRandomSplitter::CheckedRandom(x.next_splitter())
            }
        }
    }

    pub fn next_float(&mut self) -> f32 {
        self.next(f32::MANTISSA_DIGITS) as f32 * 5.9604645E-8
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct BedrockSupplier {
    min: i32,
    max: i32,
    reverse: bool,
    random_splitter: MinecraftRandomSplitter,
}

impl BedrockSupplier {
    fn test(&self, block: BlockPos) -> bool {
        let BlockPos(x, y, z) = block;
        if y <= self.min {
            return true ^ self.reverse;
        } else if y >= self.max {
            return false ^ self.reverse;
        } else {
            let d = map(y as f32, self.min as f32, self.max as f32, 1.0, 0.0);
            let mut random = self.random_splitter.split(&BlockPos(x, y, z));
            let b = random.next_float();
            return (b < d) ^ self.reverse;
        }
    }

    fn find(
        &self,
        conditions: &[BedrockCondition],
        break_on_match: bool,
        scale: i32,
        scan_y: i32,
        gpu_idx: u32,
        at_chunk_0: bool,
    ) {
        let z = (gpu_idx as i32 - (scale / 2)) * 2;
        for z in z..=(z + 1) {
            if at_chunk_0 && z % 16 != 0 {
                continue;
            }
            'a: for x in -scale..=scale {
                if at_chunk_0 && x % 16 != 0 {
                    continue;
                }
                for condition in conditions.iter() {
                    if !condition.test(self, BlockPos(x, scan_y, z)) {
                        continue 'a;
                    }
                }
                println!("Found: {} (thread: {})", BlockPos(x, scan_y, z), gpu_idx);
                if break_on_match {
                    return;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct BedrockCondition {
    relative_pos: BlockPos,
    is_there: bool,
}

impl BedrockCondition {
    fn test(&self, supplier: &BedrockSupplier, search_pos: BlockPos) -> bool {
        supplier.test(self.relative_pos + search_pos) ^ !self.is_there
    }
}

#[kernel]
pub unsafe fn main(
    supplier: BedrockSupplier,
    conditions: &[BedrockCondition],
    break_on_match: bool,
    scale: i32,
    scan_y: i32,
    at_chunk_0: bool,
) {
    let idx = thread::index_1d();
    let r = supplier.find(
        conditions,
        break_on_match,
        scale,
        scan_y,
        idx,
        at_chunk_0,
    );
}
