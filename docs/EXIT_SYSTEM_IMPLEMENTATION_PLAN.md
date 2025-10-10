# Exit System Implementation Plan - ACTUAL STATUS

## Current Status (October 10, 2025)

### ✅ FIXED - Store Instruction Form Selection (Oct 10, 2025)

#### Bug 16: Store Instruction Emitted as Wrong Form
**Status**: FIXED ✅

**Problem**: Store instruction for loop counter initialization emitted as SHORT form (PrintPaddr) instead of LONG form (Store), causing PC corruption in get_exit builtin.

**Root Cause**: Form determination logic counted operands, saw 1 operand → chose SHORT form:
```rust
// get_exit builtin line 1348:
emit_instruction(0x0D, [SmallConstant(0)], Some(239), None)
// 1 operand (value) + store_var (destination)
// Form logic: 1 operand → SHORT form (1OP)
// SHORT form 0x0D = PrintPaddr (1OP:13), NOT Store (2OP:13)!
```

**Impact**: Wrong instruction emitted, PC corruption, "Unimplemented VAR:0x0c" error

**Fix Part 1**: `codegen_instructions.rs:2084-2090` - Force LONG form for Store
```rust
(0x0D, 1 | 2) => Ok(InstructionForm::Long),  // store is 2OP
(0x0D, _) => Ok(InstructionForm::Variable),  // output_stream
```

**Fix Part 2**: `codegen_builtins.rs:1347-1355` - Pass Store operands correctly
```rust
self.emit_instruction_typed(
    Opcode::Op2(Op2::Store),
    &[Operand::Variable(index_var), Operand::SmallConstant(0)],
    None,  // Store does NOT use store_var
    None,
)
```

**Result**: Navigation system now works! PC corruption eliminated, all advances correct.

**See**: `docs/ARCHITECTURE.md` - "CRITICAL: Z-Machine Opcode Form Instability"

---

## Previous Status (October 9, 2025)

### ✅ FIXED - IR Generation Bug (Oct 9, 2025)

#### Bug 12: Builtin Pseudo-Method Property Checks
**Status**: FIXED ✅

**Problem**: Method calls like `player.location.get_exit(direction)` wrapped in conditional property checks that always failed for builtins, causing method calls to be skipped entirely.

**Root Cause**: IR generator created property lookup + conditional branch for ALL method calls:
```
1. get_property(location, "get_exit") → returns 0 (not a real property)
2. Branch: if property != 0 then call_method else return_0
3. Branch always takes else path, skips call entirely
```

**Fix**: `ir.rs:2645-2717` - Detect builtin pseudo-methods and generate direct calls
```rust
let is_builtin_pseudo_method = matches!(method.as_str(), "get_exit" | "empty" | "none");
if is_builtin_pseudo_method {
    // Generate direct Call without property check
}
```

**Impact**: `get_exit()` now executes correctly with full arguments. This was the missing link preventing navigation from working.

**See**: `docs/ARCHITECTURE.md` - "IR Generation for Builtin Pseudo-Methods"

---

## Previous Status (October 8, 2025)

### ✅ FIXED - Compiler Bugs Blocking Exit System

#### Bug 7: GetPropertyByNumber Variable Collision (Oct 8, 2025)
**Status**: FIXED ✅

**Problem**: All `GetPropertyByNumber` instructions hardcoded Variable 241, causing collisions when multiple properties accessed sequentially. This made get_exit() return 0.

**Fix**: `codegen_instructions.rs:552-590` - Allocate unique global variable per IR ID

**Impact**: get_exit() now successfully executes and returns packed values

#### Bug 8: Branch Encoding Placeholder Bit 15 (Oct 8, 2025)
**Status**: FIXED ✅

**Problem**: Branch placeholder 0x7FFF has bit 15=0 (branch on FALSE), causing get_exit to exit early even when properties exist.

**Fix**: `codegen_builtins.rs:1126, 1197, 1232` - Use -1 (0xFFFF) for "branch on TRUE"

**Impact**: get_exit now continues past initial checks, executes loop correctly

#### Bug 9: exit_data String IDs vs Packed Addresses (Oct 8, 2025)
**Status**: FIXED ✅

**Problem**: exit_data stored raw string IDs (1002) instead of packed addresses, causing garbage/errors at runtime.

**Fix**:
- `codegen_objects.rs:593-634` - Write placeholders, track messages in room_exit_messages
- `codegen.rs:4985-5016` - Create StringRef UnresolvedReferences during property serialization

**Verification**: String ID 1002 correctly resolved to packed address 0x0568 at location 0x03b4

**Impact**: Blocked exit messages now have correct packed string addresses in exit_data

#### Bug 10: V3 Property Two-Byte Format - Compiler Writing (Oct 8, 2025)
**Status**: FIXED ✅

**Problem**: Compiler generated two-byte property format for properties > 4 bytes but only wrote ONE byte instead of two, causing property tables to appear corrupted.

**Cause**: Formula `32 * (size - 1) + prop_num` correctly set bit 7=1 for sizes > 4, triggering two-byte format, but code didn't recognize this and write second size byte.

**Fix**:
- `codegen_strings.rs:759-852` - Explicit two-byte format detection (size > 4)
- Returns 4-tuple with optional second size byte
- `codegen.rs:4892-4936` - Write second byte when `two_byte_size.is_some()`

**Impact**: Property 22 (exit_data) with 6 bytes now correctly written as:
```
[0x96, 0x06, data...]  // header1, header2, data
```
Instead of broken:
```
[0xB6, 0x00, data...]  // header1, wrong-size, data
```

**See**: ARCHITECTURE.md section "CRITICAL: V3 Property Size Encoding - Two-Byte Format"

#### Bug 11: V3 Property Two-Byte Format - Interpreter Reading (Oct 8, 2025)
**Status**: FIXED ✅

**Problem**: After fixing compiler to write two-byte properties, interpreter still couldn't READ them. The `get_property_info()` function for V3 never checked bit 7 to detect two-byte format.

**Cause**: V3 code assumed single-byte format only (lines 436-440 in vm.rs):
```rust
// WRONG: Always reads size from bits 7-5, never checks bit 7
let prop_size = ((size_byte >> 5) & 0x07) + 1;
Ok((prop_num, prop_size as usize, 1))  // Always returns size_bytes=1!
```

**Fix**: `src/vm.rs:436-450` - Check bit 7 for two-byte format:
```rust
if size_byte & 0x80 != 0 {
    // Two-byte header: next byte contains size
    let size_byte_2 = self.game.memory[prop_addr + 1];
    let prop_size = if size_byte_2 == 0 { 64 } else { size_byte_2 as usize };
    Ok((prop_num, prop_size, 2))
} else {
    // Single-byte format
    let prop_size = ((size_byte >> 5) & 0x07) + 1;
    Ok((prop_num, prop_size as usize, 1))
}
```

**Impact**:
- Property 22 (exit_data) now correctly recognized with 6-byte size
- "Property 14 not found" error eliminated
- All exit properties (20, 21, 22) now accessible at runtime

**Regression**: All tests pass ✅, commercial Infocom games still work ✅

**See**: ARCHITECTURE.md section "CRITICAL: V3 Property Interpreter Bug - Two-Byte Format Support"

### ✅ IMPLEMENTED Components

#### 1. Exit Property Generation (codegen_objects.rs:508-620)
**Status**: COMPLETE and WORKING

The compiler generates three parallel array properties for each room with exits:
- `exit_directions` (property 20): Array of dictionary word addresses (2 bytes each)
- `exit_types` (property 21): Array of type bytes (0=room, 1=blocked)
- `exit_data` (property 22): Array of room IDs or message addresses (2 bytes each)

Example for west_of_house:
```
exit_directions: [dict_addr("north"), dict_addr("east"), dict_addr("south")]
exit_types: [0, 1, 0]  // north=normal, east=blocked, south=normal
exit_data: [2, 1002, 3]  // north=room#2, east=string#1002, south=room#3
```

#### 2. get_exit() Builtin (codegen_builtins.rs:1012-1320)
**Status**: COMPLETE and WORKING

Implements runtime lookup algorithm:
1. Get address of `exit_directions` property
2. Loop through directions comparing with input
3. When match found at index N:
   - Load `exit_types[N]` as type (0 or 1)
   - Load `exit_data[N]` as data (room ID or string address)
4. Pack result: `(type << 14) | data`
5. Return 0 if no match found

Returns:
- `0x0000 | room_id` for normal exits (bit 14 clear)
- `0x4000 | string_addr` for blocked exits (bit 14 set)
- `0` if direction not found

#### 3. IR Generation (ir.rs:2624-2638)
**Status**: COMPLETE and WORKING

The `get_exit` method call is recognized and translated to builtin call:
```rust
"get_exit" => {
    let builtin_id = self.next_id();
    self.builtin_functions.insert(builtin_id, "get_exit".to_string());
    let mut call_args = vec![object_temp];  // room object
    call_args.extend(arg_temps);  // direction string
    block.add_instruction(IrInstruction::Call {
        target: Some(result_temp),
        function: builtin_id,
        args: call_args,
    });
}
```

### ❌ MISSING Components - THE BUG

#### exit.blocked, exit.destination, exit.message Accessors
**Status**: NOT IMPLEMENTED

These pseudo-properties are supposed to extract data from the packed exit value:

**Expected behavior:**
```grue
let exit = player.location.get_exit("east");  // Returns 0x43EA (17386)
if exit.blocked {  // Should check: (exit >> 14) & 1 == 1
    print(exit.message);  // Should extract: exit & 0x3FFF = 1002
    return;
}
move(player, exit.destination);  // Should extract: exit & 0x3FFF
```

**Current behavior (Oct 8, 2025):**
- get_exit() now executes successfully and returns packed values
- Compiler bugs (variable collision, branch encoding, string address resolution) are FIXED
- Packed addresses verified correct in .z3 file (e.g., 0x0568 at location 0x03b4)
- **REMAINING ISSUE**: exit.blocked/destination/message pseudo-properties not implemented
- Current error: runtime tries to treat exit value as object number, fails validation

**Why it's broken:**
Property access `exit.blocked` generates:
```z-machine
get_prop object=<exit_value>, property="blocked"
```

But `exit` is a 16-bit packed integer (17386), not an object number!

## The Fix Required

### Option A: Implement Pseudo-Property Accessors

Add special handling in IR generation for these specific property names when accessed on exit values:

**Location**: `src/grue_compiler/ir.rs` - property access handling

**Implementation**:
```rust
match property_name {
    "blocked" => {
        // Generate: (value >> 14) & 1
        let shift_temp = self.next_id();
        block.add_instruction(IrInstruction::ShiftRight {
            target: shift_temp,
            value: object_temp,  // Actually the exit value
            amount: 14,
        });
        block.add_instruction(IrInstruction::And {
            target: result_temp,
            left: shift_temp,
            right: 1,
        });
    }
    "destination" | "message" => {
        // Generate: value & 0x3FFF
        block.add_instruction(IrInstruction::And {
            target: result_temp,
            left: object_temp,  // Actually the exit value
            amount: 0x3FFF,
        });
    }
    "none" => {
        // Generate: value == 0
        block.add_instruction(IrInstruction::Equal {
            target: result_temp,
            left: object_temp,
            right: 0,
        });
    }
    _ => {
        // Normal property access
    }
}
```

**Problem**: How do we know if the value is an exit vs a real object?

### Option B: Type Tracking

Track that the result of `get_exit()` has type "ExitValue" and only allow these pseudo-properties on ExitValue types.

**Requires**:
- Type system in IR
- Type inference/propagation
- Type checking on property access

**Complexity**: HIGH - adds type system to compiler

### Option C: Magic Property Numbers

Register "blocked", "destination", "message" as actual properties (like 60, 61, 62) and handle them specially in codegen.

**Problem**: Still need to distinguish exit values from objects at runtime.

### Option D: Use Methods Instead of Properties

Change syntax from `exit.blocked` to `exit.is_blocked()` and implement as builtin method calls:

```grue
if exit.is_blocked() {
    print(exit.get_message());
    return;
}
move(player, exit.get_destination());
```

**Requires**:
- Parser changes to support method calls on non-objects
- Three new builtins: `is_blocked`, `get_destination`, `get_message`

**Advantage**: Clear that these are operations, not property lookups

### Option E: Return Struct from get_exit

Instead of packed integer, return a synthetic "Exit" object with real properties.

**Problem**: Z-Machine doesn't support dynamic object creation. Would need to pre-allocate exit objects.

## Recommended Solution

**Option A with Runtime Check**: Implement pseudo-properties with defensive runtime behavior.

**Strategy**:
1. For `exit.blocked`, `exit.destination`, `exit.message`, `exit.none()`:
2. Generate bit-manipulation code (shift, and, compare)
3. Don't try to validate object number
4. Rely on type discipline in source code

**Why this works**:
- Exit values are always in specific range (0-16383 for destinations, 16384-32767 for blocked)
- Real object numbers are 1-255 (V3) or 1-65535 (V5+)
- No ambiguity if programmer uses correctly
- Fast - just bit operations

**Implementation in ir.rs**:
```rust
// In property access generation
if let Some(exit_pseudo_prop) = is_exit_pseudo_property(&property) {
    return self.generate_exit_pseudo_property_access(
        object_temp,  // Actually exit value
        exit_pseudo_prop,
        block
    );
}
```

Helper:
```rust
fn is_exit_pseudo_property(name: &str) -> Option<ExitPseudoProperty> {
    match name {
        "blocked" => Some(ExitPseudoProperty::Blocked),
        "destination" => Some(ExitPseudoProperty::Destination),
        "message" => Some(ExitPseudoProperty::Message),
        _ => None,
    }
}
```

## Next Steps

1. ✅ Document actual implementation status (this file)
2. Add `is_exit_pseudo_property()` helper to ir.rs
3. Implement `generate_exit_pseudo_property_access()` in ir.rs
4. Add IR instructions for bit operations if missing
5. Add codegen for bit operations in codegen.rs
6. Test with mini_zork navigation
7. Handle `.none()` method on exit values

## Test Case

**Input**: `east` from West of House
**Expected**:
```
get_exit returns: 0x43EA (17386)
  type = (17386 >> 14) = 1 (blocked)
  data = (17386 & 0x3FFF) = 1002 (message address)
exit.blocked = true
print(exit.message) = "The door is boarded and you can't remove the boards."
```

**Current (Oct 8, 2025)**:
```
get_exit: WORKING ✅ - returns packed values correctly
Compiler bugs FIXED:
  - Variable collision: each property gets unique variable
  - Branch encoding: uses -1 for "branch on true"
  - String addresses: StringRef resolves to packed addresses

REMAINING: exit.blocked/.destination/.message pseudo-properties not implemented
  - These try to call get_prop on the exit value (integer)
  - Need bit-manipulation code generation instead
```

## References

- Exit property generation: `src/grue_compiler/codegen_objects.rs:508-620`
- get_exit builtin: `src/grue_compiler/codegen_builtins.rs:1012-1320`
- IR method handling: `src/grue_compiler/ir.rs:2624-2638`
- Example usage: `examples/mini_zork.grue` handle_go function
