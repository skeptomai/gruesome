use rand::distributions::{Distribution, Uniform};
use rand::prelude::ThreadRng;
#[allow(unused_imports)]
use rand::Rng;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::marker::PhantomData;

pub struct GameFile<'a> {
    bytes: PhantomData<&'a Vec<u8>>,
    header: Header,
    rng: &'a mut ThreadRng,
}

impl<'a> GameFile<'a> {
    pub fn new(bytes: &'a Vec<u8>, rng: &'a mut ThreadRng) -> GameFile<'a> {
        // initialize header as first $40 == 60 dec bytes
        GameFile {
            header: Header::new(bytes),
            bytes: PhantomData,
            rng: rng,
        }
    }

    #[allow(dead_code)]
    fn unpack_addr(paddr: u16, game_version: u8) -> Option<u16> {
        // TODO: fix for versions > 5
        if game_version > 0 && game_version < 4 {
            Some(2 * paddr)
        } else if game_version > 3 && game_version < 6 {
            Some(4 * paddr)
        } else {
            None
        }
    }
}

impl<'a> Display for GameFile<'a> {
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
    pub dictionary: u16,
    pub object_table: u16,
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
            object_table: get_mem_addr(bytes, 0x0A),
            global_variables: get_mem_addr(bytes, 0x0C),
        }
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Version is {}
Full Release is { }.{}
Length of file (3+): {},
Checksum of file (3+): {:#06x},
Standard revision number: {:#06x},
Interpreter and Version (v4): {:#06x},
Initial PC: {:#06x}

Abbrev Table: {:#06x}
Object Table: {:#06x},
Global Variables: {:#06x},
Base static / End Dynamic: {:#06x}
Dictionary: {:#06x},
Base high: {:#06x}
",
            self.version,
            self.release,
            self.serial,
            self.len_file,
            self.checksum_file,
            self.standard_revision_number,
            self.interpreter_number_and_version,
            self.initial_pc,
            self.abbrev_table,
            self.object_table,
            self.global_variables,
            self.base_static_mem,
            self.dictionary,
            self.base_high_mem,
        )
    }
}

fn main() -> io::Result<()> {
    let mut f = File::open("./zork1/DATA/ZORK1.DAT")?;
    let mut all_bytes = Vec::new();
    let mut rng = rand::thread_rng();

    f.read_to_end(&mut all_bytes).unwrap();

    let g = GameFile::new(&all_bytes, &mut rng);
    println!("Gamefile: {}", g);
    Ok(())
}

/*
An example memory map of a small game
Dynamic	00000	header
        00040	abbreviation strings
        00042	abbreviation table
        00102	property defaults
        00140	objects
        002f0	object descriptions and properties
        006e3	global variables
        008c3	arrays
Static	00b48	grammar table
        010a7	actions table
        01153	preactions table
        01201	adjectives table
        0124d	dictionary
High	01a0a	Z-code
        05d56	static strings
        06ae6	end of file
*/
