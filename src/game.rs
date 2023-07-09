use std::fmt::{Debug, Display, Error, Formatter};

use rand::{RngCore,SeedableRng,Rng,rngs::StdRng};

use sub_array::SubArray;

use crate::header::Header;
use crate::property_defaults::PropertyDefaults;
use crate::zobject::{ObjectTable, Zobject};
use crate::dictionary::Dictionary;
use crate::util::{get_mem_addr, properties_size_by_version, ZTextReader, MAX_PROPERTIES_V3};

/// RandMode controls random generator behaviour. May be predictable for testing or truly random for gameplay
pub enum RandMode {
    Predictable,
    RandomUniform,
}

pub struct ZRand {
    rng : Box<dyn RngCore>,
    rand_mode : RandMode,
}

impl ZRand {
    pub fn new(rm: RandMode) -> ZRand {
        ZRand { rng: Box::new(rand::thread_rng()), rand_mode: rm }
    }

    pub fn new_uniform() -> ZRand {
        ZRand::new(RandMode::RandomUniform)
    }


    pub fn new_predictable(seed: u64) -> ZRand {
        ZRand {rng: Box::new(StdRng::seed_from_u64(seed)), rand_mode: RandMode::Predictable}
    }

    /// gen_unsigned_rand generates unsigned in range [0..32767]
    pub fn gen_unsigned_rand(&mut self) -> u16 {
        // NOTE: This could probably be (u16::MAX +1) / 2
        self.rng.gen_range(0..32768)
    }
}

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
        // It's an address, so the offset is also 
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

/// GameMemoryMap reflects the internal structure of a loaded game
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
    /// formats the GameMemoryMap
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "
            base    end     size
            {:#06x}\t{:#06x}\t{:#06x}     Story file header
            {:#06x}\t{:#06x}\t{:#06x}     Abbreviation data
            {:#06x}\t{:#06x}\t{:#06x}     Abbreviation pointer table
            {:#06x}\t{:#06x}\t{:#06x}     Object table
            {:#06x}\t{:#06x}\t{:#06x}     Properties data
            {:#06x}\t{:#06x}\t{:#06x}     Global variables
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

