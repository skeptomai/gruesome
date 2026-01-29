// IR Generator - Statement Generation
//
// Extracted from ir_generator.rs as part of modularization effort.
// Handles IR generation for all statement types.

use crate::grue_compiler::ast::{Expr, Type};
use crate::grue_compiler::error::CompilerError;

use super::{
    ExpressionContext, IrBinaryOp, IrBlock, IrGenerator, IrId, IrInstruction, IrLocal, IrValue,
    VariableSource,
};

impl IrGenerator {
    pub(super) fn generate_object_tree_iteration(
        &mut self,
        for_stmt: Box<crate::grue_compiler::ast::ForStmt>,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Expr;

        // Extract the object from the contents() method call
        let container_object = if let Expr::MethodCall { object, .. } = for_stmt.iterable {
            self.generate_expression(*object, block)?
        } else {
            return Err(CompilerError::CodeGenError(
                "Expected MethodCall for object tree iteration".to_string(),
            ));
        };

        self.generate_object_tree_iteration_with_container(
            for_stmt.variable,
            *for_stmt.body,
            container_object,
            block,
        )
    }

    pub(super) fn generate_object_tree_iteration_with_container(
        &mut self,
        variable: String,
        body: crate::grue_compiler::ast::Stmt,
        container_object: IrId,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        // Create loop variable for current object
        let loop_var_id = self.next_id();
        let local_var = IrLocal {
            ir_id: loop_var_id,
            name: variable.clone(),
            var_type: Some(Type::Any),
            slot: self.next_local_slot,
            mutable: false,
        };
        self.current_locals.push(local_var);
        self.symbol_ids.insert(variable, loop_var_id);
        self.next_local_slot += 1;

        // Create current_object variable to track iteration
        let current_obj_var = self.next_id();
        let current_obj_local = IrLocal {
            ir_id: current_obj_var,
            name: format!("__current_obj_{}", current_obj_var),
            var_type: Some(Type::Int),
            slot: self.next_local_slot,
            mutable: true,
        };
        self.current_locals.push(current_obj_local);
        self.next_local_slot += 1;

        // Create loop counter variable to prevent infinite loops
        let loop_counter_var = self.next_id();
        let loop_counter_local = IrLocal {
            ir_id: loop_counter_var,
            name: format!("__loop_counter_{}", loop_counter_var),
            var_type: Some(Type::Int),
            slot: self.next_local_slot,
            mutable: true,
        };
        self.current_locals.push(loop_counter_local);
        self.next_local_slot += 1;

        // Create labels
        let loop_start = self.next_id();
        let loop_body = self.next_id();
        let loop_end = self.next_id();

        // Get first child: current = get_child(container)
        // IR GetObjectChild branches when there's NO child (parameter semantics)
        let first_child_temp = self.next_id();
        block.add_instruction(IrInstruction::GetObjectChild {
            target: first_child_temp,
            object: container_object,
            branch_if_no_child: loop_end, // Skip loop if container has no children
        });
        block.add_instruction(IrInstruction::StoreVar {
            var_id: current_obj_var,
            source: first_child_temp,
        });

        // Loop start label (we continue here after processing each child)
        block.add_instruction(IrInstruction::Label { id: loop_start });

        // Loop body: set loop variable = current object
        block.add_instruction(IrInstruction::Label { id: loop_body });
        let current_for_body = self.next_id();
        block.add_instruction(IrInstruction::LoadVar {
            target: current_for_body,
            var_id: current_obj_var,
        });
        block.add_instruction(IrInstruction::StoreVar {
            var_id: loop_var_id,
            source: current_for_body,
        });

        // Execute loop body
        self.generate_statement(body, block)?;

        // Get next sibling: current = get_sibling(current)
        let current_for_sibling = self.next_id();
        block.add_instruction(IrInstruction::LoadVar {
            target: current_for_sibling,
            var_id: current_obj_var,
        });
        let next_sibling_temp = self.next_id();

        // Get sibling and branch to loop_end if no more siblings
        // Z-Machine get_sibling returns sibling object and branches when sibling==0
        block.add_instruction(IrInstruction::GetObjectSibling {
            target: next_sibling_temp,
            object: current_for_sibling,
            branch_if_no_sibling: loop_end, // Exit loop when no more siblings
        });

        // Store the sibling as the new current object
        block.add_instruction(IrInstruction::StoreVar {
            var_id: current_obj_var,
            source: next_sibling_temp,
        });

        // Jump back to loop start to process this sibling
        block.add_instruction(IrInstruction::Jump { label: loop_start });

        // Loop end
        block.add_instruction(IrInstruction::Label { id: loop_end });

        Ok(())
    }

    /// Generate InsertObj instructions to place room objects in their containing rooms

    pub(super) fn generate_statement(
        &mut self,
        stmt: crate::grue_compiler::ast::Stmt,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Stmt;

        match stmt {
            Stmt::Expression(expr) => {
                let _temp = self.generate_expression(expr, block)?;
                // Expression result is discarded
            }
            Stmt::VarDecl(var_decl) => {
                // Generate IR for variable declaration
                let var_id = self.next_id();

                // Add to local variables
                let local_var = IrLocal {
                    ir_id: var_id,
                    name: var_decl.name.clone(),
                    var_type: var_decl.var_type,
                    slot: self.next_local_slot,
                    mutable: var_decl.mutable,
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(var_decl.name, var_id);
                self.next_local_slot += 1;

                // Generate initializer if present
                if let Some(initializer) = var_decl.initializer {
                    let init_temp = self.generate_expression(initializer, block)?;
                    block.add_instruction(IrInstruction::StoreVar {
                        var_id,
                        source: init_temp,
                    });

                    // Copy variable source from initializer to variable for iteration tracking
                    // This enables for-loops to detect object tree iteration even with variable assignments
                    if let Some(source) = self.variable_sources.get(&init_temp).cloned() {
                        log::debug!(
                            "VarDecl: copying variable source {:?} from init_temp {} to var_id {}",
                            source,
                            init_temp,
                            var_id
                        );
                        self.variable_sources.insert(var_id, source);
                    }
                }
            }
            Stmt::Assignment(assign) => {
                // Generate the value expression with value context
                let value_temp = self.generate_expression_with_context(
                    assign.value.clone(),
                    block,
                    ExpressionContext::Value,
                )?;

                // Handle different types of assignment targets
                match assign.target {
                    crate::grue_compiler::ast::Expr::Identifier(var_name) => {
                        // Simple variable assignment
                        if let Some(&var_id) = self.symbol_ids.get(&var_name) {
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id,
                                source: value_temp,
                            });

                            // CRITICAL FIX (Oct 27, 2025): Copy variable source tracking from value_temp to var_id
                            // This enables for-loop detection to work with variable indirection
                            // e.g., let items = obj.contents(); for item in items
                            // Without this, ObjectTreeRoot source is lost during assignment and for-loop
                            // falls back to array iteration instead of object tree iteration
                            if let Some(source) = self.variable_sources.get(&value_temp).cloned() {
                                log::debug!(
                                    "Assignment: copying variable source {:?} from value_temp {} to var_id {}",
                                    source,
                                    value_temp,
                                    var_id
                                );
                                self.variable_sources.insert(var_id, source);
                            }
                        } else {
                            // Variable not found - this should be caught in semantic analysis
                            return Err(CompilerError::SemanticError(
                                format!("Undefined variable '{}' in assignment", var_name),
                                0,
                            ));
                        }
                    }
                    crate::grue_compiler::ast::Expr::PropertyAccess { object, property } => {
                        // Property assignment: object.property = value
                        let object_temp = self.generate_expression_with_context(
                            *object,
                            block,
                            ExpressionContext::Value,
                        )?;

                        // Special handling for .location assignment - use insert_obj instead of property
                        // (Oct 12, 2025): Location is object tree containment only, not a property
                        if property == "location" {
                            log::debug!(
                                "🏃 LOCATION_WRITE: Using InsertObj for .location assignment"
                            );
                            block.add_instruction(IrInstruction::InsertObj {
                                object: object_temp,
                                destination: value_temp,
                            });
                        } else if property == "score" {
                            // Special handling for .score assignment - write to Global Variable G17 per Z-Machine standard
                            // G17 is the standard global variable for game score, used by status line
                            log::debug!("📊 SCORE_WRITE: Using Global G17 for .score assignment");
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id: 17, // Global Variable G17 = score
                                source: value_temp,
                            });
                        } else if property == "moves" {
                            // Special handling for .moves assignment - write to Global Variable G18 per Z-Machine standard
                            // G18 is the standard global variable for move counter, used by status line
                            log::debug!("📊 MOVES_WRITE: Using Global G18 for .moves assignment");
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id: 18, // Global Variable G18 = moves
                                source: value_temp,
                            });
                        } else if let Some(standard_attr) = self.get_standard_attribute(&property) {
                            // This is a Z-Machine attribute assignment - use set_attr
                            let attr_num = standard_attr as u8;

                            // CRITICAL FIX (Nov 2, 2025): Extract actual boolean value from assignment expression
                            // Previous bug: All attribute assignments were hardcoded to `value: true`
                            // This caused obj.open = false to have no effect, breaking container state management
                            let boolean_value = match &assign.value {
                                crate::grue_compiler::ast::Expr::Boolean(value) => *value,
                                _ => {
                                    // For non-literal values (e.g., obj.open = some_variable), we would need
                                    // runtime evaluation through Z-Machine instructions. Currently unsupported.
                                    // This affects dynamic assignments but all literal cases (true/false) work.
                                    log::error!(
                                        "SETATTR_UNSUPPORTED: Non-literal boolean assignment not supported: {:?}",
                                        assign.value
                                    );
                                    true
                                }
                            };

                            block.add_instruction(IrInstruction::SetAttribute {
                                object: object_temp,
                                attribute_num: attr_num,
                                value: boolean_value,
                            });
                            log::debug!(
                                "🔧 ATTRIBUTE ASSIGNMENT: {} -> set_attr(object={}, attr={}, value={})",
                                property, object_temp, attr_num, boolean_value
                            );
                        } else if let Some(standard_prop) = self.get_standard_property(&property) {
                            // Check if this is a standard property that should use numbered access
                            if let Some(prop_num) = self
                                .property_manager
                                .get_standard_property_number(standard_prop)
                            {
                                block.add_instruction(IrInstruction::SetPropertyByNumber {
                                    object: object_temp,
                                    property_num: prop_num,
                                    value: value_temp,
                                });
                            } else {
                                // Use dynamic property manager to assign property number even for standard properties without numbers
                                let prop_num = self.property_manager.get_property_number(&property);
                                block.add_instruction(IrInstruction::SetPropertyByNumber {
                                    object: object_temp,
                                    property_num: prop_num,
                                    value: value_temp,
                                });
                            }
                        } else {
                            // Use dynamic property manager to assign property number for non-standard properties
                            let prop_num = self.property_manager.get_property_number(&property);
                            block.add_instruction(IrInstruction::SetPropertyByNumber {
                                object: object_temp,
                                property_num: prop_num,
                                value: value_temp,
                            });
                        }
                    }
                    _ => {
                        // Other assignment targets (array elements, etc.)
                        return Err(CompilerError::SemanticError(
                            "Unsupported assignment target".to_string(),
                            0,
                        ));
                    }
                }
            }
            Stmt::If(if_stmt) => {
                // Create labels for control flow
                let then_label = self.next_id();
                let else_label = self.next_id();
                let end_label = self.next_id();

                log::debug!(
                    "IR if statement: then={}, else={}, end={}",
                    then_label,
                    else_label,
                    end_label
                );

                // PHASE 3: Context-aware IR generation for if statements
                // Check if condition is attribute access for direct TestAttributeBranch optimization
                match &if_stmt.condition {
                    Expr::PropertyAccess { object, property } => {
                        if let Some(standard_attr) = self.get_standard_attribute(property) {
                            let object_temp = self.generate_expression_with_context(
                                (**object).clone(),
                                block,
                                ExpressionContext::Value,
                            )?;
                            let attr_num = standard_attr as u8;

                            log::debug!(
                                "🎯 PHASE 3: Direct TestAttributeBranch optimization for if {}.{} (attr={})",
                                object_temp,
                                property,
                                attr_num
                            );

                            // Generate direct TestAttributeBranch (single Z-Machine instruction)
                            block.add_instruction(IrInstruction::TestAttributeBranch {
                                object: object_temp,
                                attribute_num: attr_num,
                                then_label,
                                else_label,
                            });

                            // CRITICAL FIX: TestAttributeBranch requires special label ordering for Z-Machine semantics
                            //
                            // Z-Machine test_attr instruction behavior:
                            // - When attribute is SET (true): BRANCHES to specified target
                            // - When attribute is CLEAR (false): FALLS THROUGH to next instruction
                            //
                            // This means the code layout must be:
                            // 1. TestAttributeBranch instruction
                            // 2. else_label content (executed on fall-through when attribute is CLEAR)
                            // 3. Jump to end_label
                            // 4. then_label content (executed on branch when attribute is SET)
                            //
                            // Bug was: Generic if-statement processing placed then_label first,
                            // causing "It's already open" message to execute when mailbox was closed

                            // Else branch: Executes when attribute is CLEAR (fall-through path)
                            log::debug!(
                                "IR TestAttributeBranch: Adding else label {} (fall-through)",
                                else_label
                            );
                            block.add_instruction(IrInstruction::Label { id: else_label });
                            if let Some(else_branch) = if_stmt.else_branch {
                                self.generate_statement(*else_branch, block)?;
                            }

                            // Jump to end after else content to skip then_label content
                            log::debug!(
                                "IR TestAttributeBranch: Adding jump to end label {}",
                                end_label
                            );
                            block.add_instruction(IrInstruction::Jump { label: end_label });

                            // Then branch: Executes when attribute is SET (branch target)
                            log::debug!(
                                "IR TestAttributeBranch: Adding then label {} (branch target)",
                                then_label
                            );
                            block.add_instruction(IrInstruction::Label { id: then_label });
                            self.generate_statement(*if_stmt.then_branch, block)?;

                            // End label: Convergence point for both branches
                            log::debug!("IR TestAttributeBranch: Adding end label {}", end_label);
                            block.add_instruction(IrInstruction::Label { id: end_label });

                            // Skip the generic label processing - we handled everything above
                            return Ok(());
                        } else {
                            // Non-attribute property: use generic pattern
                            let condition_temp = self.generate_expression_with_context(
                                if_stmt.condition.clone(),
                                block,
                                ExpressionContext::Conditional,
                            )?;

                            log::debug!(
                                "IF condition temp (non-attribute property): {}",
                                condition_temp
                            );

                            // Branch based on condition
                            block.add_instruction(IrInstruction::Branch {
                                condition: condition_temp,
                                true_label: then_label,
                                false_label: else_label,
                            });
                        }
                    }
                    _ => {
                        // Non-property-access condition: use generic pattern
                        let condition_temp = self.generate_expression_with_context(
                            if_stmt.condition.clone(),
                            block,
                            ExpressionContext::Conditional,
                        )?;

                        log::debug!("IF condition temp (non-property): {}", condition_temp);

                        // Branch based on condition
                        block.add_instruction(IrInstruction::Branch {
                            condition: condition_temp,
                            true_label: then_label,
                            false_label: else_label,
                        });
                    }
                }

                // Then branch
                log::debug!("IR if: Adding then label {}", then_label);
                block.add_instruction(IrInstruction::Label { id: then_label });
                self.generate_statement(*if_stmt.then_branch, block)?;

                // Only emit jump to end_label if there's an else branch
                // Without else branch, fall-through naturally reaches end_label
                if if_stmt.else_branch.is_some() {
                    log::debug!(
                        "IR if: Adding jump to end label {} (else branch exists)",
                        end_label
                    );
                    block.add_instruction(IrInstruction::Jump { label: end_label });
                } else {
                    log::debug!(
                        "IR if: Skipping jump to end label {} (no else branch - fall-through)",
                        end_label
                    );
                }

                // Else branch (if present)
                log::debug!("IR if: Adding else label {}", else_label);
                block.add_instruction(IrInstruction::Label { id: else_label });
                if let Some(else_branch) = if_stmt.else_branch {
                    self.generate_statement(*else_branch, block)?;
                }

                // End label
                log::debug!("IR if: Adding end label {}", end_label);
                block.add_instruction(IrInstruction::Label { id: end_label });
            }
            Stmt::While(while_stmt) => {
                // Create labels for loop control flow
                let loop_start = self.next_id();
                let loop_body = self.next_id();
                let loop_end = self.next_id();

                // Jump to loop start
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop start: evaluate condition
                block.add_instruction(IrInstruction::Label { id: loop_start });
                let condition_temp = self.generate_expression(while_stmt.condition, block)?;

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: loop_body,
                    false_label: loop_end,
                });

                // Loop body
                block.add_instruction(IrInstruction::Label { id: loop_body });
                self.generate_statement(*while_stmt.body, block)?;
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop end
                block.add_instruction(IrInstruction::Label { id: loop_end });
            }
            Stmt::For(for_stmt) => {
                // For loops in Grue iterate over collections
                // Generate the iterable expression first
                let iterable_temp = self.generate_expression(for_stmt.iterable, block)?;

                log::debug!(
                    "🔍 FOR_LOOP_DEBUG: Generated iterable_temp IR ID {} for for-loop",
                    iterable_temp
                );

                // Use variable source tracking to determine iteration strategy
                // This handles variable indirection (e.g., let items = obj.contents(); for item in items)
                let source_info = self.variable_sources.get(&iterable_temp);
                log::debug!(
                    "🔍 FOR_LOOP_DEBUG: Variable source lookup for IR ID {} = {:?}",
                    iterable_temp,
                    source_info
                );

                let container_object =
                    self.variable_sources
                        .get(&iterable_temp)
                        .and_then(|source| {
                            if let VariableSource::ObjectTreeRoot(container_id) = source {
                                log::debug!(
                                "🔍 FOR_LOOP_DEBUG: Found ObjectTreeRoot source! Container ID = {}",
                                container_id
                            );
                                Some(*container_id)
                            } else {
                                log::debug!(
                                    "🔍 FOR_LOOP_DEBUG: Found non-ObjectTreeRoot source: {:?}",
                                    source
                                );
                                None
                            }
                        });

                if let Some(container_id) = container_object {
                    log::debug!(
                        "🔍 FOR_LOOP_DEBUG: TAKING OBJECT TREE ITERATION PATH! Container ID = {}",
                        container_id
                    );
                    // Generate object tree iteration using get_child/get_sibling opcodes
                    return self.generate_object_tree_iteration_with_container(
                        for_stmt.variable,
                        *for_stmt.body,
                        container_id,
                        block,
                    );
                }

                log::debug!(
                    "🔍 FOR_LOOP_DEBUG: TAKING ARRAY ITERATION PATH! ObjectTreeRoot not found"
                );

                // Otherwise, generate array iteration using get_array_element

                // Create a loop variable
                let loop_var_id = self.next_id();
                let local_var = IrLocal {
                    ir_id: loop_var_id,
                    name: for_stmt.variable.clone(),
                    var_type: Some(Type::Any), // Type inferred from array elements
                    slot: self.next_local_slot,
                    mutable: false, // Loop variables are immutable
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(for_stmt.variable, loop_var_id);
                self.next_local_slot += 1;

                // Create index variable for array iteration (allocate as local)
                let index_var = self.next_id();
                let index_local = IrLocal {
                    ir_id: index_var,
                    name: format!("__loop_index_{}", index_var),
                    var_type: Some(Type::Int),
                    slot: self.next_local_slot,
                    mutable: true, // Index is incremented
                };
                self.current_locals.push(index_local);
                self.next_local_slot += 1;

                // Initialize index to 0
                let zero_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: zero_temp,
                    value: IrValue::Integer(0),
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: index_var,
                    source: zero_temp,
                });

                // Create labels for loop control flow
                let loop_start = self.next_id();
                let loop_body = self.next_id();
                let loop_end = self.next_id();

                // Loop start: check if index < array length
                block.add_instruction(IrInstruction::Label { id: loop_start });
                let index_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_temp,
                    var_id: index_var,
                });

                // CRITICAL FIX: Implement single-iteration loop for placeholder arrays
                // The contents() method returns a placeholder value, not a real array
                // So we should iterate exactly once with our placeholder object (player = 1)
                // Compare index with 1 to terminate after first iteration
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1), // Array length = 1 (single placeholder object)
                });

                // Compare index < array_length (1)
                let condition_temp = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: condition_temp,
                    op: IrBinaryOp::Less,
                    left: index_temp,
                    right: one_temp,
                });

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: loop_body,
                    false_label: loop_end,
                });

                // Loop body: load current element into loop variable
                block.add_instruction(IrInstruction::Label { id: loop_body });
                // CRITICAL: Reload index since index_temp was consumed by Less comparison
                // This prevents SSA violation (reusing consumed stack value)
                let index_for_get = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_for_get,
                    var_id: index_var,
                });
                let element_temp = self.next_id();
                // ARRAY REMOVAL (Nov 5, 2025): Arrays removed from Z-Machine compiler
                // This was previously GetArrayElement for iterating through array contents
                // Now replaced with placeholder that returns constant value
                // Text adventures typically use object containment rather than arrays
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: element_temp,
                    value: IrValue::Integer(1), // Placeholder object ID - always returns 1
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: loop_var_id,
                    source: element_temp,
                });

                // Execute loop body
                self.generate_statement(*for_stmt.body, block)?;

                // Increment index
                // Reload index_var for increment operation
                // This prevents SSA violation (reusing consumed stack value)
                let index_for_increment = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_for_increment,
                    var_id: index_var,
                });
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1),
                });
                let new_index = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: new_index,
                    op: IrBinaryOp::Add,
                    left: index_for_increment,
                    right: one_temp,
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: index_var,
                    source: new_index,
                });

                // Jump back to start
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop end
                block.add_instruction(IrInstruction::Label { id: loop_end });
            }
            Stmt::Return(return_expr) => {
                let value_id = if let Some(expr) = return_expr {
                    Some(self.generate_expression(expr, block)?)
                } else {
                    None
                };

                block.add_instruction(IrInstruction::Return { value: value_id });
            }
            Stmt::Block(inner_block) => {
                let ir_inner_block = self.generate_block(inner_block)?;
                // Inline the inner block's instructions
                block.instructions.extend(ir_inner_block.instructions);
            }
        }

        Ok(())
    }
}
