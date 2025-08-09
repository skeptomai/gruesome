// Z-Machine Code Generator (stub for now)

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::IRProgram;
use crate::grue_compiler::ZMachineVersion;

pub struct ZMachineCodeGen {
    version: ZMachineVersion,
}

impl ZMachineCodeGen {
    pub fn new(version: ZMachineVersion) -> Self {
        ZMachineCodeGen { version }
    }

    pub fn generate(&mut self, _ir: IRProgram) -> Result<Vec<u8>, CompilerError> {
        // TODO: Implement code generation
        // For now, return a minimal valid Z-Machine file

        // Minimal Z-Machine v3 header (64 bytes)
        let mut story_data = vec![0; 64];
        story_data[0] = match self.version {
            ZMachineVersion::V3 => 3,
            ZMachineVersion::V5 => 5,
        };

        // Set minimal required header fields
        story_data[4] = 0x10; // High memory at 0x1000
        story_data[5] = 0x00;
        story_data[6] = 0x40; // Initial PC at 0x4000
        story_data[7] = 0x00;

        // Add minimal program that just quits
        story_data.resize(0x4001, 0);
        story_data[0x4000] = 0xBA; // quit opcode

        Ok(story_data)
    }
}
