# Property Table & Address Boundary Fix Plan
*Generated: August 24, 2025*  
*Session: Post-IR Variable Mapping Success - Next Phase Issues*

## Status Summary ✅

**MAJOR BREAKTHROUGH COMPLETED**: IR Variable Mapping System
- ✅ **Object 0 errors eliminated** - "Cannot insert object 0" runtime failures resolved
- ✅ **Function parameter mapping working** - Complete parameter-to-slot mapping implemented
- ✅ **Basic functionality restored** - Simple programs (test_01_basic, test_03_function) execute perfectly
- ✅ **Complex programs start execution** - mini_zork shows banner and begins execution

**CURRENT BLOCKING ISSUES**: Higher-level architectural problems
- ❌ **Property table generation**: "Property 2 not found for object 1" (test_06_player_assignment.z3)
- ❌ **Address boundary validation**: "Instruction address 7458 out of bounds" (mini_zork.z3)

---

## Problem Analysis Framework

### Issue 1: Property Table Generation Problems 🔴

**Error Pattern**: `Property 2 not found for object 1`  
**Affected Programs**: test_06_player_assignment.z3 (object property access)  
**Root Cause**: Property table generation or object property mapping failures

**Technical Analysis**:
- Object 1 (likely player object) attempting to access property #2
- Property #2 either not generated in property table OR not associated with object 1
- Property table structure may not conform to Z-Machine specification
- Property number assignment may be inconsistent across compilation phases

### Issue 2: Address Boundary Validation Problems 🔴

**Error Pattern**: `Instruction address XXXX out of bounds`  
**Affected Programs**: mini_zork.z3 (complex execution flows)  
**Root Cause**: Address resolution generating invalid addresses or incorrect boundary validation

**Technical Analysis**:
- Generated instruction addresses exceed story data buffer size
- Address resolution phase may not be patching all placeholders correctly  
- Control flow instructions (jumps, branches, calls) may have invalid targets
- Story data size calculation may be incorrect during boundary validation

---

## Systematic Fix Implementation Plan

### **Phase 1: Property Table System Overhaul** 📋
**Priority**: P0 - Critical for object manipulation  
**Estimated Duration**: 2-3 hours  
**Success Criteria**: Object property access working correctly in test_06_player_assignment.z3

#### Step 1.1: Property Registration Audit 🔍
**Goal**: Ensure all object properties are registered correctly during compilation

**Implementation**:
```bash
# Debug property registration during compilation
RUST_LOG=debug cargo run --bin grue-compiler -- examples/test_06_player_assignment.grue 2>&1 | grep -i "property"

# Audit property registration code paths
grep -rn "property.*register\|register.*property" src/grue_compiler/
```

**Files to Examine**:
- `src/grue_compiler/codegen.rs`: Property number assignment logic
- `src/grue_compiler/ir.rs`: Property collection during IR generation
- Property registration in object generation code

**Validation Criteria**:
- All object properties from Grue source appear in property registry
- Property numbers assigned consistently across compilation phases
- No duplicate or missing property registrations

#### Step 1.2: Property Table Generation Verification 🏗️
**Goal**: Validate Z-Machine property table format and content

**Implementation**:
```rust
// Add to codegen.rs generate_property_tables()
fn generate_property_tables(&mut self) -> Result<(), CompilerError> {
    log::debug!("=== PROPERTY TABLE GENERATION ===");
    log::debug!("Total properties registered: {}", self.property_numbers.len());
    
    for (name, &number) in &self.property_numbers {
        log::debug!("  Property '{}' -> #{}", name, number);
    }
    
    // Validate property table structure
    self.validate_property_table_format()?;
    
    // Generate actual property table data
    // ... existing implementation
}
```

**Validation Criteria**:
- Property table written in correct Z-Machine format
- Property data properly encoded and positioned
- Object property pointers reference correct property table locations

#### Step 1.3: Object-Property Association Fix 🔗
**Goal**: Ensure objects have correct property mappings and can access their properties

**Implementation Areas**:
- **Object property assignment audit**: Check `object_properties` HashMap population
- **Property table pointer validation**: Ensure object entries point to correct property data  
- **Property inheritance verification**: Check property defaults and overrides work correctly

**Files to Modify**:
- Object table generation in `generate_object_table()`
- Property association logic in object processing
- Property lookup validation in runtime operations

**Validation Criteria**:
- Object 1 has property 2 correctly defined and accessible
- Property table pointers from object table entries are valid
- Property values can be read/written correctly

### **Phase 2: Address Boundary System Validation** 📋
**Priority**: P0 - Critical for complex execution  
**Estimated Duration**: 3-4 hours  
**Success Criteria**: mini_zork.z3 executes without address boundary errors

#### Step 2.1: Address Resolution System Audit 🔍
**Goal**: Verify all address placeholders are properly resolved before execution

**Implementation**:
```bash
# Check for unresolved references in compilation output
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep -E "unresolved|placeholder|0xFFFF"

# Audit address resolution completion
grep -rn "resolve.*address\|patch.*address" src/grue_compiler/codegen.rs
```

**Investigation Areas**:
- UnresolvedReference processing completion
- Function call address resolution
- String literal address patching
- Label and branch target resolution

**Validation Criteria**:
- No unresolved references remaining after compilation
- No 0xFFFF placeholder addresses in final bytecode
- All function calls target valid function start addresses

#### Step 2.2: Story Data Boundary Calculation 📏
**Goal**: Ensure story data size calculations are accurate and boundary validation is correct

**Implementation**:
```rust
// Add to codegen.rs
fn validate_story_data_integrity(&self) -> Result<(), CompilerError> {
    log::debug!("=== STORY DATA INTEGRITY CHECK ===");
    log::debug!("Story data size: {} bytes", self.story_data.len());
    log::debug!("Current address: 0x{:04x}", self.current_address);
    log::debug!("Max valid address: 0x{:04x}", self.story_data.len() - 1);
    
    // Check for any addresses that exceed bounds
    if self.current_address > self.story_data.len() {
        return Err(CompilerError::CodeGenError(
            format!("Current address 0x{:04x} exceeds story data size 0x{:04x}", 
                self.current_address, self.story_data.len())
        ));
    }
    
    Ok(())
}
```

**Files to Examine**:
- Story data size management in `ZMachineCodeGen`
- Address boundary validation in interpreter
- Buffer allocation and expansion logic

**Validation Criteria**:
- Story data buffer large enough for all generated content
- Address boundary checks use correct size limits
- No buffer overruns during bytecode generation

#### Step 2.3: Jump Target and Control Flow Validation 🎯
**Goal**: Ensure all branch/jump targets resolve to valid instruction boundaries

**Implementation Areas**:
- **Label resolution audit**: Check all labels resolve to proper addresses  
- **Branch offset calculation verification**: Validate branch instruction offset math
- **Function call target validation**: Ensure function calls target valid function starts
- **Control flow integrity**: Verify jump targets land on instruction boundaries

**Key Validation Points**:
```rust
// Add validation for jump targets
fn validate_jump_target(&self, target_address: usize, context: &str) -> Result<(), CompilerError> {
    if target_address >= self.story_data.len() {
        return Err(CompilerError::CodeGenError(
            format!("{}: Jump target 0x{:04x} out of bounds (max 0x{:04x})", 
                context, target_address, self.story_data.len())
        ));
    }
    
    // Additional validation: ensure target is valid instruction start
    self.validate_instruction_boundary(target_address, context)
}
```

**Validation Criteria**:
- All branch/jump instructions target valid addresses within story data
- Jump targets land on proper instruction boundaries (not mid-instruction)
- Function call addresses point to valid function entry points

### **Phase 3: Comprehensive Debugging Infrastructure** 📋
**Priority**: P1 - Diagnostic enhancement for ongoing development  
**Estimated Duration**: 1-2 hours  
**Outcome**: Rich debugging tools for property and address issues

#### Diagnostic Tools Implementation:

**Property Table Inspector**:
```rust
fn dump_property_table_debug(&self) {
    log::info!("=== PROPERTY TABLE DETAILED DUMP ===");
    log::info!("Property table address: 0x{:04x}", self.property_table_addr);
    
    for (obj_name, &obj_num) in &self.object_numbers {
        log::info!("Object '{}' (#{}):", obj_name, obj_num);
        if let Some(properties) = self.object_properties.get(obj_name) {
            for (prop_name, prop_data) in properties {
                let prop_num = self.property_numbers.get(prop_name).unwrap_or(&0);
                log::info!("  Property '{}' (#{}) = {:?}", prop_name, prop_num, prop_data);
            }
        } else {
            log::info!("  No properties registered");
        }
    }
}
```

**Address Boundary Validator**:
```rust
fn validate_address_bounds(&self, address: usize, operation: &str) -> Result<(), String> {
    if address >= self.story_data.len() {
        Err(format!("{}: Address 0x{:04x} exceeds story data size 0x{:04x}", 
            operation, address, self.story_data.len()))
    } else {
        Ok(())
    }
}
```

---

## Implementation Strategy

### **Recommended Execution Order**:
1. **Phase 1.1** (Property Registration Audit) - Low risk, high diagnostic value
2. **Phase 2.1** (Address Resolution Audit) - Can run in parallel with 1.1  
3. **Phase 1.2** (Property Table Generation) - Builds on audit results
4. **Phase 2.2** (Story Data Boundaries) - Foundation for address validation
5. **Phase 1.3** (Object-Property Association) - Completes property system
6. **Phase 2.3** (Jump Target Validation) - Completes address system  
7. **Phase 3** (Debugging Infrastructure) - Ongoing development support

### **Testing Strategy**:
- **After Phase 1**: test_06_player_assignment.z3 should execute without property errors
- **After Phase 2**: mini_zork.z3 should execute beyond banner without address errors
- **Regression Testing**: Ensure test_01_basic.z3 and test_03_function.z3 continue working
- **Progressive Complexity**: Test simple → complex programs at each phase

### **Success Metrics**:
- ✅ **Phase 1 Complete**: Object property access operations work correctly
- ✅ **Phase 2 Complete**: Complex control flow executes without address errors  
- ✅ **Overall Success**: mini_zork.z3 executes game loop and responds to commands

### **Risk Mitigation**:
- Incremental commits at each step for easy rollback
- Comprehensive logging added before making changes
- Conservative boundary checking during development phase
- Reference to Z-Machine specification for format compliance

---

## Context & Background

**Previous Session Achievements**:
- Complete IR variable mapping system implemented
- Function parameter mapping system working  
- Call/BinaryOp/UnaryOp instruction target registration complete
- Property instruction target registration complete
- "Cannot insert object 0" errors successfully eliminated

**Architecture Status**:
- Core IR→Z-Machine translation working correctly
- Basic program execution restored and verified
- Foundation established for complex program support

**Current Focus**: 
Addressing higher-level Z-Machine specification compliance issues that prevent complex program execution.

This plan provides a systematic approach to achieving full Z-Machine program execution capability.