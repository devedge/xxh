use crossbeam_channel::bounded;
use crossterm::{cursor, terminal, ClearType};
use number_prefix::{NumberPrefix, Prefixed, Standalone};
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime};
use structopt::StructOpt;
use twox_hash::XxHash;

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

    for fp in opt.files {
        // open file here to bypass the borrow checker, so we can still use
        // 'fp' to print the filename
        let f = File::open(&fp).unwrap();
        let filename = fp.display();
        // let metadata = fs::metadata(&fp).unwrap();
        // println!("{}", metadata.len());

        // save the terminal cursor position at the start of the line, so the
        // progress can be displayed inline
        cursor.save_position().unwrap();

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
                    if bytes.is_empty() {
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
                // determine elapsed seconds, to microsecond precision
                let secs_elapsed = start.elapsed().unwrap().as_micros() as f64 / 1_000_000f64;
                let bytes_recv = rx_progress.recv().unwrap();
                let bytes_per_sec = bytes_recv as f64 / secs_elapsed;

                match NumberPrefix::binary(bytes_per_sec) {
                    Standalone(bytes) => print!("{} bytes/s\t{}", bytes, filename),
                    Prefixed(prefix, n) => print!("{:.1} {}B/s\t{}", n, prefix, filename),
                }

                cursor.reset_position().unwrap();
            }
            thread::sleep(Duration::from_millis(250));

            terminal.clear(ClearType::CurrentLine).unwrap();
        }

        handle.join().unwrap();
        println!("{:x}  {}", rx_result.recv().unwrap(), filename);
    }
}
