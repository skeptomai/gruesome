use crate::zmachine::ZMachine;
use crate::instruction::{Instruction, InstructionForm, OperandCount, Operand, OperandType};

// Create a mock GameFile for testing
struct MockGameFile {
    memory: Vec<u8>,
    version: u8,
}

impl MockGameFile {
    fn new(memory: Vec<u8>) -> Self {
        Self { memory, version: 3 }
    }
    
    fn bytes(&self) -> &[u8] {
        &self.memory
    }
    
    fn version(&self) -> usize {
        self.version as usize
    }
}

#[test]
fn test_je_branch_true() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![42, 42], // Two equal operands
        current_branch_offset: Some(10), // Branch on true, offset 10
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    let result = zmachine.op_je(42, 42);
    if let Err(e) = &result {
        println!("JE branch test error: {}", e);
    }
    assert!(result.is_ok());
    
    // Should have branched forward by offset 10, minus 2 for instruction adjustment
    assert_eq!(zmachine.pc, initial_pc + 10 - 2);
}

#[test]
fn test_je_branch_false() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![42, 99], // Two different operands
        current_branch_offset: Some(10), // Branch on true, offset 10
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    let result = zmachine.op_je(42, 99);
    assert!(result.is_ok());
    
    // Should not have branched, PC unchanged
    assert_eq!(zmachine.pc, initial_pc);
}

#[test]
fn test_jz_branch_true() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![0], // Zero operand
        current_branch_offset: Some(5), // Branch on true, offset 5
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    let result = zmachine.op_jz(0);
    assert!(result.is_ok());
    
    // Should have branched forward by offset 5, minus 2 for instruction adjustment
    assert_eq!(zmachine.pc, initial_pc + 5 - 2);
}

#[test]
fn test_jl_branch_true() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![5, 10], // 5 < 10
        current_branch_offset: Some(8), // Branch on true, offset 8
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    let result = zmachine.op_jl(5, 10);
    assert!(result.is_ok());
    
    // Should have branched forward by offset 8, minus 2 for instruction adjustment
    assert_eq!(zmachine.pc, initial_pc + 8 - 2);
}

#[test]
fn test_jg_branch_false() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![5, 10], // 5 not > 10
        current_branch_offset: Some(8), // Branch on true, offset 8
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    let result = zmachine.op_jg(5, 10);
    assert!(result.is_ok());
    
    // Should not have branched, PC unchanged
    assert_eq!(zmachine.pc, initial_pc);
}

#[test]
fn test_branch_rfalse() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![42, 42], // Equal operands
        current_branch_offset: Some(0), // Branch offset 0 = rfalse
        random_seed: 0,
    };
    
    // Add a call frame to return from
    zmachine.call_stack.push(crate::zmachine::CallFrame {
        return_pc: 0x2000,
        local_vars: [0; 15],
        num_locals: 0,
        result_var: Some(0),
    });
    
    let result = zmachine.op_je(42, 42);
    assert!(result.is_ok());
    
    // Should have returned with value 0
    assert_eq!(zmachine.pc, 0x2000);
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0);
}

#[test]
fn test_branch_rtrue() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![42, 42], // Equal operands
        current_branch_offset: Some(1), // Branch offset 1 = rtrue
        random_seed: 0,
    };
    
    // Add a call frame to return from
    zmachine.call_stack.push(crate::zmachine::CallFrame {
        return_pc: 0x2000,
        local_vars: [0; 15],
        num_locals: 0,
        result_var: Some(0),
    });
    
    let result = zmachine.op_je(42, 42);
    assert!(result.is_ok());
    
    // Should have returned with value 1
    assert_eq!(zmachine.pc, 0x2000);
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 1);
}

#[test]
fn test_branch_negative_offset() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![42, 42], // Equal operands
        current_branch_offset: Some(-5), // Branch on false (encoded as negative)
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // JE should be true, but branch offset is negative (branch on false)
    let result = zmachine.op_je(42, 42);
    assert!(result.is_ok());
    
    // Should not have branched since condition is true but we branch on false
    assert_eq!(zmachine.pc, initial_pc);
}

#[test]
fn test_test_instruction_branch() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![0b11110000, 0b00110000], // Test bits
        current_branch_offset: Some(7), // Branch on true, offset 7
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // TEST: (0b11110000 & 0b00110000) == 0b00110000 should be true
    let result = zmachine.op_test(0b11110000, 0b00110000);
    assert!(result.is_ok());
    
    // Should have branched forward by offset 7, minus 2 for instruction adjustment
    assert_eq!(zmachine.pc, initial_pc + 7 - 2);
}

#[test]
fn test_dec_chk_branch_true() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [10; 15], // Local variable 1 = 10
        running: true,
        operands_buffer: vec![1, 5], // dec var 1, check if < 5
        current_branch_offset: Some(12), // Branch on true, offset 12
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // DEC_CHK: decrement local var 1 (10->9), check if 9 < 5 (false, no branch)
    let result = zmachine.op_dec_chk(1, 5);
    assert!(result.is_ok());
    
    // Should not have branched since 9 is not < 5
    assert_eq!(zmachine.pc, initial_pc);
    // Variable should be decremented
    assert_eq!(zmachine.local_vars[0], 9);
}

#[test]
fn test_dec_chk_branch_false() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [3; 15], // Local variable 1 = 3
        running: true,
        operands_buffer: vec![1, 5], // dec var 1, check if < 5
        current_branch_offset: Some(8), // Branch on true, offset 8
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // DEC_CHK: decrement local var 1 (3->2), check if 2 < 5 (true, branch)
    let result = zmachine.op_dec_chk(1, 5);
    assert!(result.is_ok());
    
    // Should have branched since 2 < 5
    assert_eq!(zmachine.pc, initial_pc + 8 - 2);
    // Variable should be decremented
    assert_eq!(zmachine.local_vars[0], 2);
}

#[test]
fn test_inc_chk_branch_true() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [4; 15], // Local variable 1 = 4
        running: true,
        operands_buffer: vec![1, 3], // inc var 1, check if > 3
        current_branch_offset: Some(6), // Branch on true, offset 6
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // INC_CHK: increment local var 1 (4->5), check if 5 > 3 (true, branch)
    let result = zmachine.op_inc_chk(1, 3);
    assert!(result.is_ok());
    
    // Should have branched since 5 > 3
    assert_eq!(zmachine.pc, initial_pc + 6 - 2);
    // Variable should be incremented
    assert_eq!(zmachine.local_vars[0], 5);
}

#[test]
fn test_inc_chk_with_global_variable() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![16, 10], // inc global var 16, check if > 10
        current_branch_offset: Some(4), // Branch on true, offset 4
        random_seed: 0,
    };
    
    // Set global variable 16 to 10
    zmachine.global_vars.insert(16, 10);
    
    let initial_pc = zmachine.pc;
    
    // INC_CHK: increment global var 16 (10->11), check if 11 > 10 (true, branch)
    let result = zmachine.op_inc_chk(16, 10);
    assert!(result.is_ok());
    
    // Should have branched since 11 > 10
    assert_eq!(zmachine.pc, initial_pc + 4 - 2);
    // Global variable should be incremented
    assert_eq!(*zmachine.global_vars.get(&16).unwrap(), 11);
}

#[test]
fn test_object_operations_setup() {
    // Test basic object table access with a mock setup
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Set up header - property defaults at 0x100
    memory[10] = 0x01; // High byte
    memory[11] = 0x00; // Low byte = 0x100
    
    // Object table starts at 0x100 + 62 = 0x13E
    let obj_table_start = 0x13E;
    
    // Object 1 at 0x13E - set up some test data
    // Attributes (4 bytes): 0x80, 0x00, 0x00, 0x00 (attribute 0 set)
    memory[obj_table_start] = 0x80;
    memory[obj_table_start + 1] = 0x00;
    memory[obj_table_start + 2] = 0x00;
    memory[obj_table_start + 3] = 0x00;
    // Parent: 2, Sibling: 3, Child: 4
    memory[obj_table_start + 4] = 2;  // parent
    memory[obj_table_start + 5] = 3;  // sibling
    memory[obj_table_start + 6] = 4;  // child
    
    // Object 2 at 0x13E + 9 = 0x147
    memory[obj_table_start + 9 + 4] = 0;  // parent
    memory[obj_table_start + 9 + 5] = 0;  // sibling
    memory[obj_table_start + 9 + 6] = 1;  // child
    
    let mock_game = MockGameFile::new(memory.clone());
    
    let zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test object relationships
    assert_eq!(zmachine.get_object_parent(1).unwrap(), 2);
    assert_eq!(zmachine.get_object_sibling(1).unwrap(), 3);
    assert_eq!(zmachine.get_object_child(1).unwrap(), 4);
    
    // Test attribute
    assert!(zmachine.test_object_attribute(1, 0).unwrap());  // attribute 0 is set
    assert!(!zmachine.test_object_attribute(1, 1).unwrap()); // attribute 1 is not set
}

#[test]
fn test_jin_instruction() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Set up header
    memory[10] = 0x01;
    memory[11] = 0x00;
    
    let obj_table_start = 0x13E;
    
    // Object 1: parent=2
    memory[obj_table_start + 4] = 2;
    
    // Object 2: parent=0
    memory[obj_table_start + 9 + 4] = 0;
    
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![1, 2], // Check if object 1 is in object 2
        current_branch_offset: Some(5), // Branch on true, offset 5
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // JIN: object 1 parent is 2, so this should branch
    let result = zmachine.op_jin(1, 2);
    assert!(result.is_ok());
    
    // Should have branched
    assert_eq!(zmachine.pc, initial_pc + 5 - 2);
}

#[test]
fn test_test_attr_instruction() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Set up header
    memory[10] = 0x01;
    memory[11] = 0x00;
    
    let obj_table_start = 0x13E;
    
    // Object 1: set attribute 0 and 15
    memory[obj_table_start] = 0x80;     // attribute 0 set (bit 7)
    memory[obj_table_start + 1] = 0x01; // attribute 15 set (bit 0)
    
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![1, 0], // Test attribute 0 on object 1
        current_branch_offset: Some(3), // Branch on true, offset 3
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // TEST_ATTR: object 1 has attribute 0, so this should branch
    let result = zmachine.op_test_attr(1, 0);
    assert!(result.is_ok());
    
    // Should have branched
    assert_eq!(zmachine.pc, initial_pc + 3 - 2);
}

#[test]
fn test_get_sibling_instruction() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Set up header
    memory[10] = 0x01;
    memory[11] = 0x00;
    
    let obj_table_start = 0x13E;
    
    // Object 1: sibling=5
    memory[obj_table_start + 5] = 5;
    
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![1], // Get sibling of object 1
        current_branch_offset: Some(7), // Branch on true, offset 7
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // GET_SIBLING: object 1 has sibling 5, so store 5 and branch
    let result = zmachine.op_get_sibling(1);
    assert!(result.is_ok());
    
    // Should have branched (sibling != 0)
    assert_eq!(zmachine.pc, initial_pc + 7 - 2);
    // Should have stored sibling number on stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 5);
}

#[test]
fn test_get_child_no_branch() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Set up header
    memory[10] = 0x01;
    memory[11] = 0x00;
    
    let obj_table_start = 0x13E;
    
    // Object 1: child=0 (no child)
    memory[obj_table_start + 6] = 0;
    
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![1], // Get child of object 1
        current_branch_offset: Some(4), // Branch on true, offset 4
        random_seed: 0,
    };
    
    let initial_pc = zmachine.pc;
    
    // GET_CHILD: object 1 has no child (0), so store 0 and don't branch
    let result = zmachine.op_get_child(1);
    assert!(result.is_ok());
    
    // Should not have branched (child == 0)
    assert_eq!(zmachine.pc, initial_pc);
    // Should have stored 0 on stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0);
}