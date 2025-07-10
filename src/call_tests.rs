use crate::zmachine::{ZMachine, CallFrame};

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
fn test_call_routine_zero() {
    // Test CALL 0 returns 0 immediately  
    let memory = vec![0u8; 0x1000];
    let mock_game = MockGameFile::new(memory);
    
    // Create ZMachine without using GameFile constructor
    let mut zmachine = ZMachine {
        game: unsafe { std::mem::transmute(&mock_game) }, // Hack for lifetime
        memory: mock_game.bytes().to_vec(),
        pc: 0x1000,
        stack: Vec::new(),
        call_stack: Vec::new(),
        global_vars: std::collections::HashMap::new(),
        local_vars: [0; 15],
        running: true,
        operands_buffer: vec![0], // CALL 0
        current_branch_offset: None,
    };
    
    let result = zmachine.op_call();
    assert!(result.is_ok());
    
    // Should have pushed 0 to stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0);
}

#[test]
fn test_call_simple_routine() {
    // Create memory with a simple routine
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    // Place routine at 0x2000
    let routine_addr = 0x2000;
    memory[routine_addr] = 1;       // 1 local variable
    memory[routine_addr + 1] = 0x00; // default value high byte
    memory[routine_addr + 2] = 0x05; // default value low byte = 5
    memory[routine_addr + 3] = 0xB0; // rtrue instruction
    
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
        operands_buffer: vec![0x1000], // CALL routine (packed address)
        current_branch_offset: None,
    };
    
    let initial_pc = zmachine.pc;
    
    let result = zmachine.op_call();
    if let Err(e) = &result {
        println!("CALL error: {}", e);
    }
    assert!(result.is_ok());
    
    // Should have created call frame
    assert_eq!(zmachine.call_stack.len(), 1);
    
    // PC should point to routine code (after header)
    assert_eq!(zmachine.pc, 0x2000 + 1 + 2); // routine start + num_locals + default value
    
    // Local variable should have default value
    assert_eq!(zmachine.local_vars[0], 5);
    
    // Call frame should have correct return PC
    assert_eq!(zmachine.call_stack[0].return_pc, initial_pc);
}

#[test]
fn test_call_with_arguments() {
    // Create memory with a routine with 3 local variables
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    let routine_addr = 0x2000;
    memory[routine_addr] = 3;       // 3 local variables
    memory[routine_addr + 1] = 0x00; memory[routine_addr + 2] = 0x01; // default 1
    memory[routine_addr + 3] = 0x00; memory[routine_addr + 4] = 0x02; // default 2
    memory[routine_addr + 5] = 0x00; memory[routine_addr + 6] = 0x03; // default 3
    memory[routine_addr + 7] = 0xB0; // rtrue instruction
    
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
        operands_buffer: vec![0x1000, 100, 200], // CALL routine arg1 arg2
        current_branch_offset: None,
    };
    
    let result = zmachine.op_call();
    assert!(result.is_ok());
    
    // First two locals should have argument values
    assert_eq!(zmachine.local_vars[0], 100);
    assert_eq!(zmachine.local_vars[1], 200);
    // Third local should have default value
    assert_eq!(zmachine.local_vars[2], 3);
}

#[test]
fn test_call_return_sequence() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    let routine_addr = 0x2000;
    memory[routine_addr] = 0; // 0 local variables
    
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
        operands_buffer: vec![0x1000], // CALL routine
        current_branch_offset: None,
    };
    
    let initial_pc = zmachine.pc;
    let initial_locals = zmachine.local_vars;
    
    // Call routine
    let result = zmachine.op_call();
    assert!(result.is_ok());
    
    // Now execute return
    let result = zmachine.return_from_routine(42);
    assert!(result.is_ok());
    
    // Should be back at original PC
    assert_eq!(zmachine.pc, initial_pc);
    
    // Locals should be restored
    assert_eq!(zmachine.local_vars, initial_locals);
    
    // Return value should be on stack
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 42);
    
    // Call stack should be empty
    assert_eq!(zmachine.call_stack.len(), 0);
}

#[test]
fn test_call_invalid_routine_address() {
    let mut memory = vec![0u8; 0x1000];
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
        operands_buffer: vec![0xFFFF], // Invalid address
        current_branch_offset: None,
    };
    
    let result = zmachine.op_call();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of bounds"));
}

#[test]
fn test_call_too_many_locals() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    let routine_addr = 0x2000;
    memory[routine_addr] = 20; // 20 local variables (invalid)
    
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
        operands_buffer: vec![0x1000], // CALL routine
        current_branch_offset: None,
    };
    
    let result = zmachine.op_call();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Too many local variables"));
}

#[test]
fn test_call_address_conversion_v3() {
    let mut memory = vec![0u8; 0x3000];
    memory[0] = 3; // Version 3
    
    let routine_addr = 0x2000;
    memory[routine_addr] = 1;       // 1 local variable
    memory[routine_addr + 1] = 0x00; // default value high byte
    memory[routine_addr + 2] = 0x00; // default value low byte = 0
    
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
        operands_buffer: vec![0x1000], // Packed address 0x1000
        current_branch_offset: None,
    };
    
    let result = zmachine.op_call();
    assert!(result.is_ok());
    
    // PC should be at 0x2000 + 1 + 2 (routine + header)
    assert_eq!(zmachine.pc, 0x2000 + 1 + 2);
}