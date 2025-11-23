//! Save game functionality for Quetzal format

use crate::interpreter::core::vm::VM;
use crate::interpreter::quetzal::chunks::{IFhdChunk, IntDChunk, StksChunk};
use crate::interpreter::quetzal::compressed_memory::CMemChunk;
use crate::interpreter::quetzal::iff::IffFile;
use log::{debug, info};
use std::path::Path;

/// SaveGame handles creating Quetzal save files
pub struct SaveGame {
    /// The IFF file being built
    iff: IffFile,
}

impl SaveGame {
    /// Create a new save game from VM state
    pub fn from_vm(vm: &VM) -> Result<Self, String> {
        let mut iff = IffFile::new();

        // Add IFhd chunk (required)
        let ifhd = IFhdChunk::from_vm(vm);
        iff.add_chunk(*b"IFhd", ifhd.to_bytes());
        debug!("Added IFhd chunk");

        // Add CMem chunk (compressed memory)
        // We need the original game memory for XOR compression
        let original_memory = vm
            .game
            .original_memory
            .as_ref()
            .ok_or("No original memory available for compression")?;

        let dynamic_size = vm.game.header.base_static_mem;
        let current_dynamic = &vm.game.memory[..dynamic_size];
        let original_dynamic = &original_memory[..dynamic_size];

        let cmem = CMemChunk::from_memory(current_dynamic, original_dynamic);
        iff.add_chunk(*b"CMem", cmem.to_bytes());
        debug!(
            "Added CMem chunk ({} bytes compressed)",
            cmem.to_bytes().len()
        );

        // Add Stks chunk (call stack)
        let stks = StksChunk::from_vm(vm);
        iff.add_chunk(*b"Stks", stks.to_bytes());
        debug!("Added Stks chunk ({} bytes)", stks.to_bytes().len());

        // Add IntD chunk (optional interpreter data)
        let intd = IntDChunk::new();
        iff.add_chunk(*b"IntD", intd.to_bytes());
        debug!("Added IntD chunk");

        Ok(SaveGame { iff })
    }

    /// Save to a file
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        info!("Saving game to {:?}", path);
        self.iff.write_to_file(path)?;
        info!("Game saved successfully");
        Ok(())
    }

    /// Save to a prompt-selected file
    pub fn save_with_prompt(&self) -> Result<(), String> {
        use std::io::{self, Write};

        print!("Enter save filename: ");
        io::stdout()
            .flush()
            .map_err(|e| format!("Failed to flush stdout: {e}"))?;

        let mut filename = String::new();
        io::stdin()
            .read_line(&mut filename)
            .map_err(|e| format!("Failed to read filename: {e}"))?;

        let filename = filename.trim();
        if filename.is_empty() {
            return Err("No filename provided".to_string());
        }

        // Add .sav extension if not present
        let filename = if filename.ends_with(".sav") || filename.ends_with(".qzl") {
            filename.to_string()
        } else {
            format!("{filename}.sav")
        };

        let path = Path::new(&filename);

        println!("Saving game to '{filename}'...");
        self.save_to_file(path)?;
        println!("Game saved.");

        Ok(())
    }
}

/// Helper function to save VM state
pub fn save_game(vm: &VM) -> Result<(), String> {
    let save = SaveGame::from_vm(vm)?;
    save.save_with_prompt()
}
