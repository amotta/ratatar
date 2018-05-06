use std::str;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use util::{from_hex, Result};

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

fn parse_entry_type(data: &str) -> Result<EntryType> {
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

pub fn read_entry_line(index_path: &str, entry_name: &str) -> Result<String> {
    let index_file = File::open(index_path.to_owned())
        .or(Err(format!("Failed to open index file '{}'", index_path)))?;
    let index_reader = BufReader::new(index_file);

    let mut entry_line = None;
    for maybe_line in index_reader.lines() {
        let line = maybe_line?;
        if line.ends_with(entry_name) {
            entry_line = Some(line);
        }
    }

    entry_line.ok_or(format!(
        "No entry for '{}' in index file", entry_name).into())
}

pub fn parse_entry_line(entry_line: &str) -> Result<Entry> {
    // split at white space and skip header address
    let mut entry_parts = entry_line.splitn(5, ' ');

    // extract fields
    let header_begin = from_hex(entry_parts.next().unwrap()).unwrap();
    let data_begin = from_hex(entry_parts.next().unwrap()).unwrap();
    let data_end = from_hex(entry_parts.next().unwrap()).unwrap();
    let etype = parse_entry_type(entry_parts.next().unwrap()).unwrap();
    let name = entry_parts.next().unwrap();

    Ok(Entry {
        header_begin: header_begin,
        data_begin: data_begin,
        data_end: data_end,
        etype: etype,
        name: name
    })
}
