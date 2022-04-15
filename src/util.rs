use std::{io, usize};
use std::collections::HashMap;
use bitreader::{BitReader, BitReaderError};
use crate::game::GameFile;


#[derive(PartialEq, Eq, Hash)]
pub enum Alphabets {
    A0,
    A1,
    A2,
}

lazy_static! {
    pub static ref ALPHABETMAP: HashMap<Alphabets, &'static str> = {
        let mut m = HashMap::new();
        m.insert(Alphabets::A0, "abcdefghijklmnopqrstuvwxyz");
        m.insert(Alphabets::A1, "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        m.insert(Alphabets::A2, " ^0123456789.,!?_#'\"/\\-:()");
        m
    };
}

pub const MAX_PROPERTIES: usize = 32;

pub type Zchar = u8;

#[derive(Debug, Clone, Copy)]
pub struct UnpackedZChars<const U: usize> {
    pub last : bool,
    pub chars : [Zchar;U],
}

impl<const U: usize> UnpackedZChars<U> {
    fn iter(&self) -> UnpackedZCharsIter<'_, U> {
        UnpackedZCharsIter { chars: &self.chars, pos: 0 }
    }
}


pub struct UnpackedZCharsIter<'a, const U: usize> {
    pos : usize,
    chars: &'a [u8;U],
}

impl<'a, const U: usize> Iterator for UnpackedZCharsIter<'a, U> {
    type Item  = &'a u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < U {
            let cur_pos = self.pos;
            self.pos = self.pos+1;
            Some(&self.chars[cur_pos])
        } else {
            None
        }

    }
}

impl<'a, const U: usize> IntoIterator for &'a UnpackedZChars<U> {
    type Item = &'a u8;

    type IntoIter  = UnpackedZCharsIter<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub trait ZTextReader {
    fn read_text(g: &GameFile, cso: usize) -> Result<String, io::Error>;
}

pub fn lookup_char(c: u8, alphabet : &Alphabets) -> crate::util::Zchar {
    // in the published tables, read char mappings start at index 6
    ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
}

pub fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
    (32 * (abbrev_code - 1) + abbrev_index) * 2
}

pub fn read_zchars_from_word(word: &[u8; 2]) -> Result<UnpackedZChars<3>, BitReaderError> {
    log::debug!("zchars from word: [{:#04x}, {:#04x}]", word[0], word[1]);
    // start with a word
    let mut br = BitReader::new(word);

    // lop off top bit as designator of 'last chars here'
    let mut pc = UnpackedZChars{last: br.read_u8(1)? == 1, chars: [0,0,0]};

    for i in 0..3 {
        pc.chars[i] = br.read_u8(5)?;
    }

    Ok(pc)
}

pub fn get_mem_addr(addr: &[u8], counter: usize) -> usize {
    let ins_bytes = <[u8; 2]>::try_from(&addr[counter..counter + 2]).unwrap();
    let ins = u16::from_be_bytes(ins_bytes);
    ins as usize
}


