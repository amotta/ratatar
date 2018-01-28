use std::cmp;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use index::{self, EntryType};
use util::Result;

const COPY_BUF_SIZE: usize = 8192;

pub fn extract_file_to<R, W>(
    mut reader: R, mut writer: W, mut len: usize
) -> Result<()>
where R: Read, W: Write {
    let mut buf = [0u8; COPY_BUF_SIZE];

    while len > 0 {
        let read_len = cmp::min(len, buf.len());
        reader.read_exact(&mut buf[..read_len])
            .or(Err("Failed to read from tar file"))?;
        writer.write(&buf[..read_len])
            .or(Err("Failed to write to output"))?;
        len = len - read_len;
    }

    Ok(())
}

pub fn extract<I>(args: &mut I) -> Result<()>
where I: Iterator<Item = String> {
    let tar_path = args.next().ok_or("Path to tar file missing")?;
    let entry_name = args.next().ok_or("Tar entry name missing")?;
    let out_path = args.next().ok_or("Path to output file missing")?;

    let index_path = {
        let mut p = tar_path.clone();
        p.push_str(".index");
        p
    };

    let index_entry_line = index::read_entry_line(&index_path, &entry_name)?;
    let (data_begin, data_end, etype) = index::parse_entry_line(&index_entry_line)?;

    if etype != EntryType::File {
        return Err(format!("Entry '{}' is not a file \
            and thus cannot be extracted", &entry_name).into())
    }

    let mut tar_file = File::open(tar_path.to_owned())
        .or(Err(format!("Failed to open '{}'", &tar_path)))?;
    tar_file.seek(SeekFrom::Start(data_begin as u64))
        .or(Err(format!("Failed to seek to offset {}", data_begin)))?;

    let len = data_end - data_begin;

    if out_path == "-" {
        let stdout_handle = io::stdout();
        let stdout = stdout_handle.lock();
        extract_file_to(tar_file, stdout, len)
    } else {
        let out_file = File::create(out_path.to_owned())
            .or(Err(format!("Failed to create '{}'", &out_path)))?;
        extract_file_to(tar_file, out_file, len)
    }
}
