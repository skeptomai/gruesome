//! WASM-compatible save/restore using byte arrays instead of files
//!
//! This module provides Quetzal format save/restore that works in WASM
//! by serializing to/from Vec<u8> instead of using file I/O.

use crate::interpreter::core::vm::VM;
use crate::interpreter::quetzal::chunks::{IFhdChunk, IntDChunk, StksChunk};
use crate::interpreter::quetzal::compressed_memory::{decompress_memory, CMemChunk};
use log::debug;

/// Serialize VM state to Quetzal format bytes
pub fn save_to_bytes(vm: &VM) -> Result<Vec<u8>, String> {
    // Build chunks
    let ifhd = IFhdChunk::from_vm(vm);
    let ifhd_bytes = ifhd.to_bytes();

    // Get original memory for XOR compression
    let original_memory = vm
        .game
        .original_memory
        .as_ref()
        .ok_or("No original memory available for compression")?;

    let dynamic_size = vm.game.header.base_static_mem;
    let current_dynamic = &vm.game.memory[..dynamic_size];
    let original_dynamic = &original_memory[..dynamic_size];

    let cmem = CMemChunk::from_memory(current_dynamic, original_dynamic);
    let cmem_bytes = cmem.to_bytes();

    let stks = StksChunk::from_vm(vm);
    let stks_bytes = stks.to_bytes();

    let intd = IntDChunk::new();
    let intd_bytes = intd.to_bytes();

    // Build IFF structure
    let chunks: Vec<(&[u8; 4], &[u8])> = vec![
        (b"IFhd", &ifhd_bytes),
        (b"CMem", &cmem_bytes),
        (b"Stks", &stks_bytes),
        (b"IntD", &intd_bytes),
    ];

    // Calculate total size for FORM header
    let chunks_size: usize = chunks
        .iter()
        .map(|(_, data)| {
            let padded = if data.len() % 2 == 1 {
                data.len() + 1
            } else {
                data.len()
            };
            8 + padded // 4 bytes type + 4 bytes size + padded data
        })
        .sum();
    let total_size = 4 + chunks_size; // 4 bytes for "IFZS" form type

    let mut output = Vec::with_capacity(8 + total_size);

    // Write FORM header
    output.extend_from_slice(b"FORM");
    output.extend_from_slice(&(total_size as u32).to_be_bytes());

    // Write form type
    output.extend_from_slice(b"IFZS");

    // Write each chunk
    for (chunk_type, data) in chunks {
        output.extend_from_slice(chunk_type);
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(data);

        // Pad to even length
        if data.len() % 2 == 1 {
            output.push(0);
        }
    }

    debug!("Created Quetzal save data: {} bytes", output.len());
    Ok(output)
}

/// Restore VM state from Quetzal format bytes
pub fn restore_from_bytes(vm: &mut VM, data: &[u8]) -> Result<(), String> {
    // Parse IFF structure
    if data.len() < 12 {
        return Err("Save data too small".to_string());
    }

    // Check FORM header
    if &data[0..4] != b"FORM" {
        return Err("Not an IFF file (missing FORM header)".to_string());
    }

    // Check form type
    if &data[8..12] != b"IFZS" {
        return Err("Not a Quetzal save file (expected IFZS form type)".to_string());
    }

    // Parse chunks
    let mut offset = 12;
    let mut ifhd_data: Option<&[u8]> = None;
    let mut cmem_data: Option<&[u8]> = None;
    let mut umem_data: Option<&[u8]> = None;
    let mut stks_data: Option<&[u8]> = None;

    while offset + 8 <= data.len() {
        let chunk_type = &data[offset..offset + 4];
        let chunk_size = u32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        offset += 8;

        if offset + chunk_size > data.len() {
            return Err("Chunk extends beyond file".to_string());
        }

        let chunk_data = &data[offset..offset + chunk_size];

        match chunk_type {
            b"IFhd" => ifhd_data = Some(chunk_data),
            b"CMem" => cmem_data = Some(chunk_data),
            b"UMem" => umem_data = Some(chunk_data),
            b"Stks" => stks_data = Some(chunk_data),
            _ => {} // Ignore unknown chunks
        }

        // Move past chunk data, accounting for padding
        offset += chunk_size;
        if chunk_size % 2 == 1 {
            offset += 1;
        }
    }

    // Verify required chunks
    let ifhd_bytes = ifhd_data.ok_or("Missing required IFhd chunk")?;
    let stks_bytes = stks_data.ok_or("Missing required Stks chunk")?;

    if cmem_data.is_none() && umem_data.is_none() {
        return Err("Missing memory chunk (CMem or UMem)".to_string());
    }

    // Verify game matches
    let ifhd = IFhdChunk::from_bytes(ifhd_bytes)?;
    if ifhd.release != vm.game.header.release {
        return Err(format!(
            "Save file is for release {}, but game is release {}",
            ifhd.release, vm.game.header.release
        ));
    }

    // Restore memory
    let dynamic_size = vm.game.header.base_static_mem;

    if let Some(cmem_bytes) = cmem_data {
        // Compressed memory
        let original_memory = vm
            .game
            .original_memory
            .as_ref()
            .ok_or("No original memory available for decompression")?;
        let original_dynamic = &original_memory[..dynamic_size];

        let restored = decompress_memory(cmem_bytes, original_dynamic)?;
        vm.game.memory[..dynamic_size].copy_from_slice(&restored);
        debug!("Restored {} bytes from CMem", dynamic_size);
    } else if let Some(umem_bytes) = umem_data {
        // Uncompressed memory
        if umem_bytes.len() != dynamic_size {
            return Err(format!(
                "UMem size {} doesn't match dynamic memory size {}",
                umem_bytes.len(),
                dynamic_size
            ));
        }
        vm.game.memory[..dynamic_size].copy_from_slice(umem_bytes);
        debug!("Restored {} bytes from UMem", dynamic_size);
    }

    // Restore stack
    let stks = StksChunk {
        data: stks_bytes.to_vec(),
    };
    stks.restore_to_vm(vm)?;
    debug!("Restored {} call frames", vm.call_stack.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here if we had a way to create a test VM
}
