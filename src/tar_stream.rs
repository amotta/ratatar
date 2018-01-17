use std::cmp::min;
use std::io::{self, BufRead, Write};
use std::str;

use index;
use tar;
use util::Result;

#[derive(Debug)]
enum ProcState {
    ParseHeader,
    ReadLongName(usize),
    SkipFileContent(usize),
    EndOfArchive
}

pub fn index<T: Write>(mut stdout: T) -> Result<()> {
    let stdin_handle = io::stdin();
    let mut stdin = stdin_handle.lock();

    // buffers
    let mut long_name_buf = Vec::with_capacity(tar::BLOCK_SIZE);
    let mut rat_header_begin: usize = 0;

    // file-wide state
    let mut proc_state = ProcState::ParseHeader;
    let mut offset: usize = 0;

    loop {
        let consumed = match stdin.fill_buf() {
            Ok(stdin_buf) => {
                let mut buf_off: usize = 0;
                let mut long_name_slice = None;

                while stdin_buf.len() - buf_off >= tar::BLOCK_SIZE {
                    match proc_state {
                        ProcState::ParseHeader => {
                            let buf = &stdin_buf[buf_off..(buf_off + tar::BLOCK_SIZE)];
                            buf_off += tar::BLOCK_SIZE;

                            // check format
                            let format = tar::parse_format(buf);
                            if format == tar::Format::Gnu {
                                // nothing to do
                            } else if buf.iter().all(|b| *b == 0) {
                                proc_state = ProcState::EndOfArchive;
                                continue;
                            } else {
                                return Err("Only the GNU tar format is supported".into());
                            }

                            // parse type flag
                            let type_flag = tar::parse_type_flag(buf);

                            // decide what to do next
                            proc_state = match type_flag {
                                tar::TypeFlag::Unsupported(_) => {
                                    return Err("Encountered unsupported TAR block".into());
                                },
                                tar::TypeFlag::GnuLongPathName => {
                                    ProcState::ReadLongName(tar::parse_size(buf)?)
                                },
                                _ => {
                                    let file_len = match type_flag {
                                        tar::TypeFlag::RegularFile => tar::parse_size(buf)?,
                                        _ => 0
                                    };

                                    {
                                        let file_name = if long_name_slice.is_some() {
                                            long_name_slice.unwrap()
                                        } else if !long_name_buf.is_empty() {
                                            long_name_buf.as_slice()
                                        } else {
                                            tar::parse_name(buf)
                                        };

                                        let mut len = file_name.len();
                                        while len > 0 && file_name[len - 1] == 0 {
                                            len = len - 1;
                                        }

                                        let rat_data_begin = offset + buf_off;
                                        let rat_data_end = rat_data_begin + file_len;

                                        let rat_etype = match type_flag {
                                            tar::TypeFlag::RegularFile => index::EntryType::File,
                                            tar::TypeFlag::Directory => index::EntryType::Directory,
                                            _ => unreachable!()
                                        };

                                        let rat_name = unsafe {
                                            str::from_utf8_unchecked(&file_name[..len])
                                        };

                                        let rat_entry = index::Entry {
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
                                        tar::TypeFlag::RegularFile => {
                                            let file_blocks = tar::bytes_to_blocks(file_len);
                                            ProcState::SkipFileContent(file_blocks * tar::BLOCK_SIZE)
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
                                buf_off += tar::bytes_to_blocks(skip_len) * tar::BLOCK_SIZE;
                            }
                        },
                        ProcState::EndOfArchive => {
                            let buf = &stdin_buf[buf_off..(buf_off + tar::BLOCK_SIZE)];

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
