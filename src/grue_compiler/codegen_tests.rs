// Code Generation Tests for Grue Compiler

#[cfg(test)]
mod tests {
    use crate::grue_compiler::codegen::ZMachineCodeGen;
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
}
