use num::BigUint;
use std::io;

/* A block is a "fixed" piece of data. This includes things like file headers/tails, as well as
 * checksums. Bombs are only guaranteed to be valid if their corresponding payload is fully
 * initialized; that is to say that every bomb is populated. */
pub struct Block {
    data: Box<[u8]>,
    fill: Box<dyn Fn()>,
}

/* A bomb is a very repetitive, highly compressible piece of data. The repeated bytes are an
 * immutable property of a bomb, the length can be populated. */
pub struct Bomb {
    data: Box<[u8]>,
    size: BigUint,

    /* Informs lower level payloads how many bytes they contain.
    *
    * Imagine a double-compressed zip bomb.
    *   Level 2: 1 byte
    *   Level 1: 1032 bytes
    *   Payload: 1065024 bytes
    *
    * level2.fill(1) would call level1.fill(1032) which calls payload.fill(1065024).
    *
    * The fill closure only informs the lower level of its size, it does not change the size of the
    * current payload.
    * */
    fill: Box<dyn Fn(&BigUint)>,
}

/* A segment is either a block or a bomb */
pub enum Segment {
    Block(Block),
    Bomb(Bomb),
}

pub struct Payload {
    pub data: Box<[Segment]>,
}

impl Block {
    pub fn new(data: Box<[u8]>) -> Block {
        return Block {
            data: data,
            fill: Box::new(|| { }),
        };
    }

    pub fn fill(&mut self) {
        (self.fill)();
    }
}

impl Bomb {
    pub fn new(data: Box<[u8]>) -> Bomb {
        return Bomb {
            data: data,
            size: BigUint::ZERO,
            fill: Box::new(|_size: &BigUint| {}),
        };
    }

    pub fn fill(&mut self, size: BigUint) {
        (self.fill)(&size);
        self.size = size;
    }
}

fn biguint_to_u64(num: BigUint) -> Option<u64> {
    let digits = num.to_u64_digits();
    if digits.len() == 0 {
        return Option::Some(0);
    }
    if digits.len() != 1 {
        return Option::None;
    }
    return Option::Some(digits[0]);
}

impl Payload {
    pub fn new(data: Box<[Segment]>) -> Payload {
        return Payload {
            data: data,
        };
    }

    pub fn write(&self, output: &mut impl io::Write) -> Result<usize, io::Error> {
        let mut size: usize = 0;
        for segment in (*self.data).iter() {
            match segment {
                Segment::Block(b) => {
                    let r = output.write(&b.data);
                    if matches!(r, Result::Err(_)) {
                        return r;
                    }
                    if let Ok(s) = r {
                        if s < b.data.len() {
                            return Result::Err(io::Error::new(io::ErrorKind::Other,
                                    "Write failed"));
                        }
                        size += s;
                    } else {
                        return r;
                    }
                }
                Segment::Bomb(b) => {
                    let mut i = BigUint::ZERO;
                    let mut idx = 0;
                    while i < b.size {
                        let slice = [b.data[idx]];
                        let result = output.write(&slice);
                        if let Ok(s) = result {
                            if s < 1 {
                                return Result::Err(io::Error::new(io::ErrorKind::Other,
                                        "Write failed"));
                            }
                            size += 1;
                        } else {
                            return result;
                        }
                        idx = (idx + 1) % b.data.len();
                        i += 1 as usize;
                    }
                }
            }
        }
        return Result::Ok(size)
    }

    pub fn adler32(&self) -> [u8; 4] {
        let mut s0: u64 = 1;
        let mut s1: u64 = 0;
        for segment in (*self.data).iter() {
            match segment {
                Segment::Block(b) => {
                    for byte in &b.data {
                        s0 += *byte as u64;
                        s0 %= 65521;
                        s1 += s0;
                        s1 %= 65521;
                    }
                }
                Segment::Bomb(b) => {
                    /* This algorithm works, trust me */
                    let mut t0: u64 = 0;
                    let mut t1: u64 = 0;
                    for byte in &b.data {
                        t0 += *byte as u64;
                        t0 %= 65521;
                        t1 += t0;
                        t1 %= 65521;
                    }

                    let tri = t1;

                    let rect = t0 * (b.data.len() as u64);
                    let full_blocks_option = biguint_to_u64((b.size.clone() / b.data.len()) % (65521 as u64));
                    let full_blocks: u64;
                    if let Some(v) = full_blocks_option {
                        full_blocks = v;
                    } else {
                        panic!("Failed to convert BigUint to u64");
                    }
                    let extra_bytes_option = biguint_to_u64(b.size.clone() % b.data.len());
                    let extra_bytes: u64;
                    if let Some(v) = extra_bytes_option {
                        extra_bytes = v;
                    } else {
                        panic!("Failed to convert BigUint to u64");
                    }

                    let num_rects = (full_blocks * (full_blocks-1) * 32761) % 65521;

                    s1 += s0 * full_blocks * (b.data.len() as u64);
                    s0 += t0 * full_blocks;
                    s1 += tri * full_blocks + rect * num_rects;

                    s0 %= 65521;
                    s1 %= 65521;

                    for i in 0..extra_bytes {
                        s0 += b.data[i as usize] as u64;
                        s0 %= 65521;
                        s1 += s0;
                        s1 %= 65521;
                    }
                }
            }
        }

        let ret: [u8; 4] = [
            (s1 >> 8)  as u8,
            (s1 & 255) as u8,
            (s0 >> 8)  as u8,
            (s0 & 255) as u8,
        ];
        return ret;
    }
}
