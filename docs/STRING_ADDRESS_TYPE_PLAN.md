# StringAddress Type System Implementation Plan

## Executive Summary

**OBJECTIVE**: Implement a full `StringAddress` type system to elegantly handle string addresses returned by builtins like `exit_get_message`, making `println()` automatically work with both strings and string addresses.

**CURRENT STATE**: Working `print_message()` function provides explicit solution
**GOAL STATE**: `println(exit.message)` automatically works, knowing that `exit.message` returns a string address

**STRATEGY**: Feature branch implementation with full type system enhancement, maintaining backward compatibility

## Git Branching Strategy

### Branch Creation
```bash
# Create feature branch from current clean main
git checkout -b feature/string-address-type
git push -u origin feature/string-address-type
```

### Development Flow
1. **Phase commits** on feature branch with detailed commit messages
2. **Regular pushes** to remote feature branch for backup
3. **Integration testing** against main periodically
4. **Final merge** via pull request with comprehensive testing

### Rollback Safety
- **Main branch** remains untouched with working `print_message()` solution
- **Feature branch** can be abandoned if issues arise
- **Cherry-pick** individual commits if partial solution needed

## Phase 1: Type System Foundation

### 1.1 Extend Type Enum
**File**: `src/grue_compiler/ast.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Any, // For unknown or inferred types
    Bool,
    Int,
    String,
    StringAddress,  // NEW: Packed string addresses from builtins
    Room,
    Object,
    Array(Box<Type>),
}
```

**Implementation Details**:
- Add `StringAddress` variant to Type enum
- Ensure `Clone`, `PartialEq`, `Debug` derive still work
- Update any match statements that need to handle new variant

### 1.2 Update Parser Support
**File**: `src/grue_compiler/parser.rs`

```rust
// In parse_type_annotation() around line 549
match type_name.as_str() {
    "bool" => Ok(Type::Bool),
    "int" => Ok(Type::Int),
    "string" => Ok(Type::String),
    "string_address" => Ok(Type::StringAddress),  // NEW
    "room" => Ok(Type::Room),
    "object" => Ok(Type::Object),
    _ => Err(CompilerError::ParseError(
        format!("Unknown type: {}", type_name),
        self.current_token().position,
    )),
}
```

**Note**: `string_address` type annotation will mainly be used internally by builtins

### 1.3 Type Compatibility Rules
**File**: `src/grue_compiler/semantic.rs` (add new function)

```rust
impl SemanticAnalyzer {
    /// Check if two types are compatible for assignment/comparison
    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            // Exact matches
            (a, b) if a == b => true,

            // Any type accepts anything
            (Type::Any, _) | (_, Type::Any) => true,

            // StringAddress can be used where String is expected (for println)
            (Type::String, Type::StringAddress) => true,

            // Int and StringAddress are NOT compatible (different semantics)
            (Type::Int, Type::StringAddress) | (Type::StringAddress, Type::Int) => false,

            // Arrays with compatible element types
            (Type::Array(a), Type::Array(b)) => self.types_compatible(a, b),

            // Default: types must match exactly
            _ => false,
        }
    }
}
```

## Phase 2: Semantic Analysis Enhancement

### 2.1 Update Builtin Function Signatures
**File**: `src/grue_compiler/semantic.rs`

```rust
fn add_builtin_functions(&mut self) {
    let builtins = [
        // Print functions - enhanced to handle both String and StringAddress
        ("print", vec![Type::String], None),
        ("print_num", vec![Type::Int], None),
        ("print_ret", vec![Type::String], None),
        ("println", vec![Type::String], None),  // Will accept StringAddress via compatibility

        // String address returning builtins
        ("exit_get_message", vec![Type::Int], Some(Type::StringAddress)),  // CHANGED
        ("exit_get_destination", vec![Type::Int], Some(Type::Int)),        // Unchanged
        ("exit_is_blocked", vec![Type::Int], Some(Type::Bool)),            // Unchanged

        // Other potential string address builtins
        ("get_prop", vec![Type::Any, Type::Int], Some(Type::StringAddress)), // If string prop
        ("object_name", vec![Type::Any], Some(Type::StringAddress)),         // NEW potential

        // ... rest of existing builtins unchanged
    ];

    // Registration logic remains the same
    for (name, params, return_type) in builtins {
        // ... existing registration code
    }
}
```

### 2.2 Enhanced Function Call Type Checking
**File**: `src/grue_compiler/semantic.rs`

```rust
// In visit_function_call() method, enhance type compatibility checking
fn visit_function_call(&mut self, call: &FunctionCall) -> Type {
    // ... existing lookup logic ...

    if let Some(symbol) = self.current_scope.get_symbol(&call.name) {
        if let SymbolType::Function { params, return_type } = &symbol.symbol_type {
            // Enhanced argument type checking
            for (i, (param_type, arg)) in params.iter().zip(&call.arguments).enumerate() {
                let arg_type = self.visit_expression(arg);

                // Use enhanced compatibility checking
                if !self.types_compatible(param_type, &arg_type) {
                    self.report_error(SemanticError::TypeMismatch {
                        expected: param_type.clone(),
                        actual: arg_type,
                        position: arg.position(),
                    });
                }
            }

            return return_type.clone().unwrap_or(Type::Any);
        }
    }

    // ... rest of function
}
```

### 2.3 Property Access Type Resolution
**File**: `src/grue_compiler/semantic.rs`

```rust
// In visit_property_access() method
fn visit_property_access(&mut self, prop_access: &PropertyAccess) -> Type {
    let object_type = self.visit_expression(&prop_access.object);

    // Enhanced property type resolution
    match prop_access.property.as_str() {
        "message" => {
            // exit.message returns StringAddress
            Type::StringAddress
        }
        "destination" => Type::Int,  // Room number
        "blocked" => Type::Bool,
        "name" => Type::StringAddress,  // Object names are string addresses
        "desc" => Type::StringAddress,  // Descriptions are string addresses
        _ => Type::Any,  // Unknown properties default to Any
    }
}
```

## Phase 3: IR Generation Enhancement

### 3.1 Type Information Flow to IR
**File**: `src/grue_compiler/ir.rs`

```rust
// Enhance IR instructions to carry type hints
#[derive(Debug, Clone)]
pub struct IrInstruction {
    // ... existing instruction variants ...
}

// Add type tracking to IR generation context
impl IrGenerator {
    // Track expression result types during IR generation
    expression_types: HashMap<IrId, Type>,

    /// Record the type of an IR result
    fn record_expression_type(&mut self, ir_id: IrId, type_info: Type) {
        self.expression_types.insert(ir_id, type_info);
        log::debug!("IR_TYPE: IR ID {} has type {:?}", ir_id, type_info);
    }

    /// Get the type of an IR expression
    fn get_expression_type(&self, ir_id: IrId) -> Option<&Type> {
        self.expression_types.get(&ir_id)
    }
}
```

### 3.2 Enhanced Builtin Function Call IR Generation
**File**: `src/grue_compiler/ir.rs`

```rust
// In generate_builtin_function_call()
fn generate_builtin_function_call(&mut self, name: &str, arg_temps: &[IrId], block: &mut IrBlock) -> Result<IrId, CompilerError> {
    let result_id = self.next_id();

    // Determine return type based on builtin function
    let return_type = match name {
        "exit_get_message" => Type::StringAddress,
        "exit_get_destination" => Type::Int,
        "exit_is_blocked" => Type::Bool,
        "get_prop" => Type::StringAddress, // Assume string properties for now
        "object_name" => Type::StringAddress,
        _ => Type::Any,
    };

    // Record the return type
    self.record_expression_type(result_id, return_type);

    // Generate the call instruction
    // ... existing call generation logic ...

    Ok(result_id)
}
```

### 3.3 Property Access IR Enhancement
**File**: `src/grue_compiler/ir.rs`

```rust
// In generate_property_access()
fn generate_property_access(&mut self, prop: &PropertyAccess, block: &mut IrBlock, context: ValueContext) -> Result<IrId, CompilerError> {
    // ... existing property access logic ...

    // Determine property type and record it
    let property_type = match prop.property.as_str() {
        "message" => Type::StringAddress,
        "destination" => Type::Int,
        "blocked" => Type::Bool,
        "name" => Type::StringAddress,
        "desc" => Type::StringAddress,
        _ => Type::Any,
    };

    let result_id = self.next_id();
    self.record_expression_type(result_id, property_type);

    // ... rest of property access generation ...

    Ok(result_id)
}
```

## Phase 4: Code Generation Enhancement

### 4.1 Type-Aware println Implementation
**File**: `src/grue_compiler/codegen_builtins.rs`

```rust
/// Enhanced println that automatically handles String vs StringAddress
pub fn generate_println_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
    if args.len() != 1 {
        return Err(CompilerError::CodeGenError(format!(
            "println expects 1 argument, got {}",
            args.len()
        )));
    }

    let arg_id = args[0];

    // Determine argument type from IR type information
    let arg_type = self.ir_type_info.get(&arg_id).unwrap_or(&Type::Any);

    log::debug!("println: arg IR ID {} has type {:?}", arg_id, arg_type);

    match arg_type {
        Type::StringAddress => {
            // Handle string address - use print_paddr
            log::debug!("println: treating IR ID {} as string address", arg_id);
            self.generate_print_string_address(arg_id)
        }
        Type::String => {
            // Handle regular string - existing logic
            log::debug!("println: treating IR ID {} as regular string", arg_id);
            self.generate_print_string_literal(arg_id)
        }
        Type::Int => {
            // Handle integer - use print_num
            log::debug!("println: treating IR ID {} as integer", arg_id);
            self.generate_print_number(arg_id)
        }
        _ => {
            // Handle complex cases (concatenation, etc.) - existing logic
            log::debug!("println: using existing complex logic for IR ID {}", arg_id);
            self.generate_println_builtin_legacy(args)
        }
    }
}

/// Print string address using print_paddr
fn generate_print_string_address(&mut self, arg_id: IrId) -> Result<(), CompilerError> {
    let operand = self.resolve_ir_id_to_operand(arg_id)?;

    // Use print_paddr to print string at given address
    self.emit_instruction_typed(
        Opcode::Op1(Op1::PrintPaddr),
        &[operand],
        None,
        None,
    )?;

    // Add newline
    self.emit_instruction_typed(
        NEWLINE,
        &[],
        None,
        None,
    )?;

    Ok(())
}

/// Print regular string literal - existing logic
fn generate_print_string_literal(&mut self, arg_id: IrId) -> Result<(), CompilerError> {
    // ... existing string literal printing logic ...
}

/// Print number using print_num
fn generate_print_number(&mut self, arg_id: IrId) -> Result<(), CompilerError> {
    let operand = self.resolve_ir_id_to_operand(arg_id)?;

    self.emit_instruction_typed(
        Opcode::OpVar(OpVar::PrintNum),
        &[operand],
        None,
        None,
    )?;

    // Add newline
    self.emit_instruction_typed(
        NEWLINE,
        &[],
        None,
        None,
    )?;

    Ok(())
}
```

### 4.2 Type Information Transfer to Codegen
**File**: `src/grue_compiler/codegen.rs`

```rust
// Add type information to codegen context
impl ZMachineCodeGen {
    // Transfer IR type information to codegen context
    pub fn transfer_type_information(&mut self, ir: &IrProgram) {
        self.ir_type_info = ir.expression_types.clone();
        log::debug!("Transferred {} type mappings to codegen", self.ir_type_info.len());
    }

    // Add to codegen context structure
    pub struct ZMachineCodeGen {
        // ... existing fields ...
        pub ir_type_info: HashMap<IrId, Type>,  // NEW: Type information from IR
    }
}
```

### 4.3 Enhanced Type-Based Instruction Selection
**File**: `src/grue_compiler/codegen.rs`

```rust
// In generate_builtin_function() dispatch
pub fn generate_builtin_function(&mut self, function_name: &str, args: &[IrId], target: Option<IrId>) -> Result<(), CompilerError> {
    match function_name.as_str() {
        "println" => {
            // Type-aware println - no target needed
            self.generate_println_builtin(args)
        }
        "print" => {
            // Type-aware print - similar logic
            self.generate_print_builtin(args)
        }
        // Remove print_message - no longer needed
        // "print_message" => self.generate_print_message_builtin(args),

        // ... rest unchanged
    }
}
```

## Phase 5: Migration and Cleanup

### 5.1 Remove print_message Builtin
**Files to update**:
- `src/grue_compiler/codegen_builtins.rs` - Remove `generate_print_message_builtin`
- `src/grue_compiler/codegen.rs` - Remove `print_message` from dispatch
- `src/grue_compiler/semantic.rs` - Remove `print_message` from builtins
- `src/grue_compiler/ir.rs` - Remove `print_message` from builtin detection

### 5.2 Update Source Code Usage
**File**: `examples/mini_zork.grue`

```grue
// Change from:
print_message(exit.message);

// Back to:
println(exit.message);  // Now works automatically!
```

### 5.3 Update Documentation
**File**: `BUGS.md`

```markdown
* ✅ **ELEGANTLY FIXED**: Blocked exit messages printing "0" instead of message text
  - **Issue**: `exit_get_message` returned int addresses but `println()` treated them as numbers
  - **Solution**: Implemented `StringAddress` type system with automatic type detection
  - **Result**: `println(exit.message)` now automatically works, using `print_paddr` for string addresses
  - **Architecture**: Enhanced type system with String/StringAddress distinction and automatic println dispatch
```

## Phase 6: Testing and Validation

### 6.1 Functional Testing
```bash
# Test that mini_zork works with new println(exit.message)
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork_string_address.z3
echo "east" | ./target/debug/gruesome tests/mini_zork_string_address.z3

# Expected output: "The door is boarded and you can't remove the boards."
```

### 6.2 Type System Testing
**Create**: `src/grue_compiler/tests/string_address_tests.rs`

```rust
#[cfg(test)]
mod string_address_tests {
    use super::*;

    #[test]
    fn test_string_address_type_compatibility() {
        let analyzer = SemanticAnalyzer::new();

        // StringAddress should be compatible with String contexts
        assert!(analyzer.types_compatible(&Type::String, &Type::StringAddress));

        // But not with Int contexts
        assert!(!analyzer.types_compatible(&Type::Int, &Type::StringAddress));
    }

    #[test]
    fn test_exit_get_message_type() {
        // Test that exit_get_message returns StringAddress type
        let mut analyzer = SemanticAnalyzer::new();

        // Simulate: let msg = exit.message;
        let call_type = analyzer.visit_property_access(&PropertyAccess {
            object: Box::new(Expression::Identifier("exit".to_string())),
            property: "message".to_string(),
        });

        assert_eq!(call_type, Type::StringAddress);
    }

    #[test]
    fn test_println_accepts_string_address() {
        // Test that println(exit.message) type checks correctly
        let mut analyzer = SemanticAnalyzer::new();

        // This should not generate type errors
        let result = analyzer.visit_function_call(&FunctionCall {
            name: "println".to_string(),
            arguments: vec![
                Expression::PropertyAccess(PropertyAccess {
                    object: Box::new(Expression::Identifier("exit".to_string())),
                    property: "message".to_string(),
                })
            ],
        });

        // Should succeed without errors
        assert!(analyzer.errors.is_empty());
    }
}
```

### 6.3 Codegen Testing
```rust
#[test]
fn test_println_string_address_codegen() {
    // Test that println(string_address) generates print_paddr
    // Detailed codegen testing...
}

#[test]
fn test_println_string_literal_codegen() {
    // Test that println("literal") still works as before
    // Detailed codegen testing...
}

#[test]
fn test_println_integer_codegen() {
    // Test that println(123) uses print_num
    // Detailed codegen testing...
}
```

### 6.4 Integration Testing
```bash
# Full mini_zork gameplay testing
test_mini_zork_with_string_address_type() {
    echo "Testing full mini_zork gameplay..."

    # Test blocked exit message
    echo "east" | ./target/debug/gruesome tests/mini_zork_string_address.z3 | grep "boarded"

    # Test other println usage
    echo "look" | ./target/debug/gruesome tests/mini_zork_string_address.z3 | grep "field"

    # Test navigation still works
    echo -e "north\nsouth\n" | ./target/debug/gruesome tests/mini_zork_string_address.z3
}
```

## Phase 7: Branch Integration

### 7.1 Pre-merge Validation
```bash
# Ensure all tests pass on feature branch
cargo test --all
cargo fmt
cargo clippy

# Ensure mini_zork compiles and runs
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork_final.z3
echo "look" | ./target/debug/gruesome tests/mini_zork_final.z3
```

### 7.2 Merge Strategy
```bash
# Update feature branch with latest main
git checkout feature/string-address-type
git merge main  # Handle any conflicts

# Create PR for review
git push origin feature/string-address-type

# After review, merge to main
git checkout main
git merge feature/string-address-type
git push origin main

# Clean up feature branch
git branch -d feature/string-address-type
git push origin --delete feature/string-address-type
```

### 7.3 Post-merge Validation
```bash
# Ensure main branch still works correctly
cargo test --all
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork_main.z3
echo "east" | ./target/debug/gruesome tests/mini_zork_main.z3
```

## Implementation Schedule

### Week 1: Foundation (Phases 1-2)
- [ ] Create feature branch
- [ ] Implement StringAddress type enum
- [ ] Update parser and semantic analysis
- [ ] Test type compatibility rules

### Week 2: IR Enhancement (Phase 3)
- [ ] Add type tracking to IR generation
- [ ] Update builtin function signatures
- [ ] Test IR type information flow

### Week 3: Codegen Implementation (Phase 4)
- [ ] Implement type-aware println
- [ ] Test different type dispatch paths
- [ ] Validate bytecode generation

### Week 4: Migration and Testing (Phases 5-6)
- [ ] Remove print_message builtin
- [ ] Update mini_zork.grue to use println
- [ ] Comprehensive testing and validation

### Week 5: Integration (Phase 7)
- [ ] Pre-merge validation
- [ ] Branch merge and cleanup
- [ ] Post-merge testing

## Success Criteria

**Must Pass ALL of**:
1. ✅ **Type system correctly identifies StringAddress vs String vs Int**
2. ✅ **`println(exit.message)` prints blocked message text, not "1904"**
3. ✅ **All existing functionality unchanged** (navigation, objects, etc.)
4. ✅ **No print_message builtin needed** - elegant automatic solution
5. ✅ **All 174+ tests pass** - no regressions introduced
6. ✅ **Clean type-driven architecture** - extensible to other similar cases

## Future Extensions

This StringAddress type system enables future enhancements:

1. **Object names**: `println(object.name)` automatically works
2. **Room descriptions**: `println(room.desc)` automatically works
3. **Property strings**: Any string property automatically works with println
4. **Type safety**: Prevents mixing string addresses with integers accidentally
5. **Better error messages**: Clear type mismatch errors for developers

## Risk Mitigation

1. **Feature branch isolation** - main branch remains stable
2. **Incremental testing** - validate each phase independently
3. **Type compatibility** - StringAddress works in String contexts for backward compatibility
4. **Comprehensive testing** - extensive validation before merge
5. **Rollback plan** - can revert to current print_message solution if needed

---

**Status**: Ready for implementation
**Next Action**: Create feature branch and begin Phase 1