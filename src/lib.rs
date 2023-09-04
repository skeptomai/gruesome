#![crate_name = "infocom"]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

pub mod dictionary;
pub mod game;
pub mod gamememorymap;
pub mod header;
pub mod property_defaults;
pub mod util;
pub mod zobject;
pub mod instruction;
pub mod routine;
pub mod zrand;

#[cfg(test)]
mod tests {
    use crate::game::GameFile;
    use crate::zrand::ZRand;
    use std::env;
    use std::fs::File;
    use std::io;
    use std::io::prelude::*;
    use std::path::PathBuf;

    const DATAFILEPATH: &str = "resources/test/zork1/DATA/ZORK1.DAT";

    use test_log::test;
    use log;

    #[test]
    fn read_zork1() -> io::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(DATAFILEPATH);

        // open file and read all bytes into vector
        let mut f = File::open(path)?;
        let mut all_bytes = Vec::new();
        f.read_to_end(&mut all_bytes).unwrap();

        // create random generator
        let mut zrg = ZRand::new_uniform();

        // Instantiate gamefile structure
        let g = GameFile::new(&all_bytes, &mut zrg);

        // dump the game structure
        log::info!("{}", g);
        Ok(())
    }
}

/*
An example memory map of a small game
Dynamic	00000	header
        00040	abbreviation strings
        00042	abbreviation table
        00102	property defaults
        00140	objects
        002f0	object descriptions and properties
        006e3	global variables
        008c3	arrays
Static	00b48	grammar table
        010a7	actions table
        01153	preactions table
        01201	adjectives table
        0124d	dictionary
High	01a0a	Z-code
        05d56	static strings
        06ae6	end of file
*/
