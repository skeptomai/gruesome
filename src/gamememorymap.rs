use std::fmt::{Display, Error, Formatter};

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
            {:#06x}                     Default Properties
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
            // Properties defaults
            self.property_defaults,
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