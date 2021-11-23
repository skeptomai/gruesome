use std::collections::HashMap;
use std::io;

use bitreader::{BitReader, BitReaderError};

type Zchar = u8;

#[derive(Debug, Clone, Copy)]
pub struct PackedChars {
    last : bool,
    chars : [Zchar;3]
}

#[derive(PartialEq, Eq, Hash)]
enum Alphabets {
    A0,
    A1,
    A2,
}

lazy_static! {
    static ref ALPHABETMAP: HashMap<Alphabets, &'static str> = {
        let mut m = HashMap::new();
        m.insert(Alphabets::A0, "abcdefghijklmnopqrstuvwxyz");
        m.insert(Alphabets::A1, "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        m.insert(Alphabets::A2, " ^0123456789.,!?_#'\"/\\-:()");
        m
    };
}


pub fn read_text(cs: &[u8]) -> Result<String, io::Error> {
    let mut ss : Vec<u8> = vec![];
    let mut cp = 0;
    let mut is_in_abbrev = false;
    let mut abbrev_table = 0;
    let mut current_alphabet = Alphabets::A0;
    let mut found_last = false;

    // first byte tells us number of words
    let len = &cs[0];
    cp +=1;

    for _x in (0 .. *len).rev() {
        let next_chars = <[u8; 2]>::try_from(&cs[cp..cp+2]).unwrap();
        let pc = read_zchars_from_word(&next_chars).unwrap();
        cp+=2;
        
        for c in pc.chars {

            if is_in_abbrev {
                ss.append(&mut lookup_abbrev(abbrev_string_index(abbrev_table, c)));
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
                        println!("found abbrev");
                        is_in_abbrev = true;
                        abbrev_table = c;
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
                    6 ..= 25 => {
                        ss.push(lookup_char(c, &current_alphabet))
                    },
                    _ => {
                        panic!("text out of range! {}", c);
                    }
                }
            }
        }

        
        found_last = pc.last;
    }

    if !found_last {
        Err(io::Error::new(io::ErrorKind::Other, "failed to find end of string!"))
    } else {
        match std::str::from_utf8(&ss) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        }
    }
    
}

fn lookup_char(c: u8, alphabet : &Alphabets) -> Zchar {
    // in the published tables, read char mappings start at index 6
    ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
}

fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
    32 * (abbrev_code - 1) + abbrev_index
}

fn lookup_abbrev(_abbrev_index: u8) -> Vec<u8> {
    "fuck you".as_bytes().to_vec()
}

fn read_zchars_from_word(word: &[u8; 2]) -> Result<PackedChars, BitReaderError> {
    // start with a word
    let mut br = BitReader::new(word);

    // lop off top bit as designator of 'last chars here'
    let mut pc = PackedChars{last: br.read_u8(1)? == 1, chars: [0,0,0]};

    for i in 0..3 {
        pc.chars[i] = br.read_u8(5)?;
    }
    
    Ok(pc)
}

