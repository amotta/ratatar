use std::result;
use std::fmt;

pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error(err)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(err: &'a str) -> Error {
        Error(err.into())
    }
}

pub type Result<T> = result::Result<T, Error>;

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
