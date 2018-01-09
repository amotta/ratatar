use std::{cmp, env, fmt, str};
use std::fs::File;
use std::io::{self, Read, BufRead, BufReader, Seek, SeekFrom, Write};

const BLOCK_SIZE: usize = 512;
const GNU_MAGIC: &str = "ustar ";

#[derive(Debug)]
enum TypeFlag {
    RegularFile,
    Directory,
    GnuLongPathName,
    Unsupported(char)
}

#[derive(Debug)]
enum RatEntryType {
    File,
    Directory,
    Unknown
}

impl fmt::Display for RatEntryType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            RatEntryType::File => "F",
            RatEntryType::Directory => "D",
            RatEntryType::Unknown => "?"
        })
    }
}

#[derive(Debug)]
struct RatEntry<'a> {
    header_begin: usize,
    data_begin: usize,
    data_end: usize,
    etype: RatEntryType,
    name: &'a str
}

impl<'a> fmt::Display for RatEntry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f, "{:#018x} {:#018x} {:#018x} {} {}",
            self.header_begin, self.data_begin, self.data_end, self.etype, self.name)
    }
}

#[derive(Debug)]
enum ProcState {
    ParseHeader,
    ReadLongName(usize),
    SkipFileContent(usize),
    EndOfArchive
}

fn index() {
    let stdin_handle = io::stdin();
    let mut stdin = stdin_handle.lock();

    let stdout_handle = io::stdout();
    let mut stdout = stdout_handle.lock();

    // buffers
    let mut long_name_buf = Vec::with_capacity(BLOCK_SIZE);
    let mut rat_header_begin: usize = 0;

    // file-wide state
    let mut proc_state = ProcState::ParseHeader;
    let mut offset: usize = 0;

    loop {
        let consumed = match stdin.fill_buf() {
            Ok(stdin_buf) => {
                let mut buf_off: usize = 0;
                let mut long_name_slice = None;

                while stdin_buf.len() - buf_off >= BLOCK_SIZE {
                    match proc_state {
                        ProcState::ParseHeader => {
                            let buf = &stdin_buf[buf_off..(buf_off + BLOCK_SIZE)];
                            buf_off += BLOCK_SIZE;

                            // check format magic
                            let magic = &buf[257..(257 + 6)];

                            if magic == GNU_MAGIC.as_bytes() {
                                // nothing to do
                            } else if buf.iter().all(|b| *b == 0) {
                                proc_state = ProcState::EndOfArchive;
                                continue;
                            } else {
                                panic!("Only the GNU tar format is supported");
                            }

                            // parse type flag
                            let type_flag = parse_type_flag(buf[156]);

                            // decide what to do next
                            proc_state = match type_flag {
                                TypeFlag::Unsupported(_) => panic!(),
                                TypeFlag::GnuLongPathName => {
                                    let name_len = parse_size(&buf[124..(124 + 12)]);
                                    ProcState::ReadLongName(name_len)
                                },
                                _ => {
                                    let file_len = match type_flag {
                                        TypeFlag::RegularFile => parse_size(&buf[124..(124 + 12)]),
                                        _ => 0
                                    };

                                    {
                                        let file_name = if long_name_slice.is_some() {
                                            long_name_slice.unwrap()
                                        } else if !long_name_buf.is_empty() {
                                            long_name_buf.as_slice()
                                        } else {
                                            &buf[..100]
                                        };

                                        let mut len = file_name.len();
                                        while len > 0 && file_name[len - 1] == 0 {
                                            len = len - 1;
                                        }

                                        let rat_data_begin = offset + buf_off;
                                        let rat_data_end = rat_data_begin + file_len;

                                        let rat_etype = match type_flag {
                                            TypeFlag::RegularFile => RatEntryType::File,
                                            TypeFlag::Directory => RatEntryType::Directory,
                                            _ => RatEntryType::Unknown
                                        };

                                        let rat_name = unsafe {
                                            str::from_utf8_unchecked(&file_name[..len])
                                        };

                                        let rat_entry = RatEntry {
                                            header_begin: rat_header_begin,
                                            data_begin: rat_data_begin,
                                            data_end: rat_data_end,
                                            etype: rat_etype,
                                            name: rat_name
                                        };

                                        write!(stdout, "{}\n", rat_entry).unwrap();
                                    }

                                    // prepare for next entry
                                    long_name_slice = None;
                                    long_name_buf.clear();

                                    match type_flag {
                                        TypeFlag::RegularFile => {
                                            let file_blocks = bytes_to_blocks(file_len);
                                            ProcState::SkipFileContent(file_blocks * BLOCK_SIZE)
                                        },
                                        _ => {
                                            rat_header_begin = offset + buf_off;
                                            ProcState::ParseHeader
                                        }
                                    }
                                },
                            }
                        },
                        ProcState::SkipFileContent(len) => {
                            let skip_len = cmp::min(len, stdin_buf.len() - buf_off);
                            buf_off += skip_len;

                            if skip_len < len {
                                proc_state = ProcState::SkipFileContent(len - skip_len);
                            } else {
                                rat_header_begin = offset + buf_off;
                                proc_state = ProcState::ParseHeader;
                            }
                        },
                        ProcState::ReadLongName(len) => {
                            let skip_len = cmp::min(len, stdin_buf.len() - buf_off);
                            long_name_slice = Some(&stdin_buf[buf_off..(buf_off + skip_len)]);

                            if skip_len < len {
                                proc_state = ProcState::ReadLongName(len - skip_len);
                                buf_off += skip_len;
                            } else {
                                proc_state = ProcState::ParseHeader;
                                buf_off += bytes_to_blocks(skip_len) * BLOCK_SIZE;
                            }
                        },
                        ProcState::EndOfArchive => {
                            let buf = &stdin_buf[buf_off..(buf_off + BLOCK_SIZE)];

                            if buf.iter().all(|b| *b == 0) {
                                return;
                            } else {
                                panic!();
                            }
                        }
                    }
                }

                // If `long_path_slice` is set, it must be added to the buffer.
                if let Some(s) = long_name_slice {
                    long_name_buf.extend_from_slice(s);
                }

                buf_off
            },
            Err(err) => panic!("Error: {:?}", err)
        };

        stdin.consume(consumed);
        offset += consumed;
    }
}

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

fn extract<I>(args: &mut I)
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

    let entry_line = opt_entry_line.expect("Tar entry not found");

    // parse line
    let mut entry_parts = entry_line.split_whitespace().skip(1);
    let data_begin = from_hex(entry_parts.next().unwrap());
    let data_end = from_hex(entry_parts.next().unwrap());
    let etype = entry_parts.next().unwrap();

    if etype != "F" {
        panic!("Only files can be extracted");
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
}

fn from_hex(data: &str) -> usize {
    let mut hex: usize = 0;

    for c in data.bytes().skip(2) {
        let v = match c {
            b'0'...b'9' => c - b'0',
            b'A'...b'F' => c - b'A' + 10,
            b'a'...b'f' => c - b'a' + 10,
            _ => panic!("Invalid hex symbol")
        };

        hex = (16 * hex) + (v as usize);
    }

    hex
}

fn main() {
    // arguments (without program name)
    let mut args = env::args().skip(1);
    let subcommand = args.next();

    match subcommand {
        Some(c) => match c.as_ref() {
            "index" => index(),
            "extract" => extract(&mut args),
            "help" => help(),
            _ => help()
        },
        None => help()
    }
}

fn bytes_to_blocks(bytes: usize) -> usize {
    if bytes > 0 {
        1 + (bytes - 1) / BLOCK_SIZE
    } else {
        0
    }
}

fn parse_type_flag(data: u8) -> TypeFlag {
    match data as char {
        '0' | '\0' | '7' => TypeFlag::RegularFile,
        '5' => TypeFlag::Directory,
        'L' => TypeFlag::GnuLongPathName,
         c  => TypeFlag::Unsupported(c)
    }
}

fn parse_size(data: &[u8]) -> usize {
    if data[0] & (1 << 7) != 0 {
        panic!("binary size format not supported")
    } else {
        parse_size_octal(data)
    }
}

fn parse_size_octal(octals: &[u8]) -> usize {
    let mut size: usize = 0;
    let len = octals.len();

    for octal in &octals[..(len - 1)] {
        if *octal >= 48 && *octal <= 57 {
            size = size * 8 + (*octal - 48) as usize;
        } else {
            panic!("{} byte not a valid ASCII digit", octal)
        }
    }

    if octals[len - 1] != 0 && octals[len - 1] != 32 {
        // The POSIX standard requires a NUL or space as termination character.
        panic!("{} byte is not a valid size termination", octals[len - 1])
    }

    size
}
