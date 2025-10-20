# Room Description Debug Plan

## Problem Statement

**Issue**: west_of_house crashes with `print_paddr 0x0000` because it has NO Property 7 (description) at all.

**Root Cause**: Room objects skip basic property setup entirely - they only get exit properties (13, 14, 15) but never get Property 7 (description) created from their `room.description` field.

**Evidence**: Room processing code exists (`ROOM_DESC_DEBUG` in codegen_objects.rs:493-503) but never executes.

## Architecture Analysis

### Working Path: Regular Objects (mailbox, leaflet, tree, etc.)
- Get `🔍 PROPERTY_NUMBERS: Object 'X': description=7` logs
- Receive Property 7 (description) correctly
- Process through standard object property setup

### Broken Path: Room Objects (west_of_house, north_of_house, etc.)
- Skip basic property processing entirely
- Only get exit properties (13, 14, 15) added
- Missing the `ROOM_DESC_DEBUG` processing completely

## Critical Questions to Answer

1. **WHO calls the room processing code?** - The `ROOM_DESC_DEBUG` code exists but isn't being invoked
2. **WHERE is the code path split?** - Why do rooms skip basic property setup?
3. **WHEN should room properties be set?** - At what stage in the object generation pipeline?
4. **WHAT triggers the different processing?** - Is it object type, IR structure, or calling code?

## 4-Phase Investigation Plan

### Phase 1: Trace the Control Flow 🔍
**Goal**: Map the exact call chain for object vs room processing

**Actions**:

1. **Entry Point Logging** - Find where object generation starts:
   ```rust
   // In codegen_objects.rs - main generation function
   log::error!("🎯 OBJECT_GEN_START: Processing {} total objects", all_objects.len());
   for (index, obj_data) in all_objects.iter().enumerate() {
       log::error!("🎯 OBJECT_TYPE_CHECK: index={}, name='{}', type_hint={}",
                   index, obj_data.name, "room_or_object");
   }
   ```

2. **Property Setup Call Tracing** - Track what calls property setup:
   ```rust
   // Before ANY property setup code
   log::error!("🎯 PROP_SETUP_ENTRY: Object '{}' entering property setup", obj_name);
   ```

3. **Room-Specific Code Tracing** - Find the missing room processing:
   ```rust
   // Around line 478 where ROOM_LOOP_START should be
   log::error!("🎯 ROOM_LOOP_DEBUG: About to process {} rooms", ir.rooms.len());
   ```

**Test Command**:
```bash
RUST_LOG=error timeout 3 ./target/debug/grue-compiler examples/mini_zork.grue -o /tmp/trace_flow.z3 2>&1 | grep "🎯"
```

### Phase 2: Identify the Missing Call 🕵️
**Goal**: Find exactly WHY the room basic property code isn't being called

**Hypothesis Testing**:

**Hypothesis A**: Room loop never executes
- **Test**: Check if `ir.rooms.len()` is 0 or room iteration fails
- **Evidence**: No `ROOM_LOOP_START` logs appear

**Hypothesis B**: Room loop executes but skips property setup
- **Test**: Add logs inside room loop to see how far it gets
- **Evidence**: Would see `ROOM_PROCESSING` but not `ROOM_DESC_DEBUG`

**Hypothesis C**: Rooms processed in different function entirely
- **Test**: Search codebase for other room property processing
- **Evidence**: Rooms get exit properties, so SOME processing happens

**Investigation Commands**:
```bash
# Find all functions that process rooms
grep -rn "room.*description\|description.*room" src/grue_compiler/
grep -rn "room_properties\|rooms.*properties" src/grue_compiler/
grep -rn "for room in" src/grue_compiler/
```

### Phase 3: Understand the IR Structure 🏗️
**Goal**: Verify room data exists and is accessible

**Data Verification**:
```rust
// Add comprehensive IR room dumping
log::error!("🏗️ IR_ROOM_DUMP: Found {} rooms in IR", ir.rooms.len());
for (i, room) in ir.rooms.iter().enumerate() {
    log::error!("🏗️ IR_ROOM[{}]: name='{}', description.len()={}, desc='{}'",
                i, room.name, room.description.len(),
                if room.description.len() > 50 { &room.description[..50] } else { &room.description });
}
```

**Key Questions**:
- Does `ir.rooms` contain west_of_house with correct description?
- Is the description field populated from AST correctly?
- Are room objects vs room IR entities handled differently?

### Phase 4: Find the Architectural Gap 🔗
**Goal**: Bridge the gap between room IR and room object property generation

**Current Understanding**:
- Rooms exist in IR with descriptions ✅
- Rooms become objects in all_objects vector ✅
- Room objects get exit properties ✅
- Room objects DON'T get basic properties ❌

**Architecture Search**:
```bash
# Find where rooms get converted to objects
grep -rn "IrRoom\|room.*object\|Room.*Object" src/grue_compiler/codegen_objects.rs

# Find property setup decision logic
grep -rn -A5 -B5 "set_string.*description\|description.*set_string" src/grue_compiler/
```

**Critical Code Inspection**: Look for conditional logic that might skip rooms:
```rust
// Look for code like:
if obj.object_type != Room {
    // set basic properties
}
// Or similar filtering that excludes rooms
```

## Expected Outcomes by Phase

### Phase 1 Results:
- **Success**: Clear trace showing exactly when room vs object paths diverge
- **Failure**: Need to add more granular logging

### Phase 2 Results:
- **Success**: Identify the specific missing function call or conditional that skips rooms
- **Failure**: May indicate deeper architectural issue

### Phase 3 Results:
- **Success**: Confirm room data is present and accessible for property setup
- **Failure**: May indicate IR generation issues

### Phase 4 Results:
- **Success**: Find the exact location to add room property setup
- **Failure**: May require architectural refactoring

## Most Likely Scenarios

Based on the evidence, I suspect:

1. **Separate Processing Paths**: Rooms and objects are processed in completely different loops/functions
2. **Incomplete Migration**: Room property setup was moved/refactored but not fully implemented
3. **Conditional Logic Bug**: A condition that should include rooms excludes them
4. **Function Call Missing**: The room property setup function exists but is never called

## Priority Order

1. **IMMEDIATE** - Phase 1 (Control Flow Tracing) - Most likely to reveal the exact issue
2. **HIGH** - Phase 2 (Missing Call Identification) - Will pinpoint the fix location
3. **MEDIUM** - Phase 3 (IR Verification) - Validates data availability
4. **LOW** - Phase 4 (Architecture Search) - Only if previous phases don't reveal the issue

## Success Criteria

Fix is successful when:
1. All rooms get `🏠 ROOM_DESC_DEBUG` logs during compilation
2. west_of_house has Property 7 with correct description string
3. `look_around` function works without crashing
4. Game shows "You are standing in an open field west of a white house..." when starting

## Files to Modify

Primary targets:
- `src/grue_compiler/codegen_objects.rs` - Room property processing
- Potentially `src/grue_compiler/codegen.rs` - Object generation orchestration

## Current Investigation Status

**PHASE 1 READY**: Ready to add control flow tracing to identify the exact divergence point between room and object processing paths.