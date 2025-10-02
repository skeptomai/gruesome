//! Z-Machine Opcode Constants
//!
//! This module provides named constants for all Z-Machine opcodes to improve
//! code readability and prevent opcode-related bugs.
//!
//! # CRITICAL CONVENTION
//!
//! These constants represent RAW opcode numbers (0x00-0x1F), NOT encoded
//! instruction bytes. The `emit_instruction()` function will determine the
//! proper instruction form (Long, Short, Variable, Extended) and encode it.
//!
//! # Z-Machine Opcode Numbering
//!
//! Z-Machine uses different instruction forms with different encoding:
//! - **2OP** (two operands): Raw opcodes 0x00-0x1F, encoded as Long/Short form
//! - **1OP** (one operand): Raw opcodes 0x00-0x0F, encoded as Short form (0x80-0x9F)
//! - **0OP** (no operands): Raw opcodes 0x00-0x0F, encoded as Short form (0xB0-0xBF)
//! - **VAR** (variable operands): Raw opcodes 0x00-0x1F, encoded as Variable form (0xE0-0xFF)
//!
//! The same raw opcode number can mean different instructions in different forms!
//! For example, opcode 0x00 is:
//! - 2OP:0 = je (jump if equal)
//! - 1OP:0 = jz (jump if zero)
//! - 0OP:0 = rtrue (return true)
//! - VAR:224 = call_vs (call routine, variable operands, store result)

/// VAR form opcodes (pass raw opcode number 0x00-0x1F to emit_instruction)
pub mod var {
    /// call_vs (VAR:224) - Call routine with variable operands, store result
    /// Pass 0x00 to emit_instruction, NOT 0xE0
    pub const CALL_VS: u8 = 0x00;

    /// storew (VAR:225) - Store word at address
    pub const STOREW: u8 = 0x01;

    /// storeb (VAR:226) - Store byte at address
    pub const STOREB: u8 = 0x02;

    /// put_prop (VAR:227) - Set object property value
    pub const PUT_PROP: u8 = 0x03;

    /// sread (VAR:228) - Read text from keyboard (v1-3) or aread (v4+)
    pub const SREAD: u8 = 0x04;

    /// print_char (VAR:229) - Print character from ZSCII code
    pub const PRINT_CHAR: u8 = 0x05;

    /// print_num (VAR:230) - Print signed number
    pub const PRINT_NUM: u8 = 0x06;

    /// random (VAR:231) - Generate random number
    pub const RANDOM: u8 = 0x07;

    /// push (VAR:232) - Push value onto stack
    pub const PUSH: u8 = 0x08;

    /// pull (VAR:233) - Pull value from stack (v6: store in variable)
    pub const PULL: u8 = 0x09;

    /// call_vs2 (VAR:236) - Call routine with up to 7 arguments (v4+)
    pub const CALL_VS2: u8 = 0x0C;
}

/// 1OP form opcodes (pass raw opcode number 0x00-0x0F to emit_instruction)
pub mod one_op {
    /// jz (1OP:128) - Jump if value is zero
    /// Pass 0x00 to emit_instruction, NOT 0x80
    pub const JZ: u8 = 0x00;

    /// get_sibling (1OP:129) - Get sibling of object, store result
    pub const GET_SIBLING: u8 = 0x01;

    /// get_child (1OP:130) - Get first child of object, store result
    pub const GET_CHILD: u8 = 0x02;

    /// get_parent (1OP:131) - Get parent of object, store result
    pub const GET_PARENT: u8 = 0x03;

    /// get_prop_len (1OP:132) - Get length of property
    pub const GET_PROP_LEN: u8 = 0x04;

    /// inc (1OP:133) - Increment variable
    pub const INC: u8 = 0x05;

    /// dec (1OP:134) - Decrement variable
    pub const DEC: u8 = 0x06;

    /// print_addr (1OP:135) - Print string at byte address
    pub const PRINT_ADDR: u8 = 0x07;

    /// call_1s (1OP:136) - Call routine with 1 argument, store result (v4+)
    pub const CALL_1S: u8 = 0x08;

    /// remove_obj (1OP:137) - Remove object from object tree
    pub const REMOVE_OBJ: u8 = 0x09;

    /// print_obj (1OP:138) - Print short name of object
    pub const PRINT_OBJ: u8 = 0x0A;

    /// ret (1OP:139) - Return from routine with value
    pub const RET: u8 = 0x0B;

    /// jump (1OP:140) - Unconditional jump
    pub const JUMP: u8 = 0x0C;

    /// print_paddr (1OP:141) - Print string at packed address
    /// Pass 0x0D to emit_instruction, NOT 0x8D
    pub const PRINT_PADDR: u8 = 0x0D;

    /// load (1OP:142) - Load variable value
    pub const LOAD: u8 = 0x0E;

    /// not (1OP:143) - Bitwise NOT (v1-4) or call_1n (v5+)
    pub const NOT: u8 = 0x0F;
}

/// 2OP form opcodes (pass raw opcode number 0x00-0x1F to emit_instruction)
pub mod two_op {
    /// je (2OP:1) - Jump if equal (supports multiple comparisons)
    pub const JE: u8 = 0x01;

    /// jl (2OP:2) - Jump if less than
    pub const JL: u8 = 0x02;

    /// jg (2OP:3) - Jump if greater than
    pub const JG: u8 = 0x03;

    /// dec_chk (2OP:4) - Decrement variable and check
    pub const DEC_CHK: u8 = 0x04;

    /// inc_chk (2OP:5) - Increment variable and check
    pub const INC_CHK: u8 = 0x05;

    /// jin (2OP:6) - Jump if object is child of another
    pub const JIN: u8 = 0x06;

    /// test (2OP:7) - Test bitmap flags
    pub const TEST: u8 = 0x07;

    /// or (2OP:8) - Bitwise OR
    pub const OR: u8 = 0x08;

    /// and (2OP:9) - Bitwise AND
    pub const AND: u8 = 0x09;

    /// test_attr (2OP:10) - Test object attribute
    pub const TEST_ATTR: u8 = 0x0A;

    /// set_attr (2OP:11) - Set object attribute
    pub const SET_ATTR: u8 = 0x0B;

    /// clear_attr (2OP:12) - Clear object attribute
    pub const CLEAR_ATTR: u8 = 0x0C;

    /// store (2OP:13) - Store value in variable
    pub const STORE: u8 = 0x0D;

    /// insert_obj (2OP:14) - Insert object into object tree
    pub const INSERT_OBJ: u8 = 0x0E;

    /// loadw (2OP:15) - Load word from array
    pub const LOADW: u8 = 0x0F;

    /// loadb (2OP:16) - Load byte from array
    pub const LOADB: u8 = 0x10;

    /// get_prop (2OP:17) - Get property value
    pub const GET_PROP: u8 = 0x11;

    /// get_prop_addr (2OP:18) - Get property data address
    pub const GET_PROP_ADDR: u8 = 0x12;

    /// get_next_prop (2OP:19) - Get next property number
    pub const GET_NEXT_PROP: u8 = 0x13;

    /// add (2OP:20) - Add two numbers
    pub const ADD: u8 = 0x14;

    /// sub (2OP:21) - Subtract two numbers
    pub const SUB: u8 = 0x15;

    /// mul (2OP:22) - Multiply two numbers
    pub const MUL: u8 = 0x16;

    /// div (2OP:23) - Divide two numbers
    pub const DIV: u8 = 0x17;

    /// mod_op (2OP:24) - Modulo operation
    pub const MOD: u8 = 0x18;

    /// call_2s (2OP:25) - Call routine with 2 arguments, store result (v4+)
    pub const CALL_2S: u8 = 0x19;

    /// call_2n (2OP:26) - Call routine with 2 arguments, no result (v5+)
    pub const CALL_2N: u8 = 0x1A;

    /// set_colour (2OP:27) - Set foreground/background color (v5+)
    pub const SET_COLOUR: u8 = 0x1B;

    /// throw (2OP:28) - Throw exception to catch frame (v5+)
    pub const THROW: u8 = 0x1C;
}

/// 0OP form opcodes (pass raw opcode number 0x00-0x0F to emit_instruction)
pub mod zero_op {
    /// rtrue (0OP:176) - Return true
    /// Pass 0x00 to emit_instruction, NOT 0xB0
    pub const RTRUE: u8 = 0x00;

    /// rfalse (0OP:177) - Return false
    pub const RFALSE: u8 = 0x01;

    /// print (0OP:178) - Print literal string
    pub const PRINT: u8 = 0x02;

    /// print_ret (0OP:179) - Print literal string and return true
    pub const PRINT_RET: u8 = 0x03;

    /// nop (0OP:180) - No operation
    pub const NOP: u8 = 0x04;

    /// save (0OP:181) - Save game (v1-3) or branch (v4)
    pub const SAVE: u8 = 0x05;

    /// restore (0OP:182) - Restore game (v1-3) or branch (v4)
    pub const RESTORE: u8 = 0x06;

    /// restart (0OP:183) - Restart game
    pub const RESTART: u8 = 0x07;

    /// ret_popped (0OP:184) - Return value from top of stack
    pub const RET_POPPED: u8 = 0x08;

    /// pop (0OP:185) - Pop value from stack and discard (v1-4) or catch (v5+)
    pub const POP: u8 = 0x09;

    /// quit (0OP:186) - Quit game
    pub const QUIT: u8 = 0x0A;

    /// new_line (0OP:187) - Print newline
    pub const NEW_LINE: u8 = 0x0B;

    /// show_status (0OP:188) - Show status line (v3 only)
    pub const SHOW_STATUS: u8 = 0x0C;

    /// verify (0OP:189) - Verify game file checksum
    pub const VERIFY: u8 = 0x0D;

    /// piracy (0OP:191) - Anti-piracy check (v5+)
    pub const PIRACY: u8 = 0x0F;
}
