#![allow(unused_imports)]
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

/* Each instruction has a form (long, short, extended or variable) and an
 operand count (0OP, 1OP, 2OP or VAR). If the top two bits of the opcode 
 are $$11 the form is variable; if $$10, the form is short. If the opcode 
 is 190 ($BE in hexadecimal) and the version is 5 or later, the form is 
 "extended". Otherwise, the form is "long". */
pub struct Instruction<'a> {
    opcode : u16,
    operands_types: [u8;2],
    operands : [u8;16],
    store_variable: u8,
    branch_offset: [u8;2],
    text: &'a str,
}

impl<'a> Instruction<'a> {
    fn is_short(&self) {}    
    fn is_long(&self) {}
    fn is_extended(&self) {}
    fn is_variable(&self) {}
}

impl<'a> Display for Instruction<'a> {
    fn fmt(&self, _f: &mut Formatter<'_>) -> Result<(), Error> {
        Ok(())
    }
}