use std::str;
use std::cmp::min;
use std::io::{self, BufRead, Write};
use std::fmt;

use tar::*;
use util::Result;

#[derive(Debug, PartialEq)]
pub enum EntryType {
    File,
    Directory
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Write::write_char(f, match *self {
            EntryType::File => 'F',
            EntryType::Directory => 'D'
        })
    }
}

pub fn parse_entry_type(data: &str) -> Result<EntryType> {
    match data {
        "F" => Ok(EntryType::File),
        "D" => Ok(EntryType::Directory),
         i  => Err(format!("'{}' is not a invalid index entry type", i).into())
    }
}

#[derive(Debug)]
struct Entry<'a> {
    header_begin: usize,
    data_begin: usize,
    data_end: usize,
    etype: EntryType,
    name: &'a str
}

impl<'a> fmt::Display for Entry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:#018x} {:#018x} {:#018x} {} {}",
            self.header_begin, self.data_begin,
            self.data_end, self.etype, self.name)
    }
}

#[derive(Debug)]
enum ProcState {
    ParseHeader,
    ReadLongName(usize),
    SkipFileContent(usize),
    EndOfArchive
}

pub fn index() -> Result<()> {
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

                            // check format
                            let format = parse_format(buf);
                            if format == Format::Gnu {
                                // nothing to do
                            } else if buf.iter().all(|b| *b == 0) {
                                proc_state = ProcState::EndOfArchive;
                                continue;
                            } else {
                                return Err("Only the GNU tar format is supported".into());
                            }

                            // parse type flag
                            let type_flag = parse_type_flag(buf);

                            // decide what to do next
                            proc_state = match type_flag {
                                TypeFlag::Unsupported(_) => {
                                    return Err("Encountered unsupported TAR block".into());
                                },
                                TypeFlag::GnuLongPathName => {
                                    ProcState::ReadLongName(parse_size(buf)?)
                                },
                                _ => {
                                    let file_len = match type_flag {
                                        TypeFlag::RegularFile => parse_size(buf)?,
                                        _ => 0
                                    };

                                    {
                                        let file_name = if long_name_slice.is_some() {
                                            long_name_slice.unwrap()
                                        } else if !long_name_buf.is_empty() {
                                            long_name_buf.as_slice()
                                        } else {
                                            parse_name(buf)
                                        };

                                        let mut len = file_name.len();
                                        while len > 0 && file_name[len - 1] == 0 {
                                            len = len - 1;
                                        }

                                        let rat_data_begin = offset + buf_off;
                                        let rat_data_end = rat_data_begin + file_len;

                                        let rat_etype = match type_flag {
                                            TypeFlag::RegularFile => EntryType::File,
                                            TypeFlag::Directory => EntryType::Directory,
                                            _ => unreachable!()
                                        };

                                        let rat_name = unsafe {
                                            str::from_utf8_unchecked(&file_name[..len])
                                        };

                                        let rat_entry = Entry {
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
                            let skip_len = min(len, stdin_buf.len() - buf_off);
                            buf_off += skip_len;

                            if skip_len < len {
                                proc_state = ProcState::SkipFileContent(len - skip_len);
                            } else {
                                rat_header_begin = offset + buf_off;
                                proc_state = ProcState::ParseHeader;
                            }
                        },
                        ProcState::ReadLongName(len) => {
                            let skip_len = min(len, stdin_buf.len() - buf_off);
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
                                return Ok(());
                            } else {
                                return Err("Found lone zero block".into());
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
            Err(_) => return Err("Failed to fill input buffer".into())
        };

        stdin.consume(consumed);
        offset += consumed;
    }
}
