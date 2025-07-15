use crate::instruction::Instruction;
use crate::interpreter::Interpreter;
use crate::vm::VM;
use std::io::{self, Write};

/// A debugger for step-by-step execution and disassembly
pub struct Debugger {
    /// Current interpreter state
    pub interpreter: Interpreter,
    /// Breakpoints (PC addresses)
    breakpoints: Vec<u32>,
    /// Single-step mode enabled
    single_step: bool,
    /// Instruction history
    history: Vec<(u32, String)>,
    /// Maximum history size
    max_history: usize,
}

impl Debugger {
    /// Create a new debugger
    pub fn new(vm: VM) -> Self {
        Debugger {
            interpreter: Interpreter::new(vm),
            breakpoints: Vec::new(),
            single_step: false,
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Enable or disable single-step mode
    pub fn set_single_step(&mut self, enabled: bool) {
        self.single_step = enabled;
        if enabled {
            println!("Single-step mode enabled. Use 'n' for next, 'c' for continue, 'q' for quit.");
        } else {
            println!("Single-step mode disabled.");
        }
    }

    /// Add a breakpoint at the given PC
    pub fn add_breakpoint(&mut self, pc: u32) {
        if !self.breakpoints.contains(&pc) {
            self.breakpoints.push(pc);
            println!("Breakpoint added at 0x{:05x}", pc);
        } else {
            println!("Breakpoint already exists at 0x{:05x}", pc);
        }
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&mut self, pc: u32) {
        if let Some(pos) = self.breakpoints.iter().position(|&x| x == pc) {
            self.breakpoints.remove(pos);
            println!("Breakpoint removed from 0x{:05x}", pc);
        } else {
            println!("No breakpoint at 0x{:05x}", pc);
        }
    }

    /// List all breakpoints
    pub fn list_breakpoints(&self) {
        if self.breakpoints.is_empty() {
            println!("No breakpoints set.");
        } else {
            println!("Breakpoints:");
            for bp in &self.breakpoints {
                println!("  0x{:05x}", bp);
            }
        }
    }

    /// Disassemble instruction at current PC
    pub fn disassemble_current(&self) -> Result<String, String> {
        let pc = self.interpreter.vm.pc;
        self.disassemble_at(pc)
    }

    /// Disassemble instruction at given PC
    pub fn disassemble_at(&self, pc: u32) -> Result<String, String> {
        match Instruction::decode(&self.interpreter.vm.game.memory, pc as usize, self.interpreter.vm.game.header.version) {
            Ok(inst) => {
                let formatted = inst.format_with_version(self.interpreter.vm.game.header.version);
                Ok(format!("{:05x}: {}", pc, formatted))
            }
            Err(e) => Err(format!("Failed to decode instruction at 0x{:05x}: {}", pc, e))
        }
    }

    /// Disassemble a range of instructions
    pub fn disassemble_range(&self, start_pc: u32, count: usize) -> Vec<String> {
        let mut results = Vec::new();
        let mut pc = start_pc;
        
        for _ in 0..count {
            match self.disassemble_at(pc) {
                Ok(line) => {
                    results.push(line);
                    // Try to get the instruction size to advance PC
                    if let Ok(inst) = Instruction::decode(&self.interpreter.vm.game.memory, pc as usize, self.interpreter.vm.game.header.version) {
                        pc += inst.size as u32;
                    } else {
                        pc += 1; // Fallback
                    }
                }
                Err(e) => {
                    results.push(format!("{:05x}: ERROR - {}", pc, e));
                    pc += 1;
                }
            }
        }
        
        results
    }

    /// Show current VM state
    pub fn show_state(&self) {
        println!("=== VM State ===");
        println!("PC: 0x{:05x}", self.interpreter.vm.pc);
        println!("Stack size: {}", self.interpreter.vm.stack.len());
        println!("Call stack depth: {}", self.interpreter.vm.call_stack.len());
        
        // Show current instruction
        if let Ok(disasm) = self.disassemble_current() {
            println!("Current: {}", disasm);
        }
        
        // Show call stack
        if !self.interpreter.vm.call_stack.is_empty() {
            println!("\nCall Stack:");
            for (i, frame) in self.interpreter.vm.call_stack.iter().enumerate() {
                println!("  [{}] Return PC: 0x{:05x}, Locals: {}", 
                        i, frame.return_pc, frame.num_locals);
                
                // Show locals for current frame
                if i == self.interpreter.vm.call_stack.len() - 1 {
                    for j in 0..frame.num_locals as usize {
                        println!("    Local {}: {}", j + 1, frame.locals[j]);
                    }
                }
            }
        }
        
        // Show recent stack values
        if !self.interpreter.vm.stack.is_empty() {
            println!("\nStack (top 5):");
            let start = self.interpreter.vm.stack.len().saturating_sub(5);
            for (i, value) in self.interpreter.vm.stack[start..].iter().enumerate() {
                println!("  [{}] {}", start + i, value);
            }
        }
    }

    /// Show instruction history
    pub fn show_history(&self, count: usize) {
        let start = self.history.len().saturating_sub(count);
        println!("Recent instructions:");
        for (pc, inst) in &self.history[start..] {
            println!("  {:05x}: {}", pc, inst);
        }
    }

    /// Execute a single instruction
    pub fn step(&mut self) -> Result<bool, String> {
        let pc = self.interpreter.vm.pc;
        
        // Decode and record instruction for history
        if let Ok(inst) = Instruction::decode(&self.interpreter.vm.game.memory, pc as usize, self.interpreter.vm.game.header.version) {
            let formatted = inst.format_with_version(self.interpreter.vm.game.header.version);
            
            // Add to history
            self.history.push((pc, formatted.clone()));
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
            
            // Show instruction if in single-step mode
            if self.single_step {
                println!("{:05x}: {}", pc, formatted);
            }
            
            // Update PC
            self.interpreter.vm.pc += inst.size as u32;
            
            // Execute instruction
            match self.interpreter.execute_instruction(&inst) {
                Ok(_) => Ok(true),
                Err(e) => Err(format!("Execution error at 0x{:05x}: {}", pc, e))
            }
        } else {
            Err(format!("Failed to decode instruction at 0x{:05x}", pc))
        }
    }

    /// Run until breakpoint or user interruption
    pub fn run(&mut self) -> Result<(), String> {
        loop {
            let pc = self.interpreter.vm.pc;
            
            // Check for breakpoints
            if self.breakpoints.contains(&pc) {
                println!("Hit breakpoint at 0x{:05x}", pc);
                self.set_single_step(true);
            }
            
            // Handle single-step mode
            if self.single_step {
                self.show_state();
                
                print!("(debug) ");
                io::stdout().flush().ok();
                
                let mut input = String::new();
                io::stdin().read_line(&mut input).map_err(|e| format!("Input error: {}", e))?;
                let input = input.trim();
                
                match input {
                    "n" | "next" | "" => {
                        // Step one instruction
                        if let Err(e) = self.step() {
                            return Err(e);
                        }
                    }
                    "c" | "continue" => {
                        self.set_single_step(false);
                    }
                    "s" | "state" => {
                        self.show_state();
                        continue;
                    }
                    "h" | "history" => {
                        self.show_history(10);
                        continue;
                    }
                    "d" | "disasm" => {
                        let disasm = self.disassemble_range(pc, 5);
                        for line in disasm {
                            println!("{}", line);
                        }
                        continue;
                    }
                    "q" | "quit" => {
                        return Ok(());
                    }
                    cmd if cmd.starts_with("b ") => {
                        if let Ok(addr) = u32::from_str_radix(&cmd[2..], 16) {
                            self.add_breakpoint(addr);
                        } else {
                            println!("Invalid address: {}", &cmd[2..]);
                        }
                        continue;
                    }
                    cmd if cmd.starts_with("rb ") => {
                        if let Ok(addr) = u32::from_str_radix(&cmd[3..], 16) {
                            self.remove_breakpoint(addr);
                        } else {
                            println!("Invalid address: {}", &cmd[3..]);
                        }
                        continue;
                    }
                    "bl" => {
                        self.list_breakpoints();
                        continue;
                    }
                    _ => {
                        println!("Commands: n(ext), c(ontinue), s(tate), h(istory), d(isasm), q(uit)");
                        println!("         b <addr> (breakpoint), rb <addr> (remove), bl (list)");
                        continue;
                    }
                }
            } else {
                // Run normally
                if let Err(e) = self.step() {
                    return Err(e);
                }
            }
        }
    }
}
