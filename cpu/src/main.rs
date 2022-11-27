use std::{
    env,
    fmt::Display,
    ops::Add,
    time::SystemTime,
};

use cust::{launch, prelude::*, stream::Stream, DeviceCopy};

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

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[derive(Debug, Clone, Copy, DeviceCopy)]
#[repr(C)]
pub struct BlockPos(i32, i32, i32);

impl Display for BlockPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

#[derive(Clone, Copy, DeviceCopy)]
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

#[derive(Clone, Copy, DeviceCopy)]
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

    fn split_string(&self, seed: String) -> Self::Random {
        let bs = md5::compute(seed.as_bytes()).0;
        let a0 = [bs[0], bs[1], bs[2], bs[3], bs[4], bs[5], bs[6], bs[7]];
        let a1 = [bs[8], bs[9], bs[10], bs[11], bs[12], bs[13], bs[14], bs[15]];
        let seed_lo = i64::from_be_bytes(a0);
        let seed_hi = i64::from_be_bytes(a1);
        Self::Random::new(seed_lo ^ self.seed_lo, seed_hi ^ self.seed_hi)
    }
}

#[derive(Clone, Copy, DeviceCopy)]
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

#[derive(Clone, Copy, DeviceCopy)]
#[repr(C)]
pub struct BedrockSupplier {
    min: i32,
    max: i32,
    reverse: bool,
    random_splitter: MinecraftRandomSplitter,
}

impl BedrockSupplier {
    pub fn new(world: &World, location: BedrockLocation) -> BedrockSupplier {
        match location.get_info(world) {
            BedrockInfo(min, max, reverse, random_splitter) => BedrockSupplier {
                min,
                max,
                reverse,
                random_splitter,
            },
        }
    }

    pub fn test(&self, block: BlockPos) -> bool {
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

    pub fn find(
        &self,
        conditions: Vec<BedrockCondition>,
        break_on_match: bool,
        scale: i32,
        scan_y: i32,
        _log: bool,
        at_chunk_0: bool,
    ) -> Vec<BlockPos> {
        let results: Vec<BlockPos> = Vec::new();
        unsafe {
            static PTX: &str = include_str!("../gpu.ptx");
            let _ctx = cust::quick_init();
            let module = Module::from_ptx(PTX, &[]).unwrap();
            let stream = Stream::new(StreamFlags::NON_BLOCKING, None).unwrap();
            let func = module.get_function("main").unwrap();
            let (_, block_size) = func.suggested_launch_configuration(0, 0.into()).unwrap();
            let grid_size = (scale as u32 + block_size - 1) / block_size;
            let conditions_slice = conditions.as_slice();
            let args = (
                *self,
                conditions_slice.as_dbuf().unwrap(),
                break_on_match,
                scale,
                scan_y,
                at_chunk_0,
            );
            println!("START {} {grid_size} {block_size}", unix_millis());
            launch!(
                func<<<grid_size, block_size, 0, stream>>>(
                    args.0,
                    args.1.as_device_ptr(),
                    args.1.len(),
                    args.2,
                    args.3,
                    args.4,
                    args.5,
                )
            ).expect("your gpu does not support cuda, please use your cpu (main branch)");
            stream.synchronize().expect("cuda error");
            println!("END {}", unix_millis());
        }
        results
    }
}

pub struct World {
    seed: i64,
    overworld_random_splitter: XoroSplitter,
    nether_random_splitter: CheckedRandomSplitter,
}

impl World {
    pub fn new(seed: i64) -> World {
        World {
            seed,
            overworld_random_splitter: Xoroshiro128PlusPlus::from_long(seed).next_splitter(),
            nether_random_splitter: CheckedRandom::from_long(seed).next_splitter(),
        }
    }

    pub fn get_seed(&self) -> i64 {
        self.seed
    }
}

struct BedrockInfo(i32, i32, bool, MinecraftRandomSplitter);

pub enum BedrockLocation {
    NetherRoof,
    NetherFloor,
    Overworld,
}

impl BedrockLocation {
    fn get_info(self, world: &World) -> BedrockInfo {
        match self {
            Self::NetherRoof => BedrockInfo(
                127 - 5,
                127,
                true,
                MinecraftRandomSplitter::CheckedRandom(
                    world
                        .nether_random_splitter
                        .split_string("minecraft:bedrock_roof".to_owned())
                        .next_splitter(),
                ),
            ),
            Self::NetherFloor => BedrockInfo(
                0,
                5,
                false,
                MinecraftRandomSplitter::CheckedRandom(
                    world
                        .nether_random_splitter
                        .split_string("minecraft:bedrock_floor".to_owned())
                        .next_splitter(),
                ),
            ),
            Self::Overworld => BedrockInfo(
                -64,
                -64 + 5,
                false,
                MinecraftRandomSplitter::Xoroshiro128PlusPlus(
                    world
                        .overworld_random_splitter
                        .split_string("minecraft:bedrock_floor".to_owned())
                        .next_splitter(),
                ),
            ),
        }
    }
}

#[derive(Clone, Copy, DeviceCopy)]
#[repr(C)]
pub struct BedrockCondition {
    pub relative_pos: BlockPos,
    pub is_there: bool,
}

impl BedrockCondition {
    pub fn new(relative_pos: BlockPos, is_there: bool) -> BedrockCondition {
        BedrockCondition {
            relative_pos,
            is_there,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    const ARGS: &str = "\nargs (find mode): bedrock-finder <seed> <dimension> <scale> <scan_y> <at_chunk_0> <pattern = <x>,<y>,<z>:<'1'|'0'>>\nargs (pattern mode): bedrock-finder pattern <('#'|'X'|'_'|' ')...>";
    if args.len() <= 3 {
        panic!("{}", ARGS);
    }
    if args[1] == "pattern" {
        pattern(&args);
        return;
    }
    if args.len() <= 6 {
        panic!("{}", ARGS);
    }
    let mut world = World::new(args[1].parse().unwrap_or_else(|_| args[1].jhash() as i64));
    let supplier = BedrockSupplier::new(
        &mut world,
        match args[2].as_str() {
            "nether:roof" => BedrockLocation::NetherRoof,
            "nether:floor" => BedrockLocation::NetherFloor,
            "overworld" => BedrockLocation::Overworld,
            _ => panic!("invalid dimension. valid: nether:roof, nether:floor, overworld"),
        },
    );
    let mut conditions = Vec::new();
    for arg in args[6..].to_owned() {
        const MSG: &str = "invalid pattern: please provide valid conditions: x,y,z:n where x, y, and z are coordinates and n is 1 if there should be bedrock and 0 if there shouldn't be";
        let split = arg.split_once(":").expect(MSG);
        let coords: Vec<i32> = split.0.split(",").map(|x| x.parse().expect(MSG)).collect();
        if coords.len() != 3 {
            panic!("{}", MSG);
        }
        conditions.push(BedrockCondition::new(
            BlockPos(coords[0], coords[1], coords[2]),
            split.1 == "1",
        ));
    }
    let locations = supplier.find(
        conditions,
        false,
        args[3]
            .parse()
            .expect("invalid scale. please specify the range from spawn in which to search."),
        args[4]
            .parse()
            .expect("invalid scan_y. plese specify the y level to which your pattern is relative."),
        args[5]
            .parse()
            .expect("invalid at_chunk_0. please specify true or false"),
        true,
    );
    println!("\r\x1b[K\nFound:");
    for location in locations {
        println!("  {}", location);
    }
}

fn pattern(args: &Vec<String>) {
    for (z, arg) in args[3..].iter().enumerate() {
        for (x, c) in arg.chars().enumerate() {
            if c == '?' || c == 'a' {
                continue;
            }
            print!(
                "{},{},{}:{} ",
                x,
                args[2],
                z,
                if c == '#' || c == 'X' { 1 } else { 0 }
            );
        }
    }
    println!();
}
