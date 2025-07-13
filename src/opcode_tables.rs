use crate::instruction::{InstructionForm, OperandCount};

/// Get the name of an opcode based on its form and value
pub fn get_instruction_name(
    opcode: u8,
    ext_opcode: Option<u8>,
    form: InstructionForm,
    operand_count: OperandCount,
    _version: u8,
) -> &'static str {
    match form {
        InstructionForm::Extended => get_extended_opcode_name(ext_opcode.unwrap_or(0)),
        InstructionForm::Variable => get_variable_opcode_name(opcode, operand_count),
        InstructionForm::Short => match operand_count {
            OperandCount::OP0 => get_0op_opcode_name(opcode),
            OperandCount::OP1 => get_1op_opcode_name(opcode),
            _ => "unknown",
        },
        InstructionForm::Long => get_2op_opcode_name(opcode),
    }
}

/// Get name for 2OP opcodes (long form)
fn get_2op_opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x01 => "je",
        0x02 => "jl",
        0x03 => "jg",
        0x04 => "dec_chk",
        0x05 => "inc_chk",
        0x06 => "jin",
        0x07 => "test",
        0x08 => "or",
        0x09 => "and",
        0x0A => "test_attr",
        0x0B => "set_attr",
        0x0C => "clear_attr",
        0x0D => "store",
        0x0E => "insert_obj",
        0x0F => "loadw",
        0x10 => "loadb",
        0x11 => "get_prop",
        0x12 => "get_prop_addr",
        0x13 => "get_next_prop",
        0x14 => "add",
        0x15 => "sub",
        0x16 => "mul",
        0x17 => "div",
        0x18 => "mod",
        0x19 => "call_2s",
        0x1A => "call_2n",
        0x1B => "set_colour",
        0x1C => "throw",
        _ => "unknown_2op",
    }
}

/// Get name for 1OP opcodes (short form)
fn get_1op_opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "jz",
        0x01 => "get_sibling",
        0x02 => "get_child",
        0x03 => "get_parent",
        0x04 => "get_prop_len",
        0x05 => "inc",
        0x06 => "dec",
        0x07 => "print_addr",
        0x08 => "call_1s",
        0x09 => "remove_obj",
        0x0A => "print_obj",
        0x0B => "ret",
        0x0C => "jump",
        0x0D => "print_paddr",
        0x0E => "load",
        0x0F => "not", // V1-4, call_1n in V5+
        _ => "unknown_1op",
    }
}

/// Get name for 0OP opcodes (short form)
fn get_0op_opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "rtrue",
        0x01 => "rfalse",
        0x02 => "print",
        0x03 => "print_ret",
        0x04 => "nop",
        0x05 => "save",
        0x06 => "restore",
        0x07 => "restart",
        0x08 => "ret_popped",
        0x09 => "pop", // V1-4, catch in V5+
        0x0A => "quit",
        0x0B => "new_line",
        0x0C => "show_status", // V3 only
        0x0D => "verify",
        0x0E => "extended", // V5+
        0x0F => "piracy",
        _ => "unknown_0op",
    }
}

/// Get name for VAR opcodes (variable form)
fn get_variable_opcode_name(opcode: u8, operand_count: OperandCount) -> &'static str {
    // Opcodes 0xC0-0xDF are variable forms of 2OP:0-31
    if opcode >= 0x20 {
        return get_2op_opcode_name(opcode - 0x20);
    }

    // For opcodes 0x00-0x1F, check operand_count to distinguish 2OP vs VAR
    if operand_count == OperandCount::OP2 {
        return get_2op_opcode_name(opcode);
    }

    // True VAR opcodes (0xE0-0xFF in encoding, but stored as 0x00-0x1F)
    match opcode {
        0x00 => "call", // call_vs in V4+
        0x01 => "storew",
        0x02 => "storeb",
        0x03 => "put_prop",
        0x04 => "sread", // V1-4, aread in V5+
        0x05 => "print_char",
        0x06 => "print_num",
        0x07 => "random",
        0x08 => "push",
        0x09 => "pull",
        0x0A => "split_window",    // V3+
        0x0B => "set_window",      // V3+
        0x0C => "call_vs2",        // V4+
        0x0D => "erase_window",    // V4+
        0x0E => "erase_line",      // V4+
        0x0F => "set_cursor",      // V4+
        0x10 => "get_cursor",      // V4+
        0x11 => "set_text_style",  // V4+
        0x12 => "buffer_mode",     // V4+
        0x13 => "output_stream",   // V3+
        0x14 => "input_stream",    // V3+
        0x15 => "sound_effect",    // V3+
        0x16 => "read_char",       // V4+
        0x17 => "scan_table",      // V4+
        0x18 => "not",             // V5+
        0x19 => "call_vn",         // V5+
        0x1A => "call_vn2",        // V5+
        0x1B => "tokenise",        // V5+
        0x1C => "encode_text",     // V5+
        0x1D => "copy_table",      // V5+
        0x1E => "print_table",     // V5+
        0x1F => "check_arg_count", // V5+
        _ => "unknown_var",
    }
}

/// Get name for extended opcodes (V5+)
fn get_extended_opcode_name(ext_opcode: u8) -> &'static str {
    match ext_opcode {
        0x00 => "save",
        0x01 => "restore",
        0x02 => "log_shift",
        0x03 => "art_shift",
        0x04 => "set_font",
        0x05 => "draw_picture",  // V6
        0x06 => "picture_data",  // V6
        0x07 => "erase_picture", // V6
        0x08 => "set_margins",   // V6
        0x09 => "save_undo",     // V5+
        0x0A => "restore_undo",  // V5+
        0x0B => "print_unicode", // V5+
        0x0C => "check_unicode", // V5+
        0x10 => "move_window",   // V6
        0x11 => "window_size",   // V6
        0x12 => "window_style",  // V6
        0x13 => "get_wind_prop", // V6
        0x14 => "scroll_window", // V6
        0x15 => "pop_stack",     // V6
        0x16 => "read_mouse",    // V6
        0x17 => "mouse_window",  // V6
        0x18 => "push_stack",    // V6
        0x19 => "put_wind_prop", // V6
        0x1A => "print_form",    // V6
        0x1B => "make_menu",     // V6
        0x1C => "picture_table", // V6
        _ => "unknown_ext",
    }
}

/// Check if an instruction stores a result
pub fn stores_result(
    opcode: u8,
    ext_opcode: Option<u8>,
    form: InstructionForm,
    operand_count: OperandCount,
    _version: u8,
) -> bool {
    match form {
        InstructionForm::Extended => {
            match ext_opcode {
                Some(0x00..=0x04) => true, // save, restore, shifts, set_font
                Some(0x09..=0x0C) => true, // undo ops, unicode ops
                _ => false,
            }
        }
        InstructionForm::Variable => {
            if opcode >= 0x20 {
                // Variable form of 2OP
                stores_result_2op(opcode - 0x20)
            } else {
                // For opcodes 0x00-0x1F, check operand_count to distinguish 2OP vs VAR
                if operand_count == OperandCount::OP2 {
                    stores_result_2op(opcode)
                } else {
                    // True VAR opcodes
                    match opcode {
                        0x00 | 0x07 | 0x08 | 0x16 | 0x17 => true, // call, random, push, read_char, scan_table
                        _ => false,
                    }
                }
            }
        }
        InstructionForm::Short => {
            match operand_count {
                OperandCount::OP0 => {
                    match opcode {
                        0x09 => true, // catch (V5+)
                        _ => false,
                    }
                }
                OperandCount::OP1 => {
                    match opcode {
                        0x01..=0x04 => true, // get_sibling, get_child, get_parent, get_prop_len
                        0x08 => true,        // call_1s
                        0x0E => true,        // load
                        0x0F => true,        // not (V1-4)
                        _ => false,
                    }
                }
                _ => false,
            }
        }
        InstructionForm::Long => stores_result_2op(opcode),
    }
}

fn stores_result_2op(opcode: u8) -> bool {
    match opcode {
        0x08..=0x09 => true, // or, and
        0x0F..=0x13 => true, // loadw, loadb, get_prop, get_prop_addr, get_next_prop
        0x14..=0x18 => true, // add, sub, mul, div, mod
        0x19 => true,        // call_2s
        0x1F => true,        // undocumented - appears to store result
        _ => false,
    }
}

/// Check if an instruction has a branch
pub fn has_branch(
    opcode: u8,
    _ext_opcode: Option<u8>,
    form: InstructionForm,
    operand_count: OperandCount,
    _version: u8,
) -> bool {
    match form {
        InstructionForm::Extended => false,
        InstructionForm::Variable => {
            if opcode >= 0x20 {
                // Variable form of 2OP
                has_branch_2op(opcode - 0x20)
            } else {
                // For opcodes 0x00-0x1F, check operand_count to distinguish 2OP vs VAR
                if operand_count == OperandCount::OP2 {
                    has_branch_2op(opcode)
                } else {
                    false
                }
            }
        }
        InstructionForm::Short => {
            match operand_count {
                OperandCount::OP0 => {
                    match opcode {
                        0x05 | 0x06 | 0x0D | 0x0F => true, // save, restore, verify, piracy
                        _ => false,
                    }
                }
                OperandCount::OP1 => {
                    match opcode {
                        0x00..=0x02 => true, // jz, get_sibling, get_child
                        _ => false,
                    }
                }
                _ => false,
            }
        }
        InstructionForm::Long => has_branch_2op(opcode),
    }
}

fn has_branch_2op(opcode: u8) -> bool {
    match opcode {
        0x01..=0x07 => true, // je, jl, jg, dec_chk, inc_chk, jin, test
        0x0A => true,        // test_attr
        _ => false,
    }
}

/// Check if an instruction has inline text
pub fn has_text(
    opcode: u8,
    _ext_opcode: Option<u8>,
    form: InstructionForm,
    operand_count: OperandCount,
    _version: u8,
) -> bool {
    match form {
        InstructionForm::Short => {
            match operand_count {
                OperandCount::OP0 => {
                    match opcode {
                        0x02 | 0x03 => true, // print, print_ret
                        _ => false,
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Get the expected number of operands for an instruction
/// Returns None if the instruction uses all operands specified in the operand type byte
pub fn get_expected_operand_count(
    opcode: u8,
    _ext_opcode: Option<u8>,
    form: InstructionForm,
    operand_count: OperandCount,
    version: u8,
) -> Option<usize> {
    match form {
        InstructionForm::Variable => {
            if opcode >= 0x20 {
                // Variable form of 2OP - always 2 operands
                Some(2)
            } else {
                // For opcodes 0x00-0x1F, check operand_count to distinguish 2OP vs VAR
                if operand_count == OperandCount::OP2 {
                    Some(2)
                } else {
                    // True VAR opcodes
                match opcode {
                    0x00 => None,    // call - variable operands
                    0x01 => Some(3), // storew - array, word-index, value
                    0x02 => Some(3), // storeb - array, byte-index, value
                    0x03 => Some(3), // put_prop - object, property, value
                    0x04 => None,    // sread - variable operands
                    0x05 => Some(1), // print_char - character
                    0x06 => Some(1), // print_num - number
                    0x07 => Some(1), // random - range
                    0x08 => Some(1), // push - value
                    0x09 => {
                        // pull - 1 operand in V1-5, variable in V6
                        if version <= 5 {
                            Some(1)
                        } else {
                            None
                        }
                    }
                    0x0A => Some(1), // split_window - lines
                    0x0B => Some(1), // set_window - window
                    _ => None,
                }
                }
            }
        }
        _ => None, // Other forms use all operands specified
    }
}
