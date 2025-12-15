// IR Generator - Function Generation and Polymorphic Dispatch
//
// Extracted from ir_generator.rs as part of modularization effort.
// Handles function compilation, name mangling, and polymorphic dispatch generation.

use crate::grue_compiler::ast::{ObjectSpecialization, Type};
use crate::grue_compiler::error::CompilerError;

use super::{
    FunctionOverload, IrBinaryOp, IrBlock, IrFunction, IrGenerator, IrId, IrInstruction, IrLocal,
    IrParameter, IrProgram, IrValue,
};

impl IrGenerator {
    /// Mangle function names based on object specialization
    ///
    /// Creates unique names for overloaded functions:
    /// - Generic: `function_default`
    /// - Specific object: `function_objectname`
    /// - Object type: `function_type_typename`
    pub(super) fn mangle_function_name(
        &self,
        base_name: &str,
        specialization: &ObjectSpecialization,
    ) -> String {
        match specialization {
            ObjectSpecialization::Generic => format!("{}_default", base_name),
            ObjectSpecialization::SpecificObject(obj_name) => format!("{}_{}", base_name, obj_name),
            ObjectSpecialization::ObjectType(type_name) => {
                format!("{}_type_{}", base_name, type_name)
            }
        }
    }

    /// Detect object specialization based on parameter names
    ///
    /// Determines if a function is specialized for specific objects or generic.
    /// Uses heuristics and object_numbers lookup to identify specialization.
    pub(super) fn detect_specialization(
        &self,
        _func_name: &str,
        parameters: &[crate::grue_compiler::ast::Parameter],
    ) -> ObjectSpecialization {
        // Check if any parameter matches an object name
        for param in parameters {
            // First check against object_numbers if available (Pass 2+)
            if self.object_numbers.contains_key(&param.name) {
                return ObjectSpecialization::SpecificObject(param.name.clone());
            }

            // During Pass 1, object_numbers isn't populated yet.
            // Use heuristic: specific object names are typically short and specific (tree, leaflet, egg)
            // Exclude common parameter types that aren't actual objects
            let generic_names = [
                "obj",
                "object",
                "item",
                "thing",
                "target",
                "arg",
                "location",
                "direction",
                "container",
            ];
            if !generic_names.contains(&param.name.as_str()) {
                return ObjectSpecialization::SpecificObject(param.name.clone());
            }
        }
        ObjectSpecialization::Generic
    }

    /// Register a function overload for polymorphic dispatch
    ///
    /// Tracks function overloads with priority levels:
    /// - Priority 0: Specific objects (highest)
    /// - Priority 1: Object types
    /// - Priority 2: Generic fallback (lowest)
    pub(super) fn register_function_overload(
        &mut self,
        base_name: &str,
        func_id: IrId,
        specialization: ObjectSpecialization,
    ) {
        let mangled_name = self.mangle_function_name(base_name, &specialization);
        let priority = match &specialization {
            ObjectSpecialization::SpecificObject(_) => 0, // Highest priority
            ObjectSpecialization::ObjectType(_) => 1,     // Medium priority
            ObjectSpecialization::Generic => 2,           // Lowest priority (fallback)
        };

        let overload = FunctionOverload {
            function_id: func_id,
            specialization,
            mangled_name,
            priority,
        };

        if let Some(overloads) = self.function_overloads.get_mut(base_name) {
            overloads.push(overload);
        } else {
            self.function_overloads
                .insert(base_name.to_string(), vec![overload]);
        }
    }

    /// Generate dispatch functions for polymorphic function calls
    ///
    /// Creates router functions that select the appropriate specialized implementation
    /// based on object identity. Only generates dispatchers for functions with multiple overloads.
    pub(super) fn generate_dispatch_functions(
        &mut self,
        ir_program: &mut IrProgram,
    ) -> Result<(), CompilerError> {
        for (base_name, overloads) in &self.function_overloads.clone() {
            if overloads.len() > 1 {
                log::debug!(
                    "PASS 3: Generating dispatch function body for '{}' with {} overloads",
                    base_name,
                    overloads.len()
                );

                // Use pre-allocated dispatch ID from Pass 1.5
                let dispatch_id = self.dispatch_functions.get(base_name).copied();
                let dispatch_func =
                    self.create_dispatch_function(base_name, overloads, dispatch_id)?;
                ir_program.functions.push(dispatch_func);

                // Verify the ID matches what we pre-allocated
                if let Some(created_id) = ir_program.functions.last().map(|f| f.id) {
                    if let Some(expected_id) = dispatch_id {
                        assert_eq!(
                            created_id, expected_id,
                            "Dispatch function ID mismatch for '{}': expected {}, got {}",
                            base_name, expected_id, created_id
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Create a dispatch function that routes calls to the appropriate specialized function
    ///
    /// Generates IR for runtime polymorphic dispatch:
    /// 1. Compares object parameter against known object IDs
    /// 2. Calls specialized function if match found
    /// 3. Falls back to generic implementation
    pub(super) fn create_dispatch_function(
        &mut self,
        base_name: &str,
        overloads: &[FunctionOverload],
        dispatch_id: Option<u32>,
    ) -> Result<IrFunction, CompilerError> {
        let dispatch_id = dispatch_id.unwrap_or_else(|| self.next_id());

        // Create parameter to match the original function signature
        // For now, assume single object parameter (can be extended later)
        let param_id = self.next_id();
        let dispatch_param = IrParameter {
            name: "obj".to_string(),
            param_type: Some(Type::Object),
            slot: 1,
            ir_id: param_id,
        };

        let mut instructions = Vec::new();
        let mut _label_counter = 0;

        // Sort overloads by priority (specific objects first)
        let mut sorted_overloads = overloads.to_vec();
        sorted_overloads.sort_by_key(|o| o.priority);

        // Generate object ID checks for specific objects
        for overload in &sorted_overloads {
            if let ObjectSpecialization::SpecificObject(obj_name) = &overload.specialization {
                if let Some(&_obj_number) = self.object_numbers.get(obj_name) {
                    _label_counter += 1;
                    let match_label = self.next_id();
                    let continue_label = self.next_id();

                    // Load object constant the same way working code does
                    // This creates the same IrValue::Object that identifier resolution creates
                    let obj_constant_temp = self.next_id();
                    instructions.push(IrInstruction::LoadImmediate {
                        target: obj_constant_temp,
                        value: IrValue::Object(obj_name.to_string()),
                    });

                    // Compare object parameter with specific object ID
                    let comparison_temp = self.next_id();
                    instructions.push(IrInstruction::BinaryOp {
                        target: comparison_temp,
                        op: IrBinaryOp::Equal,
                        left: param_id,
                        right: obj_constant_temp,
                    });

                    // Branch: if equal, call specialized function
                    instructions.push(IrInstruction::Branch {
                        condition: comparison_temp,
                        true_label: match_label,
                        false_label: continue_label,
                    });

                    // Match label: call specialized function
                    instructions.push(IrInstruction::Label { id: match_label });
                    let result_temp = self.next_id();
                    instructions.push(IrInstruction::Call {
                        target: Some(result_temp),
                        function: overload.function_id,
                        args: vec![param_id],
                    });
                    instructions.push(IrInstruction::Return {
                        value: Some(result_temp),
                    });

                    // Continue label for next check
                    instructions.push(IrInstruction::Label { id: continue_label });
                }
            }
        }

        // Default case: call generic function
        if let Some(generic_overload) = sorted_overloads
            .iter()
            .find(|o| matches!(o.specialization, ObjectSpecialization::Generic))
        {
            let result_temp = self.next_id();
            instructions.push(IrInstruction::Call {
                target: Some(result_temp),
                function: generic_overload.function_id,
                args: vec![param_id],
            });
            instructions.push(IrInstruction::Return {
                value: Some(result_temp),
            });
        } else {
            // No generic function - return 0
            let zero_temp = self.next_id();
            instructions.push(IrInstruction::LoadImmediate {
                target: zero_temp,
                value: IrValue::Integer(0),
            });
            instructions.push(IrInstruction::Return {
                value: Some(zero_temp),
            });
        }

        // CRITICAL FIX: Convert parameter to local variable for Z-Machine function header
        //
        // ISSUE: Dispatch functions had parameters in `parameters` vec but not in `local_vars`,
        // causing Z-Machine function header generation to create 0 locals while the function
        // tries to read parameter from local variable 1. This returned 0, corrupting Variable(3)
        // and breaking object resolution in polymorphic dispatch (e.g., "take leaflet").
        //
        // SOLUTION: Add the dispatch parameter to both `parameters` (for IR semantics) and
        // `local_vars` (for Z-Machine function header generation) to ensure proper local
        // variable allocation in the generated Z-Machine bytecode.
        let dispatch_local = IrLocal {
            ir_id: dispatch_param.ir_id,
            name: dispatch_param.name.clone(),
            var_type: dispatch_param.param_type.clone(),
            slot: dispatch_param.slot,
            mutable: true, // Parameters are typically mutable
        };

        Ok(IrFunction {
            id: dispatch_id,
            name: format!("dispatch_{}", base_name),
            parameters: vec![dispatch_param],
            return_type: None, // Can be enhanced later
            body: IrBlock {
                id: self.next_id(),
                instructions,
            },
            local_vars: vec![dispatch_local], // FIXED: Include parameter as local variable for Z-Machine
        })
    }

    /// Generate IR function from AST function declaration
    ///
    /// Handles:
    /// - Parameter processing and local variable allocation
    /// - Function scope management (isolates local symbols from global)
    /// - Polymorphic function name mangling
    /// - Function body generation
    pub(super) fn generate_function(
        &mut self,
        func: crate::grue_compiler::ast::FunctionDecl,
    ) -> Result<IrFunction, CompilerError> {
        // Detect object specialization based on parameter names
        let specialization = self.detect_specialization(&func.name, &func.parameters);

        // Use the specific ID that was assigned to this function in Pass 1
        // This ensures consistent IDs between registration and generation phases
        let func_id = if let Some(&assigned_id) = self
            .function_id_map
            .get(&(func.name.clone(), specialization.clone()))
        {
            assigned_id
        } else {
            // Fallback: try symbol_ids for non-overloaded functions
            if let Some(&existing_id) = self.symbol_ids.get(&func.name) {
                if !self.function_overloads.contains_key(&func.name) {
                    existing_id
                } else {
                    self.next_id()
                }
            } else {
                self.next_id()
            }
        };

        // Function overload already registered in Pass 1

        // Store base name mapping for dispatch lookup
        self.function_base_names.insert(func_id, func.name.clone());

        // SCOPE MANAGEMENT: Save the current global symbol table before processing function
        let saved_symbol_ids = self.symbol_ids.clone();

        // Reset local variable state for this function
        self.current_locals.clear();
        self.next_local_slot = 1; // Slot 0 reserved for return value

        let mut parameters = Vec::new();
        log::debug!(
            "🔧 IR_DEBUG: Function '{}' has {} parameters in AST",
            func.name,
            func.parameters.len()
        );

        // Add parameters as local variables
        for (i, param) in func.parameters.iter().enumerate() {
            log::debug!(
                "🔧 IR_DEBUG: Processing parameter [{}/{}] '{}' for function '{}'",
                i + 1,
                func.parameters.len(),
                param.name,
                func.name
            );

            let param_id = self.next_id();
            let ir_param = IrParameter {
                name: param.name.clone(),
                param_type: param.param_type.clone(),
                slot: self.next_local_slot,
                ir_id: param_id,
            };

            // Add parameters to FUNCTION-SCOPED symbol table (not global)
            self.symbol_ids.insert(param.name.clone(), param_id);
            log::debug!(
                "Function '{}': Added parameter '{}' with IR ID {} to function scope",
                func.name,
                param.name,
                param_id
            );

            // Add parameter as local variable
            let local_param = IrLocal {
                ir_id: param_id,
                name: param.name.clone(),
                var_type: param.param_type.clone(),
                slot: self.next_local_slot,
                mutable: true, // Parameters are typically mutable
            };
            self.current_locals.push(local_param);

            parameters.push(ir_param);
            log::debug!(
                "🔧 IR_DEBUG: Added parameter '{}' (IR ID {}) to parameters Vec for function '{}'",
                param.name,
                param_id,
                func.name
            );
            self.next_local_slot += 1;
        }

        // Generate function body with function-scoped parameters
        let body = self.generate_block(func.body)?;
        let local_vars = self.current_locals.clone();

        // SCOPE MANAGEMENT: Restore the global symbol table after processing function
        self.symbol_ids = saved_symbol_ids;

        log::debug!(
            "🔧 IR_DEBUG: Creating IrFunction '{}' with {} parameters",
            func.name,
            parameters.len()
        );
        for (i, param) in parameters.iter().enumerate() {
            log::debug!(
                "🔧 IR_DEBUG: Final parameter [{}/{}]: name='{}', ir_id={}, slot={}",
                i + 1,
                parameters.len(),
                param.name,
                param.ir_id,
                param.slot
            );
        }

        // Only use mangled names when there are actual overloads
        let final_name = if let Some(overloads) = self.function_overloads.get(&func.name) {
            if overloads.len() > 1 {
                // This function has overloads, use mangled name
                self.mangle_function_name(&func.name, &specialization)
            } else {
                // Single function, keep original name
                func.name.clone()
            }
        } else {
            // No overloads registered yet, keep original name
            func.name.clone()
        };

        Ok(IrFunction {
            id: func_id,
            name: final_name,
            parameters,
            return_type: func.return_type,
            body,
            local_vars,
        })
    }
}
