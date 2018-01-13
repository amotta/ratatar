use util::Result;

pub const BLOCK_SIZE: usize = 512;
const GNU_MAGIC: &str = "ustar ";

#[derive(PartialEq)]
pub enum Format {
    Gnu,
    Unsupported
}

#[derive(Debug)]
pub enum TypeFlag {
    RegularFile,
    Directory,
    GnuLongPathName,
    Unsupported(char)
}

pub fn parse_format(block: &[u8]) -> Format {
    let format = &block[257..(257 + 6)];
    if format == GNU_MAGIC.as_bytes() {
        Format::Gnu
    } else {
        Format::Unsupported
    }
}

pub fn bytes_to_blocks(bytes: usize) -> usize {
    if bytes > 0 {
        1 + (bytes - 1) / BLOCK_SIZE
    } else {
        0
    }
}

pub fn parse_type_flag(block: &[u8]) -> TypeFlag {
    match block[156] as char {
        '0' | '\0' | '7' => TypeFlag::RegularFile,
        '5' => TypeFlag::Directory,
        'L' => TypeFlag::GnuLongPathName,
         c  => TypeFlag::Unsupported(c)
    }
}

pub fn parse_name(block: &[u8]) -> &[u8] {
    &block[..100]
}

pub fn parse_size(block: &[u8]) -> Result<usize> {
    let size = &block[124..(124 + 12)];

    if size[0] & (1 << 7) != 0 {
        Err("The binary size format is not supported yet".into())
    } else {
        parse_size_octal(size)
    }
}

fn parse_size_octal(octals: &[u8]) -> Result<usize> {
    let mut size: usize = 0;
    let len = octals.len();

    for octal in &octals[..(len - 1)] {
        if *octal >= b'0' && *octal <= b'8' {
            size = size * 8 + (*octal - 48) as usize;
        } else {
            return Err("Found invalid byte in size field".into())
        }
    }

    if octals[len - 1] != 0 && octals[len - 1] != 32 {
        // The POSIX standard requires a NUL or space as termination character.
        return Err("Size field is not properly terminated".into())
    }

    Ok(size)
}
