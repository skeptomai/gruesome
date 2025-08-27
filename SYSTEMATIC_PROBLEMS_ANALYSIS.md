# Systematic Problems Analysis - Grue Compiler Z-Machine Generation

**Date**: August 27, 2025  
**Session Context**: Branch generation debugging revealed cascade of deeper instruction generation problems

## CONFIRMED PROBLEMS (In Priority Order)

### **PROBLEM #1: Wrong Instruction for Variable Storage** 🔴 CRITICAL
**Issue**: Using 2OP `store` (0x0D) with 1OP calling convention
- **Current Code**: `emit_instruction(0x0D, &[LargeConstant(value)], Some(target_var), None)`
- **Z-Machine Spec**: `store` is **2OP:13** requiring 2 operands: `(variable) value`
- **Correct Format**: Should be `emit_instruction(0x0D, &[Variable(target_var), LargeConstant(value)], None, None)`
- **Impact**: All local variable storage operations generate malformed instructions

### **PROBLEM #2: Instruction Form Determination Broken** 🔴 CRITICAL  
**Issue**: Form determination converts opcodes incorrectly based on operand count
- **Bug**: `0x0D` + 1 operand → "Short form" → `0x80 | 0x0D` = `0x8D` (print_paddr!)
- **Result**: Variable storage instructions become print instructions
- **Root Cause**: Form determination logic doesn't respect instruction specification
- **Impact**: Any instruction with wrong operand count gets corrupted

### **PROBLEM #3: Local Variable Operations Completely Broken** 🔴 CRITICAL
**Issue**: StoreVar/LoadVar IR instructions generate invalid Z-Machine code
- **Evidence**: Working examples avoid local variables (`let x = 1`) entirely
- **Working**: Property assignments (`player.location = room`) work fine
- **Failing**: Conditional statements with local variables crash
- **Impact**: All conditional logic and local variable usage broken

### **PROBLEM #4: Stack Underflow in Complex Examples** 🟡 SECONDARY
**Issue**: Conditional logic generates unbalanced stack operations
- **Symptoms**: "Stack is empty" errors in `control_flow_simple`, `property_test`, `mini_zork`
- **Evidence**: Programs start execution, show output, then crash
- **Cause**: Likely related to malformed instructions from Problems #1-3
- **Impact**: No complex control flow works

### **PROBLEM #5: Invalid Instruction at Runtime** 🟡 SYMPTOM
**Issue**: "Invalid Long form opcode 0x00" at address 0x038a
- **Root Cause**: Malformed instructions from Problems #1-2 create invalid bytecode
- **Evidence**: PC lands on 0x00 byte, tries to decode as instruction  
- **Impact**: Simple conditional examples crash during execution

## WHAT'S ACTUALLY WORKING ✅

- **Basic print statements**: `print("text")` works perfectly
- **Property assignments**: `player.location = room` works perfectly
- **Simple sequential execution**: Linear code without variables works
- **Function calls**: Basic function invocation works
- **Room/object definitions**: Object system generates correctly

## WHAT'S BROKEN ❌

- **Local variable declarations**: `let x = 1` generates invalid instructions
- **Local variable usage**: `if x == 1` fails due to variable loading issues  
- **Conditional statements**: All `if/else` logic broken
- **Complex control flow**: Loops, nested conditions all fail
- **Stack management**: Complex examples have stack balance issues

## SYSTEMATIC APPROACH TO FIXES

### **Phase 1: Core Instruction Generation (CRITICAL)**
**Goal**: Fix fundamental instruction encoding without breaking working parts

1. **Fix Variable Storage Instruction** (Problem #1)
   - Research correct Z-Machine instruction for constant→variable storage
   - Update StoreVar IR instruction generation  
   - Test with simple `let x = 1` example
   - **Success Criteria**: Local variable declaration compiles and runs

2. **Fix Instruction Form Determination** (Problem #2)
   - Audit `determine_instruction_form()` logic
   - Ensure opcode preservation regardless of operand count mismatch
   - Add validation that generated opcodes match Z-Machine spec
   - **Success Criteria**: No more opcode corruption (0x0D→0x8D)

3. **Fix Local Variable Loading** (Problem #3)
   - Update LoadVar IR instruction to use correct Z-Machine instruction
   - Test with simple `if x == 1` conditional
   - **Success Criteria**: Variable access works in conditionals

### **Phase 2: Control Flow Validation (SECONDARY)**
**Goal**: Fix complex control flow and stack management

4. **Fix Stack Balance in Conditionals** (Problem #4)  
   - Audit conditional branch generation for stack operations
   - Fix any stack push/pop imbalances
   - Test with `control_flow_simple` example
   - **Success Criteria**: Complex conditionals execute without stack errors

5. **Validate Instruction Stream Integrity** (Problem #5)
   - Add bytecode validation after generation
   - Ensure no 0x00 bytes in instruction stream
   - **Success Criteria**: Clean execution of all test examples

### **Phase 3: Comprehensive Testing**
**Goal**: Ensure fixes don't break working functionality

6. **Regression Testing**
   - Test all basic examples still work (test_01_basic through test_06)
   - Test incremental complexity: variables → conditionals → complex flow
   - **Success Criteria**: All examples work, no regressions

## INVESTIGATION METHODOLOGY

**For each problem**:
1. **Isolate**: Create minimal reproduction case
2. **Verify**: Check Z-Machine specification for correct approach  
3. **Fix**: Implement correct instruction generation
4. **Test**: Validate fix with specific test case
5. **Regression**: Ensure no working functionality broken

**Documentation Standards**:
- Record Z-Machine spec references for all instruction choices
- Log before/after bytecode comparisons  
- Track which examples work at each stage

## RISK MITIGATION

**Critical Risks**:
- Breaking working property assignment system
- Cascading failures from instruction format changes
- Stack/memory corruption from incorrect operand handling

**Mitigation Strategy**:
- Fix one problem at a time with validation
- Maintain separate test categories (working vs. broken)  
- Use git commits between each fix for rollback capability

---

**Next Session**: Start with Problem #1 - Research correct Z-Machine instruction for constant-to-variable storage