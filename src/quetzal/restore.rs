//! Restore game functionality for Quetzal format

use crate::vm::VM;
use crate::quetzal::iff::IffFile;
use crate::quetzal::chunks::{IFhdChunk, StksChunk};
use crate::quetzal::compressed_memory::decompress_memory;
use std::path::Path;
use log::{debug, info, warn};

/// RestoreGame handles loading Quetzal save files
pub struct RestoreGame {
    /// The loaded IFF file
    iff: IffFile,
}

impl RestoreGame {
    /// Load a save game from file
    pub fn from_file(path: &Path) -> Result<Self, String> {
        info!("Loading save game from {:?}", path);
        let iff = IffFile::read_from_file(path)?;
        
        // Verify it's a Quetzal file
        if &iff.form_type != b"IFZS" {
            return Err(format!(
                "Not a Quetzal save file (found form type {:?})",
                std::str::from_utf8(&iff.form_type).unwrap_or("<invalid>")
            ));
        }
        
        // Verify required chunks exist
        if iff.find_chunk(b"IFhd").is_none() {
            return Err("Missing required IFhd chunk".to_string());
        }
        
        if iff.find_chunk(b"CMem").is_none() && iff.find_chunk(b"UMem").is_none() {
            return Err("Missing memory chunk (CMem or UMem)".to_string());
        }
        
        if iff.find_chunk(b"Stks").is_none() {
            return Err("Missing required Stks chunk".to_string());
        }
        
        Ok(RestoreGame { iff })
    }
    
    /// Restore the save game to a VM
    pub fn restore_to_vm(&self, vm: &mut VM) -> Result<(), String> {
        // First verify the game matches
        let ifhd_chunk = self.iff.find_chunk(b"IFhd")
            .ok_or("Missing IFhd chunk")?;
        let ifhd = IFhdChunk::from_bytes(&ifhd_chunk.data)?;
        
        // Check release number matches
        if ifhd.release != vm.game.header.release {
            return Err(format!(
                "Save file is for release {}, but game is release {}",
                ifhd.release, vm.game.header.release
            ));
        }
        
        // Check serial number matches
        let mut game_serial = [0u8; 6];
        for i in 0..6 {
            game_serial[i] = vm.game.memory[0x12 + i];
        }
        if ifhd.serial != game_serial {
            warn!("Serial number mismatch - save may be incompatible");
        }
        
        // Restore memory
        if let Some(cmem_chunk) = self.iff.find_chunk(b"CMem") {
            // Compressed memory
            debug!("Restoring from CMem chunk");
            
            let original_memory = vm.game.original_memory.as_ref()
                .ok_or("No original memory available for decompression")?;
            
            let dynamic_size = vm.game.header.base_static_mem;
            let original_dynamic = &original_memory[..dynamic_size];
            
            let restored = decompress_memory(&cmem_chunk.data, original_dynamic)?;
            
            // Copy restored memory to VM
            vm.game.memory[..dynamic_size].copy_from_slice(&restored);
            debug!("Restored {} bytes of dynamic memory", dynamic_size);
            
        } else if let Some(umem_chunk) = self.iff.find_chunk(b"UMem") {
            // Uncompressed memory
            debug!("Restoring from UMem chunk");
            
            let dynamic_size = vm.game.header.base_static_mem;
            if umem_chunk.data.len() != dynamic_size {
                return Err(format!(
                    "UMem size {} doesn't match dynamic memory size {}",
                    umem_chunk.data.len(), dynamic_size
                ));
            }
            
            vm.game.memory[..dynamic_size].copy_from_slice(&umem_chunk.data);
            debug!("Restored {} bytes of dynamic memory", dynamic_size);
        }
        
        // Restore stack
        let stks_chunk = self.iff.find_chunk(b"Stks")
            .ok_or("Missing Stks chunk")?;
        let stks = StksChunk { data: stks_chunk.data.clone() };
        stks.restore_to_vm(vm)?;
        debug!("Restored {} call frames", vm.call_stack.len());
        
        // In Z-Machine v1-3, restore continues from where the restore 
        // instruction was called, NOT from where save was called.
        // The PC should not be changed by restore - it will be set by
        // the interpreter after the restore instruction completes.
        debug!("Restore complete - PC will continue from restore instruction");
        
        info!("Game restored successfully");
        Ok(())
    }
    
    /// Load from a prompt-selected file
    pub fn load_with_prompt() -> Result<Self, String> {
        use std::io::{self, Write};
        
        print!("Enter save filename: ");
        io::stdout().flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;
        
        let mut filename = String::new();
        io::stdin().read_line(&mut filename)
            .map_err(|e| format!("Failed to read filename: {}", e))?;
        
        let filename = filename.trim();
        if filename.is_empty() {
            return Err("No filename provided".to_string());
        }
        
        // Add .sav extension if not present
        let filename = if filename.ends_with(".sav") || filename.ends_with(".qzl") {
            filename.to_string()
        } else {
            format!("{}.sav", filename)
        };
        
        let path = Path::new(&filename);
        
        println!("Loading game from '{}'...", filename);
        RestoreGame::from_file(path)
    }
}

/// Helper function to restore VM state
pub fn restore_game(vm: &mut VM) -> Result<(), String> {
    let restore = RestoreGame::load_with_prompt()?;
    restore.restore_to_vm(vm)?;
    println!("Game restored.");
    Ok(())
}