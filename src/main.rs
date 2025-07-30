pub mod payload;

fn main() {
    let block1_data: [u8; 1] = [65];
    let block1 = payload::Segment::Block(payload::Block::new(Box::new(block1_data)));

    /*
    let block2_data: [u8; 2] = [68, 70];
    let mut block2_b = payload::Bomb::new(Box::new(block2_data));
    block2_b.fill(num::BigUint::ZERO + (5 as u32));
    let block2 = payload::Segment::Bomb(block2_b);
    */

    let block3_data: [u8; 2] = [66, 67];
    let block3 = payload::Segment::Block(payload::Block::new(Box::new(block3_data)));

    //let payload_data: [payload::Segment; 3] = [block1, block2, block3];
    let payload_data: [payload::Segment; 2] = [block1, block3];
    let payload = payload::Payload::new(Box::new(payload_data));

    let adler32 = payload.crc32();
    println!("{:02x}", adler32[0]);
    println!("{:02x}", adler32[1]);
    println!("{:02x}", adler32[2]);
    println!("{:02x}", adler32[3]);
}
