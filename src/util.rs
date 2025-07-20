use crate::game::GameFile;
use bitreader::{BitReader, BitReaderError};
use std::array::TryFromSliceError;
use std::collections::HashMap;
use std::io;

///There are three possible alphabets: lower case, upper case, and number/symbol
///
///| Alphabet |   Z-char offset            |
///|----------|----------------------------|
///|          | 6789abcdef0123456789abcdef |
///|  A0      | abcdefghijklmnopqrstuvwxyz |
///|  A1      | ABCDEFGHIJKLMNOPQRSTUVWXYZ |
///|  A2      | ^0123456789.,!?_#'"/\-:()  |
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

pub const MAX_PROPERTIES_V3: usize = 31;

pub type Zchar = u8;

/// ZChars are the Zork character type with bit packing
#[derive(Debug, Clone, Copy)]
pub struct UnpackedZChars<const U: usize> {
    pub last: bool,
    pub chars: [Zchar; U],
}

impl<const U: usize> UnpackedZChars<U> {
    /// We can iterate over ZChars
    fn iter(&self) -> UnpackedZCharsIter<'_, U> {
        UnpackedZCharsIter {
            chars: &self.chars,
            pos: 0,
        }
    }
}

/// The actual ZChar iterator
pub struct UnpackedZCharsIter<'a, const U: usize> {
    pos: usize,
    chars: &'a [u8; U],
}

impl<'a, const U: usize> Iterator for UnpackedZCharsIter<'a, U> {
    type Item = &'a u8;
    /// return the next ZChar in 'string'-like thing
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < U {
            let cur_pos = self.pos;
            self.pos += 1;
            Some(&self.chars[cur_pos])
        } else {
            None
        }
    }
}

impl<'a, const U: usize> IntoIterator for &'a UnpackedZChars<U> {
    type Item = &'a u8;

    type IntoIter = UnpackedZCharsIter<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub trait ZTextReader {
    /// Read a rust string from ZChar packed text
    fn read_text(g: &GameFile, cso: usize) -> Result<String, io::Error>;
}

/// Look up a char in the alphabet mapping
pub fn lookup_char(c: u8, alphabet: &Alphabets) -> crate::util::Zchar {
    // in the published tables, read char mappings start at index 6
    ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
}

/// Find the abbreviated string index
pub fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
    (32 * (abbrev_code - 1) + abbrev_index) * 2
}

pub fn properties_size_by_version(version: u8) -> usize {
    match version {
        3 => MAX_PROPERTIES_V3,
        _ => {
            panic!("version out of range! {version}");
        }
    }
}

/// Read ZChars from packed word
pub fn read_zchars_from_word(word: &[u8; 2]) -> Result<UnpackedZChars<3>, BitReaderError> {
    log::debug!("zchars from word: [{:#04x}, {:#04x}]", word[0], word[1]);
    // start with a word
    let mut br = BitReader::new(word);

    // lop off top bit as designator of 'last chars here'
    let mut pc = UnpackedZChars {
        last: br.read_u8(1)? == 1,
        chars: [0, 0, 0],
    };

    for i in 0..3 {
        pc.chars[i] = br.read_u8(5)?;
    }

    Ok(pc)
}

///I believe this only works for Infocom versions 1,2 and 3 right now
///A packed address specifies where a routine or string begins in high memory. Given a packed address P, the formula to obtain the corresponding byte address B is:
///
///| Packing     | Versions                            |
///|-------------|-------------------------------------|
///| 2P          | Versions 1, 2 and 3                 |
///| 4P          | Versions 4 and 5                    |
///| 4P + 8R_O   | Versions 6 and 7, for routine calls |
///| 4P + 8S_O   | Versions 6 and 7, for print_paddr   |
///| 8P          | Version 8                           |
pub fn get_mem_addr(addr: &[u8], counter: usize) -> Result<usize, TryFromSliceError> {
    match <[u8; 2]>::try_from(&addr[counter..counter + 2]) {
        Ok(u) => Ok(u16::from_be_bytes(u) as usize),
        Err(error) => Err(error),
    }
}
