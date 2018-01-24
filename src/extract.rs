use std::cmp;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use index::{self, EntryType};
use util::Result;

const COPY_BUF_SIZE: usize = 8192;

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
    let mut out_file = File::create(out_path.to_owned())
        .or(Err(format!("Failed to create '{}'", &out_path)))?;;

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
