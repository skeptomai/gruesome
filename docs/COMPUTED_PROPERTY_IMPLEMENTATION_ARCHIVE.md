# Computed Property System Implementation Archive

**Date**: November 1, 2025
**Status**: ARCHIVED - Implementation 70% complete, reverted for simplicity
**Branch**: `computed-property-implementation-archive`
**Reason**: Complexity vs. benefit analysis favored conditional print approach

## Executive Summary

This document archives a comprehensive implementation of a computed property system for the Grue Z-Machine compiler that would have allowed dynamic object property expressions evaluated at runtime. The system was 70% complete with Phases 1 & 2 working correctly, but was reverted in favor of a simpler conditional print approach.

**Example of what was being implemented:**
```grue
object mailbox {
    desc: "The small mailbox is " + (mailbox.open ? "open" : "closed") + "."
    //    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //    This computed expression would evaluate at runtime
}
```

## Implementation Architecture

### 3-Phase System Design

#### Phase 1: IR Registration ✅ **COMPLETED**
**Purpose**: Register computed property expressions during IR generation
**Location**: `src/grue_compiler/ir.rs` lines 1784-1881
**Key Components**:
- Modified `generate_object()` to detect computed expressions
- Added `set_computed_function()` method to IrProperties
- Registration with `computed_property_manager`
- Convert `__COMPUTED_` markers to ComputedFunction properties

**Working Code Example**:
```rust
if obj.description.starts_with("__COMPUTED_") {
    let marker_key = obj.description.strip_prefix("__COMPUTED_").unwrap();
    if let Some(computed_info) = self.computed_expressions.get(marker_key) {
        let function_name = ir_program.computed_property_manager.register_computed_property(
            obj_id, &obj.identifier, "desc", computed_info.expression.clone(),
        );
        properties.set_computed_function(
            StandardProperty::LongName as u8,
            function_name, computed_info.expression.clone(), obj_id,
        );
    }
}
```

#### Phase 2: Function Generation ✅ **COMPLETED**
**Purpose**: Generate Z-Machine functions from computed expressions during compilation
**Location**: `src/grue_compiler/codegen.rs` lines 11195-11295
**Key Components**:
- `generate_computed_property_functions()` method
- `convert_computed_function_properties()` method
- Added Step 2b.5 and 2b.6 to compilation pipeline
- Function address mapping system

**Working Debug Output**:
```
[DEBUG] 🔄 Phase 2: Found 2 computed property functions to generate
[DEBUG] 🔄 Phase 2: Generated computed function 'mailbox_desc__computed' at address 0x0000 with ID 10001
[DEBUG] 🔄 Phase 2: Generated computed function 'window_desc__computed' at address 0x0007 with ID 10002
[DEBUG] ✅ Phase 2b: Converted object 1 property 2 to function address 0x0000
```

#### Phase 3: GetProperty Function Calling ❌ **PARTIAL**
**Purpose**: Modify GetProperty instruction to call computed functions when needed
**Location**: `src/grue_compiler/codegen_instructions.rs` lines 575-665
**Status**: Partially implemented but problematic
**Issue**: Complex runtime detection logic vs. compile-time knowledge

**Attempted Approaches**:
1. Heuristic-based detection (address value thresholds) - **Rejected as unreliable**
2. Runtime branching logic (Jg instruction + function calls) - **Complex implementation**
3. Computed property registry in codegen context - **Not completed**

## Technical Challenges Encountered

### 1. Architecture Complexity
- GetProperty instruction generation happens during compilation but needs runtime behavior
- Z-Machine property system expects data values, not function addresses
- Complex integration between IR system and Z-Machine instruction generation

### 2. Function Address Management
- Computed functions generated at low addresses (0x0000, 0x0007)
- Difficulty distinguishing function addresses from other small values
- Property table corruption when storing function addresses directly

### 3. Runtime vs. Compile-time Knowledge
- Need to know at GetProperty generation time which properties are computed
- Current architecture doesn't easily expose computed property registry to instruction generator
- Would require significant refactoring of codegen context

## Code Locations and Changes Made

### Files Modified:

1. **`src/grue_compiler/ir.rs`**
   - Lines 298-307: Added `set_computed_function()` method
   - Lines 1784-1813: Modified object generation for computed properties
   - Lines 1845-1881: Added computed property registration logic

2. **`src/grue_compiler/codegen.rs`**
   - Lines 725-734: Added Step 2b.5 and 2b.6 pipeline calls
   - Lines 11195-11295: Complete computed function generation system
   - Modified method signatures for mutable IR access

3. **`src/grue_compiler/codegen_instructions.rs`**
   - Lines 575-665: Modified GetProperty instruction (partial)
   - Added Phase 3 function calling logic (incomplete)

### Key Data Structures Added:

```rust
// In IrPropertyValue enum
ComputedFunction {
    function_name: String,
    expression: Expr,
    object_id: IrId,
}

// In IrProperties
pub fn set_computed_function(&mut self, prop_num: u8, function_name: String,
                            expression: Expr, object_id: IrId)
```

## Lessons Learned

### What Worked Well:
1. **Phase separation approach**: Clean separation of concerns
2. **IR integration**: Computed property registration integrated cleanly
3. **Function generation**: Z-Machine function creation worked correctly
4. **Debug logging**: Comprehensive logging made debugging effective

### What Was Challenging:
1. **Runtime detection**: Distinguishing computed vs. regular properties at runtime
2. **Z-Machine integration**: Property system not designed for function calls
3. **Architecture complexity**: Multiple systems needed coordination
4. **Maintenance overhead**: Complex system with many edge cases

### Alternative Approaches Considered:
1. **Conditional print statements**: Simple, traditional approach (chosen for revert)
2. **Preprocessor expansion**: Expand computed properties at compile time
3. **Special property markers**: Use special values to mark computed properties
4. **Separate computed property table**: Parallel system to regular properties

## Revert Decision Rationale

### Complexity Analysis:
- **High implementation complexity**: 3-phase system with complex interactions
- **High maintenance overhead**: Many edge cases and integration points
- **Low immediate benefit**: Conditional print achieves same functionality
- **Risk of bugs**: Complex runtime logic prone to issues

### Alternative Benefits:
- **Simplicity**: Traditional examine handlers are well-understood
- **Debuggability**: Conditional print logic is straightforward to debug
- **Proven pattern**: Industry-standard approach for dynamic descriptions
- **Lower risk**: Less surface area for bugs

## Future Implementation Notes

If computed properties are revisited in the future, consider:

### Architectural Improvements:
1. **Compile-time registry**: Add computed property tracking to ZMachineCodeGen struct
2. **Property type system**: Distinguish computed vs. regular properties at type level
3. **Unified property access**: Single system handling both computed and regular properties
4. **Better Z-Machine integration**: Design property system with computed properties in mind

### Implementation Strategy:
1. **Start with simple cases**: Only description properties initially
2. **Build incrementally**: Add complexity gradually with thorough testing
3. **Consider preprocessor**: Expand computed properties during compilation rather than runtime
4. **Prototype first**: Build minimal working version before full implementation

## Related Documentation

- **Original Analysis**: `docs/PHASE_SEPARATION_BUG_ANALYSIS_AND_PLAN.md`
- **Project Status**: `ONGOING_TASKS.md` (archived section)
- **Branch Location**: `computed-property-implementation-archive`
- **Example Code**: `examples/mini_zork.grue` (computed property syntax)

## Conclusion

The computed property system was a well-designed and partially implemented advanced feature that would have provided elegant syntax for dynamic object descriptions. However, the implementation complexity exceeded the immediate benefit, leading to the decision to use the simpler conditional print approach.

The work is preserved for future reference and could be resumed if the core compiler systems are stabilized and the complexity trade-off becomes favorable. The 3-phase architecture and implementation details provide a solid foundation for future attempts.