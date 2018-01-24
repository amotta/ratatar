use std::error;
use std::result;

pub type Result<T> = result::Result<T, Box<error::Error>>;

pub fn from_hex(data: &str) -> Result<usize> {
    let mut hex: usize = 0;

    for c in data.bytes().skip(2) {
        let v = match c {
            b'0'...b'9' => c - b'0',
            b'A'...b'F' => c - b'A' + 10,
            b'a'...b'f' => c - b'a' + 10,
            _ => return Err(format!("'{}' is an invalid hex digit", c as char).into())
        };

        hex = (16 * hex) + (v as usize);
    }

    Ok(hex)
}
