use num::BigUint;

pub struct AdlerEngine {
    s1: u32,
    s2: u32,
}

fn biguint_to_u32(n: BigUint) -> u32 {
    let digits = n.to_u32_digits();
    if digits.len() == 0 {
        return 0;
    }
    if digits.len() != 1 {
        panic!("Trying to convert large BigUint to u32");
    }
    return digits[0];
}

impl AdlerEngine {
    pub fn new() -> AdlerEngine {
        return AdlerEngine {
            s1: 1,
            s2: 0,
        };
    }

    pub fn apply1(&mut self, data: u8) {
        self.s1 += data as u32;
        self.s1 %= 65521;
        self.s2 += self.s1;
        self.s2 %= 65521;
    }

    pub fn apply(&mut self, data: &[u8]) {
        for byte in data {
            self.apply1(*byte);
        }
    }

    /* reps is mod 65521*/
    pub fn apply_rep(&mut self, data: &[u8], reps: BigUint) {
        /* See https://natechoe.dev/blog/2025-08-04.html */
        let mut t1: u32 = 0;
        let mut t2: u32 = 0;
        for byte in data {
            t1 += *byte as u32;
            t1 %= 65521;
            t2 += t1;
            t2 += 65521;
        }

        let tri = t2;
        let rect = t1 * ((data.len() % 65521) as u32);

        let full_blocks = biguint_to_u32(reps % 65521u16);
        let len = full_blocks * ((data.len() % 65521) as u32) % 65521;

        let num_rects_x2 = full_blocks * (full_blocks-1) % 65521;
        let num_rects = num_rects_x2 * 32761 % 65521; // 32761 = 1/2 (mod 65521)

        self.s2 += (self.s1 * len) % 65521;
        self.s1 += (t1 * full_blocks) % 65521;
        self.s2 += tri * full_blocks % 65521;
        self.s2 += rect * num_rects % 65521;

        self.s1 %= 65521;
        self.s2 %= 65521;
    }

    pub fn bytes(&self) -> [u8; 4] {
        return [
            (self.s2 >> 8) as u8,
            (self.s2 & 255) as u8,
            (self.s1 >> 8) as u8,
            (self.s1 & 255) as u8,
        ];
    }
}
