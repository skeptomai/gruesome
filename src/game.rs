use std::fmt::{Debug, Display, Error, Formatter};
use sub_array::SubArray;

use crate::header::Header;
use crate::property_defaults::PropertyDefaults;
use crate::zobject::{ObjectTable, Zobject};
use crate::dictionary::Dictionary;
use crate::util::{get_mem_addr, properties_size_by_version, ZTextReader, MAX_PROPERTIES_V3};
use crate::zrand::{RandMode, ZRand};
use crate::gamememorymap::GameMemoryMap;

/// GameFile is the main data structure for a single game instance
pub struct GameFile<'a> {
    header: Header,
    rand_mode: RandMode,
    rng: &'a mut ZRand,
    memory_map: GameMemoryMap<'a>,
    object_table: Option<ObjectTable>,
    dictionary: Option<Dictionary>,
}

impl<'a> GameFile<'a> {
    /// create new GameFile with the raw file bytes and a random entropy source
    pub fn new(bytes: &'a Vec<u8>, rng: &'a mut ZRand) -> GameFile<'a> {
        let bytes = &bytes;
        let header_addr = 0;
        let abbrev_strings = 0x40;
        // initialize header as first $40 == 60 dec bytes
        let header = Header::new(bytes);
        let abbrev_table = header.abbrev_table;
        // top of the object table (from header) is actually where the default properties start. objects follow
        // 12.2 Property defaults table
        // The table begins with a block known as the property defaults table. This contains 31 words in Versions 1 to 3
        // and 63 in Versions 4 and later. When the game attempts to read the value of property n for an object which 
        // does not provide property n, the n-th entry in this table is the resulting value.        
        let property_defaults = header.object_table_addr;
        // It's an address, so the offset is also. Object tree starts after default properties
        let object_table = header.object_table_addr + (properties_size_by_version(header.version)) * 2;
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

        // BUGBUG: take RandMode into consideration.
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

    /// default_properties creates PropertyDefault structure from memory maps property defaults
    pub fn default_properties(&self) -> PropertyDefaults<u8, MAX_PROPERTIES_V3> {
        let prop_raw: &[u8; MAX_PROPERTIES_V3] = &self.bytes_sized(self.memory_map.property_defaults);
        PropertyDefaults { prop_raw: prop_raw }
    }

    /// header is an accessor that returns the game header
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn version(&self) -> usize {
        *(&self.header.version) as usize
    }

    /// abbrev_strings is an accessor that returns the abbreviated strings
    pub fn abbrev_strings(&self) -> usize {
        self.memory_map.abbrev_strings
    }

    /// abbrev_table is an accessor that returns the abbreviation table
    pub fn abbrev_table(&self) -> usize {
        self.memory_map.abbrev_table
    }

    /// object_table is an accessor that returns the object table
    pub fn object_table(&self) -> usize {
        self.memory_map.object_table
    }

    /// bytes is an accessor that returns all the bytes in the memory map as [u8]
    pub fn bytes(&self) -> &'a [u8] {
        &self.memory_map.bytes[..]
    }

    pub fn bytes_sized<const N: usize>(&self, offset: usize) -> &'a [u8;N] {
        log::debug!("calling bytes_sized");
        let sub : &[u8; N] = &self.memory_map.bytes.sub_array_ref(offset);
        &sub
    }
}

impl<'a> Debug for GameFile<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let rand_mode_str = match self.rand_mode {
            RandMode::Predictable => "Predictable",
            RandMode::RandomUniform => "RandomUniform",
        };
        
        // Since not all fields implement Debug, we'll format them manually
        writeln!(f, "GameFile {{")?;
        writeln!(f, "  header: {{")?;
        writeln!(f, "    version: {}", self.header.version)?;
        writeln!(f, "    release: {}", self.header.release)?;
        writeln!(f, "    serial: {}", self.header.serial)?;
        writeln!(f, "    initial_pc: {:#06x}", self.header.initial_pc)?;
        writeln!(f, "    dictionary: {:#06x}", self.header.dictionary)?;
        writeln!(f, "    object_table_addr: {:#06x}", self.header.object_table_addr)?;
        writeln!(f, "    global_variables: {:#06x}", self.header.global_variables)?;
        writeln!(f, "    base_static_mem: {:#06x}", self.header.base_static_mem)?;
        writeln!(f, "    base_high_mem: {:#06x}", self.header.base_high_mem)?;
        writeln!(f, "    abbrev_table: {:#06x}", self.header.abbrev_table)?;
        writeln!(f, "    len_file: {}", self.header.len_file)?;
        writeln!(f, "    checksum_file: {:#04x}", self.header.checksum_file)?;
        writeln!(f, "  }}")?;
        writeln!(f, "  rand_mode: {}", rand_mode_str)?;
        writeln!(f, "  memory_map: {{")?;
        writeln!(f, "    bytes.len(): {}", self.memory_map.bytes.len())?;
        writeln!(f, "    header_addr: {:#06x}", self.memory_map.header_addr)?;
        writeln!(f, "    abbrev_strings: {:#06x}", self.memory_map.abbrev_strings)?;
        writeln!(f, "    abbrev_table: {:#06x}", self.memory_map.abbrev_table)?;
        writeln!(f, "    property_defaults: {:#06x}", self.memory_map.property_defaults)?;
        writeln!(f, "    object_table: {:#06x}", self.memory_map.object_table)?;
        writeln!(f, "    properties_table: {:#06x}", self.memory_map.properties_table)?;
        writeln!(f, "    global_variables: {:#06x}", self.memory_map.global_variables)?;
        writeln!(f, "  }}")?;
        writeln!(f, "  object_table: {}", if self.object_table.is_some() { "Some(...)" } else { "None" })?;
        writeln!(f, "  dictionary: {}", if self.dictionary.is_some() { "Some(...)" } else { "None" })?;
        write!(f, "}}")?;
        
        Ok(())
    }
}

impl<'a> Display for GameFile<'a> {
    /// formats the GameFile struct
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
            let abbrev_string_addr = (get_mem_addr(&self.bytes(), abbrev_table_offset as usize).unwrap() *2) as usize;
            writeln!(f, "[{}] \"{}\"", si, Dictionary::read_text(&self, abbrev_string_addr).unwrap())?;
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



