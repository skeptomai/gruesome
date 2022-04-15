#![crate_name = "infocom"]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod game;
mod zobject;
mod header;
mod property_defaults;
mod dictionary;
mod util;

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::File;
    use std::io;
    use std::io::prelude::*;
    use std::path::PathBuf;
    use crate::game::GameFile;
    use test_log::test;
    use log;

    #[test]
    fn read_zork1() -> io::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/test/zork1/DATA/ZORK1.DAT");

        let mut f = File::open(path)?;
        let mut all_bytes = Vec::new();
        let mut rng = rand::thread_rng();

        f.read_to_end(&mut all_bytes).unwrap();

        let g = GameFile::new(&all_bytes, &mut rng);
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

/*
 Z-char 6789abcdef0123456789abcdef
current   --------------------------
  A0      abcdefghijklmnopqrstuvwxyz
  A1      ABCDEFGHIJKLMNOPQRSTUVWXYZ
  A2       ^0123456789.,!?_#'"/\-:()
          --------------------------
*/
