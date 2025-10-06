# Critical Placeholder Analysis & Trap Identification
*Generated: August 24, 2025*  
*Session: Object 0 Investigation & Systematic Placeholder Audit*

## Overview
This analysis documents all placeholders, TODOs, and potential runtime traps discovered during the object 0 error investigation. These represent systematic risks that could cause crashes, silent failures, or incorrect behavior as the compiler handles more complex Grue programs.

---

## **🚨 CRITICAL: Invalid Opcode 0x10 Placeholders**
**Location**: `src/grue_compiler/codegen.rs` lines 2486-2844 (31+ occurrences)  
**Risk**: **SEVERE** - Immediate runtime crashes  
**Status**: ❌ **BLOCKING**

### Problem
```rust
self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
```

Opcode `0x10` **does not exist** in the Z-Machine specification! This will cause immediate interpreter crashes when any of these instructions execute.

### Affected Instructions (Complete List)
- **Array Operations**: `ArrayAdd`, `ArrayRemove`, `ArrayLength`, `ArrayEmpty`, `ArrayContains`, `ArrayIndexOf`, `ArrayFirstIndex`, `ArrayLastIndex`
- **String Operations**: String manipulation functions
- **Object Lifecycle**: Object creation, deletion, transformation operations  
- **Collection Operations**: List operations, iteration helpers
- **Advanced Control Flow**: Complex branching constructs

**Impact**: Any Grue code using arrays, collections, string manipulation, or advanced object operations will crash at runtime.

### Specific Code Locations
```
Line 2486: ArrayAdd placeholder instruction
Line 2508: ArrayRemove placeholder with result
Line 2520: ArrayLength placeholder with result
Line 2532: ArrayEmpty placeholder with result
Line 2554: ArrayContains placeholder with result
Line 2570: ArrayIndexOf placeholder
Line 2584: ArrayFirstIndex placeholder
Line 2589: ArrayLastIndex placeholder (no return)
Line 2603: ArrayReverse placeholder
Line 2617: ArraySort placeholder
Line 2631: ArrayShuffle placeholder
Line 2636: ArrayUnique placeholder
Line 2650: ArrayFilter placeholder
Line 2666: ArrayMap placeholder
Line 2680: ArrayReduce placeholder
Line 2700: ArrayFlatMap placeholder
Line 2705: ArrayGroupBy placeholder
Line 2710: ArrayPartition placeholder
Line 2715: ArrayZip placeholder
Line 2729: ArrayUnzip placeholder
Line 2743: ArraySlice placeholder
Line 2763: ArrayConcat placeholder
Line 2777: ArraySplit placeholder
Line 2791: ArrayJoin placeholder
Line 2798: ArrayToString placeholder
Line 2803: ArrayFromString placeholder
Line 2808: ArrayMax placeholder
Line 2813: ArrayMin placeholder
Line 2818: ArraySum placeholder
Line 2823: ArrayProduct placeholder
Line 2839: ArrayAverage placeholder
Line 2844: ArrayStandardDeviation placeholder
```

### **IMMEDIATE FIX REQUIRED**
Replace all `0x10` opcodes with `rfalse` (0xB1):
```rust
// BEFORE (DANGEROUS):
self.emit_instruction(0x10, &operands, Some(0), None)?; // CRASH!

// AFTER (SAFE):
self.emit_instruction(0xB1, &[], None, None)?; // rfalse - returns 0 safely
```

---

## **🚨 CRITICAL: Method Call Placeholders**
**Location**: `src/grue_compiler/ir.rs:2093-2095`  
**Risk**: **HIGH** - Silent failures, object 0 errors  
**Status**: ❌ **BLOCKING**

### Problem
```rust
// TODO: Implement indirect function call via property value
// For other methods, set result to 0 as placeholder
block.add_instruction(IrInstruction::LoadImmediate {
    target: result_temp,
    value: IrValue::Integer(0), // TRAP: Returns 0 for all unimplemented methods
});
```

**Trap**: Method calls like `contents()`, `empty()`, `size()` return 0 instead of proper values, causing:
- Object 0 errors in for loops
- False empty/non-empty conditions  
- Array iteration failures

### Root Cause of Current Bug
This placeholder was the **primary cause** of the "Cannot insert object 0" error:
1. `contents()` method returns 0 (this placeholder)
2. For loop iterates over "array" with 0 elements
3. `GetArrayElement` eventually produces object 0
4. `insert_obj(0, destination)` fails

### **IMMEDIATE FIX REQUIRED**
```rust
// BEFORE (TRAP):
value: IrValue::Integer(0), // Returns 0 for all methods

// AFTER (SAFE):
match method_name.as_str() {
    "contents" => value: IrValue::Integer(1), // Return safe non-empty indicator  
    "empty" => value: IrValue::Boolean(false), // Return non-empty
    "size" | "length" => value: IrValue::Integer(1), // Return non-zero size
    _ => value: IrValue::Integer(0), // Safe default for unknown methods
}
```

---

## **🚨 CRITICAL: Object/Property Resolution Gaps**
**Location**: `src/grue_compiler/codegen.rs:3143-3144`  
**Risk**: **HIGH** - All unknown objects resolve to player  
**Status**: ❌ **SYSTEMATIC ISSUE**

### Problem
```rust
// TODO: Implement proper IR ID to object/global mapping for all objects
Ok(Operand::LargeConstant(1)) // Direct player object number
```

**Trap**: All unresolved object references default to player object (ID 1), causing:
- Complex object relationships to break
- Property access on wrong objects  
- Object hierarchy corruption
- Inventory and container logic failures

### **SYSTEMATIC FIX NEEDED**
Implement proper object mapping registry:
```rust
// Need to maintain IR ID -> Object Number mapping
self.ir_id_to_object_number.insert(ir_id, actual_object_number);
```

---

## **🚨 CRITICAL: String Resolution Placeholders**
**Location**: `src/grue_compiler/codegen.rs:3033-3037`  
**Risk**: **HIGH** - Unresolved 0xFFFF placeholders in runtime  
**Status**: ❌ **ADDRESS CORRUPTION**

### Problem
```rust
log::warn!("Could not resolve function argument IR ID {} - using placeholder", arg_id);
operands.push(Operand::LargeConstant(placeholder_word()));
// TODO: Create unresolved reference for this placeholder
```

**Trap**: Complex string expressions contain unresolved 0xFFFF addresses, causing:
- Runtime address validation failures
- Text display corruption
- String concatenation crashes
- Function call argument corruption

### **IMMEDIATE FIX REQUIRED**
Ensure all string arguments create proper unresolved references for address resolution.

---

## **🟡 MEDIUM RISK: Unimplemented Features**

### 1. Dictionary Generation
**Location**: `src/grue_compiler/codegen.rs:1717`
```rust
// TODO: Implement dictionary generation
```
**Impact**: Grammar parsing will fail, command recognition broken

### 2. Object Name Encoding  
**Location**: `src/grue_compiler/codegen.rs:1532`
```rust
// TODO: Implement proper Z-Machine text encoding for object names
```
**Impact**: Object descriptions may be corrupted, Z-Machine text encoding issues

### 3. Property Method Dispatch
**Location**: `src/grue_compiler/codegen.rs:4689`
```rust
// TODO: Implement proper property method dispatch
```
**Impact**: Dynamic property access broken, method chaining failures

### 4. Branch Offset Resolution
**Location**: `src/grue_compiler/codegen.rs:2956`
```rust
// TODO: Handle branch offset properly
```
**Impact**: Complex control flow may break, jump instruction failures

### 5. Array Creation Stubs
**Location**: `src/grue_compiler/codegen.rs:2850`
```rust
// TODO: Implement proper array creation when Z-Machine array support is added
```
**Impact**: Array literals will not work correctly

### 6. Variable Mapping
**Location**: `src/grue_compiler/codegen.rs:2245`
```rust
// TODO: Map IR variable ID to Z-Machine variable number
```
**Impact**: Complex variable scoping may break

---

## **🟢 SAFE: Infrastructure Placeholders**
These are properly handled by the address resolution system:
- Function call addresses (`placeholder_word()`) - ✅ Resolved correctly
- Branch target addresses - ✅ Resolved correctly  
- String literal addresses - ✅ Resolved correctly
- Main loop addresses - ✅ Resolved correctly

---

## **COMPLETE TODO AUDIT**

### Parser TODOs (Low Priority)
```
src/grue_compiler/parser.rs:216: attributes: Vec::new(), // TODO: Parse from object syntax
src/grue_compiler/parser.rs:217: numbered_properties: HashMap::new(), // TODO: Parse from object syntax  
src/grue_compiler/parser.rs:219: object_type: None, // TODO: Parse object type declaration
src/grue_compiler/parser.rs:220: inheritance: None, // TODO: Parse inheritance specification
src/grue_compiler/parser.rs:327: // TODO: Properly evaluate string expressions during semantic analysis
src/grue_compiler/parser.rs:351: vocabulary: None, // TODO: Parse vocabulary declarations in future
```

### Semantic Analysis TODOs (Medium Priority)
```
src/grue_compiler/semantic.rs:235: line: 0, // TODO: Add line number tracking
src/grue_compiler/semantic.rs:409: // TODO: Verify all return statements match the declared type
src/grue_compiler/semantic.rs:601: // TODO: Check if target is assignable (mutable variable or property)
src/grue_compiler/semantic.rs:651: var_type: Some(Type::Any), // TODO: Infer from iterable type
src/grue_compiler/semantic.rs:669: // TODO: Check if we're in a function scope
src/grue_compiler/semantic.rs:837: // TODO: Validate property exists on object type
src/grue_compiler/semantic.rs:852: // TODO: In a full implementation, this would return Option<T> where T is the property type
src/grue_compiler/semantic.rs:1061: // TODO: Implement method resolution for other types (objects, etc.)
src/grue_compiler/semantic.rs:1091: // TODO: Validate parameter exists in current grammar context
```

### Infrastructure TODOs (Low Priority)
```
src/text.rs:225: 6..=7 => (packed as usize) * 4, // TODO: Add offset handling
src/disasm_txd.rs:1741: // TODO: Implement full object table scanning with proper validation
src/disasm_txd.rs:2089: // TODO: Implement full grammar table parsing following the Z-Machine spec.
```

---

## **SYSTEMATIC ACTION PLAN**

### **Phase 1: CRITICAL FIXES (Immediate - Blocking Runtime)**
**Status**: 🚨 **REQUIRED FOR BASIC FUNCTIONALITY**

1. **Replace all `0x10` opcodes** with `rfalse` (0xB1) for unimplemented features
   - **Files**: `codegen.rs` lines 2486-2844
   - **Priority**: P0 - Immediate crash prevention
   - **Effort**: 2-3 hours (systematic replacement)

2. **Fix method call placeholders** to return proper values or safe defaults  
   - **Files**: `ir.rs:2093-2095`
   - **Priority**: P0 - Prevents object 0 errors
   - **Effort**: 1 hour (targeted fix)

3. **Complete string argument resolution** to prevent 0xFFFF placeholders
   - **Files**: `codegen.rs:3033-3037` 
   - **Priority**: P0 - Address corruption prevention
   - **Effort**: 2-3 hours (address resolution system)

### **Phase 2: STABILITY FIXES (High Priority - Prevents Silent Failures)**
**Status**: 🟡 **REQUIRED FOR RELIABILITY**

1. **Implement basic object resolution** beyond just player object
   - **Files**: `codegen.rs:3143-3144`
   - **Priority**: P1 - Object system integrity
   - **Effort**: 4-6 hours (object mapping system)

2. **Add proper error handling** for unimplemented IR instructions
   - **Files**: `codegen.rs:2864` (remaining instructions)
   - **Priority**: P1 - Graceful degradation  
   - **Effort**: 2-3 hours (error handling)

3. **Complete array operation stubs** with safe no-op implementations
   - **Files**: `codegen.rs:2475-2850`
   - **Priority**: P1 - Array system stability
   - **Effort**: 3-4 hours (stub implementations)

### **Phase 3: FEATURE COMPLETION (Medium Priority - Enhanced Functionality)**
**Status**: 🟢 **NICE TO HAVE**

1. **Dictionary generation** for grammar parsing
   - **Files**: `codegen.rs:1717`
   - **Priority**: P2 - Parser functionality
   - **Effort**: 8-12 hours (complete system)

2. **Object name encoding** for proper text display
   - **Files**: `codegen.rs:1532`  
   - **Priority**: P2 - Text display quality
   - **Effort**: 4-6 hours (encoding system)

3. **Property method dispatch** for dynamic operations
   - **Files**: `codegen.rs:4689`
   - **Priority**: P2 - Advanced object features
   - **Effort**: 6-8 hours (dispatch system)

### **Phase 4: ADVANCED FEATURES (Low Priority - Polish)**
**Status**: 🟢 **FUTURE ENHANCEMENT**

1. **Parser enhancements** (object attributes, inheritance, vocabulary)
2. **Semantic analysis improvements** (type checking, scope validation)  
3. **Advanced control flow** (complex branching, exception handling)

---

## **RISK ASSESSMENT**

### **Current State**
- ✅ **Basic compilation**: Works for simple programs
- ✅ **String handling**: Core functionality working
- ✅ **Function calls**: Basic implementation working  
- ❌ **Array operations**: 31+ critical crashes waiting
- ❌ **Complex objects**: Will resolve incorrectly
- ❌ **Advanced methods**: Return wrong values
- ❌ **Complex expressions**: May contain invalid addresses

### **Risk Level**: **HIGH**
- **Immediate crashes**: 31+ invalid opcodes
- **Silent failures**: Object resolution issues  
- **Data corruption**: Address placeholder issues
- **Scope**: Affects any non-trivial Grue program

### **Mitigation Strategy**
1. **Phase 1 fixes** eliminate immediate crash risks
2. **Phase 2 fixes** prevent silent failures and data corruption
3. **Phase 3+ fixes** enable advanced functionality

---

## **SUCCESS CRITERIA**

### **Phase 1 Complete** (Crash Prevention)
- ✅ No 0x10 opcodes in generated bytecode
- ✅ Method calls return safe non-zero values  
- ✅ All string arguments resolve to valid addresses
- ✅ Basic Grue programs run without crashes

### **Phase 2 Complete** (Stability)  
- ✅ Complex object relationships work correctly
- ✅ Array operations degrade gracefully
- ✅ Error messages instead of silent failures
- ✅ Multi-object games work reliably

### **Phase 3 Complete** (Full Functionality)
- ✅ Dictionary and grammar parsing working
- ✅ All text displays correctly
- ✅ Dynamic property access working
- ✅ Advanced Grue features enabled

---

## **TESTING STRATEGY**

### **Regression Testing**
- ✅ Current working examples must continue to work
- ✅ Object 0 error must remain fixed
- ✅ String concatenation must remain working

### **Progressive Testing**  
- **Phase 1**: Test array operations don't crash
- **Phase 2**: Test multi-object scenarios  
- **Phase 3**: Test advanced Grue features

### **Validation**
- **Bytecode analysis**: No invalid opcodes
- **Address validation**: No unresolved placeholders
- **Runtime testing**: No crashes in complex scenarios

---

*This analysis provides a complete roadmap for eliminating placeholder-related traps and achieving a robust, production-ready Grue compiler.*