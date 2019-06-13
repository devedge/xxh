use crossbeam_channel::bounded;
use crossterm::{cursor, terminal, ClearType};
use number_prefix::{NumberPrefix, Prefixed, Standalone};
use std::fs::{metadata as Metadata, File};
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
        let filesize = Metadata(&fp).unwrap().len();

        // save the terminal cursor position at the start of the line, so the
        // progress can be displayed inline
        cursor.save_position().unwrap();

        let (tx_progress, rx_progress) = bounded(1);
        let (tx_result, rx_result) = bounded(1);

        let start = SystemTime::now();

        // start thread to hash the file
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

        // while the hash hasn't been passed in the 'result' channel, poll the
        // 'progress' channel for the latest amount of bytes last processed
        while rx_result.is_empty() {
            if rx_progress.is_full() {
                // clear terminal line
                terminal.clear(ClearType::CurrentLine).unwrap();

                // determine elapsed seconds, to microsecond precision
                let secs_elapsed = start.elapsed().unwrap().as_micros() as f64 / 1_000_000f64;
                let bytes_recv = rx_progress.recv().unwrap();
                let bytes_per_sec = bytes_recv as f64 / secs_elapsed;
                let secs_remaining = ((filesize as f64 / bytes_per_sec) - secs_elapsed).round();
                let progress_percent = ((bytes_recv as f64 / filesize as f64) * 100f64).round();

                // space out different outputs evenly
                // determine whether to either truncate the filename, or determine what
                //  the issue is when the filename is longer than the terminal line

                let time_remaining = if secs_remaining < 60f64 {
                    [secs_remaining.to_string(), "s".to_string()].join("")
                } else if secs_remaining < 3600f64 {
                    let min = (secs_remaining / 60f64).floor();
                    let sec = secs_remaining % 60f64;

                    [
                        min.to_string(),
                        "m".to_string(),
                        sec.to_string(),
                        "s".to_string(),
                    ]
                    .join("")
                } else {
                    let hour = (secs_remaining / 3600f64).floor();
                    let min = ((secs_remaining - (hour * 3600f64)) / 60f64).floor();
                    let sec = secs_remaining % 60f64;

                    [
                        hour.to_string(),
                        "h".to_string(),
                        min.to_string(),
                        "m".to_string(),
                        sec.to_string(),
                        "s".to_string(),
                    ]
                    .join("")
                };

                match NumberPrefix::binary(bytes_per_sec) {
                    Standalone(bytes) => print!(
                        " {}% {} bytes/s {}",
                        progress_percent, bytes, time_remaining
                    ),
                    Prefixed(prefix, n) => print!(
                        " {}% {:.1} {}B/s {}",
                        progress_percent, n, prefix, time_remaining
                    ),
                }

                // reset position to start of terminal line
                cursor.reset_position().unwrap();
            }

            thread::sleep(Duration::from_millis(250));
        }

        handle.join().unwrap();
        println!("{:x}  {}", rx_result.recv().unwrap(), filename);
    }
}
