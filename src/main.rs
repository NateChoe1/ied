pub mod payload;

fn biguint(n: usize) -> num::BigUint {
    return num::BigUint::ZERO + n;
}

fn main() {
    let block1_data: [u8; 1] = [97];
    let mut block1_b = payload::Bomb::new(Box::new(block1_data));
    block1_b.fill(Option::None, biguint(10).pow(100));
    let block1 = payload::Segment::Bomb(block1_b);

    let payload_data: [payload::Segment; 1] = [block1];
    let payload = payload::Payload::new(Box::new(payload_data));

    let adler32 = payload.crc32();
    println!("{:02x}", adler32[0]);
    println!("{:02x}", adler32[1]);
    println!("{:02x}", adler32[2]);
    println!("{:02x}", adler32[3]);
}
