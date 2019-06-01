// use std::env;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use twox_hash::XxHash;

use std::thread;
use std::time::{Duration, SystemTime};

use crossterm::{ClearType, cursor, terminal};

use number_prefix::{NumberPrefix, Prefixed, Standalone};

use crossbeam_channel::bounded;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "xxh", about = "xxHash cli implementation")]
struct Opt {
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    let terminal = terminal();
    let cursor = cursor();

    // save the terminal cursor position at the start of the line, so the
    // progress can be displayed inline
    cursor.save_position();

    for fp in opt.files {
        // open file here to bypass the borrow checker, so we can still use
        // 'fp' to print the filename
        let f = File::open(&fp).unwrap();
        let filename = fp.display();
        // let metadata = fs::metadata(&fp).unwrap();
        // println!("{}", metadata.len());

        let (tx_progress, rx_progress) = bounded(1);
        let (tx_result, rx_result) = bounded(1);

        let start = SystemTime::now();

        // start hashing thread
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

        // poll the queue for hashing progress
        while rx_result.is_empty() {

            if rx_progress.is_full() {
                let elapsed = start.elapsed().unwrap();
                let micros = elapsed.as_micros(); // - 250000u128;
                let sec = micros as f64 / 1000000f64;
                // dbg!(elapsed);
                // dbg!(micros);
                // dbg!(sec);
                let b = rx_progress.recv().unwrap();
                // dbg!(b);

                let bytes_sec = b as f64 / sec;

                match NumberPrefix::binary(bytes_sec) {
                    Standalone(bytes) => print!("{} bytes/s  {}", bytes, filename),
                    Prefixed(prefix, n) => print!("{:.1} {}B/s  {}", n, prefix, filename),
                }

                // reset terminal cursor to start of line
                cursor.reset_position();
            }

            // sleep for a few milliseconds
            thread::sleep(Duration::from_millis(100));
            
            // clear the current line after the cursor has been reset to the 
            // start
            terminal.clear(ClearType::CurrentLine);
        }

        // join the hashing thread
        handle.join().unwrap();

        println!("{:x}  {}", rx_result.recv().unwrap(), filename);
    }
}
