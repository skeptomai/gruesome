use crate::debug_symbols::RoutineNames;
use crate::display_manager::{DisplayManager, DisplayTrait};
use crate::instruction::{Instruction, OperandType};
use crate::text;
use crate::timed_input::TimedInput;
use crate::vm::{CallFrame, VM};
use log::{debug, info};
use std::io::{self, Write};

/// Result of executing an instruction
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Continue execution normally
    Continue,
    /// Branch taken, PC already updated
    Branched,
    /// Routine called, PC updated
    Called,
    /// Routine returned
    Returned(u16),
    /// Game should quit
    Quit,
    /// Game completed successfully
    GameOver,
    /// Execution error
    Error(String),
}

/// The main Z-Machine interpreter
pub struct Interpreter {
    /// The VM state
    pub vm: VM,
    /// Enable debug output
    pub debug: bool,
    /// Instruction count for debugging
    instruction_count: u64,
    /// Routine names for debugging
    routine_names: RoutineNames,
    /// Enable single-step debugging
    pub single_step: bool,
    /// PC range for single-stepping (start, end)
    pub step_range: Option<(u32, u32)>,
    /// Timed input handler
    timed_input: TimedInput,
    /// Display manager
    display: Option<DisplayManager>,
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new(vm: VM) -> Self {
        // Try to initialize display, but continue without it if it fails
        let display = match DisplayManager::new() {
            Ok(d) => Some(d),
            Err(e) => {
                debug!("Failed to initialize display: {}", e);
                None
            }
        };

        Interpreter {
            vm,
            debug: false,
            instruction_count: 0,
            routine_names: RoutineNames::new(),
            single_step: false,
            step_range: None,
            timed_input: TimedInput::new(),
            display,
        }
    }

    /// Enable or disable debug mode
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Enable single-step debugging for a PC range
    pub fn enable_single_step(&mut self, start: u32, end: u32) {
        self.single_step = true;
        self.step_range = Some((start, end));

        // Show what routines are in this range
        println!("\n=== Single-step debugging enabled ===");
        println!("PC range: 0x{start:04x} - 0x{end:04x}");

        // Find routines in this range
        let mut routines_in_range = Vec::new();
        for addr in start..=end {
            if let Some(name) = self.routine_names.get_name(addr) {
                routines_in_range.push((addr, name));
            }
        }

        if !routines_in_range.is_empty() {
            println!("Routines in range:");
            for (addr, name) in routines_in_range {
                println!("  0x{addr:04x}: {name}");
            }
        }
        println!();
    }

    /// Dump memory state for debugging
    fn dump_memory_state(&self) {
        println!("\n=== Memory State ===");

        // Show where we are
        let current_pc_info = if let Some((routine_addr, name)) =
            self.routine_names.get_routine_containing(self.vm.pc)
        {
            let offset = self.vm.pc - routine_addr;
            format!("0x{:05x} (in {}+0x{:02x})", self.vm.pc, name, offset)
        } else {
            format!("0x{:05x}", self.vm.pc)
        };
        println!("Current PC: {current_pc_info}");

        if let Some(frame) = self.vm.call_stack.last() {
            println!(
                "Will return to: {}",
                self.routine_names.format_address(frame.return_pc)
            );
        }

        // Dump some key globals with names
        println!("\nKey globals:");
        for i in [0, 0x48, 0x4c, 0x4e, 0x6f, 0x7f] {
            if let Ok(val) = self.vm.read_global(i) {
                let name = crate::debug_symbols::get_global_name(i + 0x10).unwrap_or("");
                if !name.is_empty() {
                    println!("  G{i:02x} {name}: {val} (0x{val:04x})");
                } else {
                    println!("  G{i:02x}: {val} (0x{val:04x})");
                }
            }
        }

        // Dump stack
        println!("\nStack (top 5):");
        for i in 0..5.min(self.vm.stack.len()) {
            let idx = self.vm.stack.len() - 1 - i;
            println!(
                "  [{}]: {} (0x{:04x})",
                i, self.vm.stack[idx], self.vm.stack[idx]
            );
        }

        // Current call frame info
        if let Some(frame) = self.vm.call_stack.last() {
            println!("\nCurrent call frame:");
            println!(
                "  Return PC: {}",
                self.routine_names.format_address(frame.return_pc)
            );
            println!("  Locals: {:?}", frame.locals);
        }

        // Show call stack
        println!("\nCall stack:");
        for (i, frame) in self.vm.call_stack.iter().rev().enumerate() {
            println!(
                "  [{}] Return to: {}",
                i,
                self.routine_names.format_address(frame.return_pc)
            );
        }

        println!();
    }

    /// Run the interpreter
    pub fn run(&mut self) -> Result<(), String> {
        self.run_with_limit(None)
    }

    /// Run the interpreter with an optional instruction limit
    /// Dump current game state for debugging
    fn dump_game_state(&self, context: &str) {
        debug!("=== GAME STATE DUMP: {} ===", context);

        // Current location (global variable 0)
        if let Ok(location) = self.vm.read_variable(16) {
            // Global 0 = variable 16
            debug!("Current location (G00): {}", location);
        }

        // Player object (often stored in a global, commonly G01 or G02)
        for i in 0..5 {
            if let Ok(value) = self.vm.read_variable(16 + i) {
                debug!("Global {:02}: {}", i, value);
            }
        }

        // Check attributes of current location
        if let Ok(location) = self.vm.read_variable(16) {
            if location > 0 && location < 256 {
                debug!("=== Location {} attributes ===", location);
                for attr in 0..32 {
                    if let Ok(has_attr) = self.vm.test_attribute(location, attr) {
                        if has_attr {
                            debug!(
                                "  Location {} has attribute {} ({})",
                                location,
                                attr,
                                match attr {
                                    2 => "LIGHT",
                                    6 => "VISITED?",
                                    8 => "LIT?",
                                    9 => "CONTAINER?",
                                    20 => "ROOM?",
                                    _ => "UNKNOWN",
                                }
                            );
                        }
                    }
                }

                // Specifically check common light attributes including ONBIT (likely attr 3)
                for light_attr in [2, 3, 8, 15] {
                    if let Ok(has_light) = self.vm.test_attribute(location, light_attr) {
                        debug!(
                            "  Location {} light check attr {} ({}): {}",
                            location,
                            light_attr,
                            match light_attr {
                                3 => "ONBIT?",
                                2 => "LIGHT",
                                8 => "LIT",
                                15 => "PROVIDE_LIGHT",
                                _ => "UNKNOWN",
                            },
                            has_light
                        );
                    }
                }
            }
        }

        // Check player object (assume it's object 4 based on insert_obj we saw)
        let player_obj = 4;
        debug!("=== Player object {} attributes ===", player_obj);
        for attr in 0..32 {
            if let Ok(has_attr) = self.vm.test_attribute(player_obj, attr) {
                if has_attr {
                    debug!("  Player {} has attribute {}", player_obj, attr);
                }
            }
        }

        // Check for objects that might be light sources or have lighting attributes
        debug!("=== Checking for light sources ===");
        for obj in 1..50 {
            for light_attr in [2, 8, 12, 15] {
                if let Ok(has_light_attr) = self.vm.test_attribute(obj, light_attr) {
                    if has_light_attr {
                        debug!("  Object {} has light attribute {}", obj, light_attr);
                    }
                }
            }
        }

        // Check what happens if we manually set a light attribute on West of House
        debug!("=== Attempting to set light attribute on West of House ===");
        if let Ok(location) = self.vm.read_variable(16) {
            if location == 180 {
                debug!("  West of House (180) currently has no light attributes");
                debug!("  In a working game, this location should be naturally lit");
                debug!("  This suggests the lighting check may use a different mechanism");
            }
        }

        debug!("=== END GAME STATE DUMP ===");
    }

    pub fn run_with_limit(&mut self, max_instructions: Option<u64>) -> Result<(), String> {
        info!("Starting Z-Machine interpreter...");
        info!("Initial PC: {:05x}", self.vm.pc);

        // Set up initial display for v3 games
        if self.vm.game.header.version == 3 {
            if let Some(ref mut display) = self.display {
                // Clear screen
                display.clear_screen()?;

                // Create status window (1 line)
                display.split_window(1)?;

                // Position cursor below status line for game output
                print!("\x1b[2;1H"); // Move to line 2, column 1
                io::stdout().flush().ok();
            }
        } else {
            // For non-v3 games, just clear screen
            if let Some(ref mut display) = self.display {
                display.clear_screen()?;
            }
        }

        loop {
            // Fetch and decode instruction
            let pc = self.vm.pc;
            let instruction = match Instruction::decode(
                &self.vm.game.memory,
                pc as usize,
                self.vm.game.header.version,
            ) {
                Ok(inst) => inst,
                Err(e) => {
                    return Err(format!("Failed to decode instruction at {pc:05x}: {e}"));
                }
            };

            // Check if we should single-step this instruction
            let should_step = self.single_step
                && match self.step_range {
                    Some((start, end)) => pc >= start && pc <= end,
                    None => true,
                };

            // Log instruction for debugging
            if should_step {
                // Print the instruction details (using println for interactive debugging)
                // Show current routine if known
                let routine_info = if let Some((routine_addr, name)) =
                    self.routine_names.get_routine_containing(pc)
                {
                    let offset = pc - routine_addr;
                    if offset == 0 {
                        format!(" (start of {name})")
                    } else {
                        format!(" (in {name}+0x{offset:02x})")
                    }
                } else {
                    String::new()
                };
                println!("\n[{pc:05x}] {instruction}{routine_info}");
                println!(
                    "  Opcode: 0x{:02x}, Form: {:?}",
                    instruction.opcode, instruction.form
                );
                if !instruction.operands.is_empty() {
                    print!("  Operands:");
                    for (i, (op_type, value)) in instruction
                        .operand_types
                        .iter()
                        .zip(instruction.operands.iter())
                        .enumerate()
                    {
                        print!(" [{i}] ");
                        match op_type {
                            crate::instruction::OperandType::Variable => {
                                if *value <= 0xFF {
                                    let var_val =
                                        self.vm.read_variable(*value as u8).unwrap_or(0xFFFF);
                                    print!("V{value:02x}={var_val} (0x{var_val:04x})");
                                } else {
                                    print!("V{value:04x}=<invalid>");
                                }
                            }
                            _ => print!("#{value:04x}"),
                        }
                    }
                    println!();
                }
                if let Some(store) = instruction.store_var {
                    println!("  Store to: V{store:02x}");
                }
                if let Some(branch) = &instruction.branch {
                    println!(
                        "  Branch: offset={}, on_true={}",
                        branch.offset, branch.on_true
                    );
                }

                // Wait for user input
                print!("  Press Enter to continue (or 'q' to quit, 'm' for memory dump)... ");
                io::stdout().flush().ok();

                let mut input = String::new();
                io::stdin().read_line(&mut input).ok();

                if input.trim() == "q" {
                    return Err("User quit".to_string());
                } else if input.trim() == "m" {
                    self.dump_memory_state();
                }
            } else if self.debug
                || (0x06f70..=0x07000).contains(&pc)
                || (0x08cb0..=0x08cc0).contains(&pc)
                || (0x4f00..=0x5000).contains(&pc)
            {
                debug!(
                    "{:05x}: {} (form={:?}, opcode={:02x})",
                    pc, instruction, instruction.form, instruction.opcode
                );
            }

            // Advance PC past the instruction
            self.vm.pc += instruction.size as u32;

            // Execute the instruction
            match self.execute_instruction(&instruction)? {
                ExecutionResult::Continue => {
                    // Normal execution, PC already advanced
                }
                ExecutionResult::Branched => {
                    // Branch taken, PC was updated by branch logic
                }
                ExecutionResult::Called => {
                    // Routine called, PC was updated
                }
                ExecutionResult::Returned(_value) => {
                    // Return value already handled by do_return
                }
                ExecutionResult::Quit => {
                    // Quit opcode executed - exit the entire program immediately
                    std::process::exit(0);
                }
                ExecutionResult::GameOver => {
                    // Game over - return normally
                    return Ok(());
                }
                ExecutionResult::Error(e) => {
                    return Err(format!("Execution error at {pc:05x}: {e}"));
                }
            }

            self.instruction_count += 1;

            // Check instruction limit
            if let Some(limit) = max_instructions {
                if self.instruction_count >= limit {
                    info!("Reached instruction limit of {}", limit);
                    return Ok(());
                }
            }

            // Safety check for runaway execution
            if self.instruction_count > 1_000_000 {
                return Err("Instruction limit exceeded".to_string());
            }
        }
    }

    /// Execute a single instruction
    pub fn execute_instruction(&mut self, inst: &Instruction) -> Result<ExecutionResult, String> {
        // Get operand values
        let operands = self.resolve_operands(inst)?;

        // Debug problematic variables
        if let Some(store_var) = inst.store_var {
            if (0x01..=0x0F).contains(&store_var) {
                let frame = self
                    .vm
                    .call_stack
                    .last()
                    .ok_or("No active routine for local variable access")?;
                if store_var as usize > frame.num_locals as usize {
                    let pc = self.vm.pc - inst.size as u32;
                    debug!("Instruction at {:05x}: {} trying to store to V{:02x} but routine only has {} locals", 
                           pc, inst, store_var, frame.num_locals);
                    debug!(
                        "Call stack depth: {}, routine started at PC {:05x}",
                        self.vm.call_stack.len(),
                        frame.return_pc
                    );
                }
            }
        }

        // Check operands that read from local variables
        for (i, &operand) in inst.operands.iter().enumerate() {
            if inst.operand_types[i] == crate::instruction::OperandType::Variable {
                let var_num = operand as u8;
                if (0x01..=0x0F).contains(&var_num) {
                    let frame = self
                        .vm
                        .call_stack
                        .last()
                        .ok_or("No active routine for local variable access")?;
                    if var_num as usize > frame.num_locals as usize {
                        let pc = self.vm.pc - inst.size as u32;
                        debug!("Instruction at {:05x}: {} trying to read from V{:02x} but routine only has {} locals", 
                               pc, inst, var_num, frame.num_locals);
                    }
                }
            }
        }

        match inst.form {
            crate::instruction::InstructionForm::Short => match inst.operand_count {
                crate::instruction::OperandCount::OP0 => self.execute_0op(inst),
                crate::instruction::OperandCount::OP1 => self.execute_1op(inst, operands[0]),
                _ => Err(format!(
                    "Invalid operand count for short form: {:?}",
                    inst.operand_count
                )),
            },
            crate::instruction::InstructionForm::Long => {
                self.execute_2op(inst, operands[0], operands[1])
            }
            crate::instruction::InstructionForm::Variable => {
                match inst.operand_count {
                    crate::instruction::OperandCount::OP2 => {
                        // IMPORTANT: Variable form 2OP instructions
                        // ==========================================
                        // When a 2OP instruction is encoded in Variable form (as opposed to Long form),
                        // the actual number of operands is determined by the operand types byte(s),
                        // NOT by the "2OP" designation. The "2OP" here means the instruction uses
                        // opcodes 0-31 from the 2OP instruction set, not that it has exactly 2 operands.
                        //
                        // Most 2OP instructions only use 2 operands even in Variable form, but some
                        // (notably 'je') can use up to 4 operands as specified in the Z-Machine spec.
                        //
                        // From the spec: "je a b c d ?(label)" - Jump if a equals any of b, c, or d

                        self.execute_2op_variable(inst, &operands)
                    }
                    _ => self.execute_var(inst, &operands),
                }
            }
            crate::instruction::InstructionForm::Extended => self.execute_ext(inst, &operands),
        }
    }

    /// Resolve operand values (handle variables vs constants)
    pub fn resolve_operands(&mut self, inst: &Instruction) -> Result<Vec<u16>, String> {
        let mut values = Vec::new();

        for (i, &operand) in inst.operands.iter().enumerate() {
            let value = match inst.operand_types[i] {
                OperandType::Variable => {
                    // Read from variable
                    let var_num = operand as u8;
                    if var_num == 0 {
                        // Variable 0 means pop from stack when used as operand
                        self.vm.pop()?
                    } else {
                        self.vm.read_variable(var_num)?
                    }
                }
                _ => {
                    // Use literal value
                    operand
                }
            };
            values.push(value);
        }

        Ok(values)
    }

    /// Execute 0OP instructions
    fn execute_0op(&mut self, inst: &Instruction) -> Result<ExecutionResult, String> {
        match inst.opcode {
            0x00 => {
                // rtrue
                self.do_return(1)
            }
            0x01 => {
                // rfalse
                self.do_return(0)
            }
            0x02 => {
                // print (literal string)
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

                    if let Some(ref mut display) = self.display {
                        display.print(text).ok();
                    } else {
                        print!("{text}");
                        io::stdout().flush().ok();
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x03 => {
                // print_ret
                if let Some(ref text) = inst.text {
                    if let Some(ref mut display) = self.display {
                        display.print(text).ok();
                        display.print("\n").ok();
                    } else {
                        println!("{text}");
                    }
                }
                self.do_return(1)
            }
            0x04 => {
                // nop
                Ok(ExecutionResult::Continue)
            }
            0x05 => {
                // save (V1-3: branch on success, V4+: store result)
                if self.vm.game.header.version <= 3 {
                    // Try to save the game
                    let save_result = match crate::quetzal::save::save_game(&self.vm) {
                        Ok(()) => {
                            debug!("Save game succeeded");
                            true
                        }
                        Err(e) => {
                            println!("\n[Save failed: {e}]");
                            false
                        }
                    };

                    // Branch on success
                    if let Some(ref _branch) = inst.branch {
                        // For V1-3, save branches if successful
                        self.do_branch(inst, save_result)
                    } else {
                        Err("save instruction without branch info".to_string())
                    }
                } else {
                    // V4+: store result (0=fail, 1=success, 2=restored)
                    // For now, just store 0 (failed)
                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, 0)?;
                    }
                    Ok(ExecutionResult::Continue)
                }
            }
            0x06 => {
                // restore (V1-3: branch on success, V4+: store result)
                if self.vm.game.header.version <= 3 {
                    // Try to restore the game
                    let restore_result = match crate::quetzal::restore::restore_game(&mut self.vm) {
                        Ok(()) => {
                            debug!("Restore game succeeded");
                            // V1-3: On successful restore, execution continues from saved PC
                            // But we need to handle the branch first
                            true
                        }
                        Err(e) => {
                            println!("\n[Restore failed: {e}]");
                            false
                        }
                    };

                    // Branch on success
                    if let Some(ref _branch) = inst.branch {
                        self.do_branch(inst, restore_result)
                    } else if restore_result {
                        // Successful restore but no branch info - just continue from restored state
                        Ok(ExecutionResult::Continue)
                    } else {
                        // Failed restore, no branch - continue normally
                        Ok(ExecutionResult::Continue)
                    }
                } else {
                    // V4+: store result (0=fail, or doesn't return on success)
                    // Note: In V4+, successful restore doesn't return here
                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, 0)?;
                    }
                    Ok(ExecutionResult::Continue)
                }
            }
            0x08 => {
                // ret_popped
                let value = self.vm.pop()?;
                self.do_return(value)
            }
            0x09 => {
                // pop (V1-4) / catch (V5+)
                if self.vm.game.header.version <= 4 {
                    self.vm.pop()?;
                    Ok(ExecutionResult::Continue)
                } else {
                    // catch: store call stack depth
                    if let Some(store_var) = inst.store_var {
                        let depth = self.vm.call_depth() as u16;
                        self.vm.write_variable(store_var, depth)?;
                    }
                    Ok(ExecutionResult::Continue)
                }
            }
            0x0A => {
                // quit
                Ok(ExecutionResult::Quit)
            }
            0x0B => {
                // new_line
                if let Some(ref mut display) = self.display {
                    display.print("\n").ok();
                } else {
                    println!();
                }
                Ok(ExecutionResult::Continue)
            }
            0x0C => {
                // show_status (V3 only)
                if self.vm.game.header.version == 3 {
                    debug!("show_status called");

                    // Get location name from G16 (player's location in v3)
                    let location_obj = self.vm.read_global(16)?; // G16 contains player location in v3
                    let location_name = if location_obj > 0 {
                        self.get_object_name(location_obj)?
                    } else {
                        "Unknown".to_string()
                    };

                    // Get score and moves from globals (G17 and G18 in v3)
                    let score = self.vm.read_global(17)? as i16;
                    let moves = self.vm.read_global(18)?;

                    if let Some(ref mut display) = self.display {
                        display.show_status(&location_name, score, moves)?;
                    } else {
                        debug!("No display available for show_status");
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x0F => {
                // piracy
                // Copy protection check - interpreters should be "gullible and unconditionally branch"
                // This means always take the branch regardless of the branch condition
                if let Some(ref branch) = inst.branch {
                    match branch.offset {
                        0 => self.do_return(0), // rfalse
                        1 => self.do_return(1), // rtrue
                        offset => {
                            // Jump is relative to instruction after branch data
                            let new_pc = (self.vm.pc as i32 + offset as i32 - 2) as u32;
                            self.vm.pc = new_pc;
                            Ok(ExecutionResult::Branched)
                        }
                    }
                } else {
                    Err("piracy instruction without branch info".to_string())
                }
            }
            _ => Err(format!(
                "Unimplemented 0OP instruction: {:02x}",
                inst.opcode
            )),
        }
    }

    /// Execute 1OP instructions
    fn execute_1op(&mut self, inst: &Instruction, operand: u16) -> Result<ExecutionResult, String> {
        match inst.opcode {
            0x00 => {
                // jz
                let condition = operand == 0;
                let current_pc = self.vm.pc - inst.size as u32;

                // Debug logging for critical checks
                if current_pc == 0x8d51 {
                    // The JZ that checks LIT in DescribeObjects
                    debug!(
                        "JZ at PC {:05x}: checking if value {} is zero, condition = {}",
                        current_pc, operand, condition
                    );
                    // Also check what variable was loaded if this is checking a variable
                    if let Some(var_num) = inst.operands.first() {
                        if *var_num == 0x52 {
                            // If checking global 0x52 (LIT)
                            debug!("  -> This is checking LIT global (0x52)");
                        }
                    }
                }

                self.do_branch(inst, condition)
            }
            0x05 => {
                // inc
                let var_num = inst.operands[0] as u8;
                let value = self.vm.read_variable(var_num)?;
                self.vm.write_variable(var_num, value.wrapping_add(1))?;
                Ok(ExecutionResult::Continue)
            }
            0x06 => {
                // dec
                let var_num = inst.operands[0] as u8;
                let value = self.vm.read_variable(var_num)?;
                self.vm.write_variable(var_num, value.wrapping_sub(1))?;
                Ok(ExecutionResult::Continue)
            }
            0x0B => {
                // ret
                self.do_return(operand)
            }
            0x0C => {
                // jump
                // Jump is a signed offset from the instruction after the branch data
                let offset = operand as i16;
                let new_pc = (self.vm.pc as i32 + offset as i32 - 2) as u32;
                self.vm.pc = new_pc;
                Ok(ExecutionResult::Branched)
            }
            0x0D => {
                // print_paddr
                // Print string at packed address
                let pc = self.vm.pc - inst.size as u32;
                debug!("print_paddr at {:05x}: operand={:04x}", pc, operand);

                // Check if this might be the problematic address
                if operand == 0xa11d || operand == 0x1da1 {
                    debug!(
                        "*** WARNING: print_paddr with suspicious address {:04x} ***",
                        operand
                    );
                }

                let abbrev_addr = self.vm.game.header.abbrev_table;
                match text::decode_string_at_packed_addr(
                    &self.vm.game.memory,
                    operand,
                    self.vm.game.header.version,
                    abbrev_addr,
                ) {
                    Ok(string) => {
                        print!("{string}");
                        io::stdout().flush().ok();
                    }
                    Err(e) => {
                        debug!("Failed to decode string at {:04x}: {}", operand, e);
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x0E => {
                // load
                if inst.operand_types[0] != OperandType::Variable {
                    return Err("load requires variable operand".to_string());
                }
                let var_num = inst.operands[0] as u8;
                let value = self.vm.read_variable(var_num)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x0F => {
                // not (V1-4) / call_1n (V5+)
                if self.vm.game.header.version <= 4 {
                    // Bitwise NOT
                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, !operand)?;
                    }
                } else {
                    // call_1n: call with no return value
                    self.do_call(operand, &[], None)?;
                    return Ok(ExecutionResult::Called);
                }
                Ok(ExecutionResult::Continue)
            }
            0x01 => {
                // get_sibling
                let sibling = self.vm.get_sibling(operand)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, sibling)?;
                }
                self.do_branch(inst, sibling != 0)
            }
            0x02 => {
                // get_child
                let child = self.vm.get_child(operand)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, child)?;
                }
                self.do_branch(inst, child != 0)
            }
            0x03 => {
                // get_parent
                debug!(
                    "get_parent: obj_num={} at PC {:05x}",
                    operand,
                    self.vm.pc - inst.size as u32
                );
                let parent = self.vm.get_parent(operand)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, parent)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x04 => {
                // get_prop_len - get the length of a property given its data address
                debug!(
                    "get_prop_len: prop_addr={:04x} at PC {:05x}",
                    operand,
                    self.vm.pc - inst.size as u32
                );

                let prop_len = if operand == 0 {
                    0
                } else {
                    // In Z-Machine v3, the size byte is immediately before the property data
                    // The size byte encodes: top 3 bits = size-1, bottom 5 bits = property number
                    let size_byte_addr = (operand as u32).saturating_sub(1);
                    let size_byte = self.vm.read_byte(size_byte_addr);
                    let size = ((size_byte >> 5) & 0x07) + 1;
                    debug!(
                        "  Size byte at {:04x}: {:02x}, property size: {}",
                        size_byte_addr, size_byte, size
                    );
                    size as u16
                };

                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, prop_len)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x07 => {
                // print_addr
                // Print string at unpacked address
                let addr = operand as usize;
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

                match text::decode_string(&self.vm.game.memory, addr, abbrev_addr) {
                    Ok((string, _)) => {
                        print!("{string}");
                        io::stdout().flush().ok();
                    }
                    Err(e) => {
                        debug!("Failed to decode string at {:04x}: {}", addr, e);
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x09 => {
                // remove_obj
                let obj_num = operand;
                debug!(
                    "remove_obj: obj_num={} at PC {:05x}",
                    obj_num,
                    self.vm.pc - inst.size as u32
                );
                self.vm.remove_object(obj_num)?;
                Ok(ExecutionResult::Continue)
            }
            0x08 => {
                // call_1s
                self.do_call(operand, &[], inst.store_var)?;
                Ok(ExecutionResult::Called)
            }
            0x0A => {
                // print_obj - print short name of object
                let obj_num = operand;
                let pc = self.vm.pc - inst.size as u32;
                debug!("print_obj: object #{} at PC 0x{:04x}", obj_num, pc);

                // Special debugging for the leaves issue
                if (0x6300..=0x6400).contains(&pc) {
                    info!(
                        "*** print_obj in error message area: object #{} at PC 0x{:04x}",
                        obj_num, pc
                    );
                }

                if obj_num == 0 {
                    // Object 0 means no object - print nothing
                    return Ok(ExecutionResult::Continue);
                }
                if obj_num > 255 {
                    return Err(format!("Invalid object number for print_obj: {obj_num}"));
                }

                // Get object table base
                let obj_table_addr = self.vm.game.header.object_table_addr;
                let property_defaults = obj_table_addr;
                let obj_tree_base = property_defaults + 31 * 2; // 31 default properties, 2 bytes each

                // Calculate object entry address (9 bytes per object in V3)
                let obj_addr = obj_tree_base + ((obj_num - 1) as usize * 9);

                // Get property table address (last 2 bytes of object entry)
                let prop_table_addr = self.vm.read_word((obj_addr + 7) as u32) as usize;

                // The first byte is the text-length of the short name
                let text_len = self.vm.game.memory[prop_table_addr] as usize;

                if text_len > 0 {
                    // Decode the object name (stored as Z-string)
                    let name_addr = prop_table_addr + 1;
                    let abbrev_addr = self.vm.game.header.abbrev_table;
                    match text::decode_string(&self.vm.game.memory, name_addr, abbrev_addr) {
                        Ok((name, _)) => {
                            if obj_num == 144 {
                                debug!("Object 144 (leaves) name: '{}' (len={})", name, name.len());
                                debug!("  Name bytes: {:?}", name.as_bytes());
                            }
                            print!("{name}");
                            io::stdout().flush().ok();
                        }
                        Err(e) => {
                            debug!("Failed to decode object name: {}", e);
                        }
                    }
                }

                Ok(ExecutionResult::Continue)
            }
            _ => Err(format!(
                "Unimplemented 1OP instruction: {:02x}",
                inst.opcode
            )),
        }
    }

    /// Execute 2OP instructions
    fn execute_2op(
        &mut self,
        inst: &Instruction,
        op1: u16,
        op2: u16,
    ) -> Result<ExecutionResult, String> {
        match inst.opcode {
            0x00 => {
                // 2OP:0x00 is not defined in the Z-Machine spec
                // This might be data being executed as code
                let pc = self.vm.pc - inst.size as u32;
                debug!("WARNING: Invalid 2OP:0x00 at PC {:05x} with operands {:04x}, {:04x} - treating as NOP", 
                       pc, op1, op2);
                Ok(ExecutionResult::Continue)
            }
            0x01 => {
                // je
                let condition = op1 == op2;
                self.do_branch(inst, condition)
            }
            0x02 => {
                // jl
                let condition = (op1 as i16) < (op2 as i16);
                self.do_branch(inst, condition)
            }
            0x03 => {
                // jg
                let condition = (op1 as i16) > (op2 as i16);
                self.do_branch(inst, condition)
            }
            0x04 => {
                // dec_chk - decrement variable and branch if less than value
                // IMPORTANT: The first operand is ALWAYS a variable number, never a value
                // This is different from most 2OP instructions
                let var_num = inst.operands[0] as u8;
                let value = self.vm.read_variable(var_num)?;
                let new_value = value.wrapping_sub(1);
                self.vm.write_variable(var_num, new_value)?;

                let pc = self.vm.pc - inst.size as u32;
                if pc == 0x5fdf {
                    debug!(
                        "dec_chk at 0x5fdf: value={} -> new_value={}, comparing with {}",
                        value, new_value, op2
                    );
                }

                let condition = (new_value as i16) < (op2 as i16);

                if pc == 0x5fdf && value == 1 {
                    debug!(
                        "  new_value as i16 = {}, op2 as i16 = {}, condition = {}",
                        new_value as i16, op2 as i16, condition
                    );
                }

                self.do_branch(inst, condition)
            }
            0x05 => {
                // inc_chk - increment variable and branch if greater than value
                // IMPORTANT: The first operand is ALWAYS a variable number, never a value
                // This is different from most 2OP instructions
                let var_num = inst.operands[0] as u8;
                let value = self.vm.read_variable(var_num)?;
                let new_value = value.wrapping_add(1);
                self.vm.write_variable(var_num, new_value)?;
                let condition = (new_value as i16) > (op2 as i16);
                self.do_branch(inst, condition)
            }
            0x06 => {
                // jin
                // Check if obj1 is inside obj2 (obj1's parent is obj2)
                let parent = self.vm.get_parent(op1)?;
                let condition = parent == op2;
                self.do_branch(inst, condition)
            }
            0x07 => {
                // test
                // Bitwise AND and test if all bits in op2 are set in op1
                let result = (op1 & op2) == op2;
                let current_pc = self.vm.pc - inst.size as u32;

                if (0x06f70..=0x06fa0).contains(&current_pc) {
                    debug!(
                        "test at {:05x}: {:04x} & {:04x} == {:04x}? {}",
                        current_pc, op1, op2, op2, result
                    );
                }
                self.do_branch(inst, result)
            }
            0x08 => {
                // or
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, op1 | op2)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x09 => {
                // and
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, op1 & op2)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x0A => {
                // test_attr
                let obj_num = op1;
                let attr_num = op2 as u8;
                let result = self.vm.test_attribute(obj_num, attr_num)?;
                let current_pc = self.vm.pc - inst.size as u32;

                // Let's follow the natural flow
                if current_pc == 0x4f7e {
                    debug!(
                        "test_attr at {:05x}: obj={}, attr={}, result={}",
                        current_pc, obj_num, attr_num, result
                    );
                }

                self.do_branch(inst, result)
            }
            0x0B => {
                // set_attr
                let obj_num = op1;
                let attr_num = op2 as u8;
                self.vm.set_attribute(obj_num, attr_num, true)?;
                Ok(ExecutionResult::Continue)
            }
            0x0C => {
                // clear_attr
                let obj_num = op1;
                let attr_num = op2 as u8;
                if attr_num > 31 {
                    debug!(
                        "clear_attr: obj={}, attr={} at PC {:05x}",
                        obj_num,
                        attr_num,
                        self.vm.pc - inst.size as u32
                    );
                }
                self.vm.set_attribute(obj_num, attr_num, false)?;
                Ok(ExecutionResult::Continue)
            }
            0x0D => {
                // store
                // Use raw operand for variable number (destination)
                let var_num = inst.operands[0] as u8;
                let current_pc = self.vm.pc - inst.size as u32;

                if var_num == 0x10 {
                    debug!(
                        "Setting location (global 0) to object {} at PC {:05x}",
                        op2, current_pc
                    );
                    if op2 == 180 {
                        debug!("  -> This is West of House!");
                    }
                }

                // Special debugging for LIT variable
                if var_num == 0x52 {
                    debug!(
                        "STORE instruction at PC {:05x}: setting global 0x{:02x} (LIT) to {}",
                        current_pc, var_num, op2
                    );
                }

                self.vm.write_variable(var_num, op2)?;
                Ok(ExecutionResult::Continue)
            }
            0x0E => {
                // insert_obj
                debug!(
                    "insert_obj: obj={}, dest={} at PC {:05x}",
                    op1,
                    op2,
                    self.vm.pc - inst.size as u32
                );
                self.vm.insert_object(op1, op2)?;
                Ok(ExecutionResult::Continue)
            }
            0x0F => {
                // loadw
                let addr = op1 as u32 + (op2 as u32 * 2);
                let value = self.vm.read_word(addr);
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x10 => {
                // loadb
                let addr = op1 as u32 + op2 as u32;
                let value = self.vm.read_byte(addr) as u16;

                // Debug the leaves issue
                let pc = self.vm.pc - inst.size as u32;
                if pc == 0x6345 || pc == 0x6349 {
                    info!(
                        "loadb at 0x{:04x}: base=0x{:04x}, offset={}, addr=0x{:04x}, value={}",
                        pc, op1, op2, addr, value
                    );
                    // Also show what V01 points to
                    if op1 == 1 {
                        // If using V01
                        if let Ok(v01) = self.vm.read_variable(1) {
                            info!("  V01 = 0x{:04x}", v01);
                            // Show parse buffer entry
                            for i in 0..4 {
                                let byte = self.vm.read_byte(v01 as u32 + i);
                                info!("    V01+{} = 0x{:02x}", i, byte);
                            }
                        }
                    }
                }

                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x11 => {
                // get_prop
                let obj_num = op1;
                let prop_num = op2 as u8;
                let value = self.vm.get_property(obj_num, prop_num)?;

                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x12 => {
                // get_prop_addr
                let obj_num = op1;
                let prop_num = op2 as u8;
                let addr = self.vm.get_property_addr(obj_num, prop_num)? as u16;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, addr)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x13 => {
                // get_next_prop
                let obj_num = op1;
                let prop_num = op2 as u8;
                let next_prop = self.vm.get_next_property(obj_num, prop_num)? as u16;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, next_prop)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x14 => {
                // add
                if let Some(store_var) = inst.store_var {
                    let result = (op1 as i16).wrapping_add(op2 as i16) as u16;
                    self.vm.write_variable(store_var, result)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x15 => {
                // sub
                if let Some(store_var) = inst.store_var {
                    let result = (op1 as i16).wrapping_sub(op2 as i16) as u16;
                    self.vm.write_variable(store_var, result)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x16 => {
                // mul
                if let Some(store_var) = inst.store_var {
                    let result = (op1 as i16).wrapping_mul(op2 as i16) as u16;
                    self.vm.write_variable(store_var, result)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x17 => {
                // div
                if op2 == 0 {
                    return Err("Division by zero".to_string());
                }
                if let Some(store_var) = inst.store_var {
                    let result = (op1 as i16) / (op2 as i16);
                    self.vm.write_variable(store_var, result as u16)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x18 => {
                // mod
                if op2 == 0 {
                    return Err("Modulo by zero".to_string());
                }
                if let Some(store_var) = inst.store_var {
                    let result = (op1 as i16) % (op2 as i16);
                    self.vm.write_variable(store_var, result as u16)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x19 => {
                // call_2s
                let routine_addr = op1;
                let arg = op2;
                self.do_call(routine_addr, &[arg], inst.store_var)?;
                Ok(ExecutionResult::Called)
            }
            0x1C => {
                // not (v1-v3) - bitwise NOT
                // In v5+ this becomes VAR:143
                if self.vm.game.header.version <= 3 {
                    if let Some(store_var) = inst.store_var {
                        let result = !op1; // op2 is ignored
                        self.vm.write_variable(store_var, result)?;
                    }
                } else {
                    return Err("2OP:0x1C (not) is only valid in v1-v3".to_string());
                }
                Ok(ExecutionResult::Continue)
            }
            0x1F => {
                // Undocumented 2OP:0x1F instruction
                // Found in some Infocom games but not in the standard
                // Based on analysis, this appears to be a logical shift instruction
                let pc = self.vm.pc - inst.size as u32;
                debug!(
                    "WARNING: Undocumented 2OP:1F at PC {:05x} with operands {:04x}, {:04x}",
                    pc, op1, op2
                );

                // Don't store anything - we don't know what this instruction does
                // Storing 0 was causing bugs (e.g., clearing the LIT variable)
                debug!("  -> Treating as NOP, not storing any result");

                Ok(ExecutionResult::Continue)
            }
            _ => {
                let pc = self.vm.pc - inst.size as u32;
                debug!(
                    "Unimplemented 2OP instruction: {:02x} at PC {:05x}, form={:?}",
                    inst.opcode, pc, inst.form
                );
                Err(format!(
                    "Unimplemented 2OP instruction: {:02x}",
                    inst.opcode
                ))
            }
        }
    }

    /// Execute 2OP instructions in Variable form
    ///
    /// This method handles 2OP instructions that are encoded in Variable form,
    /// which may have more than 2 operands. The actual operand count is determined
    /// by the operand types byte(s) in the instruction encoding.
    fn execute_2op_variable(
        &mut self,
        inst: &Instruction,
        operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        // Handle edge cases first
        if operands.is_empty() && inst.opcode == 0x09 {
            // Special case: Variable 2OP AND with no operands
            // This appears in some games - treat as AND 0, 0
            debug!(
                "Variable 2OP AND with no operands at PC {:05x} - using 0, 0",
                self.vm.pc - inst.size as u32
            );
            return self.execute_2op(inst, 0, 0);
        }

        // Most 2OP instructions require at least 2 operands
        if operands.len() < 2 {
            let pc = self.vm.pc - inst.size as u32;
            return Err(format!("Variable 2OP instruction at PC {:05x} requires at least 2 operands, got {} - opcode: {:02x}", 
                               pc, operands.len(), inst.opcode));
        }

        // Handle each 2OP instruction based on its specific requirements
        match inst.opcode {
            0x01 => {
                // je - Jump if Equal (can have 2-4 operands)
                // From the spec: "je a b c d ?(label)"
                // Jump if a is equal to any of the subsequent operands (b, c, or d)
                let mut condition = false;
                for i in 1..operands.len() {
                    if operands[0] == operands[i] {
                        condition = true;
                        break;
                    }
                }
                self.do_branch(inst, condition)
            }
            _ => {
                // All other 2OP instructions use exactly 2 operands
                // Even in Variable form, they ignore any extra operands
                self.execute_2op(inst, operands[0], operands[1])
            }
        }
    }

    /// Execute VAR instructions
    fn execute_var(
        &mut self,
        inst: &Instruction,
        operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        match inst.opcode {
            0x00 => {
                // call
                if operands.is_empty() {
                    return Err("call requires at least one operand".to_string());
                }
                let routine_addr = operands[0];
                let args = &operands[1..];
                let unpacked_addr = self.unpack_routine_address(routine_addr) as u32;
                debug!(
                    "Call to packed address 0x{:04x} (unpacked: {}) with store_var = {:?}",
                    routine_addr,
                    self.routine_names.format_address(unpacked_addr),
                    inst.store_var
                );
                self.do_call(routine_addr, args, inst.store_var)?;
                Ok(ExecutionResult::Called)
            }
            0x01 => {
                // storew
                if operands.len() < 3 {
                    // For Variable form with OP2, this might be 2OP:21 (storew) not VAR:01
                    if inst.form == crate::instruction::InstructionForm::Variable
                        && inst.operand_count == crate::instruction::OperandCount::OP2
                    {
                        // This is actually 2OP:21 (storew) in Variable form
                        debug!("Note: Variable form storew with OP2 at PC {:05x} - this is 2OP:21 in Variable form", 
                               self.vm.pc - inst.size as u32);
                    }
                    return Err(format!(
                        "storew at PC {:05x} requires 3 operands, got {} (operands: {:?}) - instruction form: {:?}, opcode: {:02x}, operand_count: {:?}",
                        self.vm.pc - inst.size as u32, operands.len(), operands, inst.form, inst.opcode, inst.operand_count
                    ));
                }
                let addr = operands[0] as u32 + (operands[1] as u32 * 2);
                self.vm.write_word(addr, operands[2])?;
                Ok(ExecutionResult::Continue)
            }
            0x02 => {
                // storeb
                if operands.len() < 3 {
                    return Err("storeb requires 3 operands".to_string());
                }
                let addr = operands[0] as u32 + operands[1] as u32;
                self.vm.write_byte(addr, operands[2] as u8)?;
                Ok(ExecutionResult::Continue)
            }
            0x03 => {
                // put_prop
                if operands.len() < 3 {
                    return Err("put_prop requires 3 operands".to_string());
                }
                let obj_num = operands[0];
                let prop_num = operands[1] as u8;
                let value = operands[2];
                self.vm.put_property(obj_num, prop_num, value)?;
                Ok(ExecutionResult::Continue)
            }
            0x04 => {
                // sread (V1-4) with timer support (V3+)
                // Proper implementation that reads from stdin
                if operands.len() < 2 {
                    return Err("sread requires at least 2 operands".to_string());
                }
                let text_buffer = operands[0] as u32;
                let parse_buffer = operands[1] as u32;

                // Debug: Show all operands
                debug!(
                    "sread at PC 0x{:04x} with {} operands",
                    self.vm.pc - inst.size as u32,
                    operands.len()
                );
                for (i, op) in operands.iter().enumerate() {
                    debug!("  operand[{}] = 0x{:04x}", i, op);
                }

                // Check for timer parameters (V3+)
                let has_timer = operands.len() >= 4 && operands[2] > 0 && operands[3] > 0;
                let time = if operands.len() > 2 { operands[2] } else { 0 };
                let routine = if operands.len() > 3 { operands[3] } else { 0 };

                if has_timer {
                    info!(
                        "SREAD WITH TIMER: time={} ({}s), routine=0x{:04x}",
                        time,
                        time as f32 / 10.0,
                        routine
                    );
                }

                // In v3 games, automatically update status line before input
                if self.vm.game.header.version == 3 {
                    // Get all data before borrowing display mutably
                    // In v3: G16 = player location, G17 = score, G18 = moves
                    let location_obj = self.vm.read_global(16)?;
                    let location_name = self.get_object_name(location_obj)?;
                    let score = self.vm.read_global(17)? as i16;
                    let moves = self.vm.read_global(18)?;

                    // Now update display
                    if let Some(ref mut display) = self.display {
                        // Create status window if not already created
                        display.split_window(1)?;

                        // Update status line
                        display.show_status(&location_name, score, moves)?;

                        debug!(
                            "Auto-updated status line: location='{}', score={}, moves={}",
                            location_name, score, moves
                        );
                    }
                }

                // Get max length from text buffer
                let max_len = self.vm.read_byte(text_buffer);

                // Read input from user
                // Note: The game prints its own prompt, we don't need to add one
                io::stdout().flush().ok();

                // Create timer callback closure if we have a timer
                let timer_callback = if has_timer && routine > 0 {
                    // Create a closure that captures self through a raw pointer
                    // This is safe because we know the interpreter outlives the input operation
                    let interp_ptr = self as *mut Interpreter;
                    Some(move || -> Result<bool, String> {
                        unsafe {
                            debug!("Timer callback triggered for routine 0x{:04x}", routine);
                            (*interp_ptr).call_timer_routine(routine)
                        }
                    })
                } else {
                    None
                };

                // Read input with optional timer callback
                let (input, was_terminated) = self
                    .timed_input
                    .read_line_with_timer(time, routine, timer_callback)
                    .map_err(|e| format!("Error reading timed input: {e}"))?;

                // For turn-based games, simulate timer firing after input if not already fired
                if has_timer && !was_terminated {
                    debug!("Turn-based timer: simulating timer after input completion");
                    let result = self.call_timer_routine(routine)?;
                    debug!("Timer routine returned: {}", result);
                }

                if was_terminated {
                    info!("Input was terminated by timer interrupt");
                }

                // Convert to lowercase - Z-Machine convention
                let input = input.to_lowercase();

                // Limit input to max_len - 1 (leaving room for length byte)
                let input_bytes = input.as_bytes();
                let input_len = input_bytes.len().min(max_len as usize - 1);

                // Write input to text buffer
                // Text buffer format: max_len, actual_len, characters...
                self.vm.write_byte(text_buffer + 1, input_len as u8)?;
                for (i, &ch) in input_bytes.iter().take(input_len).enumerate() {
                    self.vm.write_byte(text_buffer + 2 + i as u32, ch)?;
                }

                // Parse the text buffer using proper dictionary lookup
                self.vm.parse_text(text_buffer, parse_buffer)?;

                // Debug: Show the actual text in the buffer
                let text_len = self.vm.read_byte(text_buffer + 1);
                debug!("Text buffer contents (len={}):", text_len);
                for i in 0..text_len {
                    let ch = self.vm.read_byte(text_buffer + 2 + i as u32);
                    debug!("  [{}] = 0x{:02x} '{}'", i, ch, ch as char);
                }

                // Extra debug: dump the exact parse buffer contents
                if input.contains("leaves") {
                    info!("*** Parse buffer dump at 0x{:04x}:", parse_buffer);
                    for i in 0..16 {
                        let byte = self.vm.read_byte(parse_buffer + i);
                        info!("  +{}: 0x{:02x}", i, byte);
                    }
                    // Interpret the second word entry
                    let word2_addr = self.vm.read_word(parse_buffer + 6);
                    let word2_len = self.vm.read_byte(parse_buffer + 8);
                    let word2_pos = self.vm.read_byte(parse_buffer + 9);
                    info!(
                        "  Word 2: addr=0x{:04x}, len={}, pos={}",
                        word2_addr, word2_len, word2_pos
                    );
                }

                // Special check for leaves
                if input.contains("leaves") {
                    info!("*** Special debug for 'leaves' issue ***");
                    info!("Input string: '{}'", input);
                    info!("Text buffer contents at 0x{:04x}:", text_buffer);
                    for i in 0..20 {
                        let ch = self.vm.read_byte(text_buffer + i as u32);
                        info!(
                            "  +{}: 0x{:02x} '{}'",
                            i,
                            ch,
                            if (32..127).contains(&ch) {
                                ch as char
                            } else {
                                '.'
                            }
                        );
                    }
                }

                let pc = self.vm.pc - inst.size as u32;
                // Debug removed for cleaner output
                debug!(
                    "sread at PC {:05x}: text_buffer={:04x}, parse_buffer={:04x} - input: '{}'",
                    pc, text_buffer, parse_buffer, input
                );

                // Debug: Show what's in the parse buffer
                let word_count = self.vm.read_byte(parse_buffer + 1);
                debug!("  Parse buffer word count: {}", word_count);
                for i in 0..word_count {
                    let offset = parse_buffer + 2 + (i as u32 * 4);
                    let dict_addr = self.vm.read_word(offset);
                    let word_len = self.vm.read_byte(offset + 2);
                    let text_pos = self.vm.read_byte(offset + 3);
                    debug!(
                        "    Word {}: dict_addr=0x{:04x}, len={}, pos={}",
                        i, dict_addr, word_len, text_pos
                    );

                    // Special check for leaves
                    if input.contains("leaves") && i == 1 {
                        info!(
                            "*** 'leaves' parse entry: dict_addr=0x{:04x}, len={}, pos={}",
                            dict_addr, word_len, text_pos
                        );
                        if word_len != 6 {
                            info!(
                                "*** ERROR: 'leaves' has wrong length! Expected 6, got {}",
                                word_len
                            );
                        }
                        // Check actual characters at this position
                        info!("*** Characters at text buffer position {}:", text_pos);
                        for j in 0..8 {
                            let ch = self.vm.read_byte(text_buffer + 2 + text_pos as u32 + j);
                            info!(
                                "      pos {}: 0x{:02x} '{}'",
                                text_pos + j as u8,
                                ch,
                                if (32..127).contains(&ch) {
                                    ch as char
                                } else {
                                    '.'
                                }
                            );
                        }
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x05 => {
                // print_char
                if !operands.is_empty() {
                    let ch = operands[0] as u8 as char;
                    let pc = self.vm.pc - inst.size as u32;

                    // Debug all print_char in error area
                    if (0x6300..=0x6400).contains(&pc) {
                        info!(
                            "print_char at 0x{:04x}: '{}' (0x{:02x})",
                            pc, ch, operands[0]
                        );
                    }

                    if operands[0] > 127 || operands[0] == 63 {
                        // 63 is '?'
                        debug!(
                            "print_char: value={} (0x{:02x}) char='{}' at PC {:05x}",
                            operands[0],
                            operands[0],
                            ch,
                            self.vm.pc - inst.size as u32
                        );
                    }
                    if let Some(ref mut display) = self.display {
                        display.print_char(ch).ok();
                    } else {
                        print!("{ch}");
                        io::stdout().flush().ok();
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x06 => {
                // print_num
                if !operands.is_empty() {
                    let num_str = format!("{}", operands[0] as i16);
                    if let Some(ref mut display) = self.display {
                        display.print(&num_str).ok();
                    } else {
                        print!("{num_str}");
                        io::stdout().flush().ok();
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x07 => {
                // random
                if !operands.is_empty() {
                    let range = operands[0] as i16;
                    let result = if range <= 0 {
                        // Negative = seed the RNG with |range|
                        // Zero = seed with random value
                        // For now, we're using thread_rng which doesn't need seeding
                        debug!("Random seed requested: {}", range);
                        0
                    } else {
                        // Return a value from 1 to range inclusive
                        use rand::Rng;
                        let mut rng = rand::thread_rng();
                        let value = rng.gen_range(1..=range as u16);
                        debug!("Random({}) = {}", range, value);
                        value
                    };

                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, result)?;
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x08 => {
                // push
                if !operands.is_empty() {
                    self.vm.push(operands[0])?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x09 => {
                // pull
                if !inst.operands.is_empty() {
                    let current_pc = self.vm.pc - inst.size as u32;
                    if (0x06f70..=0x06fa0).contains(&current_pc) {
                        debug!(
                            "pull at {:05x}: stack depth before pop: {}",
                            current_pc,
                            self.vm.stack.len()
                        );
                    }
                    let value = self.vm.pop()?;
                    // Use the raw operand value, not the resolved one
                    // (Variable 0 as destination means V00, not pop)
                    let var_num = inst.operands[0] as u8;
                    if (0x06f70..=0x06fa0).contains(&current_pc) {
                        debug!(
                            "pull at {:05x}: storing popped value {:04x} into V{:02x}",
                            current_pc, value, var_num
                        );
                    }
                    self.vm.write_variable(var_num, value)?;
                }
                Ok(ExecutionResult::Continue)
            }
            0x0A => {
                // split_window (V3+)
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
            0x0B => {
                // set_window (V3+)
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
            0x0D => {
                // erase_window - actually used in v3 (Seastalker uses it)
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
            0x13 => {
                // output_stream
                if !operands.is_empty() {
                    let stream_num = operands[0] as i16;
                    // For now, we only support stream 1 (screen output)
                    // Stream 1 is always on by default
                    // Positive numbers enable streams, negative disable
                    if stream_num.abs() != 1 {
                        debug!("Unsupported output stream: {}", stream_num);
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x0F => {
                // set_cursor - v3 uses this too (especially Seastalker)
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
            0x11 => {
                // set_text_style
                // Style bits: 1=reverse, 2=bold, 4=italic, 8=fixed-pitch
                if !operands.is_empty() {
                    let style = operands[0];
                    debug!("set_text_style: style={}", style);

                    // Apply text styles directly to stdout
                    // For now, just handle the common styles
                    if style == 0 {
                        // Reset to normal
                        print!("\x1b[0m");
                    } else if style & 1 != 0 {
                        // Reverse video
                        print!("\x1b[7m");
                    } else if style & 2 != 0 {
                        // Bold
                        print!("\x1b[1m");
                    }
                    io::stdout().flush().ok();
                }
                Ok(ExecutionResult::Continue)
            }
            0x15 => {
                // sound_effect - V3 only supports bleeps
                // Format: sound_effect number effect volume routine
                // For v3: number 1 or 2 are bleeps, no repeats, no callbacks

                if operands.is_empty() {
                    // No operands - beep if possible
                    print!("\x07");
                    io::stdout().flush().ok();
                } else {
                    let number = operands[0];

                    if number == 1 || number == 2 {
                        // Built-in bleeps (1 = high, 2 = low)
                        // On terminal, both just use bell character
                        print!("\x07");
                        io::stdout().flush().ok();
                    }
                    // For v3, ignore other sound numbers and effects
                    // The Lurking Horror would use numbers 3+ for real sounds
                }

                Ok(ExecutionResult::Continue)
            }
            0x16 => {
                // read_char (V4+)
                if self.vm.game.header.version < 4 {
                    return Err("read_char is only available in V4+".to_string());
                }

                // read_char has 1-3 operands:
                // 1. keyboard (1 = read from keyboard, must be 1)
                // 2. time (optional) - timeout in tenths of seconds
                // 3. routine (optional) - routine to call on timeout

                if operands.is_empty() || operands[0] != 1 {
                    return Err("read_char requires keyboard parameter = 1".to_string());
                }

                let time = if operands.len() > 1 { operands[1] } else { 0 };
                let routine = if operands.len() > 2 { operands[2] } else { 0 };
                let has_timer = time > 0 && routine > 0;

                debug!(
                    "read_char: time={}, routine=0x{:04x}, has_timer={}",
                    time, routine, has_timer
                );

                // Flush any pending output
                io::stdout().flush().ok();

                // Create timer callback if needed
                let timer_callback = if has_timer {
                    let interp_ptr = self as *mut Interpreter;
                    Some(move || -> Result<bool, String> {
                        unsafe {
                            debug!("read_char timer callback for routine 0x{:04x}", routine);
                            (*interp_ptr).call_timer_routine(routine)
                        }
                    })
                } else {
                    None
                };

                // Read a single character with optional timeout
                let (ch, was_terminated) = self.read_single_char(time, routine, timer_callback)?;

                // Store the result
                if let Some(store_var) = inst.store_var {
                    let char_code = if was_terminated {
                        0 // Return 0 if terminated by timer
                    } else {
                        match ch {
                            '\n' | '\r' => 13,    // Return
                            '\x08' | '\x7f' => 8, // Backspace/Delete
                            '\x1b' => 27,         // Escape
                            _ => ch as u16,
                        }
                    };
                    self.vm.write_variable(store_var, char_code)?;
                }

                if was_terminated {
                    debug!("read_char terminated by timer");
                }

                Ok(ExecutionResult::Continue)
            }
            0x1B => {
                // tokenise (V5+) or unknown in V3
                // In V3, this opcode is not documented
                // Some V3 games might use it as a NOP
                let pc = self.vm.pc - inst.size as u32;
                debug!("VAR:0x1B at PC {:05x} - treating as NOP for V3", pc);
                Ok(ExecutionResult::Continue)
            }
            _ => {
                let pc = self.vm.pc - inst.size as u32;
                debug!(
                    "Unimplemented VAR instruction: {:02x} at PC {:05x}",
                    inst.opcode, pc
                );
                Err(format!(
                    "Unimplemented VAR instruction: {:02x}",
                    inst.opcode
                ))
            }
        }
    }

    /// Execute EXT instructions
    fn execute_ext(
        &mut self,
        _inst: &Instruction,
        _operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        Err("Extended instructions not yet implemented".to_string())
    }

    /// Handle branching
    fn do_branch(
        &mut self,
        inst: &Instruction,
        condition: bool,
    ) -> Result<ExecutionResult, String> {
        if let Some(ref branch) = inst.branch {
            let should_branch = condition == branch.on_true;

            if should_branch {
                match branch.offset {
                    0 => return self.do_return(0), // rfalse
                    1 => return self.do_return(1), // rtrue
                    offset => {
                        // Jump is relative to instruction after branch data
                        let new_pc = (self.vm.pc as i32 + offset as i32 - 2) as u32;

                        // Add specific debug for the problematic branch
                        if self.vm.pc >= 0x08cc0 && self.vm.pc <= 0x08cd0 {
                            debug!(
                                "Branch at PC {:05x}: offset={} ({:04x}), new_pc={:05x}",
                                self.vm.pc, offset, offset as u16, new_pc
                            );
                        }

                        if self.vm.pc >= 0x06f70 && self.vm.pc <= 0x06fa0
                            || self.vm.pc >= 0x4f70 && self.vm.pc <= 0x5000
                        {
                            debug!(
                                "Branch from {:05x} with offset {} to {:05x}",
                                self.vm.pc, offset, new_pc
                            );
                        }
                        self.vm.pc = new_pc;
                        return Ok(ExecutionResult::Branched);
                    }
                }
            }
        }
        Ok(ExecutionResult::Continue)
    }

    /// Read a single character with optional timeout
    fn read_single_char<F>(
        &mut self,
        time_tenths: u16,
        routine_addr: u16,
        timer_callback: Option<F>,
    ) -> Result<(char, bool), String>
    where
        F: FnMut() -> Result<bool, String>,
    {
        // For now, delegate to timed_input's character reading
        // This will need to be implemented in timed_input.rs
        self.timed_input
            .read_char_with_timeout_callback(time_tenths, routine_addr, timer_callback)
    }

    /// Get the name of an object
    fn get_object_name(&self, obj_num: u16) -> Result<String, String> {
        if obj_num == 0 || obj_num > 255 {
            return Ok("".to_string());
        }

        // Calculate object address
        let obj_table_addr = self.vm.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let obj_tree_base = property_defaults + 31 * 2;
        let obj_addr = obj_tree_base + ((obj_num - 1) as usize * 9);

        if obj_addr + 9 > self.vm.game.memory.len() {
            return Err(format!("Object {obj_num} address out of bounds"));
        }

        // Get property table address
        let prop_table_addr = ((self.vm.game.memory[obj_addr + 7] as u16) << 8)
            | self.vm.game.memory[obj_addr + 8] as u16;

        if prop_table_addr == 0 || prop_table_addr as usize >= self.vm.game.memory.len() {
            return Ok("".to_string());
        }

        // First byte is text length in words
        let text_len = self.vm.game.memory[prop_table_addr as usize] as usize;
        if text_len == 0 {
            return Ok("".to_string());
        }

        // Decode the object name
        let name_addr = prop_table_addr as usize + 1;
        let abbrev_addr = self.vm.game.header.abbrev_table;

        match text::decode_string(&self.vm.game.memory, name_addr, abbrev_addr) {
            Ok((name, _)) => Ok(name),
            Err(e) => Err(format!("Failed to decode object name: {e}")),
        }
    }

    /// Handle routine calls
    /// Call a timer routine and execute it to completion
    fn call_timer_routine(&mut self, routine_addr: u16) -> Result<bool, String> {
        debug!("Calling timer routine at 0x{:04x}", routine_addr);

        // Save current PC and call depth
        let _saved_pc = self.vm.pc;
        let saved_call_depth = self.vm.call_depth();

        // Call routine with 0 args, store result in temp variable (stack)
        self.do_call(routine_addr, &[], Some(0))?;

        // Execute until routine returns (when call depth returns to saved level)
        let mut return_value = 0;
        let mut instruction_count = 0;
        const MAX_TIMER_INSTRUCTIONS: u64 = 10000; // Safety limit

        while self.vm.call_depth() > saved_call_depth {
            instruction_count += 1;
            if instruction_count > MAX_TIMER_INSTRUCTIONS {
                return Err("Timer routine exceeded instruction limit".to_string());
            }

            // Fetch and decode instruction
            let pc = self.vm.pc;
            let inst = match Instruction::decode(
                &self.vm.game.memory,
                pc as usize,
                self.vm.game.header.version,
            ) {
                Ok(inst) => inst,
                Err(e) => return Err(format!("Error decoding instruction at {pc:05x}: {e}")),
            };

            // Update PC
            self.vm.pc += inst.size as u32;

            // Execute instruction
            match self.execute_instruction(&inst)? {
                ExecutionResult::Returned(value) => {
                    return_value = value;
                    if self.vm.call_depth() <= saved_call_depth {
                        break;
                    }
                }
                ExecutionResult::Quit | ExecutionResult::GameOver => {
                    return Err("Timer routine tried to quit/end game".to_string());
                }
                _ => {
                    // Continue executing
                }
            }
        }

        // Pop the return value from stack (since we stored to var 0)
        let _ = self.vm.pop();

        debug!("Timer routine returned: {}", return_value);

        // Return true if routine wants to terminate input
        Ok(return_value != 0)
    }

    fn do_call(
        &mut self,
        packed_addr: u16,
        args: &[u16],
        return_store: Option<u8>,
    ) -> Result<(), String> {
        // Special case: calling address 0 returns false
        if packed_addr == 0 {
            if let Some(var) = return_store {
                self.vm.write_variable(var, 0)?;
            }
            return Ok(());
        }

        // Unpack the address
        let addr = self.unpack_routine_address(packed_addr) as u32;

        if self.debug {
            debug!("CALL to 0x{:05x} with args: {:?}", addr, args);
        }

        // Save current state
        let frame = CallFrame {
            return_pc: self.vm.pc,
            return_store,
            num_locals: 0, // Will be set when we read routine header
            locals: [0; 16],
            stack_base: self.vm.stack.len(),
        };
        debug!(
            "Creating call frame with return_store={:?}, stack_base={}",
            return_store,
            self.vm.stack.len()
        );

        // Read routine header
        let mut num_locals = self.vm.read_byte(addr) as usize;
        if num_locals > 15 {
            debug!(
                "Routine at {:05x} claims {} locals - clamping to 15",
                addr, num_locals
            );
            // Some games have corrupt headers or use this byte for other purposes
            // Clamp to 15 locals for V3
            num_locals = 15;
        }

        let mut new_frame = frame;
        new_frame.num_locals = num_locals as u8;

        // Set PC to start of routine code
        self.vm.pc = addr + 1;

        // Initialize locals
        if self.vm.game.header.version <= 4 {
            // V1-4: Read initial values from routine header
            for i in 0..num_locals {
                let value = self.vm.read_word(self.vm.pc);
                new_frame.locals[i] = value;
                self.vm.pc += 2;
            }

            // CRITICAL: Arguments overwrite the first N locals in V1-4
            // This is the key part that was missing!
            for (i, &arg) in args.iter().enumerate() {
                if i < num_locals {
                    new_frame.locals[i] = arg;
                }
            }
        } else {
            // V5+: Initialize to zero, except for arguments
            new_frame.locals[..num_locals.min(args.len())]
                .copy_from_slice(&args[..num_locals.min(args.len())]);
        }

        // Push the call frame
        self.vm.call_stack.push(new_frame);

        Ok(())
    }

    /// Handle routine returns
    fn do_return(&mut self, value: u16) -> Result<ExecutionResult, String> {
        // Pop the call frame
        let frame = self
            .vm
            .call_stack
            .pop()
            .ok_or("Return with empty call stack")?;

        debug!(
            "Returning from routine: value={}, return_pc={:05x}",
            value, frame.return_pc
        );

        // Restore PC
        self.vm.pc = frame.return_pc;

        // Restore stack
        debug!(
            "Stack before truncate: len={}, base={}",
            self.vm.stack.len(),
            frame.stack_base
        );
        self.vm.stack.truncate(frame.stack_base);

        // Store return value if needed
        if let Some(var) = frame.return_store {
            debug!("Storing return value {} to variable {}", value, var);
            self.vm.write_variable(var, value)?;
            debug!("Stack len after store: {}", self.vm.stack.len());
        }

        // Check if we're back at main
        if self.vm.call_stack.is_empty() {
            return Ok(ExecutionResult::GameOver);
        }

        Ok(ExecutionResult::Returned(value))
    }

    /// Unpack a routine address based on version
    fn unpack_routine_address(&self, packed: u16) -> usize {
        match self.vm.game.header.version {
            1..=3 => (packed as usize) * 2,
            4..=5 => (packed as usize) * 4,
            6..=7 => {
                // Would need to handle routine offset from header
                (packed as usize) * 4
            }
            8 => (packed as usize) * 8,
            _ => (packed as usize) * 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::Game;

    fn create_test_interpreter() -> Interpreter {
        let mut memory = vec![0u8; 0x10000];

        // Set up header
        memory[0x00] = 3; // Version 3
        memory[0x04] = 0x10; // High memory at 0x1000
        memory[0x05] = 0x00;
        memory[0x06] = 0x50; // Initial PC at 0x5000
        memory[0x07] = 0x00;
        memory[0x0c] = 0x01; // Global table at 0x0100
        memory[0x0d] = 0x00;
        memory[0x0e] = 0x02; // Static memory at 0x0200
        memory[0x0f] = 0x00;

        // Add a simple program at 0x5000: push 42, pop, quit
        memory[0x5000] = 0xE8; // VAR:OP1 push
        memory[0x5001] = 0x7F; // Operand types: small constant (01), then omitted
        memory[0x5002] = 0x2A; // Value: 42

        memory[0x5003] = 0xB9; // 0OP pop

        memory[0x5004] = 0xBA; // 0OP quit

        let game = Game::from_memory(memory).unwrap();
        let vm = VM::new(game);
        Interpreter::new(vm)
    }

    #[test]
    fn test_simple_execution() {
        let mut interp = create_test_interpreter();

        // Execute push instruction
        let inst = Instruction::decode(&interp.vm.game.memory, 0x5000, 3).unwrap();
        interp.vm.pc = 0x5003; // Advance past instruction
        let result = interp.execute_instruction(&inst).unwrap();
        assert!(matches!(result, ExecutionResult::Continue));
        assert_eq!(interp.vm.stack.len(), 1);
        assert_eq!(interp.vm.stack[0], 42);

        // Execute pop instruction
        let inst = Instruction::decode(&interp.vm.game.memory, 0x5003, 3).unwrap();
        interp.vm.pc = 0x5004;
        let result = interp.execute_instruction(&inst).unwrap();
        assert!(matches!(result, ExecutionResult::Continue));
        assert_eq!(interp.vm.stack.len(), 0);

        // Execute quit instruction
        let inst = Instruction::decode(&interp.vm.game.memory, 0x5004, 3).unwrap();
        let result = interp.execute_instruction(&inst).unwrap();
        assert!(matches!(result, ExecutionResult::Quit));
    }

    #[test]
    fn test_arithmetic() {
        let mut interp = create_test_interpreter();

        // Test add instruction - use a global variable for storage
        let memory = vec![
            0x14, // Long form, add, both small constants (00 01 0100)
            0x0A, // Constant 10
            0x20, // Constant 32
            0x10, // Store to global variable 0x10
        ];

        let inst = Instruction::decode(&memory, 0, 3).unwrap();
        // Set PC past the instruction (simulating that it was fetched)
        interp.vm.pc = inst.size as u32;
        let result = interp.execute_instruction(&inst).unwrap();
        assert!(matches!(result, ExecutionResult::Continue));
        // Check that global variable 0x10 now contains 42
        assert_eq!(interp.vm.read_global(0x10).unwrap(), 42);
    }
}
