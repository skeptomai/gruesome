#![crate_name = "infocom"]
#![allow(dead_code)]

use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

#[macro_use]
extern crate lazy_static;

mod game;
mod zobject;
use crate::game::GameFile;
mod header;
mod property_defaults;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = Path::new(&args[1]);
    let mut f = File::open(path)?;
    let mut all_bytes = Vec::new();
    let mut rng = rand::thread_rng();

    f.read_to_end(&mut all_bytes).unwrap();

    let g = GameFile::new(&all_bytes, &mut rng);
    println!("{}", g);
    Ok(())
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
