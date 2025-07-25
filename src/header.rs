use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use crate::util::get_mem_addr;

pub struct Header {
    pub version: u8,
    pub release: u16,
    pub serial: String,
    pub base_high_mem: usize,
    pub base_static_mem: usize,
    pub initial_pc: usize,
    pub abbrev_table: usize,
    pub len_file: usize,
    pub checksum_file: usize,
    pub standard_revision_number: usize,
    pub interpreter_number_and_version: usize,
    pub dictionary: usize,
    pub object_table_addr: usize,
    pub global_variables: usize,
}

impl Header {
    pub fn new(bytes: &[u8]) -> Header {
        Header {
            version: bytes[0],
            release: (bytes[2] as u16) * 256 + (bytes[3] as u16),
            serial: {
                let mut serial: String = String::from("");
                for b in &bytes[0x12..0x18] {
                    serial.push(*b as char);
                }
                serial
            },
            base_high_mem: get_mem_addr(bytes, 4).unwrap(),
            base_static_mem: get_mem_addr(bytes, 14).unwrap(),
            initial_pc: get_mem_addr(bytes, 6).unwrap(),
            abbrev_table: get_mem_addr(bytes, 0x18).unwrap(),
            len_file: get_mem_addr(bytes, 0x1A).unwrap() as usize * 2,
            checksum_file: get_mem_addr(bytes, 0x1C).unwrap(),
            standard_revision_number: get_mem_addr(bytes, 0x32).unwrap(),
            interpreter_number_and_version: get_mem_addr(bytes, 0x1e).unwrap(),
            dictionary: get_mem_addr(bytes, 0x08).unwrap(),
            object_table_addr: get_mem_addr(bytes, 0x0A).unwrap(),
            global_variables: get_mem_addr(bytes, 0x0C).unwrap(),
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
