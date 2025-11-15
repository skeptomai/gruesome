use crate::debug_symbols::RoutineNames;
use crate::display_manager::{create_display, DisplayMode};
use crate::display_trait::ZMachineDisplay;
use crate::input_v3::V3Input;
use crate::input_v4::V4Input;
use crate::instruction::{Instruction, OperandType};
use crate::text;
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
    /// V3 input handler (for v1-v3 games)
    pub(crate) v3_input: Option<V3Input>,
    /// V4+ input handler (for v4+ games)  
    pub(crate) v4_input: Option<V4Input>,
    /// Display manager
    pub(crate) display: Option<Box<dyn ZMachineDisplay>>,
    /// Output stream state
    output_streams: OutputStreamState,
}

/// State for managing output stream redirection
struct OutputStreamState {
    /// Stack of output stream 3 tables (for nested redirection)
    stream3_stack: Vec<u16>,
    /// Current stream 3 table address (if active)
    current_stream3_table: Option<u16>,
}

impl OutputStreamState {
    fn new() -> Self {
        OutputStreamState {
            stream3_stack: Vec::new(),
            current_stream3_table: None,
        }
    }
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new(vm: VM) -> Self {
        // Get the game version for creating appropriate display
        let version = vm.game.header.version;

        // Determine display mode from environment or default to Auto
        let display_mode = match std::env::var("DISPLAY_MODE").as_deref() {
            Ok("ratatui") => DisplayMode::Ratatui,
            Ok("terminal") => DisplayMode::Terminal,
            Ok("headless") => DisplayMode::Headless,
            _ => DisplayMode::Auto,
        };

        // Try to initialize display, but continue without it if it fails
        let display = match create_display(version, display_mode) {
            Ok(d) => Some(d),
            Err(e) => {
                debug!("Failed to initialize display: {}", e);
                None
            }
        };

        // Create version-specific input handler
        let (v3_input, v4_input) = if version <= 3 {
            (Some(V3Input::new()), None)
        } else {
            (None, Some(V4Input::new()))
        };

        Interpreter {
            vm,
            debug: false,
            instruction_count: 0,
            routine_names: RoutineNames::new(),
            single_step: false,
            step_range: None,
            v3_input,
            v4_input,
            display,
            output_streams: OutputStreamState::new(),
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

        // Initialize header screen dimensions (required by Z-Machine spec)
        if let Some(ref display) = self.display {
            let (width, height) = display.get_terminal_size();
            // Byte 0x20: Screen height in lines
            self.vm.write_byte(0x20, height as u8)?;
            // Byte 0x21: Screen width in characters
            self.vm.write_byte(0x21, width as u8)?;
        }

        loop {
            // Fetch and decode instruction
            let pc = self.vm.pc;

            // Debug: Show raw bytes at critical addresses and quote area execution flow
            if pc == 0xcc6a {
                let bytes: Vec<u8> = self.vm.game.memory[pc as usize..pc as usize + 8].to_vec();
                debug!("DEBUG: At PC {:05x}, raw bytes: {:02x?}", pc, bytes);
            }
            if (0xcc6a..=0xcc70).contains(&pc) {
                debug!("*** QUOTE EXECUTION: About to execute at PC {:05x}", pc);
            }
            if (0x33b1c..=0x33b40).contains(&pc) {
                debug!("*** CENTERING ROUTINE: About to execute at PC {:05x}", pc);
            }
            // Debug spacing routine execution
            if (0x19ad8..=0x19b00).contains(&pc) {
                debug!("*** SPACING ROUTINE: About to execute at PC {:05x}", pc);
            }

            // Check for problematic Trinity PC range
            if (0x13fc0..=0x13ff0).contains(&pc) {
                debug!("🚨 TRINITY EXECUTION at PC {:05x}", pc);
            }

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
            let old_pc = self.vm.pc;
            self.vm.pc += instruction.size as u32;

            // Debug PC advancement for Trinity offset issue
            if old_pc == 0x125c7 {
                debug!(
                    "🚨 PC ADVANCEMENT: 0x{:05x} + {} = 0x{:05x} (instruction: {})",
                    old_pc,
                    instruction.size,
                    self.vm.pc,
                    instruction.name(self.vm.game.header.version)
                );
            }

            // Add single-step disassembly for Trinity PC tracking
            if old_pc >= 0x125bf && old_pc <= 0x125e0 {
                debug!(
                    "📍 EXECUTE: {:05x}: {} (size={}, next_pc={:05x})",
                    old_pc,
                    instruction.format_with_version(self.vm.game.header.version),
                    instruction.size,
                    self.vm.pc
                );
                // Show raw instruction bytes
                let end_addr = (old_pc as usize + instruction.size).min(self.vm.game.memory.len());
                let bytes: Vec<String> = self.vm.game.memory[old_pc as usize..end_addr]
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();
                debug!("📍 RAW BYTES: {}", bytes.join(" "));
            }

            // Track PC changes to catch jumps to invalid addresses like 13fe7
            let pc_before_exec = self.vm.pc;

            // Execute the instruction
            match self.execute_instruction(&instruction)? {
                ExecutionResult::Continue => {
                    // Normal execution, PC already advanced
                    // Debug PC state after execution for Trinity tracking
                    if old_pc >= 0x125bf && old_pc <= 0x125e0 {
                        debug!("📍 AFTER EXEC: PC remains at {:05x} (expected)", self.vm.pc);
                    }
                }
                ExecutionResult::Branched => {
                    // Branch taken, PC was updated by branch logic
                    let pc_after_exec = self.vm.pc;
                    if pc_after_exec == 0x13fe7 {
                        debug!("🚨 INVALID JUMP: Branch from {:05x} to invalid PC {:05x} (opcode: {:02x})", 
                               pc_before_exec, pc_after_exec, instruction.opcode);
                    }
                }
                ExecutionResult::Called => {
                    // Routine called, PC was updated
                    let pc_after_exec = self.vm.pc;
                    if pc_after_exec == 0x13fe7 {
                        debug!("🚨 INVALID JUMP: Call from {:05x} to invalid PC {:05x} (opcode: {:02x})", 
                               pc_before_exec, pc_after_exec, instruction.opcode);
                    }
                }
                ExecutionResult::Returned(_value) => {
                    // Return value already handled by do_return
                    let pc_after_exec = self.vm.pc;
                    if pc_after_exec == 0x13fe7 {
                        debug!("🚨 INVALID JUMP: Return from {:05x} to invalid PC {:05x} (opcode: {:02x})", 
                               pc_before_exec, pc_after_exec, instruction.opcode);
                    }
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

        // Check if this is a stack operation and handle it in the dedicated module
        if Interpreter::is_stack_opcode(inst.opcode, &inst.operand_count) {
            return self.execute_stack_op(inst, &operands);
        }

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
                crate::instruction::OperandCount::OP0 => {
                    // Check if this is a display operation and route to display module
                    if Interpreter::is_display_opcode(inst.opcode, &inst.operand_count) {
                        self.execute_display_op(inst, &[])
                    } else {
                        self.execute_0op(inst)
                    }
                }
                crate::instruction::OperandCount::OP1 => {
                    // Check if this is a math operation and route to math module
                    if Interpreter::is_math_opcode(inst.opcode, &inst.operand_count) {
                        self.execute_math_op(inst, &[operands[0]])
                    // Check if this is a memory operation and route to memory module
                    } else if Interpreter::is_memory_opcode(inst.opcode, &inst.operand_count) {
                        self.execute_memory_op(inst, &[operands[0]])
                    // Check if this is an object operation and route to object module
                    } else if Interpreter::is_object_opcode(inst.opcode, &inst.operand_count) {
                        self.execute_object_op(inst, &[operands[0]])
                    // Check if this is a display operation and route to display module
                    } else if Interpreter::is_display_opcode(inst.opcode, &inst.operand_count) {
                        self.execute_display_op(inst, &[operands[0]])
                    } else {
                        self.execute_1op(inst, operands[0])
                    }
                }
                _ => Err(format!(
                    "Invalid operand count for short form: {:?}",
                    inst.operand_count
                )),
            },
            crate::instruction::InstructionForm::Long => {
                // Check if this is a math operation and route to math module
                if Interpreter::is_math_opcode(inst.opcode, &inst.operand_count) {
                    self.execute_math_op(inst, &[operands[0], operands[1]])
                // Check if this is a memory operation and route to memory module
                } else if Interpreter::is_memory_opcode(inst.opcode, &inst.operand_count) {
                    self.execute_memory_op(inst, &[operands[0], operands[1]])
                // Check if this is an object operation and route to object module
                } else if Interpreter::is_object_opcode(inst.opcode, &inst.operand_count) {
                    self.execute_object_op(inst, &[operands[0], operands[1]])
                // Check if this is a display operation and route to display module
                } else if Interpreter::is_display_opcode(inst.opcode, &inst.operand_count) {
                    self.execute_display_op(inst, &[operands[0], operands[1]])
                } else {
                    self.execute_2op(inst, operands[0], operands[1])
                }
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

                        // Check if this is a math operation and route to math module
                        if Interpreter::is_math_opcode(inst.opcode, &inst.operand_count) {
                            self.execute_math_op(inst, &operands)
                        // Check if this is a memory operation and route to memory module
                        } else if Interpreter::is_memory_opcode(inst.opcode, &inst.operand_count) {
                            self.execute_memory_op(inst, &operands)
                        // Check if this is an object operation and route to object module
                        } else if Interpreter::is_object_opcode(inst.opcode, &inst.operand_count)
                            || Interpreter::is_var_13_object_opcode(inst)
                        {
                            self.execute_object_op(inst, &operands)
                        // Check if this is a display operation and route to display module
                        } else if Interpreter::is_display_opcode(inst.opcode, &inst.operand_count) {
                            self.execute_display_op(inst, &operands)
                        } else {
                            self.execute_2op_variable(inst, &operands)
                        }
                    }
                    _ => {
                        // Check if this is a memory operation and route to memory module
                        if Interpreter::is_memory_opcode(inst.opcode, &inst.operand_count) {
                            self.execute_memory_op(inst, &operands)
                        // Check if this is an object operation and route to object module
                        } else if Interpreter::is_object_opcode(inst.opcode, &inst.operand_count)
                            || Interpreter::is_var_13_object_opcode(inst)
                        {
                            self.execute_object_op(inst, &operands)
                        // Check if this is a display operation and route to display module
                        } else if Interpreter::is_display_opcode(inst.opcode, &inst.operand_count) {
                            self.execute_display_op(inst, &operands)
                        } else {
                            self.execute_var(inst, &operands)
                        }
                    }
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

    // ========================================================================
    // OPCODE EXECUTION METHODS
    // ========================================================================
    //
    // These methods handle the different instruction forms and organize opcodes
    // by their logical function:
    // - Control flow (return, quit, branch, jump)
    // - Stack operations (push, pull, call)
    // - Memory operations (load, store, loadw, storew)
    // - Arithmetic/logical operations (add, sub, mul, div, and, or, not)
    // - Object system operations (get_prop, put_prop, insert_obj, remove_obj)
    // - I/O operations (sread, read_char, print_*)
    // - System operations (save, restore, random, etc.)

    /// Execute 0OP instructions (control flow and system operations)
    fn execute_0op(&mut self, inst: &Instruction) -> Result<ExecutionResult, String> {
        match inst.opcode {
            // ---- CONTROL FLOW OPERATIONS ----
            0x00 => {
                // rtrue
                self.do_return(1)
            }
            0x01 => {
                // rfalse
                self.do_return(0)
            }

            // ---- I/O OPERATIONS ----
            0x04 => {
                // nop
                Ok(ExecutionResult::Continue)
            }

            // ---- SYSTEM OPERATIONS ----
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
            0x0A => {
                // quit
                Ok(ExecutionResult::Quit)
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

    /// Execute 1OP instructions (mostly memory and object operations)
    fn execute_1op(&mut self, inst: &Instruction, operand: u16) -> Result<ExecutionResult, String> {
        match inst.opcode {
            // ---- CONTROL FLOW OPERATIONS ----
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
            0x0C => {
                // jump
                // Jump is a signed offset from the instruction after the branch data
                let offset = operand as i16;
                let new_pc = (self.vm.pc as i32 + offset as i32 - 2) as u32;
                self.vm.pc = new_pc;
                Ok(ExecutionResult::Branched)
            }
            _ => Err(format!(
                "Unimplemented 1OP instruction: {:02x}",
                inst.opcode
            )),
        }
    }

    /// Execute 2OP instructions (arithmetic, logical, and comparison operations)
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
            0x19 => {
                // call_2s
                let routine_addr = op1;
                let arg = op2;
                let pc = self.vm.pc - inst.size as u32;

                // Debug logging for spacing routine calls
                if pc == 0xcc6e || pc == 0xcc84 || pc == 0xcca4 {
                    let unpacked = (routine_addr as u32).wrapping_mul(4);
                    debug!("*** SPACING ROUTINE CALL at PC {:05x}: calling packed addr {:04x} (unpacked: {:05x}) with arg {}", 
                           pc, routine_addr, unpacked, arg);
                }

                self.do_call(routine_addr, &[arg], inst.store_var)?;
                Ok(ExecutionResult::Called)
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
        if operands.is_empty() {
            let pc = self.vm.pc - inst.size as u32;
            match inst.opcode {
                0x09 => {
                    // Special case: Variable 2OP AND with no operands
                    // This appears in some games - treat as AND 0, 0
                    debug!(
                        "Variable 2OP AND with no operands at PC {:05x} - using 0, 0",
                        pc
                    );
                    return self.execute_2op(inst, 0, 0);
                }
                0x01 => {
                    // je with no operands means "jump if true" (always false)
                    debug!(
                        "Variable 2OP je with no operands at PC {:05x} - always false",
                        pc
                    );
                    return self.do_branch(inst, false);
                }
                _ => {}
            }
        }

        // Handle special cases for instructions that can work with fewer operands
        if operands.len() < 2 {
            let pc = self.vm.pc - inst.size as u32;
            match inst.opcode {
                0x01 => {
                    // je - Jump if Equal with 1 operand means "jump if operand equals 0"
                    if operands.len() == 1 {
                        debug!(
                            "Variable 2OP je with 1 operand at PC {:05x} - testing if {:04x} == 0",
                            pc, operands[0]
                        );
                        let condition = operands[0] == 0;
                        return self.do_branch(inst, condition);
                    }
                }
                0x13 => {
                    // get_next_prop - can be called with 1 operand (property 0 = get first property)
                    if operands.len() == 1 {
                        debug!("Variable 2OP get_next_prop with 1 operand at PC {:05x} - treating as get_next_prop {:04x}, 0", pc, operands[0]);
                        return self.execute_2op(inst, operands[0], 0);
                    }
                }
                _ => {}
            }
            return Err(format!("Variable 2OP instruction at PC {:05x} requires at least 2 operands, got {} - opcode: {:02x}", 
                               pc, operands.len(), inst.opcode));
        }

        // Handle each 2OP instruction based on its specific requirements
        match inst.opcode {
            0x01 => {
                // je - Jump if Equal (can have 2-4 operands)
                // From the spec: "je a b c d ?(label)"
                // Jump if a is equal to any of the subsequent operands (b, c, or d)
                let pc = self.vm.pc - inst.size as u32;

                // Debug output for the problematic JE at 13fd7
                if pc == 0x13fd7 {
                    debug!(
                        "🚨 TRINITY JE at 13fd7: operands={:?}, should branch to 1406d",
                        operands
                    );
                    for (i, op) in operands.iter().enumerate() {
                        debug!("  operand[{}] = {:04x}", i, op);
                    }
                }

                let mut condition = false;
                for i in 1..operands.len() {
                    if operands[0] == operands[i] {
                        condition = true;
                        break;
                    }
                }

                if pc == 0x13fd7 {
                    debug!(
                        "  condition={}, branch_on_true={:?}",
                        condition,
                        inst.branch.as_ref().map(|b| b.on_true)
                    );
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
                    "🕐 SREAD at PC 0x{:04x} with {} operands",
                    self.vm.pc - inst.size as u32,
                    operands.len()
                );
                for (i, op) in operands.iter().enumerate() {
                    debug!("🕐   operand[{}] = 0x{:04x}", i, op);
                }

                // Trinity-specific timer debug
                if operands.len() >= 4 {
                    debug!(
                        "🕐 TIMER CHECK: operands[2]={}, operands[3]=0x{:04x}, has_timer={}",
                        operands[2],
                        operands[3],
                        operands.len() >= 4 && operands[2] > 0 && operands[3] > 0
                    );
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

                        // Update status line with version info
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

                // Force display refresh to show any pending output (like prompt)
                // Also flush any buffered text (like '>' prompt that doesn't end with newline)
                if let Some(ref mut display) = self.display {
                    // First flush any buffered content
                    if let Err(_) = display.set_buffer_mode(false) {
                        // If flush fails, try force refresh
                        display.force_refresh().ok();
                    }
                    // Re-enable buffering for normal operation
                    display.set_buffer_mode(true).ok();
                }

                // Create timer callback closure if we have a timer
                let timer_callback = if has_timer && routine > 0 {
                    // Create a closure that captures self through a raw pointer
                    // This is safe because we know the interpreter outlives the input operation
                    let interp_ptr = self as *mut Interpreter;
                    Some(move || -> Result<bool, String> {
                        unsafe {
                            debug!("Timer callback triggered for routine 0x{:04x}", routine);
                            (*interp_ptr)
                                .call_timer_routine(routine)
                                .map(|result| result != 0)
                        }
                    })
                } else {
                    None
                };

                // Use version-specific input handling
                let (input, was_terminated) = if self.vm.game.header.version <= 3 {
                    // V3 and earlier - use simple input handler
                    debug!("Using V3 input handler for sread");
                    if let Some(ref mut v3_input) = self.v3_input {
                        v3_input
                            .read_line_with_timer(time, routine, timer_callback)
                            .map_err(|e| format!("Error reading V3 input: {e}"))?
                    } else {
                        return Err("V3 input handler not initialized".to_string());
                    }
                } else {
                    // V4+ - use advanced input handler
                    debug!("Using V4+ input handler for sread");
                    if let Some(ref mut v4_input) = self.v4_input {
                        if let Some(ref mut display) = self.display {
                            v4_input
                                .read_line(time, routine, timer_callback, display.as_mut())
                                .map_err(|e| format!("Error reading V4+ input: {e}"))?
                        } else {
                            return Err("Display not initialized for V4+ input".to_string());
                        }
                    } else {
                        return Err("V4+ input handler not initialized".to_string());
                    }
                };

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
            0x13 => {
                // output_stream
                if !operands.is_empty() {
                    let stream_num = operands[0] as i16;
                    debug!("output_stream: stream_num={}", stream_num);

                    match stream_num {
                        1 => {
                            // Enable screen output (always on, ignore)
                            debug!("output_stream: enabling screen output (always on)");
                        }
                        -1 => {
                            // Disable screen output (not implemented)
                            debug!("output_stream: disabling screen output (not implemented)");
                        }
                        3 => {
                            // Enable stream 3 (table redirection)
                            if operands.len() >= 2 {
                                let table_addr = operands[1];
                                debug!(
                                    "output_stream: enabling stream 3, table at 0x{:04x}",
                                    table_addr
                                );
                                self.enable_stream3(table_addr as u32)?;
                            } else {
                                debug!("output_stream: stream 3 requested but no table address provided");
                            }
                        }
                        -3 => {
                            // Disable stream 3
                            debug!("output_stream: disabling stream 3");
                            self.disable_stream3()?;
                        }
                        _ => {
                            debug!("output_stream: unsupported stream {}", stream_num);
                        }
                    }
                }
                Ok(ExecutionResult::Continue)
            }
            0x14 => {
                // input_stream (V3+)
                if !operands.is_empty() {
                    let stream_num = operands[0] as i16;
                    debug!("input_stream: stream_num={}", stream_num);

                    match stream_num {
                        0 => {
                            // Select keyboard input (default, always active)
                            debug!("input_stream: selecting keyboard input (default)");
                        }
                        1 => {
                            // Select file input (not commonly used)
                            debug!("input_stream: selecting file input (not implemented)");
                        }
                        _ => {
                            debug!("input_stream: unsupported stream {}", stream_num);
                        }
                    }
                } else {
                    debug!("input_stream: no stream number provided");
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
                            (*interp_ptr)
                                .call_timer_routine(routine)
                                .map(|result| result != 0)
                        }
                    })
                } else {
                    None
                };

                // Use V4+ input handler for character input
                let (ch, was_terminated) = if let Some(ref mut v4_input) = self.v4_input {
                    debug!("Using V4+ input handler for read_char");
                    v4_input
                        .read_char(time, routine, timer_callback)
                        .map_err(|e| format!("Error reading V4+ character: {e}"))?
                } else {
                    return Err("V4+ input handler not initialized for read_char".to_string());
                };

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
            0x17 => {
                // scan_table (V4+)
                // Searches for a value in a table
                // scan_table x table len form -> (result) ?(label)
                if self.vm.game.header.version < 4 {
                    return Err("scan_table is only available in V4+".to_string());
                }

                if operands.len() < 3 {
                    return Err("scan_table requires at least 3 operands".to_string());
                }

                let search_value = operands[0];
                let table_addr = operands[1] as u32;
                let table_len = operands[2];
                let form = if operands.len() > 3 {
                    operands[3]
                } else {
                    0x82
                }; // Default form

                // Parse form: bit 7 = word/byte, bits 0-6 = field length
                let is_word = (form & 0x80) != 0;
                let field_length = (form & 0x7F) as u32;

                debug!(
                    "scan_table: searching for 0x{:04x} in table at 0x{:04x}, len={}, form=0x{:02x}",
                    search_value, table_addr, table_len, form
                );
                debug!(
                    "scan_table: current PC = 0x{:05x} (EXPECTED: 0x125cb)",
                    self.vm.pc
                );

                // Debug the PC before scan_table to trace the offset issue
                if self.vm.pc >= 0x125c0 && self.vm.pc <= 0x125e0 {
                    debug!(
                        "🚨 PC OFFSET ISSUE: scan_table at PC {:05x}, should be 125cb",
                        self.vm.pc
                    );
                }
                debug!("  is_word={}, field_length={}", is_word, field_length);

                let mut found_addr = 0u16;
                let mut current_addr = table_addr;

                // Search through the table
                for i in 0..table_len {
                    let table_value = if is_word {
                        self.vm.read_word(current_addr)
                    } else {
                        self.vm.read_byte(current_addr) as u16
                    };

                    debug!(
                        "  Entry {}: 0x{:04x} at addr 0x{:04x}",
                        i, table_value, current_addr
                    );

                    if table_value == search_value {
                        found_addr = current_addr as u16;
                        debug!("  *** MATCH FOUND at addr 0x{:04x} ***", found_addr);
                        break;
                    }

                    // Move to next entry
                    current_addr += field_length;
                }

                // Store result
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, found_addr)?;
                }

                debug!(
                    "scan_table result: found_addr=0x{:04x}, condition={}",
                    found_addr,
                    found_addr != 0
                );

                // Branch if found
                let condition = found_addr != 0;

                // Debug Trinity scan_table branch calculation
                if self.vm.pc == 0x125cb {
                    debug!(
                        "🔍 TRINITY SCAN_TABLE at 125cb: condition={}, found_addr=0x{:04x}",
                        condition, found_addr
                    );
                    if let Some(ref branch) = inst.branch {
                        debug!(
                            "🔍 Branch info: on_true={}, offset={}",
                            branch.on_true, branch.offset
                        );
                        let should_branch = condition == branch.on_true;
                        debug!(
                            "🔍 Should branch: {} (condition={}, on_true={})",
                            should_branch, condition, branch.on_true
                        );
                        if should_branch {
                            let calc_target = (self.vm.pc as i32 + branch.offset as i32 - 2) as u32;
                            debug!(
                                "🔍 Calculated target: 0x{:05x} (expected: 0x125dc)",
                                calc_target
                            );
                        }
                    }
                }

                self.do_branch(inst, condition)
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
    pub(crate) fn do_branch(
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

                        // Debug output for Trinity JE branch
                        if self.vm.pc == 0x13fde {
                            debug!("🚨 TRINITY BRANCH from 13fde: offset={}, new_pc={:05x} (should be 1406d)", offset, new_pc);
                        }

                        // Add specific debug for the problematic branch
                        if self.vm.pc >= 0x08cc0 && self.vm.pc <= 0x08cd0 {
                            debug!(
                                "Branch at PC {:05x}: offset={} ({:04x}), new_pc={:05x}",
                                self.vm.pc, offset, offset as u16, new_pc
                            );
                        }

                        // Debug scan_table branch calculation for Trinity quit issue
                        if self.vm.pc >= 0x125cb && self.vm.pc <= 0x125dd {
                            debug!("🔍 SCAN_TABLE BRANCH at PC {:05x}: condition={}, branch.on_true={}, should_branch={}, offset={}", 
                                   self.vm.pc, condition, branch.on_true, should_branch, branch.offset);
                            if should_branch {
                                let calc_pc = (self.vm.pc as i32 + offset as i32 - 2) as u32;
                                debug!("🔍 SCAN_TABLE: Current PC={:05x}, offset={}, calculated target={:05x} (expected 125dc)", 
                                       self.vm.pc, offset, calc_pc);
                            }
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
    pub(crate) fn call_timer_routine(&mut self, routine_addr: u16) -> Result<u16, String> {
        debug!("Calling timer routine at 0x{:04x}", routine_addr);

        // Save current PC and call depth
        let saved_pc = self.vm.pc;
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

        // Restore the original PC (critical for proper execution flow)
        debug!(
            "Restoring PC from 0x{:05x} to 0x{:05x}",
            self.vm.pc, saved_pc
        );
        self.vm.pc = saved_pc;

        // Return true if routine wants to terminate input
        Ok(return_value)
    }

    pub(crate) fn do_call(
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

        debug!("do_call: saving return_pc={:05x}", self.vm.pc);

        // Special debug for calls that return to the newlines after the quote
        if self.vm.pc == 0xcc6a {
            debug!(
                "*** CALL TO ROUTINE #{:04x} that returns to newlines at 0cc6a",
                packed_addr
            );
        }

        // Special debug for the centering routine
        if packed_addr == 0xcec7 {
            debug!(
                "*** CALLING CENTERING ROUTINE #cec7 with {} args: {:?}",
                args.len(),
                args
            );
        }

        debug!(
            "Creating call frame with return_store={:?}, stack_base={}",
            return_store,
            self.vm.stack.len()
        );
        debug!(
            "Call stack before push: depth={}, frames={:?}",
            self.vm.call_stack.len(),
            self.vm
                .call_stack
                .iter()
                .map(|f| format!("{:05x}", f.return_pc))
                .collect::<Vec<_>>()
        );

        // Special debug for centering routine
        if packed_addr == 0xcec7 {
            debug!("*** ENTERING CENTERING ROUTINE at address {:05x}", addr);
        }

        // Special debug for spacing routine #66b6
        if packed_addr == 0x66b6 {
            debug!(
                "*** ENTERING SPACING ROUTINE #66b6 at address {:05x} with {} args: {:?}",
                addr,
                args.len(),
                args
            );
        }

        // Read routine header
        let num_locals = self.vm.read_byte(addr) as usize;
        if num_locals > 15 {
            panic!(
                "CORRUPTION DETECTED: Routine at {:05x} claims {} locals but Z-Machine max is 15. \
                This indicates severe bytecode corruption that must not be silently ignored.",
                addr, num_locals
            );
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
    pub(crate) fn do_return(&mut self, value: u16) -> Result<ExecutionResult, String> {
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

        debug!(
            "Call stack before return: depth={}, frames={:?}",
            self.vm.call_stack.len(),
            self.vm
                .call_stack
                .iter()
                .map(|f| format!("{:05x}", f.return_pc))
                .collect::<Vec<_>>()
        );

        // Restore PC
        self.vm.pc = frame.return_pc;

        debug!(
            "After setting PC to {:05x}, call stack depth={}",
            self.vm.pc,
            self.vm.call_stack.len()
        );

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
        let unpacked = match self.vm.game.header.version {
            1..=3 => (packed as usize) * 2,
            4..=5 => (packed as usize) * 4,
            6..=7 => {
                // Would need to handle routine offset from header
                (packed as usize) * 4
            }
            8 => (packed as usize) * 8,
            _ => (packed as usize) * 2,
        };

        // COMPLIANCE: Strict bounds checking - panic on invalid addresses
        if unpacked >= self.vm.game.memory.len() {
            panic!(
                "COMPLIANCE VIOLATION: Invalid packed routine address 0x{:04x} unpacks to 0x{:04x}, exceeds memory size {} bytes",
                packed, unpacked, self.vm.game.memory.len()
            );
        }

        unpacked
    }

    /// Enable output stream 3 (text redirection to table)
    pub(crate) fn enable_stream3(&mut self, table_addr: u32) -> Result<(), String> {
        debug!(
            "enable_stream3: redirecting to table at 0x{:04x}",
            table_addr
        );

        // Push current state onto stack (for nested redirection)
        if let Some(current) = self.output_streams.current_stream3_table {
            self.output_streams.stream3_stack.push(current);
        }

        // Set new table
        self.output_streams.current_stream3_table = Some(table_addr as u16);

        // Initialize table with 0 characters written
        self.vm.write_word(table_addr, 0)?;

        Ok(())
    }

    /// Disable output stream 3 (stop text redirection)
    pub(crate) fn disable_stream3(&mut self) -> Result<(), String> {
        debug!("disable_stream3: stopping text redirection");

        if self.output_streams.current_stream3_table.is_some() {
            // Pop from stack if there are nested redirections
            self.output_streams.current_stream3_table = self.output_streams.stream3_stack.pop();
        } else {
            debug!("disable_stream3: no active stream 3 to disable");
        }

        Ok(())
    }

    /// Output text, handling stream 3 redirection
    pub(crate) fn output_text(&mut self, text: &str) -> Result<(), String> {
        // Handle stream 3 redirection first
        if let Some(table_addr) = self.output_streams.current_stream3_table {
            let current_count = self.vm.read_word(table_addr as u32);
            debug!("output_text: capturing '{}' to stream 3 table at 0x{:04x}, current_count={} (CAPTURE ONLY)", 
                   text, table_addr, current_count);

            // Write text to table starting at table+2+current_count
            for (i, ch) in text.chars().enumerate() {
                let addr = table_addr + 2 + current_count + i as u16;
                // Write byte - VM will handle bounds checking
                self.vm.write_byte(addr as u32, ch as u8)?;
            }

            // Update character count in table
            let new_count = current_count + text.len() as u16;
            self.vm.write_word(table_addr as u32, new_count)?;

            debug!("output_text: stream 3 updated count to {} (text captured ONLY - display handled separately)", new_count);

            // IMPORTANT: When stream 3 is active, DON'T display text here
            // Stream 3 is for text measurement only - display is handled by separate routine
            return Ok(());
        }

        // Send to screen (whether stream 3 is active or not)
        if let Some(ref mut display) = self.display {
            display.print(text).ok();
        } else {
            print!("{}", text);
            io::stdout().flush().ok();
        }

        Ok(())
    }

    /// Output a single character, handling stream 3 redirection
    pub(crate) fn output_char(&mut self, ch: char) -> Result<(), String> {
        // Handle stream 3 redirection first
        if let Some(table_addr) = self.output_streams.current_stream3_table {
            let current_count = self.vm.read_word(table_addr as u32);
            debug!("output_char: redirecting '{}' to stream 3 table at 0x{:04x}, current_count={} (CAPTURE ONLY)", 
                   ch, table_addr, current_count);

            // Write character to table at table+2+current_count
            let addr = table_addr + 2 + current_count;
            // Write byte - VM will handle bounds checking
            self.vm.write_byte(addr as u32, ch as u8)?;

            // Update character count in table
            let new_count = current_count + 1;
            self.vm.write_word(table_addr as u32, new_count)?;

            debug!(
                "output_char: stream 3 updated count to {} (char captured AND will be displayed)",
                new_count
            );

            // IMPORTANT: Continue to display character even when stream 3 is active
            // Stream 3 is used for text measurement, but text should still be visible
        }

        // Send to screen (whether stream 3 is active or not)
        if let Some(ref mut display) = self.display {
            display.print_char(ch).ok();
        } else {
            print!("{}", ch);
            io::stdout().flush().ok();
        }

        Ok(())
    }

    /// Clean up terminal state on exit
    pub fn cleanup(&mut self) {
        debug!("Interpreter: Performing terminal cleanup");

        // The display Drop implementations will handle most cleanup,
        // but we can force it by dropping the display explicitly
        if self.display.is_some() {
            self.display = None;
        }

        debug!("Interpreter: Terminal cleanup completed");
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
