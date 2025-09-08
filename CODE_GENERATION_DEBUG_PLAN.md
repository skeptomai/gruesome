# Code Generation Debugging & Logging Plan

**Context**: mini_zork generates only 4 bytes of code from a complex program, indicating massive code generation pipeline failure. Need systematic debugging using rigorous approach.

## Phase 1: IR Input Analysis & Validation
**Goal**: Understand what IR is being fed into code generation

### 1.1 IR Inventory Logging
```rust
// Add to generate_complete_game_image() entry point
log::info!("🔍 IR INVENTORY: Comprehensive input analysis");
log::info!("  ├─ Functions: {} definitions", ir.functions.len());
log::info!("  ├─ Init blocks: {} blocks", ir.init_blocks.len()); 
log::info!("  ├─ Grammar rules: {} rules", ir.grammar_rules.len());
log::info!("  ├─ Objects: {} definitions", ir.objects.len());
log::info!("  ├─ String literals: {} strings", ir.string_literals.len());
log::info!("  └─ Total IR instructions: {} instructions", count_total_ir_instructions(&ir));

for (name, function) in &ir.functions {
    log::debug!("📋 Function '{}': {} instructions", name, function.body.len());
}
```

### 1.2 IR Instruction Enumeration
```rust
fn log_ir_instruction_breakdown(ir: &IrProgram) {
    let mut instruction_counts = HashMap::new();
    // Count every IR instruction by type across all functions/blocks
    // Log distribution: "Call: 45, Assignment: 23, Branch: 12, etc."
}
```

### 1.3 IR Validation Checkpoint
```rust
// Crash early if IR is malformed or empty
assert!(!ir.functions.is_empty(), "COMPILER BUG: No functions in IR");
assert!(!ir.init_blocks.is_empty(), "COMPILER BUG: No init blocks in IR");
```

## Phase 2: IR → Instruction Translation Tracking
**Goal**: Trace every IR instruction through translation pipeline

### 2.1 Translation Progress Logging
```rust
// In generate_program_code() - track every IR → bytecode conversion
for (function_name, function) in &ir.functions {
    log::debug!("🔧 TRANSLATING: Function '{}' ({} IR instructions)", 
                function_name, function.body.len());
    
    for (i, ir_instruction) in function.body.iter().enumerate() {
        log::trace!("  [{:02}] IR: {:?}", i, ir_instruction);
        
        // Track translation result
        let bytecode_before = self.code_space.len();
        self.translate_ir_instruction(ir_instruction)?;
        let bytecode_after = self.code_space.len();
        let bytes_generated = bytecode_after - bytecode_before;
        
        log::trace!("  [{:02}] Generated: {} bytes (0x{:04x}-0x{:04x})", 
                   i, bytes_generated, bytecode_before, bytecode_after);
                   
        if bytes_generated == 0 {
            log::error!("🚨 ZERO BYTES: IR instruction generated no bytecode: {:?}", ir_instruction);
        }
    }
    
    log::info!("✅ Function '{}' complete: {} bytes generated", 
               function_name, function_bytecode_size);
}
```

### 2.2 Translation Failure Detection
```rust
// Crash immediately on translation failures
fn translate_ir_instruction(&mut self, ir: &IrInstruction) -> Result<(), CompilerError> {
    let initial_size = self.code_space.len();
    
    // Actual translation logic...
    match ir {
        IrInstruction::Call(_) => self.generate_call_instruction(ir)?,
        IrInstruction::Assignment(_) => self.generate_assignment(ir)?,
        // ... etc
        _ => panic!("UNIMPLEMENTED: IR instruction {:?} has no translation", ir),
    }
    
    let final_size = self.code_space.len();
    if final_size == initial_size {
        panic!("COMPILER BUG: IR instruction {:?} generated zero bytes", ir);
    }
    
    Ok(())
}
```

## Phase 3: Instruction Emission Verification
**Goal**: Ensure every generated instruction is properly emitted

### 3.1 Emission Tracking
```rust
// In emit_instruction() - comprehensive emission logging  
fn emit_instruction(&mut self, opcode: u8, operands: &[Operand], 
                   store_var: Option<u8>, branch_offset: Option<i16>) -> Result<(), CompilerError> {
    let start_address = self.code_address;
    
    log::debug!("📝 EMIT: opcode=0x{:02x} operands={:?} store={:?} branch={:?} at=0x{:04x}", 
                opcode, operands, store_var, branch_offset, start_address);
    
    // Actual emission logic...
    
    let end_address = self.code_address;
    let instruction_size = end_address - start_address;
    
    log::debug!("✅ EMITTED: {} bytes (0x{:04x}-0x{:04x})", 
                instruction_size, start_address, end_address);
                
    if instruction_size == 0 {
        panic!("COMPILER BUG: Instruction emission generated zero bytes");
    }
    
    Ok(())
}
```

### 3.2 Code Space Growth Monitoring
```rust
// Track code space growth throughout generation
fn monitor_code_space_growth(&self, stage: &str) {
    log::info!("📊 CODE_SPACE[{}]: {} bytes, last_addr=0x{:04x}", 
               stage, self.code_space.len(), self.code_address);
               
    if self.code_space.len() < 50 && stage.contains("complete") {
        log::error!("🚨 SUSPICIOUSLY_SMALL: Code space only {} bytes after {}", 
                    self.code_space.len(), stage);
    }
}
```

## Phase 4: Init Block Deep Dive Analysis
**Goal**: Specifically debug why init block generates almost nothing

### 4.1 Init Block IR Dump
```rust
// Before init block generation
log::info!("🚀 INIT_BLOCK: Beginning generation");
for (i, init_block) in ir.init_blocks.iter().enumerate() {
    log::info!("📋 Init Block #{}: {} statements", i, init_block.statements.len());
    for (j, stmt) in init_block.statements.iter().enumerate() {
        log::debug!("  [{:02}] {:?}", j, stmt);
    }
}

// After init block generation  
let init_bytes_generated = code_space_after - code_space_before;
log::info!("✅ INIT_BLOCK: Generated {} bytes", init_bytes_generated);

if init_bytes_generated < 20 {
    log::error!("🚨 INIT_BLOCK_FAILURE: Only {} bytes generated from {} statements", 
                init_bytes_generated, total_init_statements);
}
```

### 4.2 Statement-Level Generation Tracking
```rust
// In generate_init_blocks()
for stmt in &init_block.statements {
    let stmt_start = self.code_space.len();
    log::trace!("🔧 Processing init statement: {:?}", stmt);
    
    self.generate_statement(stmt)?;
    
    let stmt_end = self.code_space.len();
    let stmt_bytes = stmt_end - stmt_start;
    
    log::trace!("✅ Init statement generated: {} bytes", stmt_bytes);
    
    if stmt_bytes == 0 {
        log::error!("🚨 INIT_STMT_ZERO: Statement generated no code: {:?}", stmt);
    }
}
```

## Phase 5: Main Loop Investigation
**Goal**: Understand why main loop generation is disabled

### 5.1 Main Loop Decision Logging
```rust
// Around the "Interactive mode: Main loop generation disabled" logic
log::info!("🔍 MAIN_LOOP: Analyzing generation decision");
log::info!("  ├─ Interactive mode: {}", self.is_interactive_mode());
log::info!("  ├─ Grammar rules: {}", ir.grammar_rules.len());
log::info!("  ├─ Main loop required: {}", self.needs_main_loop(&ir));
log::info!("  └─ Architecture status: {}", self.architecture_completion_status());

if self.should_skip_main_loop() {
    log::warn!("⚠️ MAIN_LOOP_SKIPPED: Reason: {}", self.main_loop_skip_reason());
} else {
    log::info!("🔧 MAIN_LOOP: Beginning generation");
    // ... main loop generation with detailed logging
}
```

## Phase 6: Reference Resolution Validation
**Goal**: Ensure references don't break code execution flow

### 6.1 Reference Completeness Check
```rust
// After reference resolution
log::info!("🔍 REFERENCE_RESOLUTION: Validation");
log::info!("  ├─ String references: {}/{} resolved", resolved_string_refs, total_string_refs);
log::info!("  ├─ Function references: {}/{} resolved", resolved_func_refs, total_func_refs);
log::info!("  └─ Unresolved remaining: {}", unresolved_count);

if unresolved_count > 0 {
    for unresolved in &self.unresolved_references {
        log::error!("🚨 UNRESOLVED: {:?} at 0x{:04x}", unresolved.target_type, unresolved.location);
    }
    panic!("COMPILER BUG: {} references remain unresolved", unresolved_count);
}
```

## Phase 7: Final Assembly Validation Enhancement
**Goal**: Comprehensive validation of generated code

### 7.1 Code Section Validation
```rust
fn validate_final_assembly(&self) -> Result<(), CompilerError> {
    // Existing validation...
    
    // NEW: Code section validation
    let code_section_size = self.final_code_base + self.code_space.len();
    log::info!("🔍 CODE_VALIDATION: {} bytes in final code section", code_section_size);
    
    if code_section_size < 50 {
        return Err(CompilerError::CodeGenError(format!(
            "Code section suspiciously small: {} bytes (expected hundreds+)", code_section_size
        )));
    }
    
    // NEW: Entry point validation
    let entry_point = self.final_code_base;
    if entry_point < self.final_code_base || entry_point >= self.final_code_base + self.code_space.len() {
        return Err(CompilerError::CodeGenError(format!(
            "Entry point 0x{:04x} outside code section bounds 0x{:04x}-0x{:04x}", 
            entry_point, self.final_code_base, self.final_code_base + self.code_space.len()
        )));
    }
    
    // NEW: Instruction validity check (first few bytes should be valid opcodes)
    self.validate_entry_point_instructions()?;
    
    Ok(())
}
```

## Implementation Priority

1. **Phase 1 & 2** (High Priority): IR analysis and translation tracking
2. **Phase 4** (Critical): Init block deep dive - likely where the bug is
3. **Phase 5** (High): Main loop investigation
4. **Phase 3, 6, 7** (Medium): Emission and validation enhancements

## Expected Outcome

This systematic approach will reveal:
- **Where the IR → bytecode pipeline breaks**
- **Which specific IR instructions aren't being translated** 
- **Why only 4 bytes are generated from complex code**
- **The exact point of failure in init block generation**

The logging will provide a complete **audit trail** from IR input to final bytecode, making it impossible for generation failures to hide.

## Progress Tracking

- [ ] Phase 1: IR Input Analysis & Validation
- [ ] Phase 2: IR → Instruction Translation Tracking  
- [ ] Phase 3: Instruction Emission Verification
- [ ] Phase 4: Init Block Deep Dive Analysis
- [ ] Phase 5: Main Loop Investigation
- [ ] Phase 6: Reference Resolution Validation
- [ ] Phase 7: Final Assembly Validation Enhancement

Created: August 29, 2025
Status: Ready for implementation