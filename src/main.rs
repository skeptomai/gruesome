use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::fs::File;
use std::io;
use std::io::prelude::*;

pub struct GameFile {
    header: Header,
}

impl GameFile {
    pub fn new(bytes: Vec<u8>) -> GameFile {
        // initialize header as first $40 == 60 dec bytes
        GameFile {
            header: Header::new(&bytes),
        }
    }
}

impl Display for GameFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Gamefile header {}", self.header)
    }
}

pub fn get_mem_addr(addr: &[u8], counter: usize) -> u16 {
    let ins_bytes = <[u8; 2]>::try_from(&addr[counter..counter + 2]).unwrap();
    let ins = u16::from_be_bytes(ins_bytes);
    ins
}
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
            base_static_mem: get_mem_addr(bytes, 15),
            initial_pc: get_mem_addr(bytes, 6),
            abbrev_table: get_mem_addr(bytes, 0x18),
            len_file: get_mem_addr(bytes, 0x1A) as usize * 2,
            checksum_file: get_mem_addr(bytes, 0x1C),
            standard_revision_number: get_mem_addr(bytes, 0x32),
            interpreter_number_and_version: get_mem_addr(bytes, 0x1e),
        }
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Version is {}
Full Release is { }.{}
Base static: {:#06x}
Base high: {:#06x} 
Initial PC: {:#06x}
Abbrev Table: {:#06x}
Length of file (3+): {},
Checksum of file (3+): {:#06x},
Standard revision number: {:#06x},
Interpreter and Version (v4): {:#06x}
",
            self.version,
            self.release,
            self.serial,
            self.base_static_mem,
            self.base_high_mem,
            self.initial_pc,
            self.abbrev_table,
            self.len_file,
            self.checksum_file,
            self.standard_revision_number,
            self.interpreter_number_and_version
        )
    }
}

fn main() -> io::Result<()> {
    let mut f = File::open("./zork1/DATA/ZORK1.DAT")?;
    let mut all_bytes = Vec::new();

    f.read_to_end(&mut all_bytes).unwrap();

    let g = GameFile::new(all_bytes);
    println!("Gamefile: {}", g);
    Ok(())
}
