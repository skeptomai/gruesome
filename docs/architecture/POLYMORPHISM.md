# Polymorphic Dispatch Implementation Plan

## Overview

This document outlines a plan for implementing polymorphic dispatch on handle_* function calls in the Grue language, enabling both generic object handlers and specialized handlers for specific objects.

## Current State Analysis

### Manual Polymorphic Dispatch

The current system uses manual polymorphic dispatch with conditional logic within single functions:

```grue
fn handle_take(obj) {
    // Generic logic first
    if obj.location == player {
        println("You already have that.");
        return;
    }

    // Object-specific logic
    if (obj == leaflet) {
        player.score = player.score + 2;  // Special scoring for leaflet
    }

    // Generic completion
    move(obj, player);
    println("Taken.");
}

fn handle_climb(obj) {
    if (obj == tree) {
        if (player.location == forest_path) {
            handle_go("up");
            player.score = player.score + 5;
        }
    } else {
        println("You can't climb that.");
    }
}
```

### Problems with Current Approach

1. **Poor Separation of Concerns**: Generic and object-specific logic mixed together
2. **Maintenance Burden**: Adding new objects requires modifying existing functions
3. **Code Duplication**: Common patterns repeated across conditional branches
4. **Poor Extensibility**: Difficult to add complex object-specific behaviors

## Proposed Polymorphic System

### Target Syntax

Enable clean function overloading based on object types:

```grue
// Generic handler (fallback)
fn handle_take(obj) {
    if obj.location == player {
        println("You already have that.");
        return;
    }

    if !obj.takeable {
        println("You can't take that.");
        return;
    }

    move(obj, player);
    println("Taken.");
}

// Specialized handler for leaflet
fn handle_take(leaflet) {
    player.score = player.score + 2;
    move(leaflet, player);
    println("You discover something valuable! Taken.");
}

// Specialized handler for egg
fn handle_take(egg) {
    if egg.location.container && !egg.location.open {
        println("The nest is too high to reach into safely.");
        return;
    }

    player.score = player.score + 10;
    move(egg, player);
    println("You carefully take the precious egg. Taken.");
}

// Tree climbing with location-specific behavior
fn handle_climb(tree) {
    if player.location == forest_path {
        handle_go("up");
        player.score = player.score + 5;
        println("You climb up into the tree branches.");
    } else {
        println("The tree's branches are too high to reach from here.");
    }
}

// Generic climbing handler
fn handle_climb(obj) {
    println("You can't climb that.");
}
```

## Implementation Complexity Analysis

### Medium Complexity Overall

The implementation involves multiple interconnected systems but builds on existing infrastructure.

## Phase 1: Function Overloading (Medium Effort)

### 1.1 AST Extensions

**File**: `src/grue_compiler/ast.rs`

Add support for function signature tracking:

```rust
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<ParameterDecl>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub specialization: Option<ObjectSpecialization>,  // NEW
}

#[derive(Debug, Clone)]
pub struct ParameterDecl {
    pub name: String,
    pub param_type: Option<TypeAnnotation>,
    pub object_constraint: Option<String>,  // NEW: specific object name
}

#[derive(Debug, Clone)]
pub enum ObjectSpecialization {
    Generic,                    // Default handler
    SpecificObject(String),     // Handler for specific object (e.g., "leaflet")
    ObjectType(String),         // Handler for object type (future extension)
}
```

### 1.2 IR Extensions

**File**: `src/grue_compiler/ir.rs`

Track multiple function variants:

```rust
pub struct IrGenerator {
    // Existing fields...
    function_overloads: HashMap<String, Vec<FunctionOverload>>,  // NEW
    dispatch_functions: HashMap<String, IrId>,                   // NEW
}

#[derive(Debug, Clone)]
pub struct FunctionOverload {
    pub function_id: IrId,
    pub specialization: ObjectSpecialization,
    pub mangled_name: String,
    pub priority: u8,  // Lower number = higher priority
}
```

### 1.3 Name Mangling Strategy

Generate unique Z-Machine function names:

```rust
fn mangle_function_name(base_name: &str, specialization: &ObjectSpecialization) -> String {
    match specialization {
        ObjectSpecialization::Generic => format!("{}_default", base_name),
        ObjectSpecialization::SpecificObject(obj_name) => format!("{}_{}", base_name, obj_name),
        ObjectSpecialization::ObjectType(type_name) => format!("{}_type_{}", base_name, type_name),
    }
}

// Examples:
// handle_take + Generic -> "handle_take_default"
// handle_take + SpecificObject("leaflet") -> "handle_take_leaflet"
// handle_climb + SpecificObject("tree") -> "handle_climb_tree"
```

### 1.4 Dispatch Function Generation

**File**: `src/grue_compiler/codegen.rs`

Generate runtime dispatch functions:

```rust
fn generate_dispatch_function(&mut self, function_name: &str, overloads: &[FunctionOverload]) -> Result<IrId, CompilerError> {
    let dispatch_id = self.next_id();

    // Create dispatch function with object parameter
    let param_id = self.next_id();

    // Generate if-else chain for object ID matching
    for overload in overloads.iter().sorted_by_key(|o| o.priority) {
        match &overload.specialization {
            ObjectSpecialization::SpecificObject(obj_name) => {
                // if (obj_id == OBJECT_ID) { call specialized_handler(obj_id) }
                let obj_id = self.get_object_id(obj_name)?;
                self.generate_object_id_check(param_id, obj_id, overload.function_id)?;
            }
            ObjectSpecialization::Generic => {
                // Default case - always matches
                self.generate_default_call(overload.function_id)?;
            }
            _ => {} // Future extension
        }
    }

    Ok(dispatch_id)
}
```

### 1.5 Grammar System Integration

**File**: `src/grue_compiler/codegen.rs` (Grammar pattern matching)

Update grammar calls to use dispatch functions:

```rust
// OLD: Direct function call
call_vs(handle_take_function_id, object_id)

// NEW: Dispatch function call
call_vs(dispatch_take_function_id, object_id)
```

## Phase 2: Enhanced Type System (High Effort)

### 2.1 Object Type Annotations

Enable object type hierarchies:

```grue
object leaflet : readable_item {
    names: ["leaflet", "paper"]
    desc: "Welcome to DORK!"
    takeable: true
}

object tree : climbable_object {
    names: ["tree", "large tree"]
    desc: "The tree has low branches."
    climbable: true
}
```

### 2.2 Type-Based Dispatch

Support handlers for object types:

```grue
fn handle_read(readable_item item) {
    println(item.desc);
}

fn handle_climb(climbable_object obj) {
    if obj.climbable {
        println("You climb the " + obj.name + ".");
    }
}
```

### 2.3 Runtime Type Tables

Maintain object type information in Z-Machine memory:

```
Object Type Table:
[Object ID] -> [Type ID] -> [Handler Function Address]
```

## Architecture Integration Points

### Existing Infrastructure (Supports Implementation)

✅ **Function Calls**: `call_vs` instruction handles any function
✅ **Parameter Passing**: String-to-dictionary conversion handles object parameters
✅ **UnresolvedReference System**: Can handle multiple function references
✅ **IR Generation**: Already tracks function calls and object IDs
✅ **Object Resolution**: Grammar system already resolves object names to IDs

### Required Modifications

⚠️ **Grammar System**: Change from direct calls to dispatch calls
⚠️ **Function Registration**: Support multiple functions with same name
⚠️ **Code Generation**: Generate dispatch functions and object ID comparisons
⚠️ **Name Resolution**: Track mangled function names and dispatch tables

## Implementation Strategy

### Phase 1: Basic Function Overloading (Recommended Start)

**Goals**:
- Support multiple function definitions with same name
- Compile-time dispatch based on object IDs
- Maintain backward compatibility

**Implementation Steps**:

1. **Extend AST** (1-2 days)
   - Add ObjectSpecialization to FunctionDecl
   - Update parser to recognize object-specific parameters

2. **Update IR Generation** (2-3 days)
   - Track function overloads
   - Generate dispatch functions
   - Implement name mangling

3. **Modify Codegen** (3-4 days)
   - Generate object ID comparison chains
   - Update grammar system integration
   - Handle UnresolvedReference for dispatch calls

4. **Testing** (2-3 days)
   - Unit tests for dispatch generation
   - Integration tests with mini_zork
   - Performance benchmarking

**Total Effort: 8-12 days**

### Phase 2: Type System Extensions (Future Work)

**Goals**:
- Object type hierarchies
- Type-based dispatch
- Runtime type checking

**Implementation Steps**:

1. **Object Type System** (5-7 days)
2. **Runtime Type Tables** (3-5 days)
3. **Dynamic Dispatch** (4-6 days)
4. **Inheritance Support** (3-5 days)

**Total Effort: 15-23 days**

## Benefits and Trade-offs

### Benefits

✅ **Clean Code Organization**: Separate concerns per object type
✅ **Maintainability**: Add new objects without modifying existing handlers
✅ **Extensibility**: Easy to add complex object-specific behaviors
✅ **Performance**: Compile-time dispatch has no runtime overhead
✅ **Backward Compatibility**: Existing single-function approach still works

### Trade-offs

⚠️ **Code Size**: More functions = larger Z-Machine bytecode
⚠️ **Compilation Complexity**: More complex IR and codegen phases
⚠️ **Debug Difficulty**: Multiple functions with same name harder to debug
⚠️ **Z-Machine Constraints**: Limited by 64KB code space

## Risk Assessment

### Low Risk
- **Existing Infrastructure**: Most required systems already exist
- **Backward Compatibility**: Can implement without breaking current code
- **Incremental Development**: Phase 1 provides immediate value

### Medium Risk
- **Code Size Growth**: Need monitoring for Z-Machine memory limits
- **Complexity Increase**: More moving parts in compilation pipeline

### High Risk
- **Phase 2 Scope**: Type system extensions are significantly more complex

## Recommendation

**Implement Phase 1 Only** - provides 80% of the benefit with manageable complexity:

```grue
// Compiler generates efficient dispatch:
fn dispatch_take(obj_id) {
    if (obj_id == 30) { call_vs(handle_take_leaflet, obj_id); return; }
    if (obj_id == 25) { call_vs(handle_take_egg, obj_id); return; }
    call_vs(handle_take_default, obj_id);
}
```

**Advantages of Phase 1**:
- ✅ Clean source syntax for object-specific handlers
- ✅ Reuses existing grammar and object resolution infrastructure
- ✅ Compile-time dispatch = no runtime performance overhead
- ✅ Manageable implementation complexity (8-12 days)
- ✅ Immediate improvement to code organization

**Defer Phase 2** until Phase 1 proves successful and additional type system features are needed.

## Files to Modify

### Core Language Files
- `src/grue_compiler/ast.rs` - Function declaration extensions
- `src/grue_compiler/parser.rs` - Parse object-specific parameters
- `src/grue_compiler/ir.rs` - Function overload tracking and dispatch generation
- `src/grue_compiler/codegen.rs` - Dispatch function code generation
- `src/grue_compiler/codegen_instructions.rs` - Grammar system integration updates

### Test Files
- `tests/test_polymorphism.rs` - New test file for dispatch functionality
- `examples/polymorphism_test.grue` - Test game with object-specific handlers

## Success Criteria

### Phase 1 Complete When:
1. Can define multiple `handle_*` functions for same verb
2. Object-specific handlers called correctly at runtime
3. Generic handlers provide fallback behavior
4. mini_zork.grue compiles and runs with polymorphic handlers
5. No performance regression vs manual if-statement approach
6. Code size increase < 20% for equivalent functionality

### Long-term Success:
- Game developers prefer polymorphic syntax over manual conditionals
- Larger games benefit from improved code organization
- Extension to new object types requires minimal existing code changes

## References

- **Grammar System Architecture**: docs/ARCHITECTURE.md lines 733-1123
- **Function Call Implementation**: docs/ARCHITECTURE.md lines 1522-1708
- **String Parameter Passing**: docs/ARCHITECTURE.md lines 1522-1708
- **IR Generation Patterns**: docs/ARCHITECTURE.md lines 1126-1269
- **Z-Machine Function Calls**: Z-Machine Standards Document Section 15 (call_vs, call_1s, etc.)