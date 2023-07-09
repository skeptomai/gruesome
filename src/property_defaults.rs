use std::array::TryFromSliceError;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use crate::util::get_mem_addr;
use crate::util::MAX_PROPERTIES_V3;

pub struct PropertyDefaults<'a, T, const N: usize> {
    pub prop_raw: &'a [T; N], // 31 words in Versions 1-3. 63 words in Versions 4 or later.
}

impl<'a> Display for PropertyDefaults<'a, u8, MAX_PROPERTIES_V3> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        for i in 0..MAX_PROPERTIES_V3 - 1 {
            write!(
                f,
                "{} ",
                get_mem_addr(&self.prop_raw[(i * 2) as usize..], 2).unwrap()
            )
            .unwrap();
        }
        Ok(())
    }
}

impl<'a> Debug for PropertyDefaults<'a, u8, MAX_PROPERTIES_V3> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        for i in 0..MAX_PROPERTIES_V3 - 1 {
            write!(
                f,
                "{} ",
                get_mem_addr(&self.prop_raw[(i * 2) as usize..], 2).unwrap()
            )
            .unwrap();
        }
        Ok(())
    }
}

impl<'a> PropertyDefaults<'a, u8, MAX_PROPERTIES_V3> {
    pub fn property(&self, index: usize) -> Result<u16, TryFromSliceError> {
        // The table begins with a block known as the property defaults table. This contains 31 words in Versions 1 to 3
        // and 63 in Versions 4 and later. When the game attempts to read the value of property n for an object which 
        // does not provide property n, the n-th entry in this table is the resulting value. 
               
        //BUGBUG this is just repeated code from get_mem_addr. Factor out to util

        let actual_index = std::cmp::min(index, MAX_PROPERTIES_V3);

        match <[u8; 2]>::try_from(&self.prop_raw[actual_index..actual_index + 2]) {
            Ok(u) => Ok(u16::from_be_bytes(u)),
            Err(error) => Err(error)
        }

    }
}
