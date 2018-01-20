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
    EndOfArchive,
    Done
}

struct State {
    entry_begin: usize,
    long_name: Vec<u8>
}

pub fn index<T: Write>(mut out: T) -> Result<()> {
    let stdin_handle = io::stdin();
    let mut stdin = stdin_handle.lock();

    // buffers
    let mut state = State {
        entry_begin: 0,
        long_name: Vec::with_capacity(tar::BLOCK_SIZE)
    };

    // file-wide state
    let mut proc_state = ProcState::ParseHeader;
    let mut offset: usize = 0;

    loop {
        let consumed = match stdin.fill_buf() {
            Ok(stdin_buf) => {
                let stdin_offset = offset;
                let mut long_name = None;

                let consumed = loop {
                    let buf_off = offset - stdin_offset;
                    let buf = &stdin_buf[buf_off..];

                    if buf.len() < tar::BLOCK_SIZE {
                        // need more data to continue
                        break buf_off;
                    }

                    let (consumed, next_proc_state) = match proc_state {
                        ProcState::ParseHeader => {
                            let result = parse_header(buf, offset, long_name, &state, &mut out);

                            // prepare for next entry
                            let skip_len = match result {
                                Ok((_, ProcState::SkipFileContent(len))) => len,
                                _ => 0
                            };

                            state.entry_begin = offset + tar::BLOCK_SIZE + skip_len;
                            state.long_name.clear();
                            long_name = None;

                            result
                        },
                        ProcState::SkipFileContent(len) => skip_file_content(buf, len),
                        ProcState::ReadLongName(len) => read_long_name(buf, len, &mut long_name),
                        ProcState::EndOfArchive => end_of_archive(buf),
                        ProcState::Done => return Ok(())
                    }?;

                    offset += consumed;
                    proc_state = next_proc_state;
                };

                // If `long_path_slice` is set, it must be added to the buffer.
                if let Some(s) = long_name {
                    state.long_name.extend_from_slice(s);
                }

                consumed
            },
            Err(_) => return Err("Failed to fill input buffer".into())
        };

        stdin.consume(consumed);
    }
}

fn parse_header<T: Write>(
    buf: &[u8], offset: usize, long_name: Option<&[u8]>, state: &State, out: &mut T
) -> Result<(usize, ProcState)> {
    // check format
    let format = tar::parse_format(buf);
    if format == tar::Format::Gnu {
        // nothing to do
    } else if (&buf[..tar::BLOCK_SIZE]).iter().all(|b| *b == 0) {
        return Ok((tar::BLOCK_SIZE, ProcState::EndOfArchive));
    } else {
        return Err("Only the GNU tar format is supported".into());
    }

    // parse type flag
    let type_flag = tar::parse_type_flag(buf);

    // decide what to do next
    let next_proc_state = match type_flag {
        tar::TypeFlag::Unsupported(_) => {
            return Err("Encountered unsupported TAR block".into());
        },
        tar::TypeFlag::GnuLongPathName => {
            let long_name_len = tar::parse_size(buf)?;
            ProcState::ReadLongName(long_name_len)
        },
        _ => {
            let file_len = match type_flag {
                tar::TypeFlag::RegularFile => tar::parse_size(buf)?,
                _ => 0
            };

            {
                let file_name = if long_name.is_some() {
                    long_name.unwrap()
                } else if !state.long_name.is_empty() {
                    state.long_name.as_slice()
                } else {
                    tar::parse_name(buf)
                };

                let mut len = file_name.len();
                while len > 0 && file_name[len - 1] == 0 {
                    len = len - 1;
                }

                let rat_header_begin = state.entry_begin;
                let rat_data_begin = offset + tar::BLOCK_SIZE;
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

                write!(out, "{}\n", rat_entry).unwrap();
            }

            match type_flag {
                tar::TypeFlag::RegularFile => {
                    let skip_len = tar::bytes_to_blocks(file_len) * tar::BLOCK_SIZE;
                    ProcState::SkipFileContent(skip_len)
                },
                _ => ProcState::ParseHeader
            }
        }
    };

    Ok((tar::BLOCK_SIZE, next_proc_state))
}

fn skip_file_content(buf: &[u8], len: usize) -> Result<(usize, ProcState)> {
    let skip_len = min(len, buf.len());

    if skip_len < len {
        Ok((skip_len, ProcState::SkipFileContent(len - skip_len)))
    } else {
        Ok((skip_len, ProcState::ParseHeader))
    }
}

fn read_long_name<'a>(
    buf: &'a [u8], len: usize, out: &mut Option<&'a [u8]>
) -> Result<(usize, ProcState)> {
    let skip_len = min(len, buf.len());
    *out = Some(&buf[..skip_len]);

    if skip_len < len {
        Ok((skip_len, ProcState::ReadLongName(len - skip_len)))
    } else {
        let skip_len = tar::bytes_to_blocks(skip_len) * tar::BLOCK_SIZE;
        Ok((skip_len, ProcState::ParseHeader))
    }
}

fn end_of_archive(buf: &[u8]) -> Result<(usize, ProcState)> {
    if (&buf[..tar::BLOCK_SIZE]).iter().all(|b| *b == 0) {
        Ok((tar::BLOCK_SIZE, ProcState::Done))
    } else {
        Err("Found lone zero block".into())
    }
}
