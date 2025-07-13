use crate::instruction::{Instruction, InstructionForm, OperandCount};
use crate::vm::Game;
use std::fmt::Write;
// use log::{debug, info}; // Ready for future use

/// A disassembler for Z-Machine code
pub struct Disassembler<'a> {
    game: &'a Game,
    version: u8,
}

impl<'a> Disassembler<'a> {
    /// Create a new disassembler
    pub fn new(game: &'a Game) -> Self {
        Disassembler {
            game,
            version: game.header.version,
        }
    }

    /// Disassemble a single instruction at the given address
    pub fn disassemble_instruction(&self, addr: u32) -> Result<(Instruction, String), String> {
        let instruction = Instruction::decode(&self.game.memory, addr as usize, self.version)?;

        let mut output = String::new();
        write!(output, "{:05x}: ", addr).unwrap();

        // Write raw bytes
        let mut byte_str = String::new();
        for i in 0..instruction.size.min(8) {
            write!(byte_str, "{:02x} ", self.game.memory[(addr as usize) + i]).unwrap();
        }
        if instruction.size > 8 {
            byte_str.push_str("... ");
        }
        write!(output, "{:<24} ", byte_str).unwrap();

        // Write decoded instruction
        write!(output, "{}", instruction.format_with_version(self.version)).unwrap();

        Ok((instruction, output))
    }

    /// Disassemble a range of memory
    pub fn disassemble_range(&self, start: u32, end: u32) -> Result<String, String> {
        let mut output = String::new();
        let mut addr = start;

        while addr < end {
            match self.disassemble_instruction(addr) {
                Ok((instruction, line)) => {
                    writeln!(output, "{}", line).unwrap();
                    addr += instruction.size as u32;
                }
                Err(e) => {
                    writeln!(output, "{:05x}: <error: {}>", addr, e).unwrap();
                    addr += 1; // Skip bad byte
                }
            }
        }

        Ok(output)
    }

    /// Disassemble a routine at the given packed address
    pub fn disassemble_routine(&self, packed_addr: u16) -> Result<String, String> {
        let addr = self.unpack_address(packed_addr) as u32;
        let mut output = String::new();

        writeln!(
            output,
            "\n; Routine at packed address {:04x} (unpacked: {:05x})",
            packed_addr, addr
        )
        .unwrap();

        // Read routine header
        let num_locals = self.game.memory[addr as usize];
        writeln!(output, "; {} local variables", num_locals).unwrap();

        let mut pc = addr + 1;

        // In V1-4, local variable initial values follow
        if self.version <= 4 {
            for i in 0..num_locals {
                let value = ((self.game.memory[pc as usize] as u16) << 8)
                    | (self.game.memory[(pc + 1) as usize] as u16);
                writeln!(output, "; Local {} = {:04x}", i + 1, value).unwrap();
                pc += 2;
            }
        }

        writeln!(output, "\n; Code begins at {:05x}", pc).unwrap();

        // Disassemble until we hit a return or invalid instruction
        let mut seen_addrs = std::collections::HashSet::new();
        while pc < self.game.memory.len() as u32 {
            if seen_addrs.contains(&pc) {
                writeln!(output, "; Loop detected at {:05x}", pc).unwrap();
                break;
            }
            seen_addrs.insert(pc);

            match self.disassemble_instruction(pc) {
                Ok((instruction, line)) => {
                    writeln!(output, "{}", line).unwrap();

                    // Check for routine-ending instructions
                    match instruction.opcode {
                        0x00 | 0x08 | 0x09 => {
                            // rtrue, ret_popped, pop (sometimes ends routines)
                            if instruction.form == InstructionForm::Short
                                && instruction.operand_count == OperandCount::OP0
                            {
                                writeln!(output, "; End of routine").unwrap();
                                break;
                            }
                        }
                        0x0B => {
                            // ret
                            writeln!(output, "; End of routine").unwrap();
                            break;
                        }
                        _ => {}
                    }

                    pc += instruction.size as u32;
                }
                Err(e) => {
                    writeln!(output, "{:05x}: <error: {}>", pc, e).unwrap();
                    break;
                }
            }

            // Safety check to prevent infinite loops
            if pc > addr + 10000 {
                writeln!(output, "; Routine too large, stopping disassembly").unwrap();
                break;
            }
        }

        Ok(output)
    }

    /// Unpack a packed address based on version
    fn unpack_address(&self, packed: u16) -> usize {
        match self.version {
            1..=3 => (packed as usize) * 2,
            4..=5 => (packed as usize) * 4,
            6..=7 => {
                // In V6/7, need to add offset from header
                let offset =
                    ((self.game.memory[0x28] as usize) << 8) | (self.game.memory[0x29] as usize);
                (packed as usize) * 4 + offset * 8
            }
            8 => (packed as usize) * 8,
            _ => (packed as usize) * 2, // Default to V3 behavior
        }
    }

    /// Find and disassemble the main routine
    pub fn disassemble_main(&self) -> Result<String, String> {
        let main_pc = self.game.header.initial_pc as u32;
        let mut output = String::new();

        writeln!(output, "Main routine starts at PC {:05x}", main_pc).unwrap();
        writeln!(output, "Version: {}", self.version).unwrap();
        writeln!(output, "").unwrap();

        // Start disassembling from main
        let disasm = self.disassemble_range(main_pc, main_pc + 100)?;
        output.push_str(&disasm);

        Ok(output)
    }

    /// Create a full opcode reference table
    pub fn opcode_table(&self) -> String {
        let mut output = String::new();

        writeln!(
            output,
            "Z-Machine Opcode Reference (Version {})",
            self.version
        )
        .unwrap();
        writeln!(output, "=========================================").unwrap();

        // 0OP opcodes
        writeln!(output, "\n0OP Instructions:").unwrap();
        writeln!(output, "  B0: rtrue").unwrap();
        writeln!(output, "  B1: rfalse").unwrap();
        writeln!(output, "  B2: print (literal string)").unwrap();
        writeln!(output, "  B3: print_ret (literal string)").unwrap();
        writeln!(output, "  B4: nop").unwrap();
        writeln!(output, "  B5: save [branch]").unwrap();
        writeln!(output, "  B6: restore [branch]").unwrap();
        writeln!(output, "  B7: restart").unwrap();
        writeln!(output, "  B8: ret_popped").unwrap();
        writeln!(output, "  B9: pop (V1-4) / catch (V5+)").unwrap();
        writeln!(output, "  BA: quit").unwrap();
        writeln!(output, "  BB: new_line").unwrap();
        writeln!(output, "  BC: show_status (V3)").unwrap();
        writeln!(output, "  BD: verify [branch]").unwrap();
        writeln!(output, "  BE: [extended opcode]").unwrap();
        writeln!(output, "  BF: piracy [branch]").unwrap();

        // 1OP opcodes
        writeln!(output, "\n1OP Instructions:").unwrap();
        writeln!(output, "  80-8F: jz operand [branch]").unwrap();
        writeln!(output, "  90-9F: get_sibling object -> result [branch]").unwrap();
        writeln!(output, "  A0-AF: get_child object -> result [branch]").unwrap();
        writeln!(output, "  B0-BF: get_parent object -> result").unwrap();
        writeln!(output, "  C0-CF: get_prop_len prop_addr -> result").unwrap();
        writeln!(output, "  D0-DF: inc variable").unwrap();
        writeln!(output, "  E0-EF: dec variable").unwrap();
        writeln!(output, "  F0-FF: print_addr addr").unwrap();

        // 2OP opcodes
        writeln!(output, "\n2OP Instructions:").unwrap();
        writeln!(output, "  01-1F: je a b [branch]").unwrap();
        writeln!(output, "  02-1F: jl a b [branch]").unwrap();
        writeln!(output, "  03-1F: jg a b [branch]").unwrap();
        writeln!(output, "  04-1F: dec_chk variable value [branch]").unwrap();
        writeln!(output, "  05-1F: inc_chk variable value [branch]").unwrap();
        writeln!(output, "  06-1F: jin obj1 obj2 [branch]").unwrap();
        writeln!(output, "  07-1F: test bitmap flags [branch]").unwrap();
        writeln!(output, "  08-1F: or a b -> result").unwrap();
        writeln!(output, "  09-1F: and a b -> result").unwrap();
        writeln!(output, "  0A-1F: test_attr object attribute [branch]").unwrap();
        writeln!(output, "  0B-1F: set_attr object attribute").unwrap();
        writeln!(output, "  0C-1F: clear_attr object attribute").unwrap();
        writeln!(output, "  0D-1F: store variable value").unwrap();
        writeln!(output, "  0E-1F: insert_obj object destination").unwrap();
        writeln!(output, "  0F-1F: loadw array index -> result").unwrap();
        writeln!(output, "  10-1F: loadb array index -> result").unwrap();
        writeln!(output, "  11-1F: get_prop object property -> result").unwrap();
        writeln!(output, "  12-1F: get_prop_addr object property -> result").unwrap();
        writeln!(output, "  13-1F: get_next_prop object property -> result").unwrap();
        writeln!(output, "  14-1F: add a b -> result").unwrap();
        writeln!(output, "  15-1F: sub a b -> result").unwrap();
        writeln!(output, "  16-1F: mul a b -> result").unwrap();
        writeln!(output, "  17-1F: div a b -> result").unwrap();
        writeln!(output, "  18-1F: mod a b -> result").unwrap();

        // VAR opcodes
        writeln!(output, "\nVAR Instructions:").unwrap();
        writeln!(output, "  E0: call routine arg1 ... -> result").unwrap();
        writeln!(output, "  E1: storew array index value").unwrap();
        writeln!(output, "  E2: storeb array index value").unwrap();
        writeln!(output, "  E3: put_prop object property value").unwrap();
        writeln!(output, "  E4: read text parse").unwrap();
        writeln!(output, "  E5: print_char char").unwrap();
        writeln!(output, "  E6: print_num number").unwrap();
        writeln!(output, "  E7: random range -> result").unwrap();
        writeln!(output, "  E8: push value").unwrap();
        writeln!(output, "  E9: pull variable").unwrap();
        writeln!(output, "  EA: split_window lines").unwrap();
        writeln!(output, "  EB: set_window window").unwrap();
        writeln!(output, "  EC: call_2s routine arg -> result").unwrap();
        writeln!(output, "  ED: erase_window window").unwrap();
        writeln!(output, "  EE: erase_line").unwrap();
        writeln!(output, "  EF: set_cursor line column").unwrap();
        writeln!(output, "  F0: get_cursor array").unwrap();
        writeln!(output, "  F1: set_text_style style").unwrap();
        writeln!(output, "  F2: buffer_mode flag").unwrap();
        writeln!(output, "  F3: output_stream number table").unwrap();
        writeln!(output, "  F4: input_stream number").unwrap();
        writeln!(output, "  F5: sound_effect number effect volume routine").unwrap();

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_game() -> Game {
        let mut memory = vec![0u8; 0x10000];

        // Set up header
        memory[0x00] = 3; // Version 3
        memory[0x06] = 0x10; // Initial PC at 0x1000
        memory[0x07] = 0x00;

        // Add some test instructions at 0x1000
        // rtrue (0xB0)
        memory[0x1000] = 0xB0;

        // je #01 #02 [branch +3]
        memory[0x1001] = 0x41; // Long form, je, both small constants
        memory[0x1002] = 0x01; // Operand 1
        memory[0x1003] = 0x02; // Operand 2
        memory[0x1004] = 0x43; // Branch on true, offset +3

        // print "Hello"
        memory[0x1005] = 0xB2; // print

        Game::from_memory(memory).unwrap()
    }

    #[test]
    fn test_disassemble_instruction() {
        let game = create_test_game();
        let disasm = Disassembler::new(&game);

        let (inst, output) = disasm.disassemble_instruction(0x1000).unwrap();
        assert_eq!(inst.opcode, 0x00); // rtrue has opcode 0 in short form
        assert!(output.contains("rtrue"));

        let (inst, output) = disasm.disassemble_instruction(0x1001).unwrap();
        assert_eq!(inst.opcode, 0x01); // je
        assert!(output.contains("je"));
        // Instruction 0x41 = variable first operand, constant second
        assert!(output.contains("V01"));
        assert!(output.contains("#0002"));
    }

    #[test]
    fn test_disassemble_range() {
        let game = create_test_game();
        let disasm = Disassembler::new(&game);

        let output = disasm.disassemble_range(0x1000, 0x1006).unwrap();
        assert!(output.contains("rtrue"));
        assert!(output.contains("je"));
        assert!(output.contains("print"));
    }

    #[test]
    fn test_unpack_address() {
        let game = create_test_game();
        let disasm = Disassembler::new(&game);

        // Version 3: multiply by 2
        assert_eq!(disasm.unpack_address(0x1000), 0x2000);
        assert_eq!(disasm.unpack_address(0x0800), 0x1000);
    }
}
