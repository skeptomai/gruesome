#[cfg(test)]
mod tests {
    use crate::disassembler::Disassembler;
    use crate::instruction::Instruction;
    use crate::interpreter::Interpreter;
    use crate::vm::{Game, VM};
    use log::{debug, info};

    /// Create a test program that calls a routine and prints
    fn create_test_with_call() -> Vec<u8> {
        let mut memory = vec![0u8; 0x10000];

        // Set up header (V3)
        memory[0x00] = 3; // Version 3
        memory[0x04] = 0x10; // High memory at 0x1000
        memory[0x05] = 0x00;
        memory[0x06] = 0x10; // Initial PC at 0x1000
        memory[0x07] = 0x00;
        memory[0x0c] = 0x01; // Global variables at 0x0100
        memory[0x0d] = 0x00;
        memory[0x0e] = 0x02; // Static memory at 0x0200
        memory[0x0f] = 0x00;
        memory[0x18] = 0x00; // Abbreviations at 0x0040
        memory[0x19] = 0x40;

        // Main routine at 0x1000:
        // call print_hello -> discard
        // quit

        let mut pc = 0x1000;

        // call print_hello (at packed address 0x1010/2 = 0x808)
        memory[pc] = 0xE0; // VAR:0 call
        memory[pc + 1] = 0x1F; // one small constant operand
        memory[pc + 2] = 0x08; // packed address high
        memory[pc + 3] = 0x08; // packed address low
        memory[pc + 4] = 0x00; // store to nowhere (discard)
        pc += 5;

        // quit
        memory[pc] = 0xBA; // 0OP:A quit

        // print_hello routine at 0x1010:
        // 0 locals
        // print "Hello, Z-Machine!"
        // rtrue

        pc = 0x1010;
        memory[pc] = 0; // 0 locals
        pc += 1;

        // print "Hello"
        memory[pc] = 0xB2; // 0OP:2 print
        pc += 1;

        // Encode "Hello" as Z-string
        // H=8, e=5, l=12, l=12, o=15 (add 6 for a-z encoding)
        // = 14, 11, 18, 18, 21
        // Pack into 5-bit chunks: 01110 01011 10010 10010 10101
        // First word: 01110 01011 10010 = 0111001011 10010 = 0x72E4
        memory[pc] = 0x72;
        memory[pc + 1] = 0xE4;
        pc += 2;

        // Second word with end bit: 10010 10101 00101 with bit 15 set
        // = 1 0010101 0100101 = 0x9545
        memory[pc] = 0x95;
        memory[pc + 1] = 0x45;
        pc += 2;

        // rtrue
        memory[pc] = 0xB0; // 0OP:0 rtrue

        memory
    }

    #[test]
    fn test_basic_call_and_print() {
        let memory = create_test_with_call();
        let game = Game::from_memory(memory).unwrap();

        // Disassemble to verify
        let disasm = Disassembler::new(&game);
        info!("=== Disassembly ===");
        match disasm.disassemble_range(0x1000, 0x1020) {
            Ok(output) => info!("{}", output),
            Err(e) => info!("Disassembly error: {}", e),
        }

        // Create interpreter
        let vm = VM::new(game);
        let mut interpreter = Interpreter::new(vm);
        interpreter.set_debug(true);

        info!("=== Execution ===");

        // Execute a few instructions
        for i in 0..10 {
            let pc = interpreter.vm.pc;
            let inst = match Instruction::decode(&interpreter.vm.game.memory, pc as usize, 3) {
                Ok(inst) => inst,
                Err(e) => {
                    info!("Failed to decode at {:04x}: {}", pc, e);
                    break;
                }
            };

            debug!("Step {}: {:04x}: {}", i, pc, inst);

            // Advance PC
            interpreter.vm.pc += inst.size as u32;

            // Execute
            match interpreter.execute_instruction(&inst) {
                Ok(result) => {
                    debug!("Result: {:?}", result);
                    match result {
                        crate::interpreter::ExecutionResult::Quit => {
                            info!("Program quit");
                            break;
                        }
                        crate::interpreter::ExecutionResult::Called => {
                            debug!("Called routine, PC now at {:04x}", interpreter.vm.pc);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    info!("Execution error: {}", e);
                    break;
                }
            }
        }
    }

    #[test]
    fn test_stack_operations() {
        let mut memory = vec![0u8; 0x10000];

        // Set up header
        memory[0x00] = 3; // Version 3
        memory[0x06] = 0x10; // Initial PC at 0x1000
        memory[0x07] = 0x00;
        memory[0x0c] = 0x01; // Global variables at 0x0100
        memory[0x0d] = 0x00;
        memory[0x0e] = 0x02; // Static memory at 0x0200
        memory[0x0f] = 0x00;

        // Initialize global variables to 0
        for i in 0..240 {
            memory[0x0100 + i * 2] = 0;
            memory[0x0101 + i * 2] = 0;
        }

        // Simple test: push and pop
        let mut pc = 0x1000;

        // push 42
        memory[pc] = 0xE8; // VAR:8 push
        memory[pc + 1] = 0x7F; // one small constant
        memory[pc + 2] = 42;
        pc += 3;

        // pull V10 (pop into V10)
        memory[pc] = 0xE9; // VAR:9 pull
        memory[pc + 1] = 0x7F; // one small constant
        memory[pc + 2] = 0x10; // variable 10
        pc += 3;

        // quit
        memory[pc] = 0xBA;

        let game = Game::from_memory(memory).unwrap();
        let vm = VM::new(game);
        let mut interpreter = Interpreter::new(vm);

        match interpreter.run() {
            Ok(()) => {}
            Err(e) => panic!("Execution error: {e}"),
        }

        // Check that global 10 contains 42
        assert_eq!(interpreter.vm.read_global(0x10).unwrap(), 42);
    }

    #[test]
    fn test_variable_0_pop() {
        let mut memory = vec![0u8; 0x10000];

        // Set up header
        memory[0x00] = 3; // Version 3
        memory[0x06] = 0x10; // Initial PC at 0x1000
        memory[0x07] = 0x00;
        memory[0x0c] = 0x01; // Global variables at 0x0100
        memory[0x0d] = 0x00;
        memory[0x0e] = 0x02; // Static memory at 0x0200
        memory[0x0f] = 0x00;

        // Simple test: just push two values and add using stack
        let mut pc = 0x1000;

        // push 42
        memory[pc] = 0xE8; // VAR:8 push
        memory[pc + 1] = 0x7F; // one small constant
        memory[pc + 2] = 42;
        pc += 3;

        // push 7
        memory[pc] = 0xE8; // VAR:8 push
        memory[pc + 1] = 0x7F; // one small constant
        memory[pc + 2] = 7;
        pc += 3;

        // store V10 V00 (pop from stack into V10)
        memory[pc] = 0x6D; // 2OP:0D store, both variables (01 10 11 01)
        memory[pc + 1] = 0x10; // variable 10
        memory[pc + 2] = 0x00; // value from V00 (pop from stack)
        pc += 3;

        // store V11 V00 (pop from stack into V11)
        memory[pc] = 0x6D; // 2OP:0D store, both variables
        memory[pc + 1] = 0x11; // variable 11
        memory[pc + 2] = 0x00; // value from V00 (pop from stack)
        pc += 3;

        // add V10 V11 -> V12
        memory[pc] = 0x54; // Long form add, both small constants
        memory[pc + 1] = 0x10; // V10
        memory[pc + 2] = 0x11; // V11
        memory[pc + 3] = 0x12; // Store to V12
        pc += 4;

        // quit
        memory[pc] = 0xBA;

        let game = Game::from_memory(memory).unwrap();
        let vm = VM::new(game);
        let mut interpreter = Interpreter::new(vm);

        match interpreter.run() {
            Ok(()) => {}
            Err(e) => panic!("Execution error: {e}"),
        }

        // Check that global 12 contains 49 (7 + 42)
        assert_eq!(interpreter.vm.read_global(0x12).unwrap(), 49);
    }

    #[test]
    fn test_print_paddr() {
        let mut memory = vec![0u8; 0x10000];

        // Set up header
        memory[0x00] = 3; // Version 3
        memory[0x06] = 0x10; // Initial PC
        memory[0x07] = 0x00;
        memory[0x18] = 0x00; // Abbrev at 0x40
        memory[0x19] = 0x40;

        // Program: print_paddr 0x100, quit
        memory[0x1000] = 0x8D; // 1OP:0D print_paddr
        memory[0x1001] = 0x01; // packed addr
        memory[0x1002] = 0x00;
        memory[0x1003] = 0xBA; // quit

        // String at 0x200 (packed 0x100): "Test"
        // T needs shift to A1, T is at position 19 in A1 (0-based), so Z-char = 19+6 = 25
        // e is at position 4 in A0, so Z-char = 4+6 = 10
        // s is at position 18 in A0, so Z-char = 18+6 = 24
        // t is at position 19 in A0, so Z-char = 19+6 = 25
        // Sequence: shift(4), T(25), e(10)
        // First word: 00100 11001 01010 = 0001001100101010 = 0x132A
        // Second word: s(24), t(25), padding(5) with end bit
        // 11000 11001 00101 with bit 15 = 1110001100100101 = 0xE325
        memory[0x200] = 0x13;
        memory[0x201] = 0x2A;
        memory[0x202] = 0xE3;
        memory[0x203] = 0x25;

        let game = Game::from_memory(memory).unwrap();
        let vm = VM::new(game);
        let mut interpreter = Interpreter::new(vm);

        info!("Testing print_paddr");
        match interpreter.run() {
            Ok(()) => info!("Completed"),
            Err(e) => info!("Error: {}", e),
        }
    }
}
