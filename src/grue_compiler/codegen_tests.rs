// Code Generation Tests for Grue Compiler

#[cfg(test)]
mod codegen_tests {
    use crate::grue_compiler::ast::Type;
    use crate::grue_compiler::codegen::{
        InstructionForm, Operand, OperandType, ReferenceType, ZMachineCodeGen,
    };
    use crate::grue_compiler::ir::*;
    use crate::grue_compiler::ZMachineVersion;

    fn create_minimal_ir() -> IrProgram {
        IrProgram {
            functions: Vec::new(),
            globals: Vec::new(),
            rooms: Vec::new(),
            objects: Vec::new(),
            grammar: Vec::new(),
            init_block: None,
            string_table: std::collections::HashMap::new(),
            property_defaults: IrPropertyDefaults {
                defaults: std::collections::HashMap::new(),
            },
        }
    }

    #[test]
    fn test_minimal_code_generation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        let ir = create_minimal_ir();

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();

        // Check basic header structure
        assert!(story_data.len() >= 64); // At least header size
        assert_eq!(story_data[0], 3); // Version 3
    }

    #[test]
    fn test_v5_code_generation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V5);
        let ir = create_minimal_ir();

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();

        // Check basic header structure
        assert!(story_data.len() >= 64); // At least header size
        assert_eq!(story_data[0], 5); // Version 5
    }

    #[test]
    fn test_function_generation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Create IR with a simple function
        let mut ir = create_minimal_ir();
        let func_id = 1;
        let block_id = 2;

        let function = IrFunction {
            id: func_id,
            name: "test_func".to_string(),
            parameters: Vec::new(),
            return_type: None,
            body: IrBlock {
                id: block_id,
                instructions: vec![
                    IrInstruction::LoadImmediate {
                        target: 10,
                        value: IrValue::Integer(42),
                    },
                    IrInstruction::Return { value: Some(10) },
                ],
            },
            local_vars: Vec::new(),
        };

        ir.functions.push(function);

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();
        assert!(story_data.len() > 64); // Should have code beyond header
    }

    #[test]
    fn test_string_encoding() {
        let codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test basic string encoding
        let encoded = codegen.encode_string("hello").unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.len() >= 2); // At least one Z-Machine word

        // Check that the string is properly terminated (high bit set on last word)
        let last_word_high = encoded[encoded.len() - 2];
        assert!(last_word_high & 0x80 != 0); // High bit should be set
    }

    #[test]
    fn test_empty_string_encoding() {
        let codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        let encoded = codegen.encode_string("").unwrap();
        assert_eq!(encoded.len(), 2); // Should be exactly one word
        assert_eq!(encoded[0], 0x80); // High byte with termination bit set
        assert_eq!(encoded[1], 0x00); // Low byte
    }

    #[test]
    fn test_header_generation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        let ir = create_minimal_ir();

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();

        // Check key header fields
        assert_eq!(story_data[0], 3); // Version

        // Check that addresses are non-zero (indicating proper layout)
        let dict_addr = (story_data[8] as u16) << 8 | (story_data[9] as u16);
        let obj_table_addr = (story_data[10] as u16) << 8 | (story_data[11] as u16);
        let globals_addr = (story_data[12] as u16) << 8 | (story_data[13] as u16);

        assert!(dict_addr >= 64); // Dictionary should be at or after header
        assert!(obj_table_addr >= 64); // Object table should be at or after header
        assert!(globals_addr >= 64); // Globals should be at or after header
    }

    #[test]
    fn test_init_block_generation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Create IR with init block
        let mut ir = create_minimal_ir();
        ir.init_block = Some(IrBlock {
            id: 1,
            instructions: vec![IrInstruction::LoadImmediate {
                target: 5,
                value: IrValue::Integer(1),
            }],
        });

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();
        assert!(story_data.len() > 64); // Should have additional code
    }

    #[test]
    fn test_complex_program_generation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Create a more complex IR program
        let mut ir = create_minimal_ir();

        // Add a string to the string table
        ir.string_table.insert("Hello, World!".to_string(), 100);

        // Add a function with control flow
        let function = IrFunction {
            id: 1,
            name: "main".to_string(),
            parameters: Vec::new(),
            return_type: None,
            body: IrBlock {
                id: 2,
                instructions: vec![
                    IrInstruction::LoadImmediate {
                        target: 10,
                        value: IrValue::Integer(5),
                    },
                    IrInstruction::LoadImmediate {
                        target: 11,
                        value: IrValue::Integer(3),
                    },
                    IrInstruction::BinaryOp {
                        target: 12,
                        op: IrBinaryOp::Add,
                        left: 10,
                        right: 11,
                    },
                    IrInstruction::Return { value: Some(12) },
                ],
            },
            local_vars: Vec::new(),
        };

        ir.functions.push(function);

        // Add init block
        ir.init_block = Some(IrBlock {
            id: 3,
            instructions: vec![IrInstruction::Call {
                target: Some(20),
                function: 1,
                args: Vec::new(),
            }],
        });

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();

        // Verify the program is non-trivial
        assert!(story_data.len() > 100); // Should be reasonably sized

        // Verify header is properly set up
        assert_eq!(story_data[0], 3); // Version

        // Check that all major data structures have been placed
        let globals_addr = (story_data[12] as u16) << 8 | (story_data[13] as u16);
        assert!(globals_addr >= 64); // Should be after header
    }

    #[test]
    fn test_operand_encoding() {
        let codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test operand type detection
        assert_eq!(
            codegen.get_operand_type(&Operand::SmallConstant(5)),
            OperandType::SmallConstant
        );
        assert_eq!(
            codegen.get_operand_type(&Operand::LargeConstant(300)),
            OperandType::LargeConstant
        );
        assert_eq!(
            codegen.get_operand_type(&Operand::Variable(1)),
            OperandType::Variable
        );
        assert_eq!(
            codegen.get_operand_type(&Operand::Constant(42)),
            OperandType::SmallConstant
        );
        assert_eq!(
            codegen.get_operand_type(&Operand::Constant(300)),
            OperandType::LargeConstant
        );
    }

    #[test]
    fn test_instruction_form_detection() {
        let codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test instruction form detection
        assert_eq!(
            codegen.determine_instruction_form(0, 0x00),
            InstructionForm::Short
        );
        assert_eq!(
            codegen.determine_instruction_form(1, 0x00),
            InstructionForm::Short
        );
        assert_eq!(
            codegen.determine_instruction_form(2, 0x14),
            InstructionForm::Long
        );
        assert_eq!(
            codegen.determine_instruction_form(3, 0x00),
            InstructionForm::Variable
        );
        assert_eq!(
            codegen.determine_instruction_form(4, 0x00),
            InstructionForm::Variable
        );
    }

    #[test]
    fn test_short_form_instruction_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test 0OP form encoding (rtrue)
        let result = codegen.emit_instruction(0x00, &[], None, None);
        assert!(result.is_ok());

        // Check that at least one byte was written
        assert!(codegen.current_address > 64); // Past header

        // Reset for next test
        codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test 1OP form encoding
        let operands = vec![Operand::SmallConstant(42)];
        let result = codegen.emit_instruction(0x0B, &operands, None, None);
        assert!(result.is_ok());

        assert!(codegen.current_address > 64); // Should have written instruction
    }

    #[test]
    fn test_long_form_instruction_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test 2OP long form encoding (add)
        let operands = vec![
            Operand::Variable(1),      // Local variable 1
            Operand::SmallConstant(5), // Constant 5
        ];

        let result = codegen.emit_instruction(0x14, &operands, Some(0), None);
        assert!(result.is_ok());

        // Should have written instruction + operands + store variable
        assert!(codegen.current_address > 64);
    }

    #[test]
    fn test_variable_form_instruction_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test VAR form encoding (call_vs)
        let operands = vec![
            Operand::Constant(0x1000),  // Function address
            Operand::Variable(1),       // Argument 1
            Operand::SmallConstant(42), // Argument 2
        ];

        let result = codegen.emit_instruction(0x00, &operands, Some(0), None);
        assert!(result.is_ok());

        // Should have written instruction + types byte + operands + store variable
        assert!(codegen.current_address > 64);
    }

    #[test]
    fn test_branch_offset_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test short branch offset (1 byte)
        let result = codegen.emit_branch_offset(10);
        assert!(result.is_ok());
        let short_bytes = codegen.current_address - 64;

        // Reset
        codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test long branch offset (2 bytes)
        let result = codegen.emit_branch_offset(100);
        assert!(result.is_ok());
        let long_bytes = codegen.current_address - 64;

        // Long offset should use more bytes than short
        assert!(long_bytes > short_bytes);
    }

    #[test]
    fn test_comprehensive_instruction_generation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Create a simple function with various instruction types
        let mut ir = create_minimal_ir();
        let function = IrFunction {
            id: 1,
            name: "test_comprehensive".to_string(),
            parameters: Vec::new(),
            return_type: None,
            body: IrBlock {
                id: 2,
                instructions: vec![
                    // Load immediate values
                    IrInstruction::LoadImmediate {
                        target: 10,
                        value: IrValue::Integer(42),
                    },
                    IrInstruction::LoadImmediate {
                        target: 11,
                        value: IrValue::Boolean(true),
                    },
                    // Binary operation
                    IrInstruction::BinaryOp {
                        target: 12,
                        op: IrBinaryOp::Add,
                        left: 10,
                        right: 11,
                    },
                    // Variable operations
                    IrInstruction::LoadVar {
                        target: 13,
                        var_id: 1,
                    },
                    IrInstruction::StoreVar {
                        var_id: 2,
                        source: 13,
                    },
                    // Print operation
                    IrInstruction::Print { value: 12 },
                    // Return
                    IrInstruction::Return { value: Some(12) },
                ],
            },
            local_vars: vec![
                IrLocal {
                    name: "temp1".to_string(),
                    var_type: Some(Type::Int),
                    slot: 1,
                    mutable: true,
                },
                IrLocal {
                    name: "temp2".to_string(),
                    var_type: Some(Type::Int),
                    slot: 2,
                    mutable: true,
                },
            ],
        };

        ir.functions.push(function);

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();

        // Verify the program generated successfully with all instruction types
        assert!(story_data.len() > 100); // Should be substantial
        assert_eq!(story_data[0], 3); // Correct version

        // Check that function addresses were recorded
        assert!(!codegen.function_addresses.is_empty());
    }

    #[test]
    fn test_address_resolution_system() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test recording addresses
        codegen.record_address(100, 0x1000);
        codegen.record_address(200, 0x2000);

        assert_eq!(
            codegen.reference_context.ir_id_to_address.get(&100),
            Some(&0x1000)
        );
        assert_eq!(
            codegen.reference_context.ir_id_to_address.get(&200),
            Some(&0x2000)
        );
    }

    #[test]
    fn test_unresolved_reference_tracking() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Add unresolved references
        let result1 = codegen.add_unresolved_reference(ReferenceType::Jump, 100, false);
        let result2 = codegen.add_unresolved_reference(ReferenceType::FunctionCall, 200, true);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(codegen.reference_context.unresolved_refs.len(), 2);

        // Check reference types
        assert_eq!(
            codegen.reference_context.unresolved_refs[0].reference_type,
            ReferenceType::Jump
        );
        assert_eq!(
            codegen.reference_context.unresolved_refs[1].reference_type,
            ReferenceType::FunctionCall
        );
        assert!(codegen.reference_context.unresolved_refs[1].is_packed_address);
    }

    #[test]
    fn test_packed_address_calculation() {
        let codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test v3 routine address packing (divide by 2)
        let packed = codegen.pack_routine_address(0x1000).unwrap();
        assert_eq!(packed, 0x800);

        let packed = codegen.pack_string_address(0x2000).unwrap();
        assert_eq!(packed, 0x1000);

        // Test v5 packing
        let codegen_v5 = ZMachineCodeGen::new(ZMachineVersion::V5);
        let packed = codegen_v5.pack_routine_address(0x1000).unwrap();
        assert_eq!(packed, 0x400); // divide by 4
    }

    #[test]
    fn test_packed_address_alignment() {
        let codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Test error for odd address in v3
        let result = codegen.pack_routine_address(0x1001);
        assert!(result.is_err());

        let codegen_v5 = ZMachineCodeGen::new(ZMachineVersion::V5);

        // Test error for non-multiple-of-4 in v5
        let result = codegen_v5.pack_routine_address(0x1002);
        assert!(result.is_err());
    }

    #[test]
    fn test_jump_offset_calculation() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Set up some story data
        codegen.story_data.resize(200, 0);

        // Test jump offset patching
        let result = codegen.patch_jump_offset(100, 150);
        assert!(result.is_ok());

        // Check that the offset was written (150 - 102 = 48)
        let written_offset =
            ((codegen.story_data[100] as u16) << 8) | (codegen.story_data[101] as u16);
        assert_eq!(written_offset, 48);
    }

    #[test]
    fn test_address_patching() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        codegen.story_data.resize(100, 0);

        // Test 1-byte patching
        let result = codegen.patch_address(10, 0x42, 1);
        assert!(result.is_ok());
        assert_eq!(codegen.story_data[10], 0x42);

        // Test 2-byte patching (big-endian)
        let result = codegen.patch_address(20, 0x1234, 2);
        assert!(result.is_ok());
        assert_eq!(codegen.story_data[20], 0x12);
        assert_eq!(codegen.story_data[21], 0x34);
    }

    #[test]
    fn test_function_with_jumps_and_calls() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Create IR with function containing jumps and calls
        let mut ir = create_minimal_ir();

        let function = IrFunction {
            id: 1,
            name: "test_control_flow".to_string(),
            parameters: Vec::new(),
            return_type: None,
            body: IrBlock {
                id: 2,
                instructions: vec![
                    // Label 1
                    IrInstruction::Label { id: 10 },
                    // Load a value
                    IrInstruction::LoadImmediate {
                        target: 100,
                        value: IrValue::Integer(42),
                    },
                    // Jump to label 2
                    IrInstruction::Jump { label: 20 },
                    // Label 2
                    IrInstruction::Label { id: 20 },
                    // Call another function
                    IrInstruction::Call {
                        target: Some(101),
                        function: 1, // Self-recursive call
                        args: Vec::new(),
                    },
                    // Return
                    IrInstruction::Return { value: Some(101) },
                ],
            },
            local_vars: Vec::new(),
        };

        ir.functions.push(function);

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();

        // Verify the program generated successfully with control flow
        assert!(story_data.len() > 100);
        assert_eq!(story_data[0], 3); // Correct version

        // All references should be resolved (empty unresolved list)
        assert_eq!(codegen.reference_context.unresolved_refs.len(), 0);

        // Function and label addresses should be recorded
        assert!(!codegen.reference_context.ir_id_to_address.is_empty());
    }

    #[test]
    fn test_complex_control_flow_resolution() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

        // Create IR with multiple functions and complex control flow
        let mut ir = create_minimal_ir();

        // Function 1: main
        let main_func = IrFunction {
            id: 1,
            name: "main".to_string(),
            parameters: Vec::new(),
            return_type: None,
            body: IrBlock {
                id: 2,
                instructions: vec![
                    IrInstruction::Call {
                        target: Some(10),
                        function: 2, // Call helper function
                        args: Vec::new(),
                    },
                    IrInstruction::Return { value: Some(10) },
                ],
            },
            local_vars: Vec::new(),
        };

        // Function 2: helper
        let helper_func = IrFunction {
            id: 2,
            name: "helper".to_string(),
            parameters: Vec::new(),
            return_type: None,
            body: IrBlock {
                id: 3,
                instructions: vec![
                    IrInstruction::LoadImmediate {
                        target: 20,
                        value: IrValue::Integer(100),
                    },
                    IrInstruction::Jump { label: 30 }, // Jump to end
                    IrInstruction::Label { id: 30 },
                    IrInstruction::Return { value: Some(20) },
                ],
            },
            local_vars: Vec::new(),
        };

        ir.functions.push(main_func);
        ir.functions.push(helper_func);

        let result = codegen.generate(ir);
        assert!(result.is_ok());

        let story_data = result.unwrap();

        // Verify successful generation with multiple functions
        assert!(story_data.len() > 150);
        assert_eq!(story_data[0], 3);

        // All references should be resolved
        assert_eq!(codegen.reference_context.unresolved_refs.len(), 0);

        // Both functions should have recorded addresses
        assert!(codegen.reference_context.ir_id_to_address.contains_key(&1));
        assert!(codegen.reference_context.ir_id_to_address.contains_key(&2));
    }
}
