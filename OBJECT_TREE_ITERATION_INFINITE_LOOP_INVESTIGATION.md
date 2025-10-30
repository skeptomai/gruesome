# Object Tree Iteration Infinite Loop Investigation Plan

## Overview

Investigation into the infinite loop that occurs during "open mailbox" command execution. This issue is **separate** from the previously fixed `get_object_contents` placeholder bug and occurs during object tree traversal in the `list_contents()` function.

## Current Status Analysis

### ✅ **What's Working**
1. **Stack Discipline**: Complete migration to local variables, Z-Machine compliant ✅
2. **Object Lookup**: Property 18 dictionary addresses fully implemented ✅
3. **get_object_contents**: Proper GetObjectChild implementation with branch handling ✅
4. **Basic Object Commands**: "examine mailbox" works perfectly ✅
5. **Navigation**: All movement commands work flawlessly ✅

### ⚠️ **Current Issue: Object Tree Iteration Infinite Loop**

**Trigger**: Commands that call `list_contents(obj)` → `obj.contents()` → object tree traversal
**Affected Commands**:
- "open mailbox" (mailbox contains leaflet)
- "open window" (likely same pattern)
- Any container-opening command that lists contents

**Execution Flow**:
```
handle_open(mailbox) →
obj.open = true →
if obj.container && !obj.empty() →
  print("The " + obj.name + " contains:") →
  list_contents(obj) →
    let contents = obj.contents() →
      [INFINITE LOOP IN OBJECT TREE TRAVERSAL]
```

## Technical Analysis

### **Stack Underflow Pattern**
Based on debug trace analysis:

1. **PC 0x1019**: Jump instruction (offset 5) → PC 0x101c
2. **PC 0x101c**: JE instruction (0 vs 0, always true) → branches to PC 0x1027
3. **PC 0x1027**: Stack underflow occurs - tries to pop from empty stack

**Key Observation**: This appears to be an infinite loop in GetObjectChild/GetObjectSibling sequence where the code expects stack values that don't exist.

### **Root Cause Hypothesis**

**Most Likely**: Bug in Z-Machine object tree traversal opcodes (GetObjectChild/GetObjectSibling)
- GetObjectChild (0x02) should return first child and store to stack
- GetObjectSibling (0x02) should return next sibling and store to stack
- Loop expects these values on stack but they're not being pushed correctly

**Possible Issues**:
1. **Opcode Implementation Bug**: GetObjectChild/GetObjectSibling not storing results correctly
2. **Object Tree Structure**: Parent-child relationships incorrectly established
3. **Branch Handling**: Incorrect branch logic causing infinite loop instead of termination
4. **Stack Management**: Results not being properly pushed/pulled during iteration

## Investigation Strategy

### **Phase 1: Opcode Verification** 🔍
**Objective**: Verify GetObjectChild/GetObjectSibling implementations are correct

**Actions**:
1. Examine `src/grue_compiler/codegen_instructions.rs:1033-1109` (GetObjectChild/GetObjectSibling)
2. Check opcode numbers: GetObjectChild = 0x02, GetObjectSibling = 0x02 (verify this is correct)
3. Verify store_var and branch handling in both opcodes
4. Compare with Z-Machine specification

### **Phase 2: Object Tree Structure Verification** 🏗️
**Objective**: Ensure mailbox→leaflet parent-child relationship is correct

**Actions**:
1. Examine object tree compilation logs for mailbox (Object #10) and leaflet (Object #11)
2. Verify parent-child pointers are correctly set during compilation
3. Check if mailbox.child points to leaflet and leaflet.parent points to mailbox
4. Test with simpler object tree structure (empty container)

### **Phase 3: Execution Trace Analysis** 📊
**Objective**: Understand exactly where the infinite loop occurs

**Actions**:
1. Add detailed logging to GetObjectChild/GetObjectSibling implementations
2. Trace exactly what object IDs are being processed
3. Identify if loop is in child traversal or sibling traversal
4. Determine why termination condition (child=0 or sibling=0) is not met

### **Phase 4: Minimal Reproduction** 🧪
**Objective**: Create isolated test case for debugging

**Actions**:
1. Create minimal test with single container + single child object
2. Test `container.contents()` in isolation (without open command complexity)
3. Verify if issue is specific to mailbox/leaflet or affects all container objects
4. Compare behavior with empty containers vs containers with children

## Expected Findings

### **Likely Root Causes**:
1. **GetObjectSibling Infinite Loop**: Object #11 (leaflet) has incorrect sibling pointer, causing infinite traversal
2. **GetObjectChild Branch Bug**: Incorrect branch handling when no children, causing loop instead of exit
3. **Stack Management Error**: Results not properly stored to stack, causing underflow in subsequent operations

### **Z-Machine Specification Reference**

**GetObjectChild (1OP:2, opcode 0x02)**:
- Input: Object number
- Output: Child object number (stored to Variable 0/stack)
- Branch: When result is 0 (no child), branch

**GetObjectSibling (1OP:3, opcode 0x03)**:
- Input: Object number
- Output: Sibling object number (stored to Variable 0/stack)
- Branch: When result is 0 (no sibling), branch

**CRITICAL**: Verify opcode numbers are correct - GetObjectChild should be 0x02, GetObjectSibling should be 0x03

## Success Criteria

1. ✅ "open mailbox" command completes without infinite loop
2. ✅ Lists contents correctly: "The mailbox contains: leaflet"
3. ✅ No stack underflow errors during object tree traversal
4. ✅ All existing functionality continues to work (regression test)
5. ✅ Other container objects (if any) also work correctly

## Files to Investigate

### **Primary Suspects**:
1. `src/grue_compiler/codegen_instructions.rs:1033-1109` - GetObjectChild/GetObjectSibling implementation
2. `src/grue_compiler/opcodes.rs` - Opcode definitions and numbers
3. `src/opcodes_object.rs` - Object tree opcode runtime implementations
4. `src/grue_compiler/ir.rs:2070-2119` - For-loop object tree iteration logic

### **Object Tree Structure**:
1. Object compilation logs (verify parent-child relationships)
2. `src/grue_compiler/codegen_objects.rs` - Object tree structure generation
3. Mini_zork.grue - Mailbox and leaflet object definitions

## Timeline

**Phase 1**: Opcode verification (~30 minutes)
**Phase 2**: Object tree verification (~20 minutes)
**Phase 3**: Execution trace analysis (~45 minutes)
**Phase 4**: Fix implementation (~30 minutes)

**Total Estimated Time**: ~2 hours

## Risk Assessment

**Low Risk**: This is a contained bug in object tree iteration that doesn't affect core stack discipline or navigation systems.

**High Impact**: Once fixed, will enable all container-based gameplay mechanics (opening containers, listing contents, etc.)

---

**Date**: October 29, 2025
**Status**: Investigation Plan Created
**Previous Work**: get_object_contents placeholder bug completely fixed (October 27, 2025)