# Property 28 Crash Reassessment Plan (Oct 20, 2025)

## Current Status

**STACK DISCIPLINE**: ✅ 100% COMPLETE - All Variable(0) operations converted to proper push/pull semantics
**PROPERTY 28 CRASH**: 🚨 PERSISTS - Stack discipline was NOT the root cause

## Key Evidence

### Crash Details
- **Error**: `print_paddr called with invalid packed address 0x0000 at PC 011f9`
- **Trigger**: "look" command during game initialization
- **Location**: Room description processing (likely West of House)
- **Impact**: Game crashes immediately after welcome message

### What Stack Discipline Fixed
- ✅ Eliminated ALL Variable(0) collision scenarios during gameplay
- ✅ Proper LIFO stack semantics for all operations (25+ conversions)
- ✅ Unique temporary globals (200+) for each operation result
- ✅ Verified working via PHASE_C2 debug logs

### What Stack Discipline Did NOT Fix
- ❌ Property 28 crash still occurs during game initialization
- ❌ 0x0000 packed address generation during room description access
- ❌ Root cause is in game initialization, not Variable(0) operations during gameplay

## Fresh Investigation Approach

### Phase 1: Crash Point Analysis
**Goal**: Understand exactly where and why the crash occurs

**Actions**:
1. **PC Analysis**: Identify what instruction at PC 011f9 is trying to print 0x0000
2. **Call Stack**: Trace back from crash point to find what function/handler calls print_paddr
3. **Property Access**: Determine which object and property number is being accessed
4. **IR Analysis**: Review IR for room description logic (`look_around` function)

**Commands**:
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/debug_crash.z3 2>&1 | grep -E "(look_around|print_paddr|property|PC)"
```

### Phase 2: Room Property Investigation
**Goal**: Verify if west_of_house actually has Property 7 (description) in final binary

**Actions**:
1. **Binary Analysis**: Check if west_of_house has Property 7 in compiled game
2. **Object Dump**: Verify object property tables are correctly structured
3. **String Reference**: Confirm description strings exist and have valid packed addresses
4. **Property Generation**: Trace room property creation during compilation

**Commands**:
```bash
./target/debug/gruesome tests/debug_crash.z3 --debug-objects | grep -A 10 "West of House"
```

### Phase 3: Compilation Pipeline Analysis
**Goal**: Find where room descriptions get lost or corrupted during compilation

**Actions**:
1. **Source Analysis**: Verify `examples/mini_zork.grue` contains room descriptions
2. **IR Generation**: Confirm room descriptions become IR strings with valid IDs
3. **String Creation**: Verify description strings are added to string table
4. **Reference Resolution**: Check if string references get properly resolved

**Evidence to Gather**:
- Does `west_of_house` have `desc: "You are standing..."` in source?
- Does IR contain string for room description?
- Does string table contain the description text?
- Are UnresolvedReferences created for room description properties?

### Phase 4: Object vs Room Processing Differential
**Goal**: Understand why rooms might be processed differently than objects

**Actions**:
1. **Code Path Analysis**: Compare object property generation vs room property generation
2. **Processing Order**: Verify rooms get same property setup as objects
3. **Property Assignment**: Check if room descriptions get assigned to Property 7
4. **Debug Logging**: Add targeted logging to room-specific processing code

**Key Questions**:
- Do rooms follow same property generation path as objects?
- Are room descriptions explicitly assigned to Property 7?
- Is there separate room property processing that might skip descriptions?

### Phase 5: Reference Resolution Deep Dive
**Goal**: Verify string reference resolution works correctly for room descriptions

**Actions**:
1. **UnresolvedReference Tracking**: Trace creation and resolution of room description references
2. **Address Translation**: Verify string IDs get correct packed addresses
3. **Memory Layout**: Check if room property tables point to correct string locations
4. **Final Patching**: Ensure reference patching writes correct addresses to property tables

## Investigation Priorities

### Priority 1: Immediate Crash Analysis
- Identify exact instruction and property causing 0x0000 address
- Determine if crash is in west_of_house or different room
- Confirm crash happens on room description access

### Priority 2: Room Description Pipeline
- Trace room description from source → IR → string table → property table
- Verify each step preserves description content and generates correct references
- Compare with working object description pipeline

### Priority 3: Binary Structure Verification
- Confirm west_of_house has Property 7 in final binary
- Verify property points to valid string location
- Check string content matches source description

## Success Criteria

### Investigation Success
1. **Root Cause Identified**: Exact location where room descriptions get lost/corrupted
2. **Pipeline Mapped**: Complete understanding of room description compilation flow
3. **Differential Found**: Clear difference between working and broken code paths

### Fix Success
1. **No Crash**: `echo "look" | ./target/debug/gruesome tests/mini_zork.z3` succeeds
2. **Correct Output**: West of House description displays properly
3. **All Rooms Work**: Navigation and room descriptions work for all rooms
4. **Tests Pass**: All existing tests continue to pass

## Key Insights

### Stack Discipline vs Property 28
- **Stack Discipline**: Fixed Variable(0) collisions during gameplay operations
- **Property 28**: Game initialization issue, not Variable(0) operation issue
- **Conclusion**: These are separate problems requiring different solutions

### Investigation Focus
- Focus on **game initialization** and **room property generation**
- NOT on Variable(0) operations or stack management
- Look for **compilation pipeline** issues, not runtime execution issues

## Next Steps

1. **Phase 1**: Analyze crash point to understand exact failure mode
2. **Create targeted debugging**: Add specific logging for room description processing
3. **Compare pipelines**: Understand why objects work but rooms fail
4. **Fix root cause**: Address the specific compilation issue found

## Files to Investigate

### Primary Targets
- `src/grue_compiler/ir.rs`: Room description IR generation
- `src/grue_compiler/codegen_objects.rs`: Room vs object property generation
- `src/grue_compiler/codegen.rs`: Reference resolution for room descriptions
- `examples/mini_zork.grue`: Source room descriptions

### Secondary Targets
- Room handler generation code
- String table creation for room descriptions
- Property number assignment for rooms vs objects

---

**STATUS**: Ready for Phase 1 crash point analysis