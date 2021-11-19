use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use crate::game::get_mem_addr;

pub struct Header {
    pub version: u8,
    pub release: u16,
    pub serial: String,
    pub base_high_mem: u16,
    pub base_static_mem: u16,
    pub initial_pc: u16,
    pub abbrev_table: u16,
    pub len_file: usize,
    pub checksum_file: u16,
    pub standard_revision_number: u16,
    pub interpreter_number_and_version: u16,
    pub dictionary: u16,
    pub object_table_addr: u16,
    pub global_variables: u16,
}

impl Header {
    pub fn new(bytes: &Vec<u8>) -> Header {
        Header {
            version: bytes[0],
            release: (bytes[2] as u16) * 256 + (bytes[3] as u16),
            serial: || -> String {
                let mut serial: String = String::from("");
                for b in &bytes[0x12..0x18] {
                    serial.push(*b as char);
                }
                serial
            }(),
            base_high_mem: get_mem_addr(bytes, 4),
            base_static_mem: get_mem_addr(bytes, 14),
            initial_pc: get_mem_addr(bytes, 6),
            abbrev_table: get_mem_addr(bytes, 0x18),
            len_file: get_mem_addr(bytes, 0x1A) as usize * 2,
            checksum_file: get_mem_addr(bytes, 0x1C),
            standard_revision_number: get_mem_addr(bytes, 0x32),
            interpreter_number_and_version: get_mem_addr(bytes, 0x1e),
            dictionary: get_mem_addr(bytes, 0x08),
            object_table_addr: get_mem_addr(bytes, 0x0A),
            global_variables: get_mem_addr(bytes, 0x0C),
        }
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "
Z-code version:           {}
Interpreter flags:
Release number:           {}
Size of resident memory:  {:#06x}
Start PC:                 {:#06x}
Dictionary address:       {:#06x}
Object table address:     {:#06x}
Global variables address: {:#06x}
Size of dynamic memory:   {:#06x}
Game flags:               None
Serial number:            {}
Abbreviations address:    {:#06x}
File size:                {:#06x}
Checksum:                 {:#06x}
",
            self.version,
            self.release,
            self.base_high_mem,
            self.initial_pc,
            self.dictionary,
            self.object_table_addr,
            self.global_variables,
            self.base_static_mem,
            self.serial,
            self.abbrev_table,
            self.len_file,
            self.checksum_file,
        )
    }
}
