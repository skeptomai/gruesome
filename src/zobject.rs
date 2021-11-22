use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::collections::HashMap;

use bitreader::{BitReader, BitReaderError};

type Zchar = u8;

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

// In Versions 1 to 3, there are at most 255 objects, each having a 9-byte entry as follows
#[derive(Debug)]
pub struct ObjectTree {}

#[derive(Debug)]
pub struct ObjectTable<'a> {
    obj_raw: &'a [u8],
    pub objects: Vec<Zobject>,
}

impl<'a> ObjectTable<'a> {
    pub fn new(obj_table_addr: &'a [u8], num_obj: u16) -> Self {
        let mut base = 0;
        let mut n = num_obj;
        let mut objs = vec![];

        while n > 0 {
            let zobj = Zobject::new(&obj_table_addr[base..base + std::mem::size_of::<Zobject>()]);
            objs.push(zobj);
            n -= 1;
            base += std::mem::size_of::<Zobject>();
        }

        ObjectTable {
            obj_raw: obj_table_addr,
            objects: objs,
        }
    }
}

impl<'a> Display for ObjectTable<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        writeln!(f, "There are {} objects.", self.objects.len())?;
        for (i, x) in self.objects.iter().enumerate() {
            writeln!(
                f,
                "
            {}:
            {}",
                i + 1,
                x
            )?;
        }
        Ok(())
    }
}

// NOTE: this is only up to v3
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Zobject {
    pub attribute_bits: [u8; 4],
    pub parent: u8,
    pub next: u8,
    pub child: u8,
    pub properties_offsets: [u8; 2],
}

impl Zobject {
    pub fn new(bytes: &[u8]) -> Zobject {
        let sz = std::mem::size_of::<Zobject>();
        let (_prefix, zobj, _suffix) = unsafe { &bytes[0..sz].align_to::<Zobject>() };
        zobj[0].clone()
    }

    pub fn attributes(&self) -> Vec<u8> {
        let mut attrs = vec![];
        let mut index = 0;
        for i in self.attribute_bits {
            let mut mask = 0x80;

            for _j in 0..8 {
                let r = mask & i;
                if r != 0 {
                    attrs.push(index);
                }
                mask >>= 1;
                index += 1;
            }
        }
        attrs
    }

    pub fn properties_addr(&self) -> u16 {
        u16::from_be_bytes(self.properties_offsets)
    }

    fn read_text(cs: &[u8]) -> String {
        let mut ss : Vec<u8> = vec![];
        let mut cp = 0;
        let mut is_in_abbrev = false;
        let mut abbrev_table = 0;
        let mut current_alphabet = Alphabets::A0;

        loop {
            //BUGBUG: this is dumb. We reset every loop [cb]
            let next_chars = <[u8; 2]>::try_from(&cs[cp..cp+2]).unwrap();
            let pc = Self::read_zchars_from_word(&next_chars).unwrap();
            cp+=2;
            
            for c in pc.chars {
                if is_in_abbrev {
                    ss.append(&mut Self::lookup_abbrev(Self::abbrev_string_index(abbrev_table, c)));
                    is_in_abbrev = false;
                }

                match c {
                    // current char denotes an abbreviation table
                    // next char denotes the index
                    1 | 2 | 3 => {
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
                        ss.push(Self::lookup_char(c, &current_alphabet))
                    },
                    _ => {
                        panic!("text out of range!");
                    }
                }
            }

            if pc.last {break;}

        }

        "string".to_string()
    }

    fn lookup_char(c: u8, alphabet : &Alphabets) -> Zchar {
        // in the published tables, read char mappings start at index 6
        ALPHABETMAP[alphabet].as_bytes()[(c as usize) - 6]
    }

    fn abbrev_string_index(abbrev_code: u8, abbrev_index: u8) -> u8 {
        32 * (abbrev_code - 1) + abbrev_index
    }

    fn lookup_abbrev(_abbrev_index: u8) -> Vec<u8> {
        vec![0,1]
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
    
    fn description(&self) -> String {
        "fuck you, that's what".to_string()
    }
}

impl Display for Zobject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
                    f,
                    "
                    Attributes: {:?}, 
                    Parent object: {}, Sibling object: {}, Child object: {}, 
                    Property Address {:#04x},
                    Description: '{}',
                    Properties:
                    ",
                    self.attributes(),
                    self.parent,
                    self.next,
                    self.child,
                    self.properties_addr(),
                    self.description()
                )
    }
}

pub struct ZobjectPostV3 {
    pub attributes: [u16; 3],
    pub parent: u16,
    pub next: u16,
    pub child: u16,
    pub property_offset: u16,
}
