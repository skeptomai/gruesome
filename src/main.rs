#![allow(dead_code)]
#![allow(unused_imports)]

use bitreader::{BitReader, BitReaderError};
use bitvec::prelude::*;
use rand::distributions::{Distribution, Uniform};
use rand::prelude::ThreadRng;
use rand::Rng;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::fs::File;
use std::io;
use std::io::prelude::*;

const MAX_PROPERTIES: u16 = 32;

// NOTE: this is only up to v3
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Zobject {
    pub attribute_bits: [u8; 4],
    pub parent: u8,
    pub next: u8,
    pub child: u8,
    pub property_offset: [u8; 2],
}

impl Zobject {
    pub fn new(bytes: &[u8]) -> Zobject {
        let sz = std::mem::size_of::<Zobject>();
        let (_prefix, zobj, _suffix) = unsafe { &bytes[0..sz].align_to::<Zobject>() };
        zobj[0].clone()
    }

    pub fn attributes(&self) -> Vec<u8> {
        let mut attrs = vec![];
        let mut index = 0;
        for i in self.attribute_bits {
            let mut mask = 0x80;

            for _j in 0..8 {
                let r = mask & i;
                if r != 0 {
                    attrs.push(index);
                }
                mask >>= 1;
                index += 1;
            }
        }
        attrs
    }

    pub fn properties_addr(&self) -> u16 {
        u16::from_be_bytes(self.property_offset)
    }
}

impl Display for Zobject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Attributes: {:?}, Parent: {}, Next: {}, Child: {}, Properties Address {:#04x}",
            self.attributes(),
            self.parent,
            self.next,
            self.child,
            self.properties_addr()
        )
    }
}

pub struct ZobjectPostV3 {
    pub attributes: [u16; 3],
    pub parent: u16,
    pub next: u16,
    pub child: u16,
    pub property_offset: u16,
}

enum RandMode {
    Predictable,
    RandomUniform,
}

#[derive(PartialEq)]
enum Alphabets {
    A0,
    A1,
    A2,
}

#[derive(Debug)]
pub struct GameMemoryMap {
    pub header_addr: u16,
    pub abbrev_strings: u16,
    pub abbrev_table: u16,
    pub property_defaults: u16,
    pub object_table: u16,
    pub global_variables: u16,
}

impl Display for GameMemoryMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "
            base\tend\tsize
            {:#04x}\t{:#04x}\t{:#04x}     Story file header
            {:#04x}\t{:#04x}\t{:#04x}     Abbreviation data
            {:#04x}\t{:#04x}\t{:#04x}     Abbreviation pointer table
            {:#04x}\t{:#04x}\t{:#04x}     Object table
        ",
            self.header_addr,
            self.header_addr + 0x40 - 1,
            self.header_addr + 0x40,
            self.abbrev_strings,
            self.abbrev_table - 1,
            self.abbrev_table - self.abbrev_strings,
            self.abbrev_table,
            self.object_table - 1,
            self.object_table - self.abbrev_table,
            self.object_table,
            self.global_variables - 1,
            self.global_variables,
        )
    }
}

pub struct GameFile<'a> {
    bytes: &'a [u8],
    header: Header,
    rand_mode: RandMode,
    current_alphabet: Alphabets,
    rng: &'a mut ThreadRng,
    memory_map: GameMemoryMap,
}

impl<'a> GameFile<'a> {
    pub fn new(bytes: &'a Vec<u8>, rng: &'a mut ThreadRng) -> GameFile<'a> {
        // initialize header as first $40 == 60 dec bytes
        let header = Header::new(bytes);
        let memory_map: GameMemoryMap = GameMemoryMap {
            header_addr: 0,
            abbrev_strings: 0x40,
            abbrev_table: header.abbrev_table,
            property_defaults: header.object_table_addr,
            object_table: header.object_table_addr,
            global_variables: header.global_variables,
        };
        let g = GameFile {
            bytes: &bytes,
            header: header,
            rng: rng,
            rand_mode: RandMode::RandomUniform,
            current_alphabet: Alphabets::A0,
            memory_map: memory_map,
        };
        g
    }

    pub fn default_properties(&self) -> PropertyDefaults {
        let prop_raw = &self.bytes[self.header.object_table_addr as usize
            ..(self.header.object_table_addr + MAX_PROPERTIES * 2) as usize];
        PropertyDefaults { prop_raw: prop_raw }
    }

    pub fn objects(&self) -> ObjectTable {
        let object_table = ObjectTable {
            obj_raw: &self.bytes
                [(self.header.object_table_addr + (MAX_PROPERTIES - 1) * 2) as usize..],
        };
        object_table
    }

    pub fn unpack_addr(paddr: u16, game_version: u8) -> Option<u16> {
        // TODO: fix for versions > 5
        if game_version > 0 && game_version < 4 {
            Some(2 * paddr)
        } else if game_version > 3 && game_version < 6 {
            Some(4 * paddr)
        } else {
            None
        }
    }

    pub fn gen_unsigned_rand(&mut self) -> u16 {
        // NOTE: This could probably be (u16::MAX +1) / 2
        self.rng.gen_range(0..32768)
    }

    pub fn change_alphabet(&mut self, zchar: u8) {
        self.current_alphabet = match self.current_alphabet {
            Alphabets::A0 => {
                if zchar == 4 {
                    Alphabets::A1
                } else {
                    Alphabets::A2
                }
            }
            Alphabets::A1 => {
                if zchar == 4 {
                    Alphabets::A2
                } else {
                    Alphabets::A0
                }
            }
            Alphabets::A2 => {
                if zchar == 4 {
                    Alphabets::A0
                } else {
                    Alphabets::A1
                }
            }
        }
    }

    pub fn abbrev_string(abbrev_code: u8, abbrev_index: u8) -> u8 {
        32 * (abbrev_code - 1) + abbrev_index
    }

    pub fn read_zchars(word: &[u8; 2]) -> Result<Vec<u8>, BitReaderError> {
        let mut br = BitReader::new(word);
        br.read_u8(1).unwrap();
        let mut brv = vec![];
        for _i in 0..2 {
            brv.push(br.read_u8(5));
        }
        brv.into_iter().collect()
    }
}

impl<'a> Display for GameFile<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "header:
            {}
            memory map:
            {}",
            self.header, self.memory_map
        )
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
    pub object_table_addr: u16,
    pub global_variables: u16,
}

// In Versions 1 to 3, there are at most 255 objects, each having a 9-byte entry as follows
#[derive(Debug)]
pub struct ObjectTree {}

pub struct PropertyDefaults<'a> {
    pub prop_raw: &'a [u8], // 31 words in Versions 1-3. 63 words in Versions 4 or later.
}

impl<'a> Display for PropertyDefaults<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        for i in 0..MAX_PROPERTIES - 1 {
            write!(
                f,
                "{} ",
                get_mem_addr(&self.prop_raw[(i * 2) as usize..], 2)
            )
            .unwrap();
        }
        Ok(())
    }
}

impl<'a> Debug for PropertyDefaults<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        for i in 0..MAX_PROPERTIES - 1 {
            write!(
                f,
                "{} ",
                get_mem_addr(&self.prop_raw[(i * 2) as usize..], 2)
            )
            .unwrap();
        }
        Ok(())
    }
}

impl<'a> PropertyDefaults<'a> {
    pub fn property(&self, index: usize) -> u16 {
        //BUGBUG no range checking here [cb]
        //BUGBUG this is just repeated code from get_mem_addr. Factor out to util
        let ins_bytes = <[u8; 2]>::try_from(&self.prop_raw[index..index + 2]).unwrap();
        let ins = u16::from_be_bytes(ins_bytes);
        ins
    }
}

pub struct ObjectTable<'a> {
    pub obj_raw: &'a [u8],
}

impl<'a> Display for ObjectTable<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "fuck object table")
    }
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

fn main() -> io::Result<()> {
    let mut f = File::open("./zork1/DATA/ZORK1.DAT")?;
    let mut all_bytes = Vec::new();
    let mut rng = rand::thread_rng();

    f.read_to_end(&mut all_bytes).unwrap();

    let g = GameFile::new(&all_bytes, &mut rng);
    println!("{}", g);

    let _ot = g.objects();
    println!("object table? {}", _ot);
    println!("default properties {:?}", g.default_properties());
    let _raw_object_bytes = &_ot.obj_raw;
    let _zobj = Zobject::new(_raw_object_bytes);
    println!("maybe an object? {}", _zobj);
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

/*
 Z-char 6789abcdef0123456789abcdef
current   --------------------------
  A0      abcdefghijklmnopqrstuvwxyz
  A1      ABCDEFGHIJKLMNOPQRSTUVWXYZ
  A2       ^0123456789.,!?_#'"/\-:()
          --------------------------
*/
