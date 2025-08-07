use crate::payload::checksum::ChecksumEngine;
use crate::payload::crc32::matrix::CrcMatrix;
use num::BigUint;

mod matrix;

pub struct Crc32Engine {
    v: u32,
}

impl Crc32Engine {
    pub fn new() -> Crc32Engine {
        return Crc32Engine {
            v: 0xffffffff,
        };
    }
}

impl ChecksumEngine for Crc32Engine {
    fn apply1(&mut self, data: u8) {
        self.v ^= data as u32;
        for _i in 0..8 {
            if (self.v & 1) != 0 {
                self.v = (self.v >> 1) ^ 0xedb88320;
            } else {
                self.v = self.v >> 1;
            }
        }
    }

    fn apply_rep(&mut self, data: &[u8], reps: BigUint) {
        let mut matr = CrcMatrix::new();

        for i in 0..data.len() {
            let byte = data[data.len() - i - 1];
            for j in 0..8 {
                if (byte & (1 << (7 - j))) != 0 {
                    matr.push_1();
                } else {
                    matr.push_0();
                }
            }
        }

        matr.exponentiate(&reps);
        self.v = matr.apply(self.v);
    }

    fn bytes(&self) -> [u8; 4] {
        let crc = !self.v;
        return [
            (crc >> 24) as u8,
            (crc >> 16) as u8,
            (crc >> 8)  as u8,
            (crc >> 0)  as u8,
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        let mut engine = Crc32Engine::new();
        engine.apply1(0x74);
        engine.apply(&[0x65, 0x73, 0x74, 0x20]);
        engine.apply_rep(&[0x61, 0x62, 0x63], BigUint::ZERO + 3u8);
        engine.apply1(0x64);
        assert_eq!(engine.bytes(), [0x9d, 0x1e, 0xef, 0xde]);
    }
}
