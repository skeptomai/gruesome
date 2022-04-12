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
pub struct PackedChars<const U: usize> {
    pub last : bool,
    pub chars : [Zchar;U],
}

impl<const U: usize> PackedChars<U> {
    fn iter(&self) -> PackedCharsIter<'_, U> {
        PackedCharsIter { chars: &self.chars, pos: 0 }
    }
}


pub struct PackedCharsIter<'a, const U: usize> {
    pos : usize,
    chars: &'a [u8;U],
}

impl<'a, const U: usize> Iterator for PackedCharsIter<'a, U> {
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

impl<'a, const U: usize> IntoIterator for &'a PackedChars<U> {
    type Item = &'a u8;

    type IntoIter  = PackedCharsIter<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub fn read_text(g: &GameFile, cso: usize) -> Result<String, io::Error> {
    let mut ss : Vec<u8> = vec![];
    let mut cp = 0;
    let mut is_in_abbrev = false;
    let mut abbrev_table = 0;
    let mut current_alphabet = Alphabets::A0;
    
    let cs = &g.bytes()[cso..];
    let _abs = &g.bytes()[g.abbrev_strings()..];
    let abt = &g.bytes()[g.abbrev_table()..];

    loop {
        let next_chars = <[u8; 2]>::try_from(&cs[cp..cp+2]).unwrap();
        let pc = crate::util::read_zchars_from_word(&next_chars).unwrap();
        cp+=2;
        
        for c in &pc {

            if is_in_abbrev {
                let asi = crate::util::abbrev_string_index(abbrev_table, *c) as usize; // word address
                //println!("abbrev table {}, index {}, resulting offset: {}", abbrev_table, c, asi);
                let abbrev_string_addr = (get_mem_addr(abt, asi) *2) as usize;
                //println!("addr? {:#04x}", abbrev_string_addr);
                unsafe {ss.append(read_text(g, abbrev_string_addr).unwrap().as_mut_vec())};
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
                        abbrev_table = *c;
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
                        ss.push(lookup_char(*c, &current_alphabet));
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

fn lookup_char(c: u8, alphabet : &Alphabets) -> crate::util::Zchar {
    // in the published tables, read char mappings start at index 6
    ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
}

fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
    (32 * (abbrev_code - 1) + abbrev_index) * 2
}

fn read_zchars_from_word(word: &[u8; 2]) -> Result<PackedChars<3>, BitReaderError> {
    // start with a word
    let mut br = BitReader::new(word);

    // lop off top bit as designator of 'last chars here'
    let mut pc = PackedChars{last: br.read_u8(1)? == 1, chars: [0,0,0]};

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


