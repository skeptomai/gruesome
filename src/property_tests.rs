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

fn setup_property_test_memory() -> Vec<u8> {
    let mut memory = vec![0u8; 0x4000];
    memory[0] = 3; // Version 3
    
    // Set up header - property defaults at 0x0100
    memory[10] = 0x01; // High byte
    memory[11] = 0x00; // Low byte = 0x100
    
    // Set up property defaults table (31 entries * 2 bytes each = 62 bytes)
    let prop_defaults_start = 0x0100;
    for i in 0..31 {
        let addr = prop_defaults_start + (i * 2);
        memory[addr] = 0x00;        // High byte
        memory[addr + 1] = (i + 100) as u8;  // Low byte = default value (100-130)
    }
    
    // Object table starts at 0x100 + 62 = 0x13E
    let obj_table_start = 0x13E;
    
    // Object 1 setup:
    // Attributes (4 bytes): all zeros
    memory[obj_table_start] = 0x00;
    memory[obj_table_start + 1] = 0x00;
    memory[obj_table_start + 2] = 0x00;
    memory[obj_table_start + 3] = 0x00;
    // Parent, Sibling, Child: all zeros
    memory[obj_table_start + 4] = 0;
    memory[obj_table_start + 5] = 0;
    memory[obj_table_start + 6] = 0;
    // Properties pointer to 0x0200
    memory[obj_table_start + 7] = 0x02;
    memory[obj_table_start + 8] = 0x00;
    
    // Object 1 properties at 0x0200:
    // Description length: 0 (no description)
    memory[0x0200] = 0x00;
    
    // Property list starts at 0x0201:
    // Property 5: 2-byte value 0x1234
    memory[0x0201] = (2 - 1) << 5 | 5;  // Size=2, Prop=5 -> 0x25
    memory[0x0202] = 0x12;  // High byte
    memory[0x0203] = 0x34;  // Low byte
    
    // Property 10: 1-byte value 0xAB
    memory[0x0204] = (1 - 1) << 5 | 10;  // Size=1, Prop=10 -> 0x0A
    memory[0x0205] = 0xAB;
    
    // Property 15: 2-byte value 0x5678
    memory[0x0206] = (2 - 1) << 5 | 15;  // Size=2, Prop=15 -> 0x2F
    memory[0x0207] = 0x56;  // High byte
    memory[0x0208] = 0x78;  // Low byte
    
    // End of properties
    memory[0x0209] = 0x00;
    
    memory
}

#[test]
fn test_get_prop_existing_property() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test GET_PROP object 1, property 5 (should return 0x1234)
    let result = zmachine.op_get_prop(1, 5);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0x1234);
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_PROP object 1, property 10 (should return 0xAB)
    let result = zmachine.op_get_prop(1, 10);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0xAB);
}

#[test]
fn test_get_prop_default_value() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test GET_PROP object 1, property 1 (not on object, should return default)
    let result = zmachine.op_get_prop(1, 1);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 100); // Default for property 1
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_PROP object 1, property 3 (not on object, should return default)
    let result = zmachine.op_get_prop(1, 3);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 102); // Default for property 3
}

#[test]
fn test_put_prop_operation() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![1, 5, 0x9999], // Object 1, Property 5, New value 0x9999
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test PUT_PROP object 1, property 5, value 0x9999
    let result = zmachine.op_put_prop();
    assert!(result.is_ok());
    
    // Verify the property was changed
    zmachine.operands_buffer.clear();
    let result = zmachine.op_get_prop(1, 5);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0x9999);
    
    // Test 1-byte property
    zmachine.stack.clear();
    zmachine.operands_buffer = vec![1, 10, 0xFF]; // Object 1, Property 10, New value 0xFF
    let result = zmachine.op_put_prop();
    assert!(result.is_ok());
    
    // Verify the 1-byte property was changed (only low byte stored)
    zmachine.operands_buffer.clear();
    let result = zmachine.op_get_prop(1, 10);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0xFF);
}

#[test]
fn test_get_prop_addr_operation() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test GET_PROP_ADDR object 1, property 5
    let result = zmachine.op_get_prop_addr(1, 5);
    if let Err(e) = &result {
        println!("GET_PROP_ADDR error: {}", e);
    }
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0x0202); // Address of property 5 data
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_PROP_ADDR object 1, property 99 (doesn't exist)
    let result = zmachine.op_get_prop_addr(1, 31); // Use valid property number
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0); // Should return 0 for non-existent property
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_PROP_ADDR object 0 (special case)
    let result = zmachine.op_get_prop_addr(0, 5);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0); // Should return 0 for object 0
}

#[test]
fn test_get_prop_len_operation() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test GET_PROP_LEN for property 5 (2-byte property at 0x0202)
    let result = zmachine.op_get_prop_len(0x0202);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 2); // Should return 2 for 2-byte property
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_PROP_LEN for property 10 (1-byte property at 0x0205)
    let result = zmachine.op_get_prop_len(0x0205);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 1); // Should return 1 for 1-byte property
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_PROP_LEN 0 (special case)
    let result = zmachine.op_get_prop_len(0);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0); // Should return 0
}

#[test]
fn test_get_next_prop_operation() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test GET_NEXT_PROP object 1, property 0 (should return first property)
    let result = zmachine.op_get_next_prop(1, 0);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 5); // First property is 5
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_NEXT_PROP object 1, property 5 (should return next property)
    let result = zmachine.op_get_next_prop(1, 5);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 10); // Next property is 10
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_NEXT_PROP object 1, property 10 (should return next property)
    let result = zmachine.op_get_next_prop(1, 10);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 15); // Next property is 15
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_NEXT_PROP object 1, property 15 (should return 0, it's the last)
    let result = zmachine.op_get_next_prop(1, 15);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0); // No next property
    
    // Clear stack for next test
    zmachine.stack.clear();
    
    // Test GET_NEXT_PROP object 1, property 99 (doesn't exist, should return 0)
    let result = zmachine.op_get_next_prop(1, 99);
    assert!(result.is_ok());
    assert_eq!(zmachine.stack.len(), 1);
    assert_eq!(zmachine.stack[0], 0); // Property not found
}

#[test]
fn test_put_prop_nonexistent_property() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![1, 31, 0x1111], // Object 1, Property 31 (doesn't exist), Value 0x1111
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test PUT_PROP on non-existent property (should fail)
    let result = zmachine.op_put_prop();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not have property"));
}

#[test]
fn test_property_error_conditions() {
    let memory = setup_property_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test GET_PROP with object 0
    let result = zmachine.op_get_prop(0, 5);
    assert!(result.is_err());
    
    // Test GET_PROP with invalid property number
    let result = zmachine.op_get_prop(1, 0);
    assert!(result.is_err());
    
    let result = zmachine.op_get_prop(1, 32);
    assert!(result.is_err());
    
    // Test GET_NEXT_PROP with object 0
    let result = zmachine.op_get_next_prop(0, 5);
    assert!(result.is_err());
}

// Object manipulation tests
fn setup_object_test_memory() -> Vec<u8> {
    let mut memory = vec![0u8; 0x4000];
    memory[0] = 3; // Version 3
    
    // Set up header - object table at 0x0200 (after property defaults)
    memory[10] = 0x01; // High byte
    memory[11] = 0x00; // Low byte = 0x100 for property defaults
    
    // Property defaults table at 0x0100 (31 words = 62 bytes)
    // Ends at 0x013E, so object table starts at 0x013E
    
    // Object table layout (9 bytes per object in v1-3):
    // Object 1 at 0x013E: attributes(4) + parent(1) + sibling(2) + child(3) + props(0x0300)
    let obj1_addr = 0x013E;
    memory[obj1_addr + 4] = 0; // parent = 0 (none)
    memory[obj1_addr + 5] = 2; // sibling = object 2 
    memory[obj1_addr + 6] = 0; // child = 0 (none)
    memory[obj1_addr + 7] = 0x03; // properties high byte
    memory[obj1_addr + 8] = 0x00; // properties low byte
    
    // Object 2 at 0x0147: attributes(4) + parent(0) + sibling(3) + child(0) + props(0x0310)
    let obj2_addr = 0x0147;
    memory[obj2_addr + 4] = 0; // parent = 0 (none)
    memory[obj2_addr + 5] = 3; // sibling = object 3
    memory[obj2_addr + 6] = 0; // child = 0 (none) 
    memory[obj2_addr + 7] = 0x03; // properties high byte
    memory[obj2_addr + 8] = 0x10; // properties low byte
    
    // Object 3 at 0x0150: attributes(4) + parent(0) + sibling(0) + child(0) + props(0x0320)
    let obj3_addr = 0x0150;
    memory[obj3_addr + 4] = 0; // parent = 0 (none)
    memory[obj3_addr + 5] = 0; // sibling = 0 (none)
    memory[obj3_addr + 6] = 0; // child = 0 (none)
    memory[obj3_addr + 7] = 0x03; // properties high byte  
    memory[obj3_addr + 8] = 0x20; // properties low byte
    
    // Object 4 at 0x0159: attributes(4) + parent(0) + sibling(0) + child(0) + props(0x0330)
    let obj4_addr = 0x0159;
    memory[obj4_addr + 4] = 0; // parent = 0 (none)
    memory[obj4_addr + 5] = 0; // sibling = 0 (none)
    memory[obj4_addr + 6] = 0; // child = 0 (none) 
    memory[obj4_addr + 7] = 0x03; // properties high byte
    memory[obj4_addr + 8] = 0x30; // properties low byte
    
    // Set up minimal property tables (just description length 0)
    memory[0x0300] = 0; // Object 1 description length
    memory[0x0310] = 0; // Object 2 description length
    memory[0x0320] = 0; // Object 3 description length
    memory[0x0330] = 0; // Object 4 description length
    
    memory
}

#[test]
fn test_remove_obj_basic() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Initially object 2 has parent 0
    assert_eq!(zmachine.get_object_parent(2).unwrap(), 0);
    
    // Remove object 2 (which has no parent - should be no-op)
    let result = zmachine.op_remove_obj(2);
    assert!(result.is_ok());
    assert_eq!(zmachine.get_object_parent(2).unwrap(), 0);
}

#[test]
fn test_insert_obj_basic() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Insert object 2 into object 1
    let result = zmachine.op_insert_obj(2, 1);
    assert!(result.is_ok());
    
    // Check that object 2 now has parent 1
    assert_eq!(zmachine.get_object_parent(2).unwrap(), 1);
    
    // Check that object 1 now has child 2  
    assert_eq!(zmachine.get_object_child(1).unwrap(), 2);
    
    // Check that object 2 has no siblings (since it was the first/only child)
    assert_eq!(zmachine.get_object_sibling(2).unwrap(), 0);
}

#[test]
fn test_insert_multiple_children() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Insert object 2 into object 1
    let result = zmachine.op_insert_obj(2, 1);
    assert!(result.is_ok());
    
    // Insert object 3 into object 1 (should become new first child)
    let result = zmachine.op_insert_obj(3, 1);
    assert!(result.is_ok());
    
    // Check that object 1's first child is now 3
    assert_eq!(zmachine.get_object_child(1).unwrap(), 3);
    
    // Check that object 3's sibling is 2
    assert_eq!(zmachine.get_object_sibling(3).unwrap(), 2);
    
    // Check that object 2 has no sibling (it's the last)
    assert_eq!(zmachine.get_object_sibling(2).unwrap(), 0);
    
    // Check both objects have parent 1
    assert_eq!(zmachine.get_object_parent(2).unwrap(), 1);
    assert_eq!(zmachine.get_object_parent(3).unwrap(), 1);
}

#[test]
fn test_remove_from_sibling_chain() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Set up: Insert 2, 3, 4 into object 1
    zmachine.op_insert_obj(2, 1).unwrap();
    zmachine.op_insert_obj(3, 1).unwrap();
    zmachine.op_insert_obj(4, 1).unwrap();
    
    // Object 1 should have child chain: 4 -> 3 -> 2
    assert_eq!(zmachine.get_object_child(1).unwrap(), 4);
    assert_eq!(zmachine.get_object_sibling(4).unwrap(), 3);
    assert_eq!(zmachine.get_object_sibling(3).unwrap(), 2);
    assert_eq!(zmachine.get_object_sibling(2).unwrap(), 0);
    
    // Remove object 3 from the middle
    let result = zmachine.op_remove_obj(3);
    assert!(result.is_ok());
    
    // Check that 3 is removed from chain: 4 -> 2
    assert_eq!(zmachine.get_object_child(1).unwrap(), 4);
    assert_eq!(zmachine.get_object_sibling(4).unwrap(), 2);
    assert_eq!(zmachine.get_object_sibling(2).unwrap(), 0);
    
    // Check that object 3 has no parent or sibling
    assert_eq!(zmachine.get_object_parent(3).unwrap(), 0);
    assert_eq!(zmachine.get_object_sibling(3).unwrap(), 0);
}

#[test]
fn test_remove_first_child() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Set up: Insert 2, 3 into object 1
    zmachine.op_insert_obj(2, 1).unwrap();
    zmachine.op_insert_obj(3, 1).unwrap();
    
    // Object 1 should have child chain: 3 -> 2
    assert_eq!(zmachine.get_object_child(1).unwrap(), 3);
    assert_eq!(zmachine.get_object_sibling(3).unwrap(), 2);
    
    // Remove object 3 (first child)
    let result = zmachine.op_remove_obj(3);
    assert!(result.is_ok());
    
    // Check that object 1's child is now 2
    assert_eq!(zmachine.get_object_child(1).unwrap(), 2);
    assert_eq!(zmachine.get_object_sibling(2).unwrap(), 0);
    
    // Check that object 3 has no parent
    assert_eq!(zmachine.get_object_parent(3).unwrap(), 0);
}

#[test]
fn test_insert_obj_edge_cases() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test inserting object 0 (should be no-op)
    let result = zmachine.op_insert_obj(0, 1);
    assert!(result.is_ok());
    assert_eq!(zmachine.get_object_child(1).unwrap(), 0);
    
    // Test inserting into object 0 (should be no-op)
    let result = zmachine.op_insert_obj(1, 0);
    assert!(result.is_ok());
    assert_eq!(zmachine.get_object_parent(1).unwrap(), 0);
}

#[test]
fn test_remove_obj_edge_cases() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Test removing object 0 (should be no-op)
    let result = zmachine.op_remove_obj(0);
    assert!(result.is_ok());
    
    // Test removing object with no parent (should be no-op)
    let result = zmachine.op_remove_obj(1);
    assert!(result.is_ok());
    assert_eq!(zmachine.get_object_parent(1).unwrap(), 0);
}

#[test]
fn test_move_object_between_parents() {
    let memory = setup_object_test_memory();
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
        operands_buffer: vec![],
        current_branch_offset: None,
        random_seed: 0,
    };
    
    // Insert object 3 into object 1
    zmachine.op_insert_obj(3, 1).unwrap();
    assert_eq!(zmachine.get_object_parent(3).unwrap(), 1);
    assert_eq!(zmachine.get_object_child(1).unwrap(), 3);
    
    // Move object 3 from object 1 to object 2
    zmachine.op_insert_obj(3, 2).unwrap();
    
    // Check that object 3 is now child of object 2
    assert_eq!(zmachine.get_object_parent(3).unwrap(), 2);
    assert_eq!(zmachine.get_object_child(2).unwrap(), 3);
    
    // Check that object 1 no longer has children
    assert_eq!(zmachine.get_object_child(1).unwrap(), 0);
}