# Space Separation Architectural Fix

**Date**: September 1, 2025  
**Issue**: code_space contamination due to improper routing of data structure writes  
**Root Cause**: write_byte_at() routes all writes through emit_byte() → code_space, violating separated space architecture  

## Problem Analysis

### Current Broken Architecture
All writes go through `write_byte_at()` → `emit_byte()` → `code_space`:
- Header writes → code_space ❌
- Object table writes → code_space ❌  
- Property table writes → code_space ❌
- String writes → code_space ❌
- Global variable writes → code_space ❌
- **Executable code writes → code_space ✅** (only this should happen)

**Result**: PC points to beginning of code_space but finds header/object data instead of executable instructions.

### Proper Separated Space Architecture

```rust
header_space: Vec<u8>,     // 64-byte Z-Machine header
globals_space: Vec<u8>,    // Global variables  
object_space: Vec<u8>,     // Object/property tables
string_space: Vec<u8>,     // Encoded strings
code_space: Vec<u8>,       // Executable instructions ONLY
```

## Implementation Plan

### Phase 1: Add Space-Specific Write Functions
Replace `write_byte_at()` with explicit, intentional routing:

```rust
fn write_to_header_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError>
fn write_to_globals_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError>  
fn write_to_object_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError>
fn write_to_string_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError>
// emit_byte() already handles code_space correctly
```

**Critical**: Each function maintains single-path logged emission for debugging.

### Phase 2: Replace write_byte_at() Call Sites
Systematically replace ~50 `write_byte_at()` calls with space-appropriate functions:

- **Object/property generation** → `write_to_object_space()`
- **String encoding** → `write_to_string_space()`  
- **Global variable setup** → `write_to_globals_space()`
- **Header initialization** → `write_to_header_space()`
- **IR instruction translation** → `emit_byte()` (unchanged)

### Phase 3: Separate Retroactive Patching
For post-assembly fixups (PC address, memory layout addresses):

```rust
fn patch_final_byte(&mut self, addr: usize, byte: u8) -> Result<(), CompilerError>
```

This writes directly to `final_data` with full logging, separate from construction spaces.

### Phase 4: Remove write_byte_at()
Delete the architectural violation function entirely.

## Architectural Principles

1. **Explicit Intentional Routing**: Caller specifies target space, no magic address-based routing
2. **Single-Path Emission**: Each space maintains logged write path for debugging  
3. **Clean Separation**: Construction spaces never contaminate each other
4. **Clear Phases**: Construction → Assembly → Fixup, each with distinct write paths

## Success Criteria

- ✅ code_space contains ONLY executable instructions
- ✅ PC points to actual executable code (not data structures)
- ✅ All spaces maintain proper separation
- ✅ Single-path logging preserved for debugging
- ✅ mini_zork.z3 executes without "stack underflow" errors

## Files to Modify

- `src/grue_compiler/codegen.rs`: Add space-specific functions, replace ~50 call sites
- Test with `examples/mini_zork.grue` throughout implementation

## Risk Mitigation

- Implement incrementally, one space type at a time
- Test compilation success after each phase
- Verify space contents with logging before proceeding to next phase
- Maintain single-path emission principle throughout

This fix restores the intended separated space architecture and eliminates the fundamental contamination causing PC to point to data instead of executable code.