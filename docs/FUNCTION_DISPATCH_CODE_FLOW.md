# Function Dispatch Bug - Detailed Code Flow

## Execution Timeline

### PASS 1: Function Name Registration (Lines 1432-1443)

```rust
pub fn generate(&mut self, ast: Program) -> Result<IrProgram, CompilerError> {
    // TWO-PASS APPROACH: First pass registers all function definitions
    for item in ast.items.iter() {
        if let crate::grue_compiler::ast::Item::Function(func) = item {
            let func_id = self.next_id();
            self.symbol_ids.insert(func.name.clone(), func_id);  // LINE 1436
            // After this:
            // symbol_ids["handle_climb"] = 42  (generic function)
            // symbol_ids["handle_climb"] = 43  (OVERWRITES with second "handle_climb"!)
        }
    }
```

**Critical Issue:** If there are multiple `handle_climb` functions in PASS 1, the second one OVERWRITES the first in symbol_ids!

### PASS 2: Function Body Generation (Lines 1445-1471)

```rust
    for item in ast.items.iter() {
        match item {
            crate::grue_compiler::ast::Item::Grammar(grammar) => {
                // Defer grammar
                deferred_grammar.push(grammar.clone());
            }
            _ => {
                self.generate_item(item.clone(), &mut ir_program)?;  // LINE 1454
            }
        }
    }
```

### Within generate_item() for functions (Line 1636+)

```rust
fn generate_item(&mut self, item: Item, ir_program: &mut IrProgram) -> Result<(), CompilerError> {
    match item {
        Item::Function(func) => {
            let ir_func = self.generate_function(func)?;
            ir_program.functions.push(ir_func);
        }
        // ...
    }
}
```

### Within generate_function() - Handle Specialization (Lines 1678-1812)

```rust
fn generate_function(&mut self, func: FunctionDecl) -> Result<IrFunction, CompilerError> {
    let specialization = self.detect_specialization(&func.name, &func.parameters);
    // For handle_climb(tree): specialization = SpecificObject("tree")
    // For handle_climb(obj): specialization = Generic

    let func_id = if let Some(&existing_id) = self.symbol_ids.get(&func.name) {
        if !self.function_overloads.contains_key(&func.name) {
            // First occurrence - reuse the ID from PASS 1
            existing_id  // LINE 1690
        } else {
            // Additional overload - create new ID
            self.next_id()  // LINE 1693
        }
    }
    
    // Register as overload
    self.register_function_overload(&func.name, func_id, specialization.clone());
    
    // IMPORTANT: Generate the function BODY HERE
    let body = self.generate_block(func.body)?;  // LINE 1768
    
    // This calls generate_statement -> generate_expression
    // Which will encounter any function calls in the body
}
```

### CRITICAL: Within generate_expression for FunctionCall (Lines 3414-3460)

**This is where the BUG manifests!**

```rust
Expr::FunctionCall { name, arguments } => {
    // Generate arguments first
    let mut arg_temps = Vec::new();
    for arg in arguments {
        let arg_temp = self.generate_expression(arg, block)?;
        arg_temps.push(arg_temp);
    }

    // Check if this is a built-in function
    if self.is_builtin_function(&name) {
        return self.generate_builtin_function_call(&name, &arg_temps, block);
    }

    // POLYMORPHIC DISPATCH CHECK - THIS IS THE BUG!
    let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name) {  // LINE 3429
        // Check dispatch functions
        log::debug!("🎯 Direct call using dispatch function for '{}': ID {}", name, dispatch_id);
        dispatch_id
    } else if let Some(&id) = self.symbol_ids.get(&name) {  // LINE 3436
        // Fallback to symbol table
        log::debug!("🎯 Direct call using original function for '{}': ID {}", name, id);
        id
    } else {
        return Err(CompilerError::SemanticError(
            format!("Function '{}' not found.", name),
            0,
        ));
    };
    
    // At this point:
    // self.dispatch_functions is EMPTY because we're still in PASS 2!
    // self.symbol_ids["handle_climb"] contains the generic function's ID
    // So we call the generic function, which then calls itself again = INFINITE RECURSION
}
```

### AFTER PASS 2: Create Dispatch Functions (Line 1471)

```rust
    // Copy symbol mappings to IR program
    ir_program.symbol_ids = self.symbol_ids.clone();
    
    // Generate dispatch functions for polymorphic functions
    self.generate_dispatch_functions(&mut ir_program)?;  // LINE 1471
    
    // NOW dispatch functions are created and added to self.dispatch_functions
    // BUT IT'S TOO LATE - all function bodies have already been generated!
```

### Dispatch Function Creation (Lines 1260-1284)

```rust
fn generate_dispatch_functions(&mut self, ir_program: &mut IrProgram) -> Result<(), CompilerError> {
    for (base_name, overloads) in &self.function_overloads.clone() {
        if overloads.len() > 1 {  // LINE 1266 - Only if multiple overloads
            let dispatch_func = self.create_dispatch_function(base_name, overloads)?;
            ir_program.functions.push(dispatch_func);
            
            // NOW populate dispatch_functions map
            if let Some(dispatch_id) = ir_program.functions.last().map(|f| f.id) {
                self.dispatch_functions.insert(base_name.clone(), dispatch_id);  // LINE 1278-1279
            }
        }
    }
}
```

### Dispatch Function Content (Lines 1287-1416)

```rust
fn create_dispatch_function(base_name: &str, overloads: &[FunctionOverload]) {
    // For each specific object specialization:
    for overload in &sorted_overloads {
        if let ObjectSpecialization::SpecificObject(obj_name) = &overload.specialization {
            // Create: if param == tree -> call handle_climb_tree(param)
            instructions.push(IrInstruction::Branch {
                condition: comparison_temp,
                true_label: match_label,
                false_label: continue_label,
            });
            
            instructions.push(IrInstruction::Label { id: match_label });
            instructions.push(IrInstruction::Call {
                target: Some(result_temp),
                function: overload.function_id,  // Calls the SPECIALIZED version
                args: vec![param_id],
            });
        }
    }
    
    // Default case: call generic function
    if let Some(generic_overload) = sorted_overloads
        .iter()
        .find(|o| matches!(o.specialization, ObjectSpecialization::Generic)) {
        instructions.push(IrInstruction::Call {
            target: Some(result_temp),
            function: generic_overload.function_id,  // Calls the GENERIC version
            args: vec![param_id],
        });
    }
}
```

### LATER: Process Grammar (Lines 1474-1477)

```rust
    // Now process deferred grammar items with dispatch functions available
    for grammar in deferred_grammar {
        let ir_grammar = self.generate_grammar(grammar)?;
        ir_program.grammar.push(ir_grammar);
    }
```

When grammar processes a call like `handle_climb(tree)`:

```rust
// In generate_grammar -> generate_handler
let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name) {  // FOUND!
    // dispatch_functions NOW has entries because we created them at line 1471
    dispatch_id  // Use dispatch function
} else if let Some(&id) = self.symbol_ids.get(&name) {
    id
}
```

## The Execution Sequence for Our Example

```
Code:
  fn handle_climb(obj) {
      handle_climb(tree);
  }
  fn handle_climb(tree) {
      // ...
  }

Timeline:
1. PASS 1 registration:
   symbol_ids["handle_climb"] = ID(1) for handle_climb(obj)
   symbol_ids["handle_climb"] = ID(2) for handle_climb(tree)  [OVERWRITES!]
   
   Result: symbol_ids["handle_climb"] points to handle_climb(tree) only!
   
   BUT WAIT - this might not be the issue...

2. PASS 2 - Generate handle_climb(obj):
   register_function_overload("handle_climb", func_id, Generic)
   During body generation: encounters handle_climb(tree)
   
   Checks: dispatch_functions.get("handle_climb") → EMPTY
   Falls back: symbol_ids.get("handle_climb") → ID(something)
   
   But which ID? If symbol_ids was overwritten, it points to handle_climb(tree)
   This would work... but let me check the overload registration

3. PASS 2 - Generate handle_climb(tree):
   detect_specialization: finds "tree" in object_numbers
   specialization = SpecificObject("tree")
   
   register_function_overload("handle_climb", new_func_id, SpecificObject("tree"))

4. After PASS 2: generate_dispatch_functions
   Sees function_overloads["handle_climb"] has 2 entries
   Creates dispatch_handle_climb
   Populates dispatch_functions["handle_climb"] = dispatch_id
   
   Too late - the bodies are already generated!

5. Grammar processing:
   Encounters handle_climb(tree) call
   dispatch_functions.get("handle_climb") → FOUND
   Uses dispatch function
   Works correctly!
```

## The Real Problem

The issue is that when we're in step 2 (PASS 2) and we check `dispatch_functions.get("handle_climb")`, it's empty because we haven't run step 4 yet.

We fall back to `symbol_ids.get("handle_climb")` which could point to either version (depending on registration order and how the overload map works).

The safest assumption is that it points to the generic function (ID from first registration), causing the infinite loop.

## Summary

| Component | When Created | When Used | Status |
|-----------|--------------|-----------|--------|
| symbol_ids["handle_climb"] | Line 1436 (PASS 1) | Line 3436 (PASS 2) | Available |
| function_overloads tracking | Line 1701 (PASS 2) | Line 1471 (after PASS 2) | Available later |
| dispatch_functions["handle_climb"] | Line 1278 (after PASS 2) | Line 3429 (during PASS 2) | **NOT YET!** |
| Grammar processing | Deferred | Line 1474 (after PASS 2) | Uses dispatch_functions |

The dispatch_functions map is populated AFTER all function bodies are generated, so function call resolution during body generation always falls back to symbol_ids.
