# Debug and Error Message Cleanup Plan

## Overview

Analysis completed: October 2, 2025
Current state: 640 println!/eprintln! statements, inconsistent log levels, mixed formatting styles

## Current Issues Identified

### 1. Inconsistent Message Styles
- **640 `println!`/`eprintln!` statements** vs **proper `log::`** calls
- Mix of emoji prefixes (🚀 🚨 ⚠️ ✅ ❌) vs plain text
- UPPERCASE keywords (COMPREHENSIVE, SYSTEMATIC, CRITICAL, ARCHITECTURAL) scattered throughout
- Inconsistent log levels (ERROR used for debug info, INFO for detailed trace)

### 2. Log Level Confusion
**Current Problems:**
- `log::error!()` used for debug trace: `"🚨 FUNCTION_CALL: calling addr=0x1476"`
- `log::warn!()` used for normal compilation progress: `"COMPREHENSIVE SCAN: Found 333 unique IR IDs"`
- `log::info!()` used for detailed execution trace that should be debug

**Standard Log Level Guidelines:**
- **ERROR**: Compilation/execution failures requiring user action
- **WARN**: Recoverable issues, deprecated patterns, potential problems
- **INFO**: High-level progress, major milestones
- **DEBUG**: Detailed technical information for troubleshooting
- **TRACE**: Very verbose execution trace

### 3. Message Categories
Distinct message categories that need consistent formatting:

1. **User-facing output** (game text, errors, prompts) → Should use `println!`
2. **Debug trace** (IR generation, instruction emission) → Should use `log::debug!`
3. **Progress indicators** (compilation phases) → Should use `log::info!`
4. **Warnings** (deprecated features, missing implementations) → Should use `log::warn!`
5. **Errors** (compilation failures, runtime crashes) → Should use `log::error!`
6. **Interactive debugger** (single-step, memory dump) → Should use `println!` (user tool)

### 4. Example Current Patterns

```rust
// Pattern 1: Emoji + uppercase + detailed context
log::info!("🚀 COMPLETE Z-MACHINE FILE GENERATION: Starting comprehensive game image generation");

// Pattern 2: Emoji markers for status
log::debug!("✅ Target ID {} FOUND in ir_id_to_address -> 0x{:04x}", id, addr);
log::debug!("❌ Target ID {} NOT FOUND in ir_id_to_address table!", id);

// Pattern 3: Plain println for user-facing output
println!("\n=== Single-step debugging enabled ===");
println!("PC range: 0x{start:04x} - 0x{end:04x}");

// Pattern 4: Lowercase technical details
log::debug!("IR INSTRUCTION: BinaryOp creates target IR ID {}", target);
```

## Cleanup Strategy - Phased Approach

### Phase 1: Quick Wins (CURRENT)
High-impact changes with minimal risk:

1. **Fix ERROR level abuse** - Change debug traces from `log::error!` to `log::debug!`
2. **Fix WARN level abuse** - Change compilation progress from `log::warn!` to `log::info!`
3. **Standardize compilation progress** - Use `log::info!` for "Phase 1", "Phase 2" etc.
4. **Remove UPPERCASE keywords in log messages** - Keep them in comments only

### Phase 2: Emoji Cleanup
Define and enforce emoji usage policy:

- **Option A**: Remove all emojis from log messages
- **Option B**: Keep emojis only for user-facing output (println!)
- **Option C**: Define specific emoji meanings (✅ = success, ❌ = failure, etc.)

### Phase 3: Module-by-Module Cleanup
Systematic cleanup of each module:

1. **src/grue_compiler/codegen.rs** - Largest offender, most log messages
2. **src/grue_compiler/codegen_instructions.rs** - Instruction emission logging
3. **src/interpreter.rs** - Runtime execution logging
4. **src/grue_compiler/ir.rs** - IR generation logging
5. **Other modules** - As needed

### Phase 4: Style Guide Creation
Document standards for future development:

- When to use `println!` vs `log::`
- Log level selection guidelines
- Message formatting standards
- Emoji usage policy
- Uppercase keyword policy

### Phase 5: Print Statement Audit
Address the 640 println!/eprintln! statements:

- Categorize: user-facing vs debug output
- Convert debug prints to `log::debug!`
- Keep user-facing prints as `println!`
- Remove temporary/outdated debug prints

## Quick Wins - Detailed Action Items

### 1. Fix ERROR Level Abuse

**Files to check:**
- `src/interpreter.rs` - `🚨 FUNCTION_CALL` messages
- `src/grue_compiler/codegen.rs` - Reference resolution error messages

**Pattern to fix:**
```rust
// BEFORE
log::error!("🚨 FUNCTION_CALL: calling addr=0x{:04x}", addr);

// AFTER
log::debug!("Function call to address 0x{:04x}", addr);
```

### 2. Fix WARN Level Abuse

**Files to check:**
- `src/grue_compiler/codegen.rs` - "COMPREHENSIVE SCAN" messages

**Pattern to fix:**
```rust
// BEFORE
log::warn!("COMPREHENSIVE SCAN: Found 333 unique IR IDs");

// AFTER
log::info!("IR mapping: found {} unique IR IDs", count);
```

### 3. Standardize Compilation Progress

**Pattern to establish:**
```rust
// Consistent phase logging
log::info!("Compilation phase 1: Memory layout planning");
log::info!("Compilation phase 2: String table generation");
log::info!("Compilation phase 3: Code generation");
```

### 4. Remove UPPERCASE Keywords

**Pattern to fix:**
```rust
// BEFORE
log::info!("🚀 COMPLETE Z-MACHINE FILE GENERATION: Starting comprehensive game image generation");

// AFTER
log::info!("Starting Z-Machine file generation");
```

**Exception:** Keep UPPERCASE in code comments for emphasis:
```rust
// CRITICAL: This function must be called before memory layout
```

## Success Criteria

### Phase 1 (Quick Wins) Complete When:
- [ ] All `log::error!` calls are genuine errors (compilation/runtime failures)
- [ ] All `log::warn!` calls are genuine warnings (recoverable issues)
- [ ] Compilation progress uses `log::info!` consistently
- [ ] UPPERCASE keywords removed from log messages (kept in comments only)
- [ ] Code still compiles and tests pass

### Overall Project Complete When:
- [ ] Consistent log level usage throughout codebase
- [ ] Clear separation: `println!` for user output, `log::` for development
- [ ] Emoji usage follows documented policy
- [ ] Style guide created and documented
- [ ] All modules follow consistent patterns

## Notes

- This is a large refactoring effort - do incrementally
- Test after each phase to avoid breaking functionality
- Some messages serve debugging purposes - don't remove valuable information
- User-facing output (game text, interactive debugger) should remain clear and readable
