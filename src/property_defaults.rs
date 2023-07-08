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
                get_mem_addr(&self.prop_raw[(i * 2) as usize..], 2)
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
                get_mem_addr(&self.prop_raw[(i * 2) as usize..], 2)
            )
            .unwrap();
        }
        Ok(())
    }
}

impl<'a> PropertyDefaults<'a, u8, 32> {
    pub fn property(&self, index: usize) -> u16 {
        //BUGBUG no range checking here [cb]
        //BUGBUG this is just repeated code from get_mem_addr. Factor out to util
        let ins_bytes = <[u8; 2]>::try_from(&self.prop_raw[index..index + 2]).unwrap();
        let ins = u16::from_be_bytes(ins_bytes);
        ins
    }
}
