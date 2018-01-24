use std::{env, io, process};

mod extract;
mod index;
mod tar;
mod tar_stream;
mod util;

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
            "extract" => extract::extract(&mut args),
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
