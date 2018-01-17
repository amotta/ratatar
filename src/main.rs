use std::{cmp, env, process};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};

mod index;
mod tar;
mod tar_stream;
mod util;

use index::{parse_entry_type, EntryType};
use util::{from_hex, Result};

fn help() {
    println!("\
ratatar - random-access to tar archives

Usage:
  - ratatar
    Displays this help message.

  - ratatar <SUBCOMMAND> [OPTIONS]
    Invokes one of the subcommands outlined below.

Subcommands:
  - index
    Reads a tar file from standard input, generates an index for
    its content and writes it to standard output.

  - extract <TAR-FILE> <ENTRY-NAME> <OUT-FILE>
    Uses the index file to extract a file from a tar archive.

  - help
    Displays this help message.

Author:
    Alessandro Motta <alessandro.motta@brain.mpg.de>"
    );
}

const COPY_BUF_SIZE: usize = 8192;

fn extract<I>(args: &mut I) -> Result<()>
where I: Iterator<Item = String> {
    let tar_path = args.next().unwrap();
    let index_path = {
        let mut p = tar_path.clone();
        p.push_str(".index");
        p
    };

    let entry_name = args.next().unwrap();
    let out_path = args.next().unwrap();

    // search begin and end for entry
    let opt_entry_line = {
        let index_file = File::open(index_path)
                              .expect("Could not open index file");
        let index = BufReader::new(index_file);

        let mut entry_line: Option<String> = None;
        for line in index.lines().map(|l| l.unwrap()) {
            if line.ends_with(entry_name.as_str()) {
                entry_line = Some(line);
            }
        }

        entry_line
    };

    let entry_line = opt_entry_line.ok_or("Tar entry not found")?;

    // parse line
    let mut entry_parts = entry_line.split_whitespace().skip(1);
    let data_begin = from_hex(entry_parts.next().unwrap())?;
    let data_end = from_hex(entry_parts.next().unwrap())?;
    let etype = parse_entry_type(entry_parts.next().unwrap())?;

    if etype != EntryType::File {
        return Err("Only files can be extracted".into())
    }

    let mut tar_file = File::open(tar_path).unwrap();
    let mut out_file = File::create(out_path).unwrap();

    let mut buf = [0u8; COPY_BUF_SIZE];
    let mut remain_len = data_end - data_begin;

    // skip to file content
    tar_file.seek(SeekFrom::Start(data_begin as u64)).unwrap();

    while remain_len > 0 {
        let read_len = cmp::min(remain_len, buf.len());
        tar_file.read_exact(&mut buf[..read_len]).unwrap();
        out_file.write(&buf[..read_len]).unwrap();
        remain_len = remain_len - read_len;
    }

    Ok(())
}

fn main() {
    // arguments (without program name)
    let mut args = env::args().skip(1);
    let subcommand = args.next();

    let res = match subcommand {
        Some(c) => match c.as_ref() {
            "index" => {
                let stdout_handle = io::stdout();
                let stdout = stdout_handle.lock();
                tar_stream::index(stdout)
            },
            "extract" => extract(&mut args),
            "help" => Ok(help()),
            _ => Ok(help())
        },
        None => Ok(help())
    };

    let exit_code = match res {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    };

    process::exit(exit_code);
}
