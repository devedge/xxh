// use std::env;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use twox_hash::XxHash;

use std::time::Duration;
use std::thread;

// extern crate crossbeam_channel;
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
// print the size of the file
// run the hash in a thread - DONE
// put a channel in the thread - DONE
// send a message through the channel - DONE
// etc...

fn main() {
    let opt = Opt::from_args();

    //  88% (514.2 GB/s)  /path/to/file
    // d4341417a49741c3  /path/to/file

    // s, r -> chan of size 1
    // hash {
    //   keep hashing
    //   try to update channel
    //   if queue is full, don't wait for update
    //     pop the value off so its always up to date?? nah
    //   when done, return hash somehow?
    // }
    // print {
    //   print line indicating progress
    //   wait predetermined time
    //   pull value off of channel (will be old??)
    // }

    for fp in opt.files {
        let f = File::open(&fp).unwrap();

        let (s, r) = bounded(1);
        let (s2, r2) = bounded(1);

        let handle = thread::spawn(move || {
            let mut buffer = BufReader::new(f);
            let mut hasher = XxHash::with_seed(0);

            loop {
                let consumed = {
                    let bytes = buffer.fill_buf().unwrap();
                    if bytes.len() == 0 {
                        break;
                    }
                    hasher.write(bytes);

                    if s.is_empty() {
                        s.send(bytes.len()).unwrap();
                    }
                    bytes.len()
                };
                buffer.consume(consumed);
            }

            s2.send(hasher.finish()).unwrap();
        });

        while r2.is_empty() {
            if r.is_full() {
                println!("{}", r.recv().unwrap());
            }
            thread::sleep(Duration::from_millis(100));
        }

        // block on thread completion
        handle.join().unwrap();


        println!("{:x}  {}", r2.recv().unwrap(), fp.display());
    }
}
