use std::{
    io, 
    fmt::{Display,Error, Formatter}};
use crate::{
    game::GameFile, 
    util::{get_mem_addr, ZTextReader, Alphabets, lookup_char, read_zchars_from_word}};
#[derive(Debug, Clone)]
pub struct Dictionary {
    pub n : u8,
    pub input_codes : Vec<u8>,
    pub entry_length : u8,
    pub number_of_entries : usize,
    pub words : Vec<DictionaryWord>,
}

impl Dictionary {
    pub fn new(g : &GameFile) -> Dictionary {
        let mut cur_pos = g.header().dictionary;
        let bytes = g.bytes();
        let n = bytes[cur_pos];
        cur_pos+=1;
        let input_codes = bytes[cur_pos..cur_pos+n as usize].to_vec();
        /* from the doc:
        The "entry length" is the length of each word's entry in the dictionary table. 
        (It must be at least 4 in Versions 1 to 3, and at least 6 in later Versions.) 
         */
        cur_pos+=n as usize;
        let entry_length = bytes[cur_pos];
        assert!(entry_length>=4, "Word length must be at least 4 chars in versions 1 to 3, and at least 6 in later versions");
        cur_pos+=1;
        let number_of_entries = get_mem_addr(bytes, cur_pos);
        cur_pos+=2;
        let mut words = vec![];
        for _i in 0..number_of_entries{
            let _dict_entry = &bytes[cur_pos..cur_pos+entry_length as usize];
            let word = Dictionary::read_text(g, cur_pos).expect("failed to read dict text");
            let data = bytes[cur_pos+4 as usize..cur_pos+entry_length as usize].to_vec();
            words.push(DictionaryWord{word,data});
            cur_pos+=entry_length as usize;
        }
        Dictionary{n, input_codes, entry_length, number_of_entries, words}
    }
}

impl Display for Dictionary {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        writeln!(f, "\n***** Dictionary *****")?;
        writeln!(
            f,
            "Number of separator / input codes: {}, \nword size: {}, word count: {}", self.n, self.entry_length, self.number_of_entries)?;
        write!(f, "separators: ")?;
        for c in &self.input_codes {
            write!(f, "'{}' , ", *c as char)?;
        }
        writeln!(f, "\n\n****** words ******")?;
        for (i,w) in self.words.iter().enumerate() {
            write!(f, "[{}]: {} ", i+1, w)?;
            if (i+1) % 3 == 0 {writeln!(f, "")?};
        }

        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct DictionaryWord {
    pub word : String,
    pub data : Vec<u8>,
}

impl Display for DictionaryWord {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        //write!(f,"{}, {:?}", self.word, self.data)
        write!(f,"{}", self.word)
    }
}

impl ZTextReader for Dictionary {
    fn read_text(g: &GameFile, cso: usize) -> Result<String, io::Error> {
        let mut ss : Vec<u8> = vec![];
        let mut cp = 0;
        let mut is_in_abbrev = false;
        let mut abbrev_table = 0;
        let mut current_alphabet = Alphabets::A0;
        
        let cs = &g.bytes()[cso..];
        let _abs = &g.bytes()[g.abbrev_strings()..];
        let abt = &g.bytes()[g.abbrev_table()..];

        let mut is_in_punctuation = false;
        let mut is_in_ascii = false;
        let mut ascii_value : Option<u32> = None;

        loop {
            let next_chars = <[u8; 2]>::try_from(&cs[cp..cp+2]).unwrap();
            let pc = read_zchars_from_word(&next_chars).unwrap();

            cp+=2;
            
            for c in &pc {

                /* Checking lead-in for 10 bit ascii value
                 We get here by having read char 06 from Alphabet 2 (punctuation)
                 in version 3 and later, we reset to Alphabet 0 after a single char like this */ 
                if is_in_ascii {
                    log::debug!("in ascii!");
                    if ascii_value.is_none(){
                        // set what will be the upper 5 bits
                        ascii_value = Some(*c as u32);
                        log::debug!("Set ascii_value to {:?}", ascii_value);
                        continue;                        
                    } else {
                        // shift upper 5 bits and add lower 5 bits
                        ascii_value = Some((ascii_value.unwrap() << 5) + *c as u32);
                        log::debug!("c is {}, and ascii composed value is {:?}", c, ascii_value);
                        // reset alphabet and ascii and punctuation bool trackers
                        current_alphabet = Alphabets::A0;
                        is_in_ascii = false;
                        is_in_punctuation = false;
                        ss.push(ascii_value.unwrap().try_into().unwrap());
                        continue;
                    }
                }

                if is_in_punctuation {
                    /* 'in punctuation' means choosing Alphabet 2
                     there are weird rules about 10 bit ascii chars, etc
                     which we are checking here */ 
                    if *c == 6 {
                        log::debug!("setting ascii!");
                        is_in_ascii = true;
                        continue;
                    }
                }
    
                if is_in_abbrev {
                    let asi = crate::util::abbrev_string_index(abbrev_table, *c) as usize; // word address
                    log::debug!("abbrev table {}, index {}, resulting offset: {}", abbrev_table, c, asi);
                    let abbrev_string_addr = (get_mem_addr(abt, asi) *2) as usize;
                    log::debug!("addr? {:#04x}", abbrev_string_addr);
                    unsafe {ss.append(Dictionary::read_text(g, abbrev_string_addr).unwrap().as_mut_vec())};
                    is_in_abbrev = false;
                } else {
                    match c {
                        // zero is a space
                        0 => {
                            ss.push(0x20); // char for space
                        },
                        /* current char denotes an abbreviation table
                         next char denotes the index */ 
                        1 | 2 | 3 => {
                            log::debug!("abbrev coming!");
                            is_in_abbrev = true;
                            abbrev_table = *c;
                        },
                        // current char 'shifts' alphabet
                        4 => {
                            // upper case
                            current_alphabet = Alphabets::A1;
                        },
                        5 => {
                            // punctuation
                            log::debug!("punctuation coming!");
                            current_alphabet = Alphabets::A2;
                            is_in_punctuation = true;                            
                        },  
                        // current char is normal
                        // BUGBUG: is this guard statement correct? [cb]
                        6 ..= 31 => {
                            ss.push(lookup_char(*c, &current_alphabet));
                            is_in_punctuation = false;
                            is_in_ascii = false;
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
    
        log::debug!("emitting {:?}", &ss);
        match std::str::from_utf8(&ss) {
            Ok(s) => Ok(s.to_string()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        }
        
    }
}