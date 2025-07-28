use num::BigUint;
use std::io;

/* A block is a "fixed" piece of data. This includes things like file headers/tails, as well as
 * checksums. Bombs are only guaranteed to be valid if their corresponding payload is fully
 * initialized; that is to say that every bomb is populated. */
pub struct Block {
    data: Box<[u8]>,
}

/* A bomb is a very repetitive, highly compressible piece of data. The repeated bytes are an
 * immutable property of a bomb, the length can be populated. */
pub struct Bomb {
    data: Box<[u8]>,
    size: BigUint,
}

/* A segment is either a block or a bomb */
pub enum Segment {
    Block(Block),
    Bomb(Bomb),
}

pub struct Payload {
    data: Box<[Segment]>,
}

impl Payload {
    fn write(&self, output: &mut impl io::Write) -> Result<usize, io::Error> {
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
}
