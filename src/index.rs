use std::str;
use std::fmt;

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
pub struct Entry<'a> {
    pub header_begin: usize,
    pub data_begin: usize,
    pub data_end: usize,
    pub etype: EntryType,
    pub name: &'a str
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
