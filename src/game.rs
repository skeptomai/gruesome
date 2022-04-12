use crate::header::Header;

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use bitreader::{BitReader, BitReaderError};

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::property_defaults::PropertyDefaults;
use crate::zobject::{ObjectTable, Zobject};
use crate::dictionary::Dictionary;
use crate::util::read_text;
use crate::util::get_mem_addr;
use crate::util::MAX_PROPERTIES;
use crate::util::Zchar;
use crate::util::PackedChars;
use crate::util::Alphabets;
use crate::util::ALPHABETMAP;

enum RandMode {
    Predictable,
    RandomUniform,
}

pub struct GameFile<'a> {
    header: Header,
    rand_mode: RandMode,
    rng: &'a mut ThreadRng,
    memory_map: GameMemoryMap<'a>,
    object_table: Option<ObjectTable>,
    dictionary: Option<Dictionary>,
}

impl<'a> GameFile<'a> {
    /// create new GameFile with the raw file bytes and a random entropy source
    pub fn new(bytes: &'a Vec<u8>, rng: &'a mut ThreadRng) -> GameFile<'a> {
        let bytes = &bytes;
        let header_addr = 0;
        let abbrev_strings = 0x40;
        // initialize header as first $40 == 60 dec bytes
        let header = Header::new(bytes);
        let abbrev_table = header.abbrev_table;
        let property_defaults = header.object_table_addr;
        let object_table = header.object_table_addr + (MAX_PROPERTIES - 1) * 2;
        let global_variables = header.global_variables;
        // Get the base address of the objects
        // and use the properties addr from the first object to find the end of the object table
        let properties_table = Zobject::properties_addr_from_base(&bytes[object_table..]);

        let memory_map: GameMemoryMap = GameMemoryMap {
            bytes,
            header_addr,
            abbrev_strings,
            abbrev_table,
            property_defaults,
            object_table,
            properties_table,
            global_variables,
        };

        let mut g = GameFile {
            header,
            rng,
            rand_mode: RandMode::RandomUniform,
            memory_map,
            object_table: None,
            dictionary:  None,
        };

        let ot = ObjectTable::new(&g);
        let object_table = Some(ot);

        g.object_table = object_table;

        let dict = Dictionary::new(&g);

        g.dictionary = Some(dict);

        g
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

    pub fn header(&self) -> &Header {
        &self.header
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

        writeln!(f, "\n***** Abbreviations *****")?;
        let mut abbrev_table_offset = self.memory_map.abbrev_table;
        let mut si = 1;
        loop {
            let abbrev_string_addr = (get_mem_addr(&self.bytes(), abbrev_table_offset as usize) *2) as usize;
            writeln!(f, "[{}] \"{}\"", si, read_text(&self, abbrev_string_addr).unwrap())?;
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

        match &self.dictionary {
            Some(d) => {
               write!(f,"{}", d)?;
            },
            _ => {write!(f,"no dictionary!")?;}
        }

        Ok(())
    }
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

pub fn lookup_char(c: u8, alphabet : &Alphabets) -> Zchar {
    // in the published tables, read char mappings start at index 6
    ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
}

pub fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
    (32 * (abbrev_code - 1) + abbrev_index) * 2
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

