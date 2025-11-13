# INTERPRETER TOLERANCE ANALYSIS - Why We Don't Crash on Invalid Addresses

## DISCOVERY SUMMARY (November 13, 2025)

**Our interpreter has multiple tolerance mechanisms that silently handle invalid packed addresses, masking the Z-Machine compliance violations in our compiler.**

## KEY TOLERANCE MECHANISMS IDENTIFIED

### **1. String Decoding Bounds Checking** (`src/text.rs:40`)

```rust
while !is_end && offset + 1 < memory.len() && all_zchars.len() < max_string_length {
    let word = ((memory[offset] as u16) << 8) | (memory[offset + 1] as u16);
    offset += 2;
    // ... process word
}
```

**Tolerance Behavior**:
- **Graceful termination**: When unpacked address exceeds memory size, loop simply exits
- **No error**: Returns partial string instead of failing
- **Silent truncation**: Invalid addresses result in shortened text, not crashes

### **2. Abbreviation Address Validation** (`src/text.rs:91-99`)

```rust
// Check for obviously invalid addresses
if abbrev_byte_addr >= memory.len() || abbrev_byte_addr == 0 {
    debug!(
        "Invalid abbreviation address {:04x} (memory size: {}), skipping",
        abbrev_byte_addr,
        memory.len()
    );
    abbrev_shift = 0;
    continue; // Skip invalid abbreviation, continue processing
}
```

**Tolerance Behavior**:
- **Validation with fallback**: Detects out-of-bounds addresses
- **Graceful recovery**: Skips invalid abbreviations, continues string processing
- **Debug logging**: Records issues but doesn't fail

### **3. Address Unpacking Functions**

**Routine Address Unpacking** (`src/interpreter.rs:2093-2104`):
```rust
fn unpack_routine_address(&self, packed: u16) -> usize {
    match self.vm.game.header.version {
        1..=3 => (packed as usize) * 2,
        4..=5 => (packed as usize) * 4,
        // ... other versions
    }
}
```

**String Address Unpacking** (`src/text.rs:221-229`):
```rust
fn unpack_string_address(packed: u16, version: u8) -> usize {
    match version {
        1..=3 => (packed as usize) * 2,
        4..=5 => (packed as usize) * 4,
        // ... other versions
    }
}
```

**Critical Issue**: **NO BOUNDS VALIDATION**
- Functions perform pure mathematical calculation
- No verification that result is within file/memory bounds
- Return invalid addresses that get handled by downstream tolerance mechanisms

## WHY STANDARD TOOLS CRASH

### **Standard Z-Machine Behavior**
- **Fail fast**: Invalid addresses cause immediate errors
- **Strict validation**: All packed addresses must unpack to valid file locations
- **No tolerance**: Specification violations are fatal errors

### **Our Interpreter's Non-Standard Behavior**
- **Fail soft**: Invalid addresses trigger graceful fallbacks
- **Lenient validation**: Bounds checking happens at memory access, not address calculation
- **Silent tolerance**: Continues execution despite specification violations

## SPECIFIC EXAMPLE: Mini_Zork Case

**Problematic Address**: `0x4a52` → `0x94a5` (37,957 bytes)
**File Size**: 9,156 bytes
**Violation**: Address exceeds file size by ~4x

### **What Standard Tools Do (txd)**:
1. Calculate unpacked address: `0x4a52 * 2 = 0x94a5`
2. Attempt to read from file at offset 37,957
3. **FAIL**: File only has 9,156 bytes
4. **CRASH**: "Fatal: game file read error"

### **What Our Interpreter Does**:
1. Calculate unpacked address: `0x4a52 * 2 = 0x94a5`
2. Attempt string decode at offset 37,957
3. **BOUNDS CHECK**: `offset + 1 < memory.len()` → `37958 < 9156` → FALSE
4. **GRACEFUL EXIT**: Loop terminates, returns empty/partial string
5. **CONTINUE**: Game continues with missing text

## IMPACT ON DEBUGGING

### **Hidden Bugs**
- **Masking compiler errors**: Invalid addresses don't cause visible failures
- **Silent data loss**: Missing/corrupted text may go unnoticed
- **False confidence**: Games appear to work despite fundamental errors

### **Compatibility Issues**
- **Non-portable files**: Only work with our tolerant interpreter
- **Ecosystem isolation**: Cannot use standard Z-Machine tools
- **Professional limitations**: Files unsuitable for distribution

## COMPLIANCE REQUIREMENTS

### **Address Unpacking Functions Must**
1. **Validate bounds**: Ensure unpacked address < memory.len()
2. **Fail fast**: Return errors for invalid addresses
3. **Match standard behavior**: No special tolerance for bad files

### **Memory Access Must**
1. **Enforce strict bounds**: No graceful fallback for bad addresses
2. **Report violations**: Log compliance failures as errors
3. **Fail appropriately**: Stop execution on specification violations

### **Suggested Implementation**

```rust
fn unpack_routine_address(&self, packed: u16) -> Result<usize, String> {
    let unpacked = match self.vm.game.header.version {
        1..=3 => (packed as usize) * 2,
        4..=5 => (packed as usize) * 4,
        // ... other versions
    };

    if unpacked >= self.vm.game.memory.len() {
        return Err(format!(
            "Invalid packed address 0x{:04x} unpacks to 0x{:04x}, exceeds memory size {}",
            packed, unpacked, self.vm.game.memory.len()
        ));
    }

    Ok(unpacked)
}
```

## CONCLUSION

**Our interpreter's tolerance mechanisms explain why our non-compliant files work with our system but crash standard tools. The tolerance is helpful for robustness but harmful for compliance and compatibility.**

**Priority**: Remove tolerance mechanisms and implement strict Z-Machine specification compliance to ensure our files work with the broader Z-Machine ecosystem.