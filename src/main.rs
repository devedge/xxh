use std::env;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use twox_hash::XxHash;

fn main() {
    for arg in env::args().skip(1) {
        let f = File::open(&arg).unwrap();
        let mut f = BufReader::new(f);

        let mut hasher = XxHash::with_seed(0);

        loop {
            let consumed = {
                let bytes = f.fill_buf().unwrap();
                if bytes.len() == 0 {
                    break;
                }
                hasher.write(bytes);
                bytes.len()
            };
            f.consume(consumed);
        }

        println!("{:16x}   {}", hasher.finish(), arg);
    }
}
