use crate::header::Header;

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use bitreader::{BitReader, BitReaderError};
use bitvec::prelude::*;

use rand::distributions::{Distribution, Uniform};
use rand::prelude::ThreadRng;
use rand::Rng;

use crate::property_defaults::PropertyDefaults;
use crate::zobject::Zobject;
use crate::zobject::{ObjectTable, ObjectTree};

pub const MAX_PROPERTIES: u16 = 32;

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
            object_table: header.object_table_addr + (MAX_PROPERTIES - 1) * 2,
            properties_table: 0,
            global_variables: header.global_variables,
        };
        let mut g = GameFile {
            bytes: &bytes,
            header: header,
            rng: rng,
            rand_mode: RandMode::RandomUniform,
            current_alphabet: Alphabets::A0,
            memory_map: memory_map,
        };

        let _ot = g.objects();
        let _raw_object_bytes = &_ot.obj_raw;
        let _zobj = Zobject::new(_raw_object_bytes);
        g.memory_map.properties_table = _zobj.properties_addr();

        g
    }

    pub fn default_properties(&self) -> PropertyDefaults {
        let prop_raw = &self.bytes[self.memory_map.property_defaults as usize
            ..(self.memory_map.property_defaults + MAX_PROPERTIES * 2) as usize];
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

#[derive(Debug)]
pub struct GameMemoryMap {
    pub header_addr: u16,
    pub abbrev_strings: u16,
    pub abbrev_table: u16,
    pub property_defaults: u16,
    pub object_table: u16,
    pub properties_table: u16,
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
            {:#04x}\t{:#04x}\t{:#04x}     Properties data
            {:#04x}\t{:#04x}\t{:#04x}     Global variables
        ",
            // File header
            self.header_addr,
            self.header_addr + 0x40 - 1,
            self.header_addr + 0x40,
            // Abbreviation data
            self.abbrev_strings,
            self.abbrev_table - 1,
            self.abbrev_table - self.abbrev_strings,
            // Abbreviation pointer table
            self.abbrev_table,
            self.object_table - 1,
            self.object_table - self.abbrev_table,
            // Object table
            self.object_table,
            self.properties_table - 1,
            self.properties_table - self.object_table,
            // Properties table
            self.properties_table,
            self.global_variables - 1,
            self.global_variables - self.properties_table,
            // Global variables
            self.global_variables,
            self.global_variables,
            self.global_variables,
        )
    }
}
