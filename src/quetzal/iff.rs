//! IFF (Interchange File Format) handling for Quetzal files

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// IFF file structure
pub struct IffFile {
    /// Form type - should be "IFZS" for Quetzal
    pub form_type: [u8; 4],
    /// List of chunks in the file
    pub chunks: Vec<IffChunk>,
}

/// Individual chunk in an IFF file
pub struct IffChunk {
    /// 4-character chunk type identifier
    pub chunk_type: [u8; 4],
    /// Chunk data
    pub data: Vec<u8>,
}

impl IffFile {
    /// Create a new Quetzal IFF file
    pub fn new() -> Self {
        IffFile {
            form_type: *b"IFZS",
            chunks: Vec::new(),
        }
    }
    
    /// Add a chunk to the file
    pub fn add_chunk(&mut self, chunk_type: [u8; 4], data: Vec<u8>) {
        self.chunks.push(IffChunk { chunk_type, data });
    }
    
    /// Write the IFF file to disk
    pub fn write_to_file(&self, path: &Path) -> Result<(), String> {
        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create save file: {}", e))?;
        
        // Calculate total size (all chunks + 4 bytes for form type)
        let total_size = 4 + self.chunks.iter()
            .map(|c| 8 + c.data.len())  // 4 bytes type + 4 bytes size + data
            .sum::<usize>();
        
        // Write FORM header
        file.write_all(b"FORM")
            .map_err(|e| format!("Failed to write FORM header: {}", e))?;
        
        // Write size (big-endian)
        file.write_all(&(total_size as u32).to_be_bytes())
            .map_err(|e| format!("Failed to write size: {}", e))?;
        
        // Write form type
        file.write_all(&self.form_type)
            .map_err(|e| format!("Failed to write form type: {}", e))?;
        
        // Write each chunk
        for chunk in &self.chunks {
            // Chunk type
            file.write_all(&chunk.chunk_type)
                .map_err(|e| format!("Failed to write chunk type: {}", e))?;
            
            // Chunk size (big-endian)
            file.write_all(&(chunk.data.len() as u32).to_be_bytes())
                .map_err(|e| format!("Failed to write chunk size: {}", e))?;
            
            // Chunk data
            file.write_all(&chunk.data)
                .map_err(|e| format!("Failed to write chunk data: {}", e))?;
            
            // Pad to even length if necessary
            if chunk.data.len() % 2 == 1 {
                file.write_all(&[0])
                    .map_err(|e| format!("Failed to write padding: {}", e))?;
            }
        }
        
        Ok(())
    }
    
    /// Read an IFF file from disk
    pub fn read_from_file(path: &Path) -> Result<Self, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open save file: {}", e))?;
        
        let mut header = [0u8; 4];
        file.read_exact(&mut header)
            .map_err(|e| format!("Failed to read FORM header: {}", e))?;
        
        if &header != b"FORM" {
            return Err("Not an IFF file (missing FORM header)".to_string());
        }
        
        // Read size
        let mut size_bytes = [0u8; 4];
        file.read_exact(&mut size_bytes)
            .map_err(|e| format!("Failed to read size: {}", e))?;
        let _total_size = u32::from_be_bytes(size_bytes);
        
        // Read form type
        let mut form_type = [0u8; 4];
        file.read_exact(&mut form_type)
            .map_err(|e| format!("Failed to read form type: {}", e))?;
        
        let mut iff = IffFile {
            form_type,
            chunks: Vec::new(),
        };
        
        // Read chunks
        loop {
            let mut chunk_type = [0u8; 4];
            match file.read_exact(&mut chunk_type) {
                Ok(_) => {},
                Err(_) => break,  // End of file
            }
            
            let mut size_bytes = [0u8; 4];
            file.read_exact(&mut size_bytes)
                .map_err(|e| format!("Failed to read chunk size: {}", e))?;
            let chunk_size = u32::from_be_bytes(size_bytes) as usize;
            
            let mut data = vec![0u8; chunk_size];
            file.read_exact(&mut data)
                .map_err(|e| format!("Failed to read chunk data: {}", e))?;
            
            iff.chunks.push(IffChunk { chunk_type, data });
            
            // Skip padding byte if chunk size is odd
            if chunk_size % 2 == 1 {
                let mut _padding = [0u8; 1];
                file.read_exact(&mut _padding).ok();
            }
        }
        
        Ok(iff)
    }
    
    /// Find a chunk by type
    pub fn find_chunk(&self, chunk_type: &[u8; 4]) -> Option<&IffChunk> {
        self.chunks.iter().find(|c| &c.chunk_type == chunk_type)
    }
}