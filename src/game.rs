use crate::header::Header;

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::property_defaults::PropertyDefaults;
use crate::zobject::{Zobject, ObjectTable};
use crate::util::read_text;

pub const MAX_PROPERTIES: u16 = 32;

enum RandMode {
    Predictable,
    RandomUniform,
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
        let object_table = Some(ObjectTable::new(raw_object_bytes, num_obj));
        g.object_table = object_table;

        g
    }

    pub fn default_properties(&self) -> PropertyDefaults {
        let prop_raw = &self.bytes[self.memory_map.property_defaults as usize
            ..(self.memory_map.property_defaults + MAX_PROPERTIES * 2) as usize];
        PropertyDefaults { prop_raw: prop_raw }
    }

    pub fn gen_unsigned_rand(&mut self) -> u16 {
        // NOTE: This could probably be (u16::MAX +1) / 2
        self.rng.gen_range(0..32768)
    }
}

impl<'a> Display for GameFile<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {

        writeln!(f, "***** Abbreviations *****")?;
        let mut abbrev_table_offset = self.memory_map.abbrev_table;
        let mut si = 1;
        loop {
            let abbrev_string_addr = (get_mem_addr(&self.bytes, abbrev_table_offset as usize) *2) as usize;
            writeln!(f, "[{}] \"{}\"", si, read_text(self.bytes, abbrev_string_addr, abbrev_string_addr, abbrev_string_addr).unwrap())?;
            si+=1;
            abbrev_table_offset+=2;
            if abbrev_table_offset >= self.memory_map.property_defaults {break}
        }

        writeln!(f,"***** Objects *****")?;
        // add 1 to properties_table because it's the text length in bytes
        // normally we don't use that to determine text length, but rely rather
        // on the top bit of the last word being set. This byte allows you to skip
        // over the prop header (object description) to the properties
        for (i,o) in self.object_table.as_ref().unwrap().objects.iter().enumerate() {
            let description = if self.bytes[o.properties_addr() as usize] == 0 {"".to_string()}
            else {read_text(&self.bytes, (o.properties_addr() + 1) as usize, 
                self.memory_map.abbrev_strings as usize,
                self.memory_map.abbrev_table as usize).unwrap()};
            writeln!(f, "[{}] \"{}\"", i+1, description)?;
        }

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
