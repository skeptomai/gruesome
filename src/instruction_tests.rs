use crate::zmachine::ZMachine;
use std::collections::HashMap;

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
fn test_inc_instruction() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [5; 15], // Local variable 1 = 5
        running: true,
        operands_buffer: vec![1], // INC local variable 1
        current_branch_offset: None,
        random_seed: 0,
    };
    
    let result = zmachine.op_inc(1);
    assert!(result.is_ok());
    
    // Variable should be incremented
    assert_eq!(zmachine.local_vars[0], 6);
}

#[test]
fn test_dec_instruction() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [10; 15], // Local variable 1 = 10
        running: true,
        operands_buffer: vec![1], // DEC local variable 1
        current_branch_offset: None,
        random_seed: 0,
    };
    
    let result = zmachine.op_dec(1);
    assert!(result.is_ok());
    
    // Variable should be decremented
    assert_eq!(zmachine.local_vars[0], 9);
}

#[test]
fn test_load_instruction() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [42; 15], // Local variable 1 = 42
        running: true,
        operands_buffer: vec![1], // LOAD local variable 1
        current_branch_offset: None,
        random_seed: 0,
    };
    
    let result = zmachine.op_load(1);
    assert!(result.is_ok());
    
    // Should have pushed value to stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 42);
}

#[test]
fn test_push_pull_operations() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![123], // PUSH value 123
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test PUSH
    let result = zmachine.op_push();
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 123);
    
    // Test PULL - pull into local variable 1
    zmachine.operands_buffer = vec![1];
    let result = zmachine.op_pull();
    assert!(result.is_ok());
    
    // Stack should be empty, local variable should have the value
    assert_eq!(zmachine.stack.len(), 0);
    assert_eq!(zmachine.local_vars[0], 123);
}

#[test]
fn test_storew_loadw_operations() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![0x1500, 2, 0x1234], // STOREW array_addr=0x1500, index=2, value=0x1234
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test STOREW
    let result = zmachine.op_storew();
    assert!(result.is_ok());
    
    // Check memory was written correctly (big-endian)
    let addr = 0x1500 + (2 * 2); // array_addr + index * 2
    assert_eq!(zmachine.memory[addr], 0x12);     // High byte
    assert_eq!(zmachine.memory[addr + 1], 0x34); // Low byte
    
    // Test LOADW
    zmachine.operands_buffer = vec![0x1500, 2]; // LOADW from same location
    let result = zmachine.op_loadw(0x1500, 2);
    assert!(result.is_ok());
    
    // Should have pushed the value to stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0x1234);
}

#[test]
fn test_storeb_loadb_operations() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![0x1600, 5, 0xAB], // STOREB array_addr=0x1600, index=5, value=0xAB
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test STOREB
    let result = zmachine.op_storeb();
    assert!(result.is_ok());
    
    // Check memory was written correctly
    let addr = 0x1600 + 5; // array_addr + index
    assert_eq!(zmachine.memory[addr], 0xAB);
    
    // Test LOADB
    zmachine.operands_buffer = vec![0x1600, 5]; // LOADB from same location
    let result = zmachine.op_loadb(0x1600, 5);
    assert!(result.is_ok());
    
    // Should have pushed the value to stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0xAB);
}

#[test]
fn test_set_clear_attr_operations() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Set up header
    memory[10] = 0x01;
    memory[11] = 0x00;
    
    let obj_table_start = 0x13E;
    
    // Object 1: initially no attributes set
    memory[obj_table_start] = 0x00;
    memory[obj_table_start + 1] = 0x00;
    memory[obj_table_start + 2] = 0x00;
    memory[obj_table_start + 3] = 0x00;
    
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![1, 0], // SET_ATTR object 1, attribute 0
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test SET_ATTR
    let result = zmachine.op_set_attr(1, 0);
    assert!(result.is_ok());
    
    // Attribute 0 should be set (bit 7 of first byte)
    assert_eq!(zmachine.memory[obj_table_start], 0x80);
    
    // Verify with TEST_ATTR
    assert!(zmachine.test_object_attribute(1, 0).unwrap());
    
    // Test CLEAR_ATTR
    let result = zmachine.op_clear_attr(1, 0);
    assert!(result.is_ok());
    
    // Attribute 0 should be cleared
    assert_eq!(zmachine.memory[obj_table_start], 0x00);
    assert!(!zmachine.test_object_attribute(1, 0).unwrap());
}

#[test]
fn test_get_parent_instruction() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Set up header
    memory[10] = 0x01;
    memory[11] = 0x00;
    
    let obj_table_start = 0x13E;
    
    // Object 1: parent=5
    memory[obj_table_start + 4] = 5;
    
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![1], // GET_PARENT of object 1
        current_branch_offset: None,
        random_seed: 0,
    };
    
    let result = zmachine.op_get_parent(1);
    assert!(result.is_ok());
    
    // Should have stored parent number on stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 5);
}

#[test]
fn test_global_variable_operations() {
    let mut memory = vec![0u8; 0x2000];
    memory[0] = 3; // Version 3
    let mock_game = MockGameFile::new(memory.clone());
    
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) },
        memory,
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![20], // INC global variable 20
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Set initial value for global variable 20
    zmachine.global_vars.insert(20, 100);
    
    // Test INC on global variable
    let result = zmachine.op_inc(20);
    assert!(result.is_ok());
    assert_eq!(*zmachine.global_vars.get(&20).unwrap(), 101);
    
    // Test DEC on global variable
    let result = zmachine.op_dec(20);
    assert!(result.is_ok());
    assert_eq!(*zmachine.global_vars.get(&20).unwrap(), 100);
    
    // Test LOAD on global variable
    let result = zmachine.op_load(20);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 100);
}