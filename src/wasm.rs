//! WASM entry point for the Gruesome Z-Machine interpreter
//!
//! This module provides the JavaScript-facing API for running Z-Machine games
//! in a web browser. It wraps the core VM with a WASM-friendly interface.

#![cfg(feature = "wasm")]

use wasm_bindgen::prelude::*;

use crate::interpreter::core::instruction::{BranchInfo, Instruction, OperandType};
use crate::interpreter::core::vm::{CallFrame, Game, VM};
use crate::interpreter::display::display_wasm::WasmDisplay;
use crate::interpreter::display::ZMachineDisplay;
use crate::interpreter::text;

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();
}

/// Result returned from stepping the interpreter
#[wasm_bindgen]
pub struct StepResult {
    output: String,
    needs_input: bool,
    quit: bool,
    status_location: String,
    status_score: i16,
    status_moves: u16,
    error: Option<String>,
}

#[wasm_bindgen]
impl StepResult {
    #[wasm_bindgen(getter)]
    pub fn output(&self) -> String {
        self.output.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn needs_input(&self) -> bool {
        self.needs_input
    }

    #[wasm_bindgen(getter)]
    pub fn quit(&self) -> bool {
        self.quit
    }

    #[wasm_bindgen(getter)]
    pub fn status_location(&self) -> String {
        self.status_location.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn status_score(&self) -> i16 {
        self.status_score
    }

    #[wasm_bindgen(getter)]
    pub fn status_moves(&self) -> u16 {
        self.status_moves
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

/// WASM-friendly Z-Machine interpreter
#[wasm_bindgen]
pub struct WasmInterpreter {
    vm: VM,
    display: WasmDisplay,
    pending_input: Option<String>,
    waiting_for_input: bool,
    version: u8,
    status_location: String,
    status_score: i16,
    status_moves: u16,
}

#[wasm_bindgen]
impl WasmInterpreter {
    /// Create a new interpreter from game data
    #[wasm_bindgen(constructor)]
    pub fn new(game_data: &[u8]) -> Result<WasmInterpreter, JsValue> {
        let game = Game::from_memory(game_data.to_vec())
            .map_err(|e| JsValue::from_str(&format!("Failed to load game: {}", e)))?;

        let version = game.header.version;
        let vm = VM::new(game);
        let display =
            WasmDisplay::new().map_err(|e| JsValue::from_str(&format!("Display error: {}", e)))?;

        Ok(WasmInterpreter {
            vm,
            display,
            pending_input: None,
            waiting_for_input: false,
            version,
            status_location: String::new(),
            status_score: 0,
            status_moves: 0,
        })
    }

    /// Provide input from JavaScript
    #[wasm_bindgen]
    pub fn provide_input(&mut self, input: &str) {
        self.pending_input = Some(input.to_string());
        self.waiting_for_input = false;
    }

    /// Run the interpreter until it needs input or quits
    #[wasm_bindgen]
    pub fn step(&mut self) -> StepResult {
        let mut quit = false;
        let mut error = None;

        const MAX_INSTRUCTIONS: u32 = 10000;
        let mut instruction_count = 0;

        while instruction_count < MAX_INSTRUCTIONS {
            if self.waiting_for_input {
                if self.pending_input.is_some() {
                    self.waiting_for_input = false;
                } else {
                    break;
                }
            }

            let pc = self.vm.pc;
            let inst = match Instruction::decode(&self.vm.game.memory, pc as usize, self.version) {
                Ok(inst) => inst,
                Err(e) => {
                    error = Some(format!("Decode error at {:04x}: {}", pc, e));
                    break;
                }
            };

            self.vm.pc += inst.size as u32;

            match self.execute_instruction(&inst) {
                Ok(result) => match result {
                    WasmExecutionResult::Continue => {}
                    WasmExecutionResult::WaitForInput => {
                        self.waiting_for_input = true;
                        break;
                    }
                    WasmExecutionResult::Quit => {
                        quit = true;
                        break;
                    }
                },
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }

            instruction_count += 1;
        }

        self.display.force_refresh().ok();
        let output = self.display.take_output();

        for msg in self.display.take_messages() {
            if let crate::interpreter::display::display_wasm::WasmDisplayMessage::StatusUpdate {
                location,
                score,
                moves,
            } = msg
            {
                self.status_location = location;
                self.status_score = score;
                self.status_moves = moves;
            }
        }

        StepResult {
            output,
            needs_input: self.waiting_for_input,
            quit,
            status_location: self.status_location.clone(),
            status_score: self.status_score,
            status_moves: self.status_moves,
            error,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u8 {
        self.version
    }
}

enum WasmExecutionResult {
    Continue,
    WaitForInput,
    Quit,
}

impl WasmInterpreter {
    /// Get operand value by index
    fn get_operand(&self, inst: &Instruction, index: usize) -> Result<u16, String> {
        if index >= inst.operands.len() {
            return Err(format!("Operand {} out of range", index));
        }

        let op_type = inst
            .operand_types
            .get(index)
            .copied()
            .unwrap_or(OperandType::Omitted);
        let value = inst.operands[index];

        match op_type {
            OperandType::LargeConstant | OperandType::SmallConstant => Ok(value),
            OperandType::Variable => self.vm.read_variable(value as u8),
            OperandType::Omitted => Err(format!("Operand {} is omitted", index)),
        }
    }

    /// Handle branch
    fn handle_branch(&mut self, inst: &Instruction, condition: bool) -> Result<(), String> {
        if let Some(ref branch) = inst.branch {
            let should_branch = condition == branch.on_true;
            if should_branch {
                match branch.offset {
                    0 => self.return_from_routine(0)?,
                    1 => self.return_from_routine(1)?,
                    offset => {
                        self.vm.pc = (self.vm.pc as i32 + offset as i32 - 2) as u32;
                    }
                }
            }
        }
        Ok(())
    }

    /// Unpack packed address for strings
    fn unpack_string_addr(&self, paddr: u16) -> usize {
        match self.version {
            1..=3 => (paddr as usize) * 2,
            4..=5 => (paddr as usize) * 4,
            _ => (paddr as usize) * 8,
        }
    }

    /// Unpack packed address for routines
    fn unpack_routine_addr(&self, paddr: u16) -> usize {
        match self.version {
            1..=3 => (paddr as usize) * 2,
            4..=5 => (paddr as usize) * 4,
            _ => (paddr as usize) * 8,
        }
    }

    /// Call a routine
    fn call_routine(&mut self, inst: &Instruction, store_result: bool) -> Result<(), String> {
        let packed_addr = self.get_operand(inst, 0)?;

        if packed_addr == 0 {
            if store_result {
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, 0)?;
                }
            }
            return Ok(());
        }

        let routine_addr = self.unpack_routine_addr(packed_addr);
        let num_locals = self.vm.read_byte(routine_addr as u32);

        if num_locals > 15 {
            return Err(format!("Too many locals: {}", num_locals));
        }

        let mut locals = [0u16; 16];
        let code_start = if self.version <= 4 {
            for i in 0..num_locals as usize {
                locals[i] = self.vm.read_word((routine_addr + 1 + i * 2) as u32);
            }
            routine_addr + 1 + (num_locals as usize) * 2
        } else {
            routine_addr + 1
        };

        // Copy arguments to locals
        for i in 1..inst.operands.len() {
            let op_type = inst
                .operand_types
                .get(i)
                .copied()
                .unwrap_or(OperandType::Omitted);
            if op_type != OperandType::Omitted && i - 1 < num_locals as usize {
                let arg_value = self.get_operand(inst, i)?;
                locals[i - 1] = arg_value;
            }
        }

        let frame = CallFrame {
            return_pc: self.vm.pc,
            return_store: if store_result { inst.store_var } else { None },
            num_locals,
            locals,
            stack_base: self.vm.stack.len(),
        };

        self.vm.call_stack.push(frame);
        self.vm.pc = code_start as u32;

        Ok(())
    }

    /// Return from routine
    fn return_from_routine(&mut self, value: u16) -> Result<(), String> {
        if let Some(frame) = self.vm.call_stack.pop() {
            self.vm.stack.truncate(frame.stack_base);
            if let Some(store_var) = frame.return_store {
                self.vm.write_variable(store_var, value)?;
            }
            self.vm.pc = frame.return_pc;
            Ok(())
        } else {
            Err("Return from main routine".to_string())
        }
    }

    /// Update status line (V3)
    fn update_status_line(&mut self) -> Result<(), String> {
        if self.version > 3 {
            return Ok(());
        }

        let location_obj = self.vm.read_global(0x10)?;
        let location = if location_obj > 0 {
            self.vm.get_object_name(location_obj)?
        } else {
            String::new()
        };

        let score = self.vm.read_global(0x11)? as i16;
        let moves = self.vm.read_global(0x12)?;

        self.status_location = location.clone();
        self.status_score = score;
        self.status_moves = moves;
        self.display.show_status(&location, score, moves).ok();

        Ok(())
    }

    /// Process text input
    fn process_input(&mut self, inst: &Instruction, input: &str) -> Result<(), String> {
        self.display.print(">").ok();
        self.display.print(input).ok();
        self.display.print_char('\n').ok();

        let text_buffer = self.get_operand(inst, 0)?;
        let parse_buffer = if inst.operands.len() > 1 {
            let op_type = inst
                .operand_types
                .get(1)
                .copied()
                .unwrap_or(OperandType::Omitted);
            if op_type != OperandType::Omitted {
                Some(self.get_operand(inst, 1)?)
            } else {
                None
            }
        } else {
            None
        };

        let max_len = self.vm.read_byte(text_buffer as u32) as usize;
        let input_lower = input.to_lowercase();
        let input_bytes = input_lower.as_bytes();

        if self.version <= 4 {
            for (i, &byte) in input_bytes.iter().take(max_len).enumerate() {
                self.vm
                    .write_byte((text_buffer + 1 + i as u16) as u32, byte)?;
            }
            let end_pos = input_bytes.len().min(max_len);
            self.vm
                .write_byte((text_buffer + 1 + end_pos as u16) as u32, 0)?;
        } else {
            let len = input_bytes.len().min(max_len);
            self.vm.write_byte((text_buffer + 1) as u32, len as u8)?;
            for (i, &byte) in input_bytes.iter().take(len).enumerate() {
                self.vm
                    .write_byte((text_buffer + 2 + i as u16) as u32, byte)?;
            }
        }

        if let Some(parse_addr) = parse_buffer {
            self.tokenize_input(input, text_buffer, parse_addr)?;
        }

        if self.version >= 5 {
            if let Some(store_var) = inst.store_var {
                self.vm.write_variable(store_var, 13)?;
            }
        }

        Ok(())
    }

    /// Simple tokenization for input
    fn tokenize_input(
        &mut self,
        input: &str,
        _text_buffer: u16,
        parse_buffer: u16,
    ) -> Result<(), String> {
        let max_words = self.vm.read_byte(parse_buffer as u32);
        let input_lower = input.to_lowercase();
        let words: Vec<&str> = input_lower.split_whitespace().collect();
        let word_count = words.len().min(max_words as usize);

        self.vm
            .write_byte((parse_buffer + 1) as u32, word_count as u8)?;

        for (i, word) in words.iter().take(word_count).enumerate() {
            let dict_entry = self.lookup_word(word)?;
            let word_start = input_lower.find(word).unwrap_or(0) as u8;
            let word_len = word.len() as u8;

            let entry_addr = parse_buffer + 2 + (i as u16) * 4;
            self.vm.write_word(entry_addr as u32, dict_entry)?;
            self.vm.write_byte((entry_addr + 2) as u32, word_len)?;
            self.vm
                .write_byte((entry_addr + 3) as u32, word_start + 1)?;
        }

        Ok(())
    }

    /// Look up word in dictionary
    fn lookup_word(&self, word: &str) -> Result<u16, String> {
        let dict_addr = self.vm.game.header.dictionary as usize;
        let num_separators = self.vm.read_byte(dict_addr as u32) as usize;
        let entry_length = self.vm.read_byte((dict_addr + 1 + num_separators) as u32) as usize;
        let num_entries = self.vm.read_word((dict_addr + 2 + num_separators) as u32) as i16;
        let entries_start = dict_addr + 4 + num_separators;

        let encoded = self.encode_word(word);

        if num_entries > 0 {
            let mut low = 0usize;
            let mut high = num_entries as usize;

            while low < high {
                let mid = (low + high) / 2;
                let entry_addr = entries_start + mid * entry_length;

                let mut cmp = std::cmp::Ordering::Equal;
                for (i, &enc_word) in encoded.iter().enumerate() {
                    let dict_word = self.vm.read_word((entry_addr + i * 2) as u32);
                    match enc_word.cmp(&dict_word) {
                        std::cmp::Ordering::Less => {
                            cmp = std::cmp::Ordering::Less;
                            break;
                        }
                        std::cmp::Ordering::Greater => {
                            cmp = std::cmp::Ordering::Greater;
                            break;
                        }
                        std::cmp::Ordering::Equal => continue,
                    }
                }

                match cmp {
                    std::cmp::Ordering::Equal => return Ok(entry_addr as u16),
                    std::cmp::Ordering::Less => high = mid,
                    std::cmp::Ordering::Greater => low = mid + 1,
                }
            }
        }

        Ok(0)
    }

    /// Encode word for dictionary comparison
    fn encode_word(&self, word: &str) -> Vec<u16> {
        let word_len = if self.version <= 3 { 6 } else { 9 };
        let mut zchars = Vec::with_capacity(word_len);

        for ch in word.chars().take(word_len) {
            let zchar = match ch {
                'a'..='z' => (ch as u8 - b'a' + 6) as u16,
                ' ' => 0,
                _ => 5,
            };
            zchars.push(zchar);
        }

        while zchars.len() < word_len {
            zchars.push(5);
        }

        let num_words = if self.version <= 3 { 2 } else { 3 };
        let mut result = Vec::with_capacity(num_words);

        for i in 0..num_words {
            let base = i * 3;
            let mut word_val = (zchars[base] << 10) | (zchars[base + 1] << 5) | zchars[base + 2];
            if i == num_words - 1 {
                word_val |= 0x8000;
            }
            result.push(word_val);
        }

        result
    }

    /// Execute a single instruction
    fn execute_instruction(&mut self, inst: &Instruction) -> Result<WasmExecutionResult, String> {
        use crate::interpreter::opcodes::opcode_tables::get_instruction_name;

        let opcode_name = get_instruction_name(
            inst.opcode,
            inst.ext_opcode,
            inst.form,
            inst.operand_count,
            self.version,
        );

        match opcode_name {
            // Print operations
            "print" => {
                if let Some(ref text) = inst.text {
                    self.display.print(text).ok();
                }
                Ok(WasmExecutionResult::Continue)
            }

            "print_ret" => {
                if let Some(ref text) = inst.text {
                    self.display.print(text).ok();
                    self.display.print_char('\n').ok();
                }
                self.return_from_routine(1)?;
                Ok(WasmExecutionResult::Continue)
            }

            "new_line" => {
                self.display.print_char('\n').ok();
                Ok(WasmExecutionResult::Continue)
            }

            "print_num" => {
                let value = self.get_operand(inst, 0)? as i16;
                self.display.print(&value.to_string()).ok();
                Ok(WasmExecutionResult::Continue)
            }

            "print_char" => {
                let ch = self.get_operand(inst, 0)? as u8 as char;
                self.display.print_char(ch).ok();
                Ok(WasmExecutionResult::Continue)
            }

            "print_obj" => {
                let obj_num = self.get_operand(inst, 0)?;
                let name = self.vm.get_object_name(obj_num)?;
                self.display.print(&name).ok();
                Ok(WasmExecutionResult::Continue)
            }

            "print_addr" => {
                let addr = self.get_operand(inst, 0)? as usize;
                let (decoded, _) = text::decode_string(
                    &self.vm.game.memory,
                    addr,
                    self.vm.game.header.abbrev_table,
                )?;
                self.display.print(&decoded).ok();
                Ok(WasmExecutionResult::Continue)
            }

            "print_paddr" => {
                let paddr = self.get_operand(inst, 0)?;
                let addr = self.unpack_string_addr(paddr);
                let (decoded, _) = text::decode_string(
                    &self.vm.game.memory,
                    addr,
                    self.vm.game.header.abbrev_table,
                )?;
                self.display.print(&decoded).ok();
                Ok(WasmExecutionResult::Continue)
            }

            // Input operations
            "sread" | "aread" => {
                if self.pending_input.is_some() {
                    let input = self.pending_input.take().unwrap();
                    self.process_input(inst, &input)?;
                    Ok(WasmExecutionResult::Continue)
                } else {
                    self.vm.pc -= inst.size as u32;
                    if self.version <= 3 {
                        self.update_status_line()?;
                    }
                    Ok(WasmExecutionResult::WaitForInput)
                }
            }

            "read_char" => {
                if self.pending_input.is_some() {
                    let input = self.pending_input.take().unwrap();
                    let ch = input.chars().next().unwrap_or('\n') as u16;
                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, ch)?;
                    }
                    Ok(WasmExecutionResult::Continue)
                } else {
                    self.vm.pc -= inst.size as u32;
                    Ok(WasmExecutionResult::WaitForInput)
                }
            }

            // Quit
            "quit" => Ok(WasmExecutionResult::Quit),

            // Call operations
            "call" | "call_vs" | "call_vs2" | "call_1s" | "call_2s" => {
                self.call_routine(inst, true)?;
                Ok(WasmExecutionResult::Continue)
            }

            "call_vn" | "call_vn2" | "call_1n" | "call_2n" => {
                self.call_routine(inst, false)?;
                Ok(WasmExecutionResult::Continue)
            }

            // Return operations
            "ret" => {
                let value = self.get_operand(inst, 0)?;
                self.return_from_routine(value)?;
                Ok(WasmExecutionResult::Continue)
            }

            "rtrue" => {
                self.return_from_routine(1)?;
                Ok(WasmExecutionResult::Continue)
            }

            "rfalse" => {
                self.return_from_routine(0)?;
                Ok(WasmExecutionResult::Continue)
            }

            "ret_popped" => {
                let value = self.vm.pop()?;
                self.return_from_routine(value)?;
                Ok(WasmExecutionResult::Continue)
            }

            // Variable operations
            "store" => {
                let var = inst.operands[0] as u8;
                let value = self.get_operand(inst, 1)?;
                self.vm.write_variable(var, value)?;
                Ok(WasmExecutionResult::Continue)
            }

            "load" => {
                let var = self.get_operand(inst, 0)? as u8;
                let value = self.vm.read_variable(var)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "push" => {
                let value = self.get_operand(inst, 0)?;
                self.vm.push(value)?;
                Ok(WasmExecutionResult::Continue)
            }

            "pull" => {
                let value = self.vm.pop()?;
                let var = self.get_operand(inst, 0)? as u8;
                self.vm.write_variable(var, value)?;
                Ok(WasmExecutionResult::Continue)
            }

            "inc" => {
                let var = inst.operands[0] as u8;
                let value = self.vm.read_variable(var)? as i16;
                self.vm.write_variable(var, value.wrapping_add(1) as u16)?;
                Ok(WasmExecutionResult::Continue)
            }

            "dec" => {
                let var = inst.operands[0] as u8;
                let value = self.vm.read_variable(var)? as i16;
                self.vm.write_variable(var, value.wrapping_sub(1) as u16)?;
                Ok(WasmExecutionResult::Continue)
            }

            "inc_chk" => {
                let var = inst.operands[0] as u8;
                let value = self.vm.read_variable(var)? as i16;
                let new_value = value.wrapping_add(1);
                self.vm.write_variable(var, new_value as u16)?;
                let threshold = self.get_operand(inst, 1)? as i16;
                self.handle_branch(inst, new_value > threshold)?;
                Ok(WasmExecutionResult::Continue)
            }

            "dec_chk" => {
                let var = inst.operands[0] as u8;
                let value = self.vm.read_variable(var)? as i16;
                let new_value = value.wrapping_sub(1);
                self.vm.write_variable(var, new_value as u16)?;
                let threshold = self.get_operand(inst, 1)? as i16;
                self.handle_branch(inst, new_value < threshold)?;
                Ok(WasmExecutionResult::Continue)
            }

            // Arithmetic
            "add" => {
                let a = self.get_operand(inst, 0)? as i16;
                let b = self.get_operand(inst, 1)? as i16;
                if let Some(store_var) = inst.store_var {
                    self.vm
                        .write_variable(store_var, a.wrapping_add(b) as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "sub" => {
                let a = self.get_operand(inst, 0)? as i16;
                let b = self.get_operand(inst, 1)? as i16;
                if let Some(store_var) = inst.store_var {
                    self.vm
                        .write_variable(store_var, a.wrapping_sub(b) as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "mul" => {
                let a = self.get_operand(inst, 0)? as i16;
                let b = self.get_operand(inst, 1)? as i16;
                if let Some(store_var) = inst.store_var {
                    self.vm
                        .write_variable(store_var, a.wrapping_mul(b) as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "div" => {
                let a = self.get_operand(inst, 0)? as i16;
                let b = self.get_operand(inst, 1)? as i16;
                if b == 0 {
                    return Err("Division by zero".to_string());
                }
                if let Some(store_var) = inst.store_var {
                    self.vm
                        .write_variable(store_var, a.wrapping_div(b) as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "mod" => {
                let a = self.get_operand(inst, 0)? as i16;
                let b = self.get_operand(inst, 1)? as i16;
                if b == 0 {
                    return Err("Modulo by zero".to_string());
                }
                if let Some(store_var) = inst.store_var {
                    self.vm
                        .write_variable(store_var, a.wrapping_rem(b) as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "and" => {
                let a = self.get_operand(inst, 0)?;
                let b = self.get_operand(inst, 1)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, a & b)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "or" => {
                let a = self.get_operand(inst, 0)?;
                let b = self.get_operand(inst, 1)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, a | b)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "not" => {
                let a = self.get_operand(inst, 0)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, !a)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            // Branching and comparison
            "je" => {
                let first = self.get_operand(inst, 0)?;
                let mut result = false;
                for i in 1..inst.operands.len() {
                    let op_type = inst
                        .operand_types
                        .get(i)
                        .copied()
                        .unwrap_or(OperandType::Omitted);
                    if op_type != OperandType::Omitted {
                        let val = self.get_operand(inst, i)?;
                        if first == val {
                            result = true;
                            break;
                        }
                    }
                }
                self.handle_branch(inst, result)?;
                Ok(WasmExecutionResult::Continue)
            }

            "jl" => {
                let a = self.get_operand(inst, 0)? as i16;
                let b = self.get_operand(inst, 1)? as i16;
                self.handle_branch(inst, a < b)?;
                Ok(WasmExecutionResult::Continue)
            }

            "jg" => {
                let a = self.get_operand(inst, 0)? as i16;
                let b = self.get_operand(inst, 1)? as i16;
                self.handle_branch(inst, a > b)?;
                Ok(WasmExecutionResult::Continue)
            }

            "jz" => {
                let value = self.get_operand(inst, 0)?;
                self.handle_branch(inst, value == 0)?;
                Ok(WasmExecutionResult::Continue)
            }

            "jump" => {
                let offset = self.get_operand(inst, 0)? as i16;
                self.vm.pc = (self.vm.pc as i32 + offset as i32 - 2) as u32;
                Ok(WasmExecutionResult::Continue)
            }

            // Object operations
            "test_attr" => {
                let obj = self.get_operand(inst, 0)?;
                let attr = self.get_operand(inst, 1)? as u8;
                let result = self.vm.test_attribute(obj, attr)?;
                self.handle_branch(inst, result)?;
                Ok(WasmExecutionResult::Continue)
            }

            "set_attr" => {
                let obj = self.get_operand(inst, 0)?;
                let attr = self.get_operand(inst, 1)? as u8;
                self.vm.set_attribute(obj, attr, true)?;
                Ok(WasmExecutionResult::Continue)
            }

            "clear_attr" => {
                let obj = self.get_operand(inst, 0)?;
                let attr = self.get_operand(inst, 1)? as u8;
                self.vm.set_attribute(obj, attr, false)?;
                Ok(WasmExecutionResult::Continue)
            }

            "get_parent" => {
                let obj = self.get_operand(inst, 0)?;
                let parent = self.vm.get_parent(obj)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, parent)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "get_sibling" => {
                let obj = self.get_operand(inst, 0)?;
                let sibling = self.vm.get_sibling(obj)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, sibling)?;
                }
                self.handle_branch(inst, sibling != 0)?;
                Ok(WasmExecutionResult::Continue)
            }

            "get_child" => {
                let obj = self.get_operand(inst, 0)?;
                let child = self.vm.get_child(obj)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, child)?;
                }
                self.handle_branch(inst, child != 0)?;
                Ok(WasmExecutionResult::Continue)
            }

            "jin" => {
                let obj1 = self.get_operand(inst, 0)?;
                let obj2 = self.get_operand(inst, 1)?;
                let parent = self.vm.get_parent(obj1)?;
                self.handle_branch(inst, parent == obj2)?;
                Ok(WasmExecutionResult::Continue)
            }

            "insert_obj" => {
                let obj = self.get_operand(inst, 0)?;
                let dest = self.get_operand(inst, 1)?;
                self.vm.insert_object(obj, dest)?;
                Ok(WasmExecutionResult::Continue)
            }

            "remove_obj" => {
                let obj = self.get_operand(inst, 0)?;
                self.vm.remove_object(obj)?;
                Ok(WasmExecutionResult::Continue)
            }

            "get_prop" => {
                let obj = self.get_operand(inst, 0)?;
                let prop = self.get_operand(inst, 1)? as u8;
                let value = self.vm.get_property(obj, prop)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "put_prop" => {
                let obj = self.get_operand(inst, 0)?;
                let prop = self.get_operand(inst, 1)? as u8;
                let value = self.get_operand(inst, 2)?;
                self.vm.put_property(obj, prop, value)?;
                Ok(WasmExecutionResult::Continue)
            }

            "get_prop_addr" => {
                let obj = self.get_operand(inst, 0)?;
                let prop = self.get_operand(inst, 1)? as u8;
                let addr = self.vm.get_property_addr(obj, prop)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, addr as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "get_next_prop" => {
                let obj = self.get_operand(inst, 0)?;
                let prop = self.get_operand(inst, 1)? as u8;
                let next = self.vm.get_next_property(obj, prop)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, next as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "get_prop_len" => {
                let addr = self.get_operand(inst, 0)?;
                let len = if addr == 0 {
                    0
                } else {
                    let size_byte = self.vm.read_byte((addr - 1) as u32);
                    if self.version <= 3 {
                        ((size_byte >> 5) & 0x07) + 1
                    } else if size_byte & 0x80 != 0 {
                        let len = size_byte & 0x3f;
                        if len == 0 {
                            64
                        } else {
                            len
                        }
                    } else if size_byte & 0x40 != 0 {
                        2
                    } else {
                        1
                    }
                };
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, len as u16)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            // Memory operations
            "loadw" => {
                let array = self.get_operand(inst, 0)?;
                let index = self.get_operand(inst, 1)?;
                let addr = array.wrapping_add(index.wrapping_mul(2));
                let value = self.vm.read_word(addr as u32);
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "loadb" => {
                let array = self.get_operand(inst, 0)?;
                let index = self.get_operand(inst, 1)?;
                let addr = array.wrapping_add(index);
                let value = self.vm.read_byte(addr as u32) as u16;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            "storew" => {
                let array = self.get_operand(inst, 0)?;
                let index = self.get_operand(inst, 1)?;
                let value = self.get_operand(inst, 2)?;
                let addr = array.wrapping_add(index.wrapping_mul(2));
                self.vm.write_word(addr as u32, value)?;
                Ok(WasmExecutionResult::Continue)
            }

            "storeb" => {
                let array = self.get_operand(inst, 0)?;
                let index = self.get_operand(inst, 1)?;
                let value = self.get_operand(inst, 2)?;
                let addr = array.wrapping_add(index);
                self.vm.write_byte(addr as u32, value as u8)?;
                Ok(WasmExecutionResult::Continue)
            }

            // Random
            "random" => {
                let range = self.get_operand(inst, 0)? as i16;
                let result = if range <= 0 {
                    0
                } else {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    rng.gen_range(1..=range as u16)
                };
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, result)?;
                }
                Ok(WasmExecutionResult::Continue)
            }

            // Test bits
            "test" => {
                let bitmap = self.get_operand(inst, 0)?;
                let flags = self.get_operand(inst, 1)?;
                self.handle_branch(inst, (bitmap & flags) == flags)?;
                Ok(WasmExecutionResult::Continue)
            }

            // Verify/Piracy - always pass
            "verify" | "piracy" => {
                self.handle_branch(inst, true)?;
                Ok(WasmExecutionResult::Continue)
            }

            "nop" => Ok(WasmExecutionResult::Continue),

            _ => Err(format!(
                "Unimplemented opcode: {} (0x{:02x}) at {:04x}",
                opcode_name,
                inst.opcode,
                self.vm.pc - inst.size as u32
            )),
        }
    }
}
