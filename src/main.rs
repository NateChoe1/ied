pub mod payload;

fn biguint(n: usize) -> num::BigUint {
    return num::BigUint::ZERO + n;
}

fn main() {
    let block1_data: [u8; 1] = [65];
    let block1_b = payload::Block::new(Box::new(block1_data));
    let block1 = payload::Segment::Block(block1_b);

    let block2_data: [u8; 1] = [66];
    let block2_b = payload::Bomb::new(Box::new(block2_data));
    let block2 = payload::Segment::Bomb(block2_b);

    let payload_data: [payload::Segment; 2] = [block1, block2];
    let payload = payload::Payload::new(Box::new(payload_data));

    let mut compressed_payload = payload::gzip(payload);
    compressed_payload.fill(&biguint(1));

    compressed_payload.write(&mut std::io::stdout());
}
