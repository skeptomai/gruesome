use crate::header::Header;

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use std::collections::HashMap;

use bitreader::{BitReader, BitReaderError};

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::property_defaults::PropertyDefaults;
use crate::zobject::{Zobject, ObjectTable};

type Zchar = u8;

pub const MAX_PROPERTIES: u16 = 32;

pub struct PackedChars {
    last : bool,
    chars : [Zchar;3]
}

lazy_static! {
    static ref ALPHABETMAP: HashMap<Alphabets, &'static str> = {
        let mut m = HashMap::new();
        m.insert(Alphabets::A0, "abcdefghijklmnopqrstuvwxyz");
        m.insert(Alphabets::A1, "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        m.insert(Alphabets::A2, " ^0123456789.,!?_#'\"/\\-:()");
        m
    };
}

enum RandMode {
    Predictable,
    RandomUniform,
}

#[derive(PartialEq, Eq, Hash)]
enum Alphabets {
    A0,
    A1,
    A2,
}

pub struct GameFile<'a> {
    bytes: &'a [u8],
    header: Header,
    rand_mode: RandMode,
    rng: &'a mut ThreadRng,
    memory_map: GameMemoryMap,
    object_table: Option<ObjectTable<'a>>
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
            memory_map: memory_map,
            object_table: None
        };

        // Get the base address of the objects
        // and use the properties addr from the first object to find the end of the object table
        let raw_object_bytes = &g.bytes[g.memory_map.object_table as usize..];
        let zobj = Zobject::new(raw_object_bytes);
        g.memory_map.properties_table = zobj.properties_addr();
        let obj_table_size = g.memory_map.properties_table - g.memory_map.object_table;
        let num_obj = obj_table_size / std::mem::size_of::<Zobject>() as u16;
        g.object_table = Some(ObjectTable::new(raw_object_bytes, num_obj));
        g
    }

    pub fn default_properties(&self) -> PropertyDefaults {
        let prop_raw = &self.bytes[self.memory_map.property_defaults as usize
            ..(self.memory_map.property_defaults + MAX_PROPERTIES * 2) as usize];
        PropertyDefaults { prop_raw: prop_raw }
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

    pub fn read_text(cs: &[u8]) -> String {
        let mut ss : Vec<u8> = vec![];
        let mut cp = 0;
        let mut is_in_abbrev = false;
        let mut abbrev_table = 0;
        let mut current_alphabet = Alphabets::A0;

        loop {
            //BUGBUG: this is dumb. We reset every loop [cb]
            let next_chars = <[u8; 2]>::try_from(&cs[cp..cp+2]).unwrap();
            let pc = GameFile::read_zchars_from_word(&next_chars).unwrap();
            cp+=2;
            
            for c in pc.chars {
                if is_in_abbrev {
                    ss.append(&mut GameFile::lookup_abbrev(GameFile::abbrev_string_index(abbrev_table, c)));
                    is_in_abbrev = false;
                }

                match c {
                    // current char denotes an abbreviation table
                    // next char denotes the index
                    1 | 2 | 3 => {
                        is_in_abbrev = true;
                        abbrev_table = c;
                    },
                    // current char 'shifts' alphabet
                    4 => {
                        current_alphabet = Alphabets::A1;
                    },
                    5 => {
                        current_alphabet = Alphabets::A2;
                    },                    
                    // current char is normal
                    // BUGBUG: is this guard statement correct? [cb]
                    6 ..= 25 => {
                        ss.push(GameFile::lookup_char(c, &current_alphabet))
                    },
                    _ => {
                        panic!("text out of range!");
                    }
                }
            }

            if pc.last {break;}

        }

        "string".to_string()
    }

    fn lookup_char(c: u8, alphabet : &Alphabets) -> Zchar {
        // in the published tables, read char mappings start at index 6
        ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
    }

    fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
        32 * (abbrev_code - 1) + abbrev_index
    }

    fn lookup_abbrev(_abbrev_index: u8) -> Vec<u8> {
        vec![0,1]
    }

    pub fn read_zchars_from_word(word: &[u8; 2]) -> Result<PackedChars, BitReaderError> {
        // start with a word
        let mut br = BitReader::new(word);

        // lop off top bit as designator of 'last chars here'
        let mut pc = PackedChars{last: br.read_u8(1)? == 1, chars: [0,0,0]};

        for i in 0..3 {
            pc.chars[i] = br.read_u8(5)?;
        }
        
        Ok(pc)
    }
}

impl<'a> Display for GameFile<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "
            Header:
            {}
            Memory map:
            {}
            ",
            self.header, self.memory_map
        ).and_then(|_| {
            match &self.object_table {
                Some(ot) => {
                    write!(f, "{}", ot)
                },
                _ => write!(f, "no objects found")
            }
        })
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
