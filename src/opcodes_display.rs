/// Display and text output operations for Z-Machine interpreter
///
/// This module handles all display-related operations including:
/// - Text output operations (print, print_ret, print_char, print_num, print_addr, print_paddr)
/// - Window management (split_window, set_window, erase_window, erase_line)
/// - Cursor control (set_cursor, get_cursor)
/// - Text styling (set_text_style)
/// - Output control (new_line, show_status, buffer_mode)
/// - Audio/UI feedback (sound_effect)
///
/// These operations form the user interface layer of Z-Machine games,
/// controlling how text appears on screen and managing the display system.
use crate::instruction::Instruction;
use crate::interpreter::{ExecutionResult, Interpreter};
use log::debug;
use std::io::{self, Write};

impl Interpreter {
    /// Handle display and text output opcodes
    pub fn execute_display_op(
        &mut self,
        inst: &Instruction,
        operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        match (inst.opcode, &inst.operand_count) {
            // ---- 0OP DISPLAY OPERATIONS ----

            // 0OP:0x02 - print (literal string)
            (0x02, crate::instruction::OperandCount::OP0) => {
                if let Some(ref text) = inst.text {
                    // Always log print instructions when debugging 'w' issue
                    if text.contains("can you attack") || text.contains("spirit") {
                        debug!(
                            "*** FOUND GARBAGE TEXT: print at PC {:05x}: '{}'",
                            self.vm.pc - inst.size as u32,
                            text
                        );
                    }
                    // Log first part of all print strings for debugging
                    let preview = if text.len() > 40 {
                        format!("{}...", &text[..40])
                    } else {
                        text.clone()
                    };
                    debug!(
                        "print at PC {:05x}: '{}'",
                        self.vm.pc - inst.size as u32,
                        preview
                    );

                    self.output_text(text)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 0OP:0x03 - print_ret
            (0x03, crate::instruction::OperandCount::OP0) => {
                if let Some(ref text) = inst.text {
                    self.output_text(text)?;
                    self.output_char('\n')?;
                }
                self.do_return(1)
            }

            // 0OP:0x0B - new_line
            (0x0B, crate::instruction::OperandCount::OP0) => {
                self.output_char('\n')?;
                Ok(ExecutionResult::Continue)
            }

            // 0OP:0x0C - show_status (V3 only)
            (0x0C, crate::instruction::OperandCount::OP0) => {
                if self.vm.game.header.version == 3 {
                    debug!("show_status called");

                    // Get location name from G16 (player's location in v3)
                    let location_obj = self.vm.read_global(16)?; // G16 contains player location in v3
                    let location_name = if location_obj > 0 {
                        self.vm
                            .get_object_name(location_obj)
                            .unwrap_or_else(|_| format!("Location {location_obj}"))
                    } else {
                        "Unknown".to_string()
                    };

                    let score = self.vm.read_global(17)? as i16; // G17 = score
                    let moves = self.vm.read_global(18)?; // G18 = moves

                    if let Some(ref mut display) = self.display {
                        display.show_status(&location_name, score, moves)?;
                    } else {
                        debug!("No display available for show_status");
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // ---- 1OP DISPLAY OPERATIONS ----

            // 1OP:0x0D - print_paddr
            (0x0D, crate::instruction::OperandCount::OP1) => {
                // Print string at packed address
                let pc = self.vm.pc - inst.size as u32;

                // Debug string ID 458's packed address
                if operands[0] == 0x04db {
                    log::debug!("*** print_paddr at PC {:05x} with packed address 0x{:04x} (string ID 458: 'There is ')", pc, operands[0]);
                    let unpacked = operands[0] as usize * 2;
                    log::debug!("*** Unpacked address: 0x{:04x}", unpacked);
                    log::debug!("*** Bytes at unpacked address: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                        self.vm.game.memory[unpacked], self.vm.game.memory[unpacked+1],
                        self.vm.game.memory[unpacked+2], self.vm.game.memory[unpacked+3],
                        self.vm.game.memory[unpacked+4], self.vm.game.memory[unpacked+5],
                        self.vm.game.memory[unpacked+6], self.vm.game.memory[unpacked+7]);
                }

                debug!("print_paddr at {:05x}: operand={:04x}", pc, operands[0]);

                // Check if this might be the problematic address
                if operands[0] == 0xa11d || operands[0] == 0x1da1 {
                    debug!(
                        "*** WARNING: print_paddr with suspicious address {:04x} ***",
                        operands[0]
                    );
                }

                let abbrev_addr = self.vm.game.header.abbrev_table;
                match crate::text::decode_string_at_packed_addr(
                    &self.vm.game.memory,
                    operands[0],
                    self.vm.game.header.version,
                    abbrev_addr,
                ) {
                    Ok(string) => {
                        if operands[0] == 0x04db {
                            log::debug!("*** Decoded string for 0x04db: '{}'", string);
                        }
                        self.output_text(&string)?;
                    }
                    Err(e) => {
                        debug!("Failed to decode string at {:04x}: {}", operands[0], e);
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // 1OP:0x07 - print_addr
            (0x07, crate::instruction::OperandCount::OP1) => {
                // Print string at unpacked address
                let addr = operands[0] as usize;
                let abbrev_addr = self.vm.game.header.abbrev_table;
                debug!(
                    "print_addr: addr={:04x} at PC {:05x}",
                    addr,
                    self.vm.pc - inst.size as u32
                );

                // Check if this might be related to our bug
                if addr == 0xa11d || addr == 0x1da1 {
                    debug!(
                        "*** WARNING: print_addr with suspicious address {:04x} ***",
                        addr
                    );
                    debug!("*** This might be the source of the 'w' garbage text! ***");
                }

                match crate::text::decode_string(&self.vm.game.memory, addr, abbrev_addr) {
                    Ok((string, _)) => {
                        self.output_text(&string)?;
                    }
                    Err(e) => {
                        debug!("Failed to decode string at {:04x}: {}", addr, e);
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // ---- VAR DISPLAY OPERATIONS ----

            // VAR:0x05 - print_char
            (0x05, crate::instruction::OperandCount::VAR) => {
                if !operands.is_empty() {
                    let ch = operands[0] as u8 as char;
                    let pc = self.vm.pc - inst.size as u32;

                    // Debug all print_char in error area
                    if (0x6300..=0x6400).contains(&pc) {
                        debug!(
                            "print_char at 0x{:04x}: '{}' (0x{:02x})",
                            pc, ch, operands[0]
                        );
                    }

                    // Debug spacing routine calls
                    if ch == ' ' {
                        debug!("*** SPACING ROUTINE: print_char SPACE at PC {:05x}", pc);
                    }

                    if operands[0] > 127 || operands[0] == 63 {
                        debug!(
                            "print_char: value={} (0x{:02x}) char='{}' at PC {:05x}",
                            operands[0], operands[0], ch, pc
                        );
                    }

                    self.output_char(ch)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x06 - print_num
            (0x06, crate::instruction::OperandCount::VAR) => {
                if !operands.is_empty() {
                    let num_str = format!("{}", operands[0] as i16);
                    self.output_text(&num_str)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x0A - split_window (V3+)
            (0x0A, crate::instruction::OperandCount::VAR) => {
                if !operands.is_empty() {
                    let lines = operands[0];
                    debug!("split_window: lines={}", lines);

                    if let Some(ref mut display) = self.display {
                        display.split_window(lines)?;
                    } else {
                        debug!("No display available for split_window");
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x0B - set_window (V3+)
            (0x0B, crate::instruction::OperandCount::VAR) => {
                if !operands.is_empty() {
                    let window = operands[0] as u8;
                    debug!("set_window: window={}", window);

                    if let Some(ref mut display) = self.display {
                        display.set_window(window)?;
                    } else {
                        debug!("No display available for set_window");
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x0D - erase_window
            (0x0D, crate::instruction::OperandCount::VAR) => {
                if !operands.is_empty() {
                    let window = operands[0] as i16;
                    debug!("erase_window: window={}", window);

                    if let Some(ref mut display) = self.display {
                        display.erase_window(window)?;
                    } else {
                        debug!("No display available for erase_window");
                    }
                } else {
                    debug!("erase_window called with no operands");
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x0E - erase_line (v4+)
            (0x0E, crate::instruction::OperandCount::VAR) => {
                // Erases the current line from cursor to end of line
                // Only available in v4+
                if self.vm.game.header.version < 4 {
                    return Err("erase_line is only available in v4+".to_string());
                }

                // operand[0] = pixel value (1 = current cursor position)
                // For text-only implementation, we only support value 1
                if !operands.is_empty() && operands[0] == 1 {
                    if let Some(ref mut display) = self.display {
                        display.erase_line()?;
                    } else {
                        // Simple terminal implementation: clear to end of line
                        print!("\x1b[K");
                        io::stdout().flush().ok();
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x10 - get_cursor (v4+)
            (0x10, crate::instruction::OperandCount::VAR) => {
                // Stores current cursor position in a word array
                // array-->0 = line (1-based)
                // array-->1 = column (1-based)
                if self.vm.game.header.version < 4 {
                    return Err("get_cursor is only available in v4+".to_string());
                }

                if !operands.is_empty() {
                    let array_addr = operands[0] as u32;

                    // Get cursor position from display
                    let (line, column) = if let Some(ref mut display) = self.display {
                        display.get_cursor()?
                    } else {
                        // Default position if no display
                        (1, 1)
                    };

                    // Store line and column in the array
                    self.vm.write_word(array_addr, line)?;
                    self.vm.write_word(array_addr + 2, column)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x0F - set_cursor
            (0x0F, crate::instruction::OperandCount::VAR) => {
                if operands.len() >= 2 {
                    let line = operands[0];
                    let column = operands[1];
                    debug!("set_cursor: line={}, column={}", line, column);

                    if let Some(ref mut display) = self.display {
                        display.set_cursor(line, column)?;
                    } else {
                        debug!("No display available for set_cursor");
                    }
                } else {
                    debug!("set_cursor called with insufficient operands");
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x11 - set_text_style
            (0x11, crate::instruction::OperandCount::VAR) => {
                // Style bits: 1=reverse, 2=bold, 4=italic, 8=fixed-pitch
                if !operands.is_empty() {
                    let style = operands[0];
                    debug!("set_text_style: style={}", style);

                    // Use the display system's text style handling
                    if let Some(ref mut display) = self.display {
                        display.set_text_style(style).ok();
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x15 - sound_effect
            (0x15, crate::instruction::OperandCount::VAR) => {
                if !operands.is_empty() {
                    let number = operands[0];
                    debug!("sound_effect: number={}", number);

                    // In v3, only bleeps 1 and 2 are supported (bell sounds)
                    if number == 1 || number == 2 {
                        // Output bell character for simple beep
                        self.output_char('\x07')?;
                    }
                    // For v3, ignore other sound numbers and effects
                    // The Lurking Horror would use numbers 3+ for real sounds
                    // but we don't implement actual sound effects
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x12 - buffer_mode (v4+)
            (0x12, crate::instruction::OperandCount::VAR) => {
                // Controls whether output is buffered
                // operand[0]: 0 = off (flush after every char), 1 = on (buffer output)
                if self.vm.game.header.version < 4 {
                    return Err("buffer_mode is only available in v4+".to_string());
                }

                if !operands.is_empty() {
                    let mode = operands[0];
                    debug!("buffer_mode: {}", if mode == 0 { "off" } else { "on" });

                    if let Some(ref mut display) = self.display {
                        display.set_buffer_mode(mode != 0)?;
                    } else {
                        // For stdout, we can flush immediately when buffer mode is off
                        if mode == 0 {
                            io::stdout().flush().ok();
                        }
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            _ => Err(format!(
                "Unhandled display opcode: {:02x} with operand count {:?}",
                inst.opcode, inst.operand_count
            )),
        }
    }

    /// Check if an opcode is a display operation
    pub fn is_display_opcode(opcode: u8, operand_count: &crate::instruction::OperandCount) -> bool {
        matches!(
            (opcode, operand_count),
            // 0OP display operations
            (0x02, crate::instruction::OperandCount::OP0) |  // print
            (0x03, crate::instruction::OperandCount::OP0) |  // print_ret
            (0x0B, crate::instruction::OperandCount::OP0) |  // new_line
            (0x0C, crate::instruction::OperandCount::OP0) |  // show_status
            // 1OP display operations
            (0x0D, crate::instruction::OperandCount::OP1) |  // print_paddr
            (0x07, crate::instruction::OperandCount::OP1) |  // print_addr
            // VAR display operations
            (0x05, crate::instruction::OperandCount::VAR) |  // print_char
            (0x06, crate::instruction::OperandCount::VAR) |  // print_num
            (0x0A, crate::instruction::OperandCount::VAR) |  // split_window
            (0x0B, crate::instruction::OperandCount::VAR) |  // set_window
            (0x0D, crate::instruction::OperandCount::VAR) |  // erase_window
            (0x0E, crate::instruction::OperandCount::VAR) |  // erase_line
            (0x10, crate::instruction::OperandCount::VAR) |  // get_cursor
            (0x0F, crate::instruction::OperandCount::VAR) |  // set_cursor
            (0x11, crate::instruction::OperandCount::VAR) |  // set_text_style
            (0x12, crate::instruction::OperandCount::VAR) |  // buffer_mode
            (0x15, crate::instruction::OperandCount::VAR) // sound_effect
        )
    }
}
