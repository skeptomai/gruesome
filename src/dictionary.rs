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
            let word = g.read_text(cur_pos).expect("failed to read dict text");
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