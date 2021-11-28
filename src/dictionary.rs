use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use crate::game::GameFile;
use crate::game::get_mem_addr;

#[derive(Debug, Clone)]
pub struct Dictionary {
    pub n : u8,
    pub input_codes : Vec<u8>,
    pub entry_length : u8,
    pub number_of_entries : usize,
}

impl Dictionary {
    pub fn new(g : &GameFile) -> Dictionary {
        let mut cur_pos = g.header().dictionary;
        let bytes = g.bytes();
        let n = bytes[cur_pos];
        cur_pos+=1;
        let input_codes = bytes[cur_pos..cur_pos+n as usize].to_vec();
        cur_pos+=n as usize;
        let entry_length = bytes[cur_pos];
        cur_pos+=1;
        let number_of_entries = get_mem_addr(bytes, cur_pos);

        Dictionary{n, input_codes, entry_length, number_of_entries}
    }
}

impl Display for Dictionary {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        writeln!(
            f,
            "Number of separator / input codes: {}, word size: {}, word count: {}", self.n, self.entry_length, self.number_of_entries)?;
        writeln!(f, "separators:")?;
        for c in &self.input_codes {
            write!(f, "'{}' , ", *c as char)?;
        }

        Ok(())
    }
}

pub struct DictionaryWord {
    pub enc_text : [u8;4],
    pub data_bytes : Vec<u8>,
}