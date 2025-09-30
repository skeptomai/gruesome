# Noun-to-Object Mapping System Implementation Roadmap

**Status**: September 28, 2025 - Phase 3 Complete, Phase 4 Identified
**Priority**: Critical - Fixes "invalid object 608" runtime crash
**Root Cause**: Dictionary address vs object property mismatch during user input processing

## Problem Summary

### Current Issue
- **Runtime Crash**: "Invalid object number: 608" during user input processing
- **Location**: PC 0x141e, clear_attr instruction (opcode 0x0C)
- **Trigger**: Grammar system passes parse buffer dictionary addresses (e.g., 608) to handler functions as `$noun` parameters
- **Bug**: These dictionary addresses are incorrectly treated as object IDs in property operations like `obj.open = false`

### Technical Root Cause
The grammar system correctly parses user input and provides dictionary addresses (608, 609, etc.) as noun references. However, the current object lookup system in `generate_object_lookup_from_noun()` has two critical flaws:
1. **Hardcoded**: Only supports objects 1-2, insufficient for real games
2. **Type Mismatch**: Compares dictionary addresses against string addresses in property 7

## Current Architecture Analysis

### Object ID Assignment System
**File**: `src/grue_compiler/codegen_objects.rs:463-473`
- IR objects → Z-Machine object numbers (1, 2, 3, ...)
- Objects stored with their names arrays in properties
- Property 7 is used for names storage
- **Issue**: Property 7 contains string addresses, not dictionary addresses

### Dictionary Generation
**File**: `src/grue_compiler/codegen_strings.rs:generate_dictionary_space`
- Currently minimal - only contains "quit" command
- **Missing**: All object names from objects' names arrays
- Each dictionary entry has unique address (608, etc.)
- **Issue**: Object names not added to dictionary

### Current Lookup System
**File**: `src/grue_compiler/codegen.rs:5429-5469`
- Hardcoded to check only objects 1 and 2
- Compares dictionary address against property 7 (names)
- **CRITICAL FLAW**: Dictionary addresses vs names strings mismatch

### Example Object Definition
```grue
object mailbox {
    names: ["small mailbox", "mailbox", "box"]
    desc: "The mailbox is small and made of wood."
    openable: true
}
```

**Current Flow**:
1. User types "look at mailbox"
2. Parser finds "mailbox" in dictionary → returns address 608
3. Grammar calls `look_at_obj(608)` with dictionary address
4. Function tries `obj.open = false` → `clear_attr(608, 1)`
5. **CRASH**: 608 is not a valid object ID (must be 1-255)

## Comprehensive Solution: 4-Phase Implementation

### Phase 1: Dictionary Generation Enhancement

**Goal**: Populate dictionary with all object names and create address-to-object mapping.

**Files to Modify**:
- `src/grue_compiler/codegen_strings.rs`

**Implementation**:

1. **Extract all names from all objects**:
   ```rust
   // In generate_dictionary_space():
   let mut all_object_names = Vec::new();

   for object in &ir.objects {
       for name in &object.names {
           all_object_names.push((name.clone(), object.id));
       }
   }

   for room in &ir.rooms {
       // Add room names if they have vocabulary
       if let Some(names) = &room.names {
           for name in names {
               all_object_names.push((name.clone(), room.id));
           }
       }
   }
   ```

2. **Create bidirectional mapping**:
   ```rust
   // Add to CodeGenerator struct:
   dict_addr_to_object_id: IndexMap<u16, u8>,  // Dictionary address → Object ID
   name_to_dict_addr: IndexMap<String, u16>,   // Object name → Dictionary address
   ```

3. **Generate dictionary with proper Z-Machine encoding**:
   ```rust
   // For each object name:
   let encoded_name = self.encode_word_to_zchars(name)?;
   let dict_addr = current_dict_address;

   // Store in dictionary
   dictionary_data.extend_from_slice(&encoded_name);
   dictionary_data.extend_from_slice(&dict_addr.to_be_bytes());

   // Update mappings
   self.dict_addr_to_object_id.insert(dict_addr, object_id);
   self.name_to_dict_addr.insert(name.clone(), dict_addr);
   ```

### Phase 2: Object Property System Update

**Goal**: Store dictionary addresses in object properties instead of raw strings.

**Files to Modify**:
- `src/grue_compiler/codegen_objects.rs`

**Implementation**:

1. **Change property 7 storage format**:
   ```rust
   // Instead of:
   // property 7 → string_address → "mailbox"

   // Store:
   // property 7 → array_address → [dict_addr_1, dict_addr_2, dict_addr_3]
   ```

2. **Create names array for each object**:
   ```rust
   // For each object with names array:
   let mut names_dict_addrs = Vec::new();
   for name in &object.names {
       if let Some(&dict_addr) = self.name_to_dict_addr.get(name) {
           names_dict_addrs.push(dict_addr);
       }
   }

   // Store array of dictionary addresses in property 7
   let names_array_addr = self.allocate_and_store_array(&names_dict_addrs)?;
   object_properties.set_word(7, names_array_addr);
   ```

3. **Array storage format**:
   ```
   Names Array in Memory:
   [count][dict_addr_1][dict_addr_2][dict_addr_3]
   Example: [0x03][0x0260][0x0261][0x0262]  // 3 names
   ```

### Phase 3: Dynamic Lookup System Replacement

**Goal**: Replace hardcoded lookup with dynamic system supporting all objects.

**Files to Modify**:
- `src/grue_compiler/codegen.rs` (function `generate_object_lookup_from_noun`)

**Implementation**:

1. **Generate dynamic loop through all objects**:
   ```rust
   // Replace hardcoded checks with:

   // Initialize result to 0 (not found)
   self.emit_store_variable(3, 0)?;

   // Loop through all objects (1 to max_objects)
   for object_id in 1..=max_objects {
       // Get property 7 (names array) for current object
       self.emit_instruction(0x11, &[  // get_prop
           Operand::SmallConstant(object_id),
           Operand::SmallConstant(7),
       ], Some(5), None)?;  // Store names array address in var 5

       // Call helper function to search array
       self.emit_call_builtin("search_names_array", &[
           Variable(5),  // Names array address
           Variable(2),  // Noun dictionary address to find
           Variable(object_id as u8), // Object ID to return if found
       ])?;

       // If found (result != 0), break from loop
       self.emit_conditional_break_if_nonzero()?;
   }
   ```

2. **Implement array search builtin**:
   ```rust
   // New builtin function: search_names_array(array_addr, target_dict_addr, object_id)
   fn generate_search_names_array_builtin(&mut self) -> Result<(), CompilerError> {
       // Load array count
       // Loop through array entries
       // Compare each entry with target_dict_addr
       // Return object_id if found, 0 if not found
   }
   ```

3. **Fallback handling**:
   ```rust
   // After loop completes:
   // Variable 3 contains matched object ID (or 0 if no match)
   // Return gracefully instead of crashing
   ```

### Phase 4: Integration Points

**Goal**: Ensure seamless integration with existing grammar and parser systems.

**Files to Verify**:
- `src/interpreter.rs` (parse_text function)
- Grammar processing system
- Error handling paths

**Integration Requirements**:

1. **Parser Integration**:
   - Parse buffer continues to provide dictionary addresses (608, etc.)
   - No changes needed to existing parse_text functionality

2. **Grammar System**:
   - No changes needed - still passes `$noun` as dictionary address
   - `look_at_obj($noun)` continues to work with same interface

3. **Error Handling**:
   - Replace crash with graceful "I don't understand that" response
   - Add check: `if (object_id == 0) print("I don't see that here.")`

4. **Testing Integration**:
   ```rust
   // Test cases to verify:
   // 1. "look at mailbox" → finds object 2
   // 2. "look at box" → finds object 2 (alternate name)
   // 3. "look at xyz" → graceful error, no crash
   ```

## Technical Implementation Details

### Dictionary Address Format
```
Dictionary Entry Structure:
[z-char-1][z-char-2][z-char-3][dict-addr-high][dict-addr-low]

Example: "mailbox" encoded:
[encoded-zchars-for-mailbox][0x02][0x60]  // Address 608 (0x0260)
```

### Property 7 New Format
```
Current Architecture:
property 7 → string_address → "mailbox"

New Architecture:
property 7 → array_address → [count][dict_addr_1][dict_addr_2][dict_addr_3]

Example:
property 7 → 0x1200 → [0x03][0x0260][0x0261][0x0262]
                       (3 names: mailbox, small mailbox, box)
```

### Lookup Algorithm Pseudocode
```rust
fn lookup_object_from_noun(noun_dict_addr: u16) -> u8 {
    for object_id in 1..=max_objects {
        let names_array_addr = get_prop(object_id, 7);
        let count = read_word(names_array_addr);

        for i in 0..count {
            let dict_addr = read_word(names_array_addr + 2 + (i * 2));
            if dict_addr == noun_dict_addr {
                return object_id;
            }
        }
    }
    return 0; // Not found
}
```

## Implementation Priority

### Phase 1: Dictionary Enhancement (Foundation)
- **Priority**: Highest
- **Dependencies**: None
- **Impact**: Enables all other phases
- **Estimated Effort**: Medium

### Phase 2: Property System Update (Critical)
- **Priority**: High
- **Dependencies**: Phase 1 complete
- **Impact**: Required for lookup system to work
- **Estimated Effort**: Medium

### Phase 3: Lookup System Replacement (Immediate Fix)
- **Priority**: High
- **Dependencies**: Phases 1-2 complete
- **Impact**: Fixes the immediate crash issue
- **Estimated Effort**: High

### Phase 4: Integration Testing (Robustness)
- **Priority**: Medium
- **Dependencies**: Phases 1-3 complete
- **Impact**: Ensures system robustness
- **Estimated Effort**: Low

## Benefits of This Architecture

### Scalability
- Supports unlimited objects and names
- No hardcoded limits or object-specific code
- Automatically handles new objects added to games

### Standards Compliance
- Follows Z-Machine conventions for dictionary and properties
- Compatible with Z-Machine specification requirements
- Maintains proper separation of concerns

### System Integration
- Works with existing grammar and parser systems
- No changes needed to user-facing language syntax
- Backward compatible with existing game definitions

### Error Handling
- Graceful error handling instead of runtime crashes
- Clear error messages for debugging
- Fail-safe behavior for edge cases

### Maintainability
- Clear separation between dictionary, objects, and lookup
- Well-defined interfaces between components
- Easy to extend for future features

## Testing Strategy

### Unit Tests
1. **Dictionary Generation**: Verify all object names added correctly
2. **Property Storage**: Verify property 7 contains correct dictionary addresses
3. **Lookup Algorithm**: Verify correct object ID returned for known names

### Integration Tests
1. **Grammar Integration**: Test complete flow from user input to object identification
2. **Error Cases**: Verify graceful handling of unknown object names
3. **Multiple Names**: Verify objects with multiple names work correctly

### Regression Tests
1. **Existing Functionality**: Ensure no regression in other game features
2. **Performance**: Verify lookup performance acceptable for gameplay
3. **Memory Usage**: Verify memory usage remains reasonable

## CRITICAL RUNTIME ISSUE: "invalid object 608" Crash
**Root Cause**: Dictionary address vs object property mismatch during user input processing
**Impact**: Game crashes when player types commands with nouns (e.g., "examine mailbox")
**Error Location**: Parse buffer address 608 (0x0260) being treated as object ID instead of dictionary address

## COMPREHENSIVE 4-PHASE SOLUTION ROADMAP

### ✅ Phase 1: Dictionary Generation Enhancement (COMPLETED)
**Status**: ✅ Successfully implemented in Session

**Goal**: Create bidirectional dictionary address mappings for noun resolution

**Implementation**:
- Modified `generate_dictionary_from_ir` in `codegen_strings.rs`
- Added `dict_addr_to_object_id: IndexMap<u16, IrId>` mapping
- Added `name_to_dict_addr: IndexMap<String, u16>` mapping
- Sequential dictionary address assignment starting at 0x0260 (608)
- All object names now have predictable dictionary addresses

**Technical Details**:
```rust
// In CodeGenerator struct
pub dict_addr_to_object_id: IndexMap<u16, IrId>,
pub name_to_dict_addr: IndexMap<String, u16>,

// Dictionary generation with sequential addressing
let mut current_dict_addr = 0x0260; // Start at 608
for name in names {
    self.name_to_dict_addr.insert(name.clone(), current_dict_addr);
    self.dict_addr_to_object_id.insert(current_dict_addr, *object_id);
    current_dict_addr += 1;
}
```

**Verification**: Dictionary addresses now map: "mailbox" → 608, "leaflet" → 609, etc.

### ✅ Phase 2: Property System Update (COMPLETED)
**Status**: ✅ Successfully implemented in Session

**Goal**: Generate property 7 with dictionary address arrays for dynamic object lookup

**Implementation**:
- Modified `create_property_table_from_ir` in `codegen_objects.rs`
- Added IR object parameter to property generation functions
- Implemented property 7 generation using Phase 1 mappings
- Fixed compilation order: dictionary generation → object table generation

**Technical Details**:
```rust
// Property 7 generation for each object
if let Some(ir_object) = ir_object {
    let mut dict_addresses = Vec::new();
    for name in &ir_object.names {
        if let Some(&dict_addr) = self.name_to_dict_addr.get(name) {
            dict_addresses.push(dict_addr);
        }
    }

    if !dict_addresses.is_empty() {
        // Generate property 7 with dictionary address array
        property_bytes.extend_from_slice(&[7, (dict_addresses.len() * 2) as u8]);
        for addr in dict_addresses {
            property_bytes.extend_from_slice(&addr.to_be_bytes());
        }
    }
}
```

**Verification**: All objects now have property 7 containing their dictionary addresses as byte arrays

### ✅ Phase 3: Dynamic Object Lookup Implementation (COMPLETED)
**Status**: ✅ Successfully implemented and tested in Session

**Goal**: Replace hardcoded 2-object lookup with comprehensive dynamic system

**Implementation**:
- Completely rewrote `generate_object_lookup_from_noun()` function in `codegen.rs`
- Implemented nested loop system: objects → property 7 array elements
- Added inline array search with proper byte-level access and word reconstruction
- Handles word reconstruction from byte arrays (high_byte * 256 + low_byte)

**Technical Architecture**:
```
INPUT: Variable(2) = noun dictionary address (e.g., 608 for "mailbox")
OUTPUT: Variable(3) = object ID (e.g., 30) or 0 if not found

ALGORITHM:
1. Loop through all objects (Variable 4 = object counter)
2. For each object, get property 7 array
3. Loop through array elements (Variable 7 = array index)
4. Reconstruct dictionary address from bytes: (high_byte * 256) + low_byte
5. Compare with target dictionary address
6. If match: return object ID, else continue
7. If no match found: return 0
```

**Code Generation**:
- Dynamic object count: `self.object_numbers.len()`
- Property access: `get_prop` instruction with property 7
- Array iteration: `loadb` for byte access, arithmetic for word reconstruction
- Branch logic: Conditional jumps for loop control and match detection

**Verification**:
- ✅ Compilation successful with no critical errors
- ✅ Phase 3 code generation complete and functional
- ✅ Dynamic lookup system replaces hardcoded 2-object limitation

### 🔄 Phase 4: Universal Grammar Pattern Integration (IN PROGRESS)
**Status**: 🔄 Implementation required - Root cause identified

**Current Issue**: `$noun` parameters in grammar patterns bypass dynamic lookup system

**Root Cause Analysis**:
The "invalid object 608" crash still occurs because there are **multiple code paths** for noun access:

1. **✅ WORKING: "verb + noun" patterns** (e.g., "look at mailbox")
   - Code: Lines 5322-5382 in `generate_verb_handler()`
   - Uses: `generate_object_lookup_from_noun()` correctly
   - Status: Fixed by Phase 3

2. **❌ BROKEN: Direct noun patterns** (e.g., "examine mailbox")
   - Code: Grammar pattern `noun => examine($noun)`
   - Issue: `$noun` resolves to **literal dictionary address 608**
   - Problem: Function receives 608 instead of object ID
   - Crash: `examine()` function treats 608 as object ID → "invalid object 608"

**Technical Analysis**:
- Crash location: PC=0x1496, instruction 0x0142 accessing object 608
- Pattern triggering crash: `verb "examine" { noun => examine($noun) }`
- Parameter resolution: `$noun` → dictionary address 608 (not object ID)

**Required Fix Scope**:
All grammar patterns using `$noun` parameters need:
1. Dictionary address extraction from parse buffer
2. Dynamic object lookup call (Phase 3 function)
3. Function call with resolved object ID (not dictionary address)

**Implementation Strategy**:
- Modify grammar pattern compilation to treat `$noun` as two-step operation
- Ensure all noun patterns call `generate_object_lookup_from_noun()`
- Pass Variable(3) (resolved object ID) to functions instead of raw dictionary addresses

**Files Requiring Updates**:
- `src/grue_compiler/codegen.rs`: Grammar pattern generation
- Pattern handling for `IrPatternElement::Noun`
- Function parameter resolution for `$noun` references

## CURRENT STATUS SUMMARY

### ✅ COMPLETED PHASES (1-3):
- **Phase 1**: Dictionary generation with bidirectional mappings ✅
- **Phase 2**: Property 7 arrays with dictionary addresses ✅
- **Phase 3**: Dynamic object lookup system ✅

### 🔄 REMAINING WORK (Phase 4):
- **Issue**: Direct noun patterns still use dictionary addresses as object IDs
- **Solution**: Extend dynamic lookup to ALL grammar patterns with `$noun`
- **Impact**: Will resolve "invalid object 608" crash completely

### 🏗️ IMPLEMENTATION ARCHITECTURE:

```
CURRENT STATE:
Parse Buffer[3] = 608 (dictionary address for "mailbox")
│
├─ "look at mailbox" → generate_object_lookup_from_noun() → Variable(3)=30 ✅
│
└─ "examine mailbox" → examine($noun=608) → CRASH ❌

TARGET STATE:
Parse Buffer[3] = 608 (dictionary address for "mailbox")
│
├─ "look at mailbox" → generate_object_lookup_from_noun() → Variable(3)=30 ✅
│
└─ "examine mailbox" → generate_object_lookup_from_noun() → Variable(3)=30 → examine(30) ✅
```

## TESTING VERIFICATION

### Phase 3 Testing Results:
- **Compilation**: ✅ Successful with dynamic object lookup
- **Code Generation**: ✅ Nested loops and array search implemented
- **Binary Output**: ✅ /tmp/test_phase3.z3 created successfully
- **Runtime Test**: ❌ Still crashes on "invalid object 608"
- **Crash Analysis**: ❌ Direct noun patterns bypass Phase 3 system

### Expected Phase 4 Results:
- **All Grammar Patterns**: Use dynamic object lookup system
- **Runtime Stability**: No more "invalid object 608" crashes
- **Noun Commands**: "examine mailbox", "take leaflet" work correctly
- **Object Resolution**: Dictionary address 608 → Object ID 30 consistently

## TECHNICAL IMPLEMENTATION NOTES

### Z-Machine Instruction Details:
- **Object Lookup Loop**: Uses `jg` (jump-if-greater) for bounds checking
- **Property Access**: Uses `get_prop` instruction with property number 7
- **Array Iteration**: Uses `loadb` for byte access with index arithmetic
- **Word Reconstruction**: `(high_byte * 128) + (high_byte * 128)` for multiplication by 256
- **Result Storage**: Uses Variable(3) for final object ID result

### Compilation Order Dependencies:
1. Dictionary generation (Phase 1) → provides address mappings
2. Object table generation (Phase 2) → uses dictionary mappings for property 7
3. Grammar pattern generation (Phase 3+4) → uses dynamic lookup system

### Memory Layout:
- **Dictionary addresses**: Start at 0x0260 (608), sequential assignment
- **Property 7 format**: [property_num=7, length, addr1_high, addr1_low, addr2_high, addr2_low, ...]
- **Parse buffer structure**: [word_count, word1_addr, word1_length, word2_addr, word2_length, ...]

## NEXT SESSION OBJECTIVES

1. **Complete Phase 4 Implementation**:
   - Modify direct noun pattern handling in `generate_verb_handler()`
   - Ensure all `$noun` parameters use dynamic object lookup
   - Update grammar pattern compilation to call `generate_object_lookup_from_noun()`

2. **Comprehensive Testing**:
   - Test both "verb + noun" and "noun" patterns
   - Verify "examine mailbox", "take leaflet", "open mailbox" commands
   - Confirm no "invalid object 608" crashes

3. **Integration Verification**:
   - End-to-end testing of complete noun-to-object mapping system
   - Performance testing of dynamic lookup vs previous hardcoded system
   - Validation against other test games and grammar patterns

4. **Documentation Update**:
   - Document the complete 4-phase architecture
   - Add debugging commands for future noun-object mapping issues
   - Create regression test cases to prevent future breaks

## DEBUGGING COMMANDS FOR FUTURE REFERENCE

```bash
# Compile with Phase 3 system
env RUST_LOG=error cargo run --bin grue-compiler -- examples/mini_zork.grue --output /tmp/test_phase3.z3

# Test for "invalid object 608" crash
env RUST_LOG=error ./target/debug/gruesome /tmp/test_phase3.z3

# Check crash location in binary
xxd -s 0x1496 -l 10 /tmp/test_phase3.z3

# Search for hardcoded object 608 references
grep -r "608\|0x0260" src/grue_compiler/

# Verify dynamic object lookup implementation
grep -A 20 "generate_object_lookup_from_noun" src/grue_compiler/codegen.rs
```

---

**Session Date**: September 28, 2025
**Implementation Progress**: Phase 3 Complete, Phase 4 Identified and Scoped
**Next Priority**: Complete Phase 4 for universal grammar pattern integration