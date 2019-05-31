// use std::env;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use twox_hash::XxHash;

use std::thread;
use std::time::Duration;

extern crate crossbeam_channel;
use crossbeam_channel::bounded;

// structopt stuff
// #[macro_use] // not using??
// extern crate structopt;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "xxh", about = "xxHash cli implementation")]
struct Opt {
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
}

// TODO:
// - clap argument parser - DONE
// - custom seed
// - hash in a thread - DONE
// - use channels to log hash progress

// use clap to get filename - DONE
// print the size of the file - DONE
// run the hash in a thread - DONE
// put a channel in the thread - DONE
// send a message through the channel - DONE
// etc...

fn main() {
    let opt = Opt::from_args();

    for fp in opt.files {
        // open file here to bypass the borrow checker, so we can still use
        // 'fp' to print the filename
        let f = File::open(&fp).unwrap();

        let (tx_progress, rx_progress) = bounded(1);
        let (tx_result, rx_result) = bounded(1);

        let handle = thread::spawn(move || {
            let mut buffer = BufReader::new(f);
            let mut hasher = XxHash::with_seed(0);
            let mut bytes_processed = 0;

            loop {
                let consumed = {
                    let bytes = buffer.fill_buf().unwrap();
                    if bytes.len() == 0 {
                        break;
                    }
                    hasher.write(bytes);
                    bytes.len()
                };

                bytes_processed += consumed;

                if tx_progress.is_empty() {
                    tx_progress.send(bytes_processed).unwrap();
                }

                buffer.consume(consumed);
            }

            tx_result.send(hasher.finish()).unwrap();
        });

        while rx_result.is_empty() {
            if rx_progress.is_full() {
                println!("{}", rx_progress.recv().unwrap());
            }
            thread::sleep(Duration::from_millis(100));
        }

        // block on thread completion
        handle.join().unwrap();

        println!("{:x}  {}", rx_result.recv().unwrap(), fp.display());
    }
}
