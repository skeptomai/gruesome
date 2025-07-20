//! Quetzal chunk definitions according to the specification

use crate::vm::{CallFrame, VM};

/// IFhd chunk - Interface Header
/// Contains key information about the interpreter state
pub struct IFhdChunk {
    /// Release number (from 0x02 in header)
    pub release: u16,
    /// Serial number (6 bytes from 0x12 in header)
    pub serial: [u8; 6],
    /// Checksum (from 0x1C in header)
    pub checksum: u16,
    /// Initial PC value (can be 0 for v6+)
    pub initial_pc: u16,
}

impl IFhdChunk {
    /// Create from VM state
    pub fn from_vm(vm: &VM) -> Self {
        let header = &vm.game.header;

        // Get serial number bytes
        let mut serial = [0u8; 6];
        serial.copy_from_slice(&vm.game.memory[0x12..0x18]);

        IFhdChunk {
            release: header.release,
            serial,
            checksum: header.checksum_file as u16,
            initial_pc: 0, // We'll use 0 for simplicity
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(13);

        // Release number (2 bytes, big-endian)
        bytes.extend_from_slice(&self.release.to_be_bytes());

        // Serial number (6 bytes)
        bytes.extend_from_slice(&self.serial);

        // Checksum (2 bytes, big-endian)
        bytes.extend_from_slice(&self.checksum.to_be_bytes());

        // Initial PC (3 bytes for packed address)
        // For v3, PC is a packed address divided by 2
        let pc_bytes = (self.initial_pc as u32).to_be_bytes();
        bytes.push(pc_bytes[1]); // High byte
        bytes.push(pc_bytes[2]); // Middle byte
        bytes.push(pc_bytes[3]); // Low byte

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 13 {
            return Err("IFhd chunk too small".to_string());
        }

        let release = u16::from_be_bytes([data[0], data[1]]);

        let mut serial = [0u8; 6];
        serial.copy_from_slice(&data[2..8]);

        let checksum = u16::from_be_bytes([data[8], data[9]]);

        // Initial PC is 3 bytes
        let initial_pc = ((data[10] as u16) << 8) | (data[11] as u16);

        Ok(IFhdChunk {
            release,
            serial,
            checksum,
            initial_pc,
        })
    }
}

/// UMem chunk - Uncompressed memory
/// Contains the dynamic memory that has changed from the original game file
pub struct UMemChunk {
    /// The dynamic memory bytes
    pub memory: Vec<u8>,
}

impl UMemChunk {
    /// Create from VM state (uncompressed version)
    pub fn from_vm(vm: &VM) -> Self {
        // Dynamic memory is from start to static memory base
        let dynamic_size = vm.game.header.base_static_mem;
        let memory = vm.game.memory[..dynamic_size].to_vec();

        UMemChunk { memory }
    }

    /// Get as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.memory.clone()
    }

    /// Create from bytes
    pub fn from_bytes(data: Vec<u8>) -> Self {
        UMemChunk { memory: data }
    }
}

/// Stks chunk - Stack frames
/// Contains the call stack and evaluation stack
pub struct StksChunk {
    /// Serialized stack data
    pub data: Vec<u8>,
}

impl StksChunk {
    /// Create from VM state
    pub fn from_vm(vm: &VM) -> Self {
        let mut data = Vec::new();

        // Save all call frames, including the dummy frame
        // This preserves the stack structure correctly
        for (frame_idx, frame) in vm.call_stack.iter().enumerate() {
            // Return PC (3 bytes packed address)
            let pc_bytes = frame.return_pc.to_be_bytes();
            data.push(pc_bytes[1]);
            data.push(pc_bytes[2]);
            data.push(pc_bytes[3]);

            // Flags byte:
            // Bit 0-3: local variable count
            // Bit 4: 1 if called with store variable
            // Bit 5-6: number of arguments supplied
            // Bit 7: reserved
            let mut flags = frame.num_locals & 0x0F;
            if frame.return_store.is_some() {
                flags |= 0x10;
            }
            // For simplicity, assume all locals were supplied as arguments
            let arg_count = std::cmp::min(frame.num_locals as usize, 3) as u8;
            flags |= (arg_count & 0x03) << 5;
            data.push(flags);

            // Store variable (1 byte) if present
            if let Some(var) = frame.return_store {
                data.push(var);
            }

            // Arguments supplied (1 byte each) - not needed for v3
            // Skip this for now

            // Evaluation stack count (2 bytes)
            // Calculate stack size for this frame
            let next_frame_idx = frame_idx + 1;
            let stack_end = if next_frame_idx < vm.call_stack.len() {
                vm.call_stack[next_frame_idx].stack_base
            } else {
                vm.stack.len()
            };
            let stack_size = stack_end.saturating_sub(frame.stack_base);
            data.extend_from_slice(&(stack_size as u16).to_be_bytes());

            // Local variables (2 bytes each)
            for i in 0..frame.num_locals as usize {
                data.extend_from_slice(&frame.locals[i].to_be_bytes());
            }

            // Stack values (2 bytes each)
            for i in frame.stack_base..stack_end {
                data.extend_from_slice(&vm.stack[i].to_be_bytes());
            }
        }

        StksChunk { data }
    }

    /// Get as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Restore stack frames to VM
    pub fn restore_to_vm(&self, vm: &mut VM) -> Result<(), String> {
        vm.call_stack.clear();
        vm.stack.clear();

        let mut offset = 0;
        let data = &self.data;

        while offset < data.len() {
            // Read return PC (3 bytes)
            if offset + 4 > data.len() {
                return Err("Incomplete stack frame".to_string());
            }

            let return_pc = ((data[offset] as u32) << 16)
                | ((data[offset + 1] as u32) << 8)
                | (data[offset + 2] as u32);
            offset += 3;

            // Read flags
            let flags = data[offset];
            offset += 1;

            let local_count = (flags & 0x0F) as usize;
            let has_result_var = (flags & 0x10) != 0;
            let _arg_count = ((flags >> 5) & 0x03) as usize;

            // Read result variable if present
            let return_store = if has_result_var {
                if offset >= data.len() {
                    return Err("Missing result variable".to_string());
                }
                let var = data[offset];
                offset += 1;
                Some(var)
            } else {
                None
            };

            // Skip supplied arguments (not used in v3)

            // Read evaluation stack count
            if offset + 2 > data.len() {
                return Err("Missing stack count".to_string());
            }
            let stack_count = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;

            // Read local variables
            let mut locals = [0u16; 16];
            if offset + local_count * 2 > data.len() {
                return Err("Missing local variables".to_string());
            }
            for local in locals.iter_mut().take(local_count) {
                *local = u16::from_be_bytes([data[offset], data[offset + 1]]);
                offset += 2;
            }

            // Read stack values and add to VM stack
            let stack_base = vm.stack.len();
            if offset + stack_count * 2 > data.len() {
                return Err("Missing stack values".to_string());
            }
            for _ in 0..stack_count {
                let value = u16::from_be_bytes([data[offset], data[offset + 1]]);
                vm.stack.push(value);
                offset += 2;
            }

            // Create and add frame
            let frame = CallFrame {
                return_pc,
                return_store,
                num_locals: local_count as u8,
                locals,
                stack_base,
            };
            vm.call_stack.push(frame);
        }

        Ok(())
    }
}

/// IntD chunk - Interpreter data (optional)
/// Can contain interpreter-specific data
pub struct IntDChunk {
    /// Interpreter identifier (4 bytes)
    pub interpreter_id: [u8; 4],
    /// Custom data
    pub data: Vec<u8>,
}

impl Default for IntDChunk {
    fn default() -> Self {
        Self::new()
    }
}

impl IntDChunk {
    /// Create a new interpreter data chunk
    pub fn new() -> Self {
        IntDChunk {
            interpreter_id: *b"RUST", // Our interpreter ID
            data: Vec::new(),
        }
    }

    /// Add custom data
    pub fn add_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.interpreter_id);
        bytes.extend_from_slice(&self.data);
        bytes
    }
}
