use num::BigUint;

pub trait ChecksumEngine {
    fn apply1(&mut self, data: u8);

    fn apply(&mut self, data: &[u8]) {
        for byte in data {
            self.apply1(*byte);
        }
    }

    fn apply_rep(&mut self, data: &[u8], reps: BigUint);

    fn bytes(&self) -> [u8; 4];
}
