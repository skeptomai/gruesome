# Mini Zork Z-Machine Memory Layout Analysis

**Date**: August 21, 2025  
**File**: `mini_zork.z3`  
**Status**: Active debugging - memory layout documented, execution flow issue identified

## Complete Memory Layout

### 0x0000-0x003F: Z-Machine Header
- **PC Start**: 0x10e3 (bytes 8-9: `0x10 0xe3`)
- **Static Memory Base**: 0x0040 (bytes 14-15: `0x00 0x40`)
- **High Memory Start**: 0x8000 (bytes 6-7: `0x80 0x00`)
- **Version**: 3 (byte 0: `0x03`)

### 0x0040-0x021F: Object Table
- Object entries with parent/sibling/child links
- Static 150-byte allocation per object
- 13 objects total estimated
- Contains object attribute flags and relationships

### 0x0220-0x0334: Object Table Extensions  
- Additional object data and attributes
- Object property table pointers
- Object short name pointers

### 0x0335-0x0eba: Property Tables (Static Allocation)
- **Start Address**: 0x0335
- **Allocation Strategy**: Static 150-byte blocks per object
- **Objects and Property Table Locations**:
  - 'West of House' (object #1): 0x0335 - properties: [1, 8, 17, 13]
  - 'North of House' (object #2): 0x03a2 - properties: [8, 13, 1, 17] 
  - 'South of House' (object #3): 0x0458 - properties: [17, 1, 8, 13]
  - 'Behind House' (object #4): 0x04b9 - properties: [13, 17, 1, 8]
  - 'Forest Path' (object #5): 0x0563 - properties: [1, 8, 17, 13]
  - 'Up a Tree' (object #6): 0x0621 - properties: [17, 8, 13, 1]
  - 'Forest' (object #7): 0x06b2 - properties: [13, 8, 17, 1]
  - 'Forest Clearing' (object #8): 0x071c - properties: [8, 1, 17, 13]
  - 'small mailbox' (object #9): 0x078e - properties: [2, 13, 14, 8, 1]
  - 'leaflet' (object #10): 0x07bc - properties: [1, 2, 8]
  - 'window' (object #11): 0x07e0 - properties: [1, 8, 13, 2]
  - 'bird\'s nest' (object #12): 0x0801 - properties: [1, 2, 8, 14]
  - 'jewel-encrusted egg' (object #13): 0x0828 - properties: [14, 2, 1, 13, 15, 8]

### 0x0ebb-0x10d5: String Data Section
- **Start Address**: 0x0ebb (from layout phase)
- **String Count**: 37 strings total
- **Layout Completion**: 0x10d6
- All encoded Z-Machine strings for room descriptions, object names, messages

### 0x10d6-0x13b1+: Code Section
- **Main Routine Start**: 0x10d6
- **PC Start Address**: 0x10e3 (entry point for game)
- **Init Block**: Begins around 0x10e3
- **Function Code**: All user-defined functions continue sequentially
- **Address Resolution**: All 53 references successfully resolved
- **Instruction Forms**: Variable, Short, and Long form instructions properly encoded

## Static Allocation Strategy

The compiler uses **static property table allocation** to prevent memory overlap issues:
- Each object reserves 150 bytes for properties regardless of actual usage
- This creates gaps/null bytes between actual property data and next allocation
- **Trade-off**: Memory efficiency vs reliability (prevents property table corruption)

## Current Issue: Execution Flow Problem

### Issue Description
- **Error**: "Invalid Long form opcode 0x00 at address 0127"  
- **Root Cause**: Execution jumps from valid code section (0x10d6+) to data section (0x0127)
- **Data at 0x0127**: All null bytes (part of static allocation gaps)

### Memory Layout Status
✅ **Header**: Correctly configured  
✅ **Object Tables**: Properly allocated with static spacing  
✅ **Property Tables**: Static allocation working, no corruption  
✅ **String Data**: All strings properly encoded and placed  
✅ **Code Section**: Instructions correctly generated and address resolution complete  

❌ **Execution Flow**: Something causes jump to data section instead of staying in code

### Investigation Required
1. **Branch/Jump Instructions**: Check for incorrect target address calculation
2. **Function Calls**: Verify all call addresses resolve to code section  
3. **Stack Operations**: Check for corrupted return addresses
4. **Control Flow**: Trace execution path from PC start (0x10e3) to error (0x0127)

## Z-Machine Architecture Notes

- **Memory Regions**: Dynamic (0x0040-static), Static (static-high), High (high-0xFFFF)
- **Code Placement**: Must be in static or high memory regions
- **Property Access**: Uses packed addresses, requires proper encoding
- **Instruction Encoding**: Variable form 0x18 (NOT) and other opcodes working correctly

## Related Files

- `docs/VARIABLE_FORM_ENCODING_INVESTIGATION.md` - Variable form encoding fixes
- `src/grue_compiler/codegen.rs:3759` - Property table allocation logic  
- `src/instruction.rs:168` - Long form opcode validation (where error triggers)

## Debug Commands

```bash
# Check memory layout
xxd -s 0x10d0 -l 32 mini_zork.z3  # Code section start
xxd -s 0x120 -l 16 mini_zork.z3   # Error location (null bytes)

# Trace execution
RUST_LOG=trace timeout 5s cargo run --bin gruesome mini_zork.z3

# Check compilation layout
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep -E "Layout|Property table|emit.*address"
```