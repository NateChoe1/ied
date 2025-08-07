use num::BigUint;

pub struct CrcMatrix {
    /* the rows of the matrix */
    items: [u64; 33],
}

/* Calculates the hamming weight of n, mod 2 */
fn hamming(n: u64) -> u64 {
    /* I stole this trick from Stack Overflow, although I can't seem to find it */
    let n1 = ((n  & 0xaaaaaaaaaaaaaaaau64) >> 1)  + (n & 0x5555555555555555u64);
    let n2 = ((n1 & 0xccccccccccccccccu64) >> 2)  + (n & 0x3333333333333333u64);
    let n3 = ((n2 & 0xf0f0f0f0f0f0f0f0u64) >> 4)  + (n & 0x0f0f0f0f0f0f0f0fu64);
    let n4 = ((n3 & 0xff00ff00ff00ff00u64) >> 8)  + (n & 0x00ff00ff00ff00ffu64);
    let n5 = ((n4 & 0xffff0000ffff0000u64) >> 16) + (n & 0x0000ffff0000ffffu64);
    let n6 = ((n5 & 0xffffffff00000000u64) >> 32) + (n & 0x00000000ffffffffu64);

    return n6;
}

impl CrcMatrix {
    pub fn new() -> CrcMatrix {
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
                let bit = hamming(product) << (32 - j);
                self.items[i] |= bit;
            }
        }
    }

    pub fn push_0(&mut self) {
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

    pub fn push_1(&mut self) {
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

    pub fn exponentiate(&mut self, power: &BigUint) {
        let mut reference = self.clone();
        reference.transpose();
        self.exponentiate_r(power, &reference);
    }

    pub fn apply(&self, v: u32) -> u32 {
        let vector = (v as u64) | (1 << 32);
        let mut ret: u64 = 0;
        for i in 1..33 {
            let product = self.items[i] & vector;
            let bit = hamming(product) << (32 - i);
            ret |= bit;
        }
        return ret as u32;
    }
}
