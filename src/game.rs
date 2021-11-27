use crate::header::Header;

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::collections::HashMap;
use std::io;

use bitreader::{BitReader, BitReaderError};

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::property_defaults::PropertyDefaults;
use crate::zobject::{ObjectTable, Zobject};
use crate::util::read_text;

pub const MAX_PROPERTIES: usize = 32;

type Zchar = u8;

#[derive(Debug, Clone, Copy)]
pub struct PackedChars {
    last : bool,
    chars : [Zchar;3]
}

#[derive(PartialEq, Eq, Hash)]
enum Alphabets {
    A0,
    A1,
    A2,
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

pub struct GameFile<'a> {
    header: Header,
    rand_mode: RandMode,
    rng: &'a mut ThreadRng,
    memory_map: GameMemoryMap<'a>,
    object_table: Option<ObjectTable>
}

impl<'a> GameFile<'a> {
    /// create new GameFile with the raw file bytes and a random entropy source
    pub fn new(bytes: &'a Vec<u8>, rng: &'a mut ThreadRng) -> GameFile<'a> {
        // initialize header as first $40 == 60 dec bytes
        let header = Header::new(bytes);
        let object_table_addr = header.object_table_addr + (MAX_PROPERTIES - 1) * 2;
        // Get the base address of the objects
        // and use the properties addr from the first object to find the end of the object table
        let properties_table = Zobject::properties_addr_from_base(&bytes[object_table_addr..]);

        let memory_map: GameMemoryMap = GameMemoryMap {
            bytes: &bytes,
            header_addr: 0,
            abbrev_strings: 0x40,
            abbrev_table: header.abbrev_table,
            property_defaults: header.object_table_addr,
            object_table: object_table_addr,
            properties_table: properties_table,
            global_variables: header.global_variables,
        };
        let mut g = GameFile {
            header: header,
            rng: rng,
            rand_mode: RandMode::RandomUniform,
            memory_map: memory_map,
            object_table: None
        };

        let ot = ObjectTable::new(&g);
        let object_table = Some(ot);

        g.object_table = object_table;

        g
    }

    pub fn read_text(&self, cso: usize) -> Result<String, io::Error> {
        let mut ss : Vec<u8> = vec![];
        let mut cp = 0;
        let mut is_in_abbrev = false;
        let mut abbrev_table = 0;
        let mut current_alphabet = Alphabets::A0;
        
        let cs = &self.bytes()[cso..];
        let _abs = &self.bytes()[self.abbrev_strings()..];
        let abt = &self.bytes()[self.abbrev_table()..];
    
        loop {
            let next_chars = <[u8; 2]>::try_from(&cs[cp..cp+2]).unwrap();
            let pc = read_zchars_from_word(&next_chars).unwrap();
            cp+=2;
            
            for c in pc.chars {
    
                if is_in_abbrev {
                    let asi = abbrev_string_index(abbrev_table, c) as usize; // word address
                    //println!("abbrev table {}, index {}, resulting offset: {}", abbrev_table, c, asi);
                    let abbrev_string_addr = (get_mem_addr(abt, asi) *2) as usize;
                    //println!("addr? {:#04x}", abbrev_string_addr);
                    unsafe {ss.append(self.read_text(abbrev_string_addr).unwrap().as_mut_vec())};
                    is_in_abbrev = false;
                } else {
                    match c {
                        // zero is a space
                        0 => {
                            ss.push(0x20); // char for space
                        }
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
                        6 ..= 31 => {
                            ss.push(lookup_char(c, &current_alphabet));
                            current_alphabet = Alphabets::A0;
                        },
                        _ => {
                            panic!("text out of range! {}", c);
                        }
                    }
                }
            }
    
            
            if pc.last {break}
        }
    
        match std::str::from_utf8(&ss) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        }
        
    }

    pub fn default_properties(&self) -> PropertyDefaults {
        let prop_raw = &self.bytes()[self.memory_map.property_defaults
            ..(self.memory_map.property_defaults + MAX_PROPERTIES * 2)];
        PropertyDefaults { prop_raw: prop_raw }
    }

    pub fn gen_unsigned_rand(&mut self) -> u16 {
        // NOTE: This could probably be (u16::MAX +1) / 2
        self.rng.gen_range(0..32768)
    }

    pub fn abbrev_strings(&self) -> usize {
        self.memory_map.abbrev_strings
    }

    pub fn abbrev_table(&self) -> usize {
        self.memory_map.abbrev_table
    }

    pub fn object_table(&self) -> usize {
        self.memory_map.object_table
    }
    
    pub fn bytes(&self) -> &'a [u8] {
        &self.memory_map.bytes[..]
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
        )?;

        writeln!(f, "***** Abbreviations *****")?;
        let mut abbrev_table_offset = self.memory_map.abbrev_table;
        let mut si = 1;
        loop {
            let abbrev_string_addr = (get_mem_addr(&self.bytes(), abbrev_table_offset as usize) *2) as usize;
            writeln!(f, "[{}] \"{}\"", si, read_text(self.bytes(), abbrev_string_addr, abbrev_string_addr, abbrev_string_addr).unwrap())?;
            si+=1;
            abbrev_table_offset+=2;
            if abbrev_table_offset >= self.memory_map.property_defaults {break}
        }

        match &self.object_table {
            Some(ot) => {
                write!(f, "{}", ot)?;
            },
            _ => {write!(f, "no objects found")?;}
        }

        Ok(())
    }
}

pub fn get_mem_addr(addr: &[u8], counter: usize) -> usize {
    let ins_bytes = <[u8; 2]>::try_from(&addr[counter..counter + 2]).unwrap();
    let ins = u16::from_be_bytes(ins_bytes);
    ins as usize
}

#[derive(Debug)]
pub struct GameMemoryMap<'a> {
    pub bytes: &'a [u8],    
    pub header_addr: usize,
    pub abbrev_strings: usize,
    pub abbrev_table: usize,
    pub property_defaults: usize,
    pub object_table: usize,
    pub properties_table: usize,
    pub global_variables: usize,
}

impl<'a> Display for GameMemoryMap<'a> {
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

fn lookup_char(c: u8, alphabet : &Alphabets) -> Zchar {
    // in the published tables, read char mappings start at index 6
    ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
}

fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
    (32 * (abbrev_code - 1) + abbrev_index) * 2
}

fn read_zchars_from_word(word: &[u8; 2]) -> Result<PackedChars, BitReaderError> {
    // start with a word
    let mut br = BitReader::new(word);

    // lop off top bit as designator of 'last chars here'
    let mut pc = PackedChars{last: br.read_u8(1)? == 1, chars: [0,0,0]};

    for i in 0..3 {
        pc.chars[i] = br.read_u8(5)?;
    }
    
    Ok(pc)
}

