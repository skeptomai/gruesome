//! XOR-RLE compression for Quetzal CMem chunks
//! 
//! The compression works by XORing the current memory with the original
//! game file, then run-length encoding the zeros.

use log::debug;

/// Compress dynamic memory using XOR-RLE compression
/// 
/// This XORs the current memory with the original, then compresses runs of zeros
pub fn compress_memory(current: &[u8], original: &[u8]) -> Vec<u8> {
    if current.len() != original.len() {
        panic!("Memory sizes don't match for compression");
    }
    
    let mut compressed = Vec::new();
    let mut i = 0;
    
    while i < current.len() {
        let xor_byte = current[i] ^ original[i];
        
        if xor_byte == 0 {
            // Count consecutive zeros
            let start = i;
            while i < current.len() && (current[i] ^ original[i]) == 0 {
                i += 1;
            }
            let run_length = i - start;
            
            // Encode the run of zeros
            if run_length <= 256 {
                // Short run: 0 followed by (length-1)
                compressed.push(0);
                compressed.push((run_length - 1) as u8);
            } else {
                // Long runs need to be split
                let mut remaining = run_length;
                while remaining > 256 {
                    compressed.push(0);
                    compressed.push(255); // Maximum short run
                    remaining -= 256;
                }
                if remaining > 0 {
                    compressed.push(0);
                    compressed.push((remaining - 1) as u8);
                }
            }
        } else {
            // Non-zero byte
            compressed.push(xor_byte);
            i += 1;
        }
    }
    
    debug!("Compressed {} bytes to {} bytes", current.len(), compressed.len());
    compressed
}

/// Decompress memory using XOR-RLE decompression
/// 
/// This decompresses the RLE data and XORs with the original to get current memory
pub fn decompress_memory(compressed: &[u8], original: &[u8]) -> Result<Vec<u8>, String> {
    let mut decompressed = Vec::with_capacity(original.len());
    let mut compressed_idx = 0;
    let mut original_idx = 0;
    
    while compressed_idx < compressed.len() && original_idx < original.len() {
        let byte = compressed[compressed_idx];
        compressed_idx += 1;
        
        if byte == 0 {
            // This is a run of zeros
            if compressed_idx >= compressed.len() {
                return Err("Incomplete RLE sequence".to_string());
            }
            
            let run_length = (compressed[compressed_idx] as usize) + 1;
            compressed_idx += 1;
            
            // Zeros in XOR means bytes are unchanged from original
            for _ in 0..run_length {
                if original_idx >= original.len() {
                    return Err("RLE run extends beyond memory".to_string());
                }
                decompressed.push(original[original_idx]);
                original_idx += 1;
            }
        } else {
            // Non-zero XOR byte
            if original_idx >= original.len() {
                return Err("Compressed data extends beyond memory".to_string());
            }
            decompressed.push(original[original_idx] ^ byte);
            original_idx += 1;
        }
    }
    
    // Fill any remaining bytes from original
    while original_idx < original.len() {
        decompressed.push(original[original_idx]);
        original_idx += 1;
    }
    
    if decompressed.len() != original.len() {
        return Err(format!(
            "Decompressed size {} doesn't match original size {}",
            decompressed.len(),
            original.len()
        ));
    }
    
    Ok(decompressed)
}

/// CMem chunk - Compressed memory
pub struct CMemChunk {
    /// Compressed memory data
    pub data: Vec<u8>,
}

impl CMemChunk {
    /// Create from current and original memory
    pub fn from_memory(current: &[u8], original: &[u8]) -> Self {
        let data = compress_memory(current, original);
        CMemChunk { data }
    }
    
    /// Get compressed data as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
    
    /// Restore to memory given the original
    pub fn restore_to_memory(&self, original: &[u8]) -> Result<Vec<u8>, String> {
        decompress_memory(&self.data, original)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compress_decompress() {
        let original = vec![0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];
        let current = vec![0x10, 0x21, 0x30, 0x40, 0x50, 0x60, 0x71, 0x80];
        
        let compressed = compress_memory(&current, &original);
        let restored = decompress_memory(&compressed, &original).unwrap();
        
        assert_eq!(current, restored);
    }
    
    #[test]
    fn test_long_run_of_zeros() {
        let original = vec![0xFF; 1000];
        let mut current = original.clone();
        current[500] = 0xFE; // Change one byte
        
        let compressed = compress_memory(&current, &original);
        assert!(compressed.len() < current.len()); // Should be well compressed
        
        let restored = decompress_memory(&compressed, &original).unwrap();
        assert_eq!(current, restored);
    }
}