# HISTORICAL BUGS - RESOLVED ✅ (October 27, 2025)

This file contains documentation of major bugs that have been resolved and are no longer active issues.

---

# IR VARIABLE MAPPING BUG: RESOLVED ✅ (October 27, 2025)

## ✅ RESOLUTION VERIFIED: Function Return Values Working Correctly

**VERIFICATION DATE**: October 27, 2025
**STATUS**: Bug no longer active - navigation system demonstrates proper IR ID resolution ✅

### **Evidence of Resolution**:

1. **Navigation System Working Perfectly** ✅
   - Players correctly move between rooms (West of House → North of House → Forest Path)
   - Room descriptions display accurately after movement
   - Exit system properly handles blocked and valid exits

2. **Property Access After Movement Working** ✅
   - `player.location.desc` correctly retrieves room descriptions after movement
   - Room descriptions show correct content for each location
   - No garbled output or property access failures

3. **Function Return Values Resolving Correctly** ✅
   - `get_exit()` function returns proper values (not falling back to IR ID as literal constant)
   - Exit destinations correctly point to target rooms
   - No evidence of "IR ID 11 as literal constant" fallback behavior

4. **Object Relationships Intact** ✅
   - Player location correctly updates during movement
   - No "insert_obj: obj=1, dest=11" errors when moving to room 3
   - Object tree navigation working properly

### **Resolution Mechanism**:
The bug was resolved by the systematic fixes applied to:
1. exit_is_blocked builtin (replaced inc_chk with proper comparison)
2. Store instruction form selection
3. get_prop_len V3 two-byte format support

---

## ORIGINAL BUG DOCUMENTATION (HISTORICAL)

### **ACTIVE ISSUE**: IR Variable Mapping Bug 🔧

## Problem Analysis

### **Root Cause**: IR ID to Z-Machine Variable Resolution Failure

**Symptom**: After movement, `player.location` resolves to object 11 instead of object 3 (end_room)
**Evidence**: `insert_obj: obj=1, dest=11` should be `dest=3`
**Impact**: Room descriptions fail after movement (accessing nonexistent object 11)

### **Technical Analysis**

**The Bug Chain**:
1. **IR Generation**: `get_exit()` call creates target IR ID 11 for return value ✅
2. **Compilation**: Exit data correctly stores object 3 in property table ✅
3. **Runtime**: `get_exit()` function executes and should return object 3 ✅
4. **Variable Resolution**: IR ID 11 cannot be resolved to Z-Machine variable ❌
5. **Fallback**: System uses IR ID 11 as literal constant value 11 ❌
6. **Result**: Player moves to object 11 instead of object 3 ❌

**Debug Evidence**:
```
[DEBUG] IR INSTRUCTION: Call creates target IR ID 11
[DEBUG] resolve_ir_id_to_operand: Unknown IR ID 11 - no mapping found
[DEBUG] Using IR ID 11 as literal constant
[DEBUG] insert_obj: obj=1, dest=11 at PC 00865
```

### **Assessment and Fix Plan**

#### **Phase 1: Diagnostic Analysis** (1-2 hours)

**Objective**: Understand the variable mapping failure mechanism

**Tasks**:
1. **Map IR ID Lifecycle**:
   - Trace IR ID 11 from creation in `handle_go()` function
   - Identify where IR-to-variable mapping should occur
   - Find why mapping fails for function return values

2. **Compare Working vs Broken Cases**:
   - Analyze IR IDs that resolve correctly (e.g., parameters, locals)
   - Identify what makes IR ID 11 different from working IDs
   - Document the mapping table state during compilation

3. **Function Return Value Architecture**:
   - Understand how function return values should be mapped
   - Check if issue affects other function calls or just `get_exit()`
   - Verify Z-Machine calling convention implementation

#### **Phase 2: Targeted Fix** (2-3 hours)

**Approach A: Fix IR ID Resolution**
- Add IR ID 11 to appropriate mapping table during function call generation
- Ensure `get_exit()` return value gets proper Z-Machine variable assignment
- Test that resolved variable contains correct object number

**Approach B: Alternative Return Value Handling**
- Modify function call mechanism to use stack-based return values
- Bypass IR ID resolution for function returns
- Use direct Z-Machine variable access for `get_exit()` results

**Approach C: Fallback Mechanism Fix**
- Improve literal constant fallback to detect when it's inappropriate
- Add warnings when IR IDs default to literal values
- Implement better error handling for missing mappings

#### **Phase 3: Verification and Testing** (1 hour)

**Test Cases**:
1. **Before Movement**: Verify room descriptions still work in start_room
2. **After Movement**: Verify room descriptions work in end_room
3. **Multiple Movements**: Test back-and-forth navigation
4. **Edge Cases**: Test blocked exits, multiple exits per room

**Expected Results**:
```bash
> test
Testing room properties:
Room desc: A simple test room.
Room name: Test Room

> north
You moved.

> test
Testing room properties:
Room desc: The destination room.
Room name: End Room
```

### **Success Criteria**

- ✅ Property access works correctly both before AND after movement
- ✅ `player.location` resolves to correct object after navigation
- ✅ No regressions in existing navigation functionality
- ✅ Room descriptions display properly in all rooms

### **Risk Assessment**

**Low Risk**: Targeted fix to specific IR ID resolution
**Medium Risk**: Changes to function return value architecture
**High Risk**: Major modifications to variable mapping system

**Mitigation**: Start with Phase 1 diagnostic analysis to choose lowest-risk approach

### **Technical Debt: HOTFIX vs Proper Registration**

**Current Implementation**: HOTFIX approach in `codegen.rs:2993-2998`
- **Problem**: Reactive registration during call translation instead of proactive registration
- **Issues**:
  - Late registration (only when function ID 277 is called)
  - Inconsistent with other builtins (which register during compiler initialization)
  - Recursive `translate_call()` pattern is unusual
  - Only registers name mapping, doesn't create actual Z-Machine function

**Proper Fix Should Be**:
1. **In semantic.rs**: Add get_exit to standard builtin registration during `register_builtin_functions()`
2. **In codegen.rs**: Add get_exit to function creation phase (call `create_builtin_get_exit()`)
3. **Remove HOTFIX**: Delete reactive registration code in `translate_call()`

**Impact**: HOTFIX works but creates technical debt. Proper solution would integrate get_exit into standard builtin registration pipeline.

---

**FINAL STATUS**: This bug has been RESOLVED through systematic fixes to the exit and property systems. Navigation and property access now work correctly throughout the game. ✅