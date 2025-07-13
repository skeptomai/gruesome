#[cfg(test)]
mod tests {
    use crate::disassembler::{Disassembler, disassemble_instructions, disassemble_range};
    
    #[test]
    fn test_basic_disassembly() {
        // Simple test with a few known instructions
        let memory = vec![
            0xb0,                   // RTRUE (0OP form: 10110000)
            0xb1,                   // RFALSE (0OP form: 10110001)  
            0x0d, 0x10, 0x05,       // STORE G00, #05 (2OP long form)
        ];
        
        let disasm = Disassembler::new(&memory);
        let result = disasm.disassemble(0, Some(3), None).unwrap();
        
        assert!(result.contains("RTRUE"));
        assert!(result.contains("RFALSE"));
        assert!(result.contains("STORE"));
        // STORE shows operands, not the implicit destination
        assert!(result.contains("#0x10"));
        assert!(result.contains("#0x05"));
    }
    
    #[test]
    fn test_operand_formatting() {
        // Test different operand types - using long form instructions
        let memory = vec![
            // Long form: bit 7 clear, bits 6-5 determine operand types
            0x0e,       // INSERT_OBJ: 00001110 (both small constants)
            0x12,       // object #0x12
            0x34,       // destination #0x34
            
            0x74,       // ADD: 01110100 (bit 6 set = first op is variable, bit 5 clear = second is small constant)
            0x05,       // variable L04
            0x10,       // small constant #0x10
            0x20,       // store to variable G10
        ];
        
        let result = disassemble_instructions(&memory, 0, 2).unwrap();
        
        assert!(result.contains("#0x12"));   // Small constant  
        assert!(result.contains("#0x34"));   // Small constant
        assert!(result.contains("L04"));      // Local variable
        assert!(result.contains("-> G10"));   // Store variable
    }
    
    #[test]
    fn test_branch_formatting() {
        // Test branch instruction formatting
        let memory = vec![
            // JZ - short form with branch on true
            0xa0,       // 10100000 - short form, operand type variable
            0x00,       // variable (SP)
            0xc3,       // branch byte: 11000011 (branch on true, single byte, offset +3)
            
            // Padding
            0x00, 0x00, 0x00, 0x00, 0x00,
            
            // JZ - short form with branch on false
            0xa0,       // 10100000 - short form, operand type variable
            0x00,       // variable (SP)
            0x43,       // branch byte: 01000011 (branch on false, single byte, offset +3)
            
            // More padding
            0x00, 0x00, 0x00,
        ];
        
        // Disassemble first instruction
        let result1 = disassemble_instructions(&memory, 0, 1).unwrap();
        assert!(result1.contains("JZ"));
        assert!(result1.contains("[TRUE:"));
        
        // Disassemble second instruction
        let result2 = disassemble_instructions(&memory, 8, 1).unwrap();
        assert!(result2.contains("JZ"));
        assert!(result2.contains("[FALSE:"));
    }
    
    #[test]
    fn test_variable_instruction() {
        // Test variable form instruction (CALL)
        let memory = vec![
            0xe0,       // 11100000 - variable form, VAR operands
            0x0f,       // 00001111 - operand types: large, large, omitted, omitted
            0x12, 0x34, // large constant #0x1234
            0x00, 0x05, // large constant #0x0005
            0x10,       // store to G00
        ];
        
        let result = disassemble_instructions(&memory, 0, 1).unwrap();
        
        assert!(result.contains("CALL"));
        assert!(result.contains("#0x1234"));
        assert!(result.contains("#0x0005"));
        assert!(result.contains("-> G00"));
    }
    
    #[test]
    fn test_range_disassembly() {
        // Test disassembling a specific byte range
        let memory = vec![
            0xb0,                   // RTRUE (1 byte)
            0xb1,                   // RFALSE (1 byte)
            0x0d, 0x10, 0x05,       // STORE (3 bytes)
        ];
        
        // Disassemble only first 2 bytes (should get RTRUE and RFALSE)
        let result = disassemble_range(&memory, 0, 2).unwrap();
        
        assert!(result.contains("RTRUE"));
        assert!(result.contains("RFALSE"));
        assert!(!result.contains("STORE")); // Should not include STORE
        assert!(result.contains("Disassembled 2 instructions"));
    }
}