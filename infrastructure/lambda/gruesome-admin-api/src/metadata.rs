use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZMachineHeader {
    pub version: u8,
    pub release: u16,
    pub serial: String,
    pub checksum: String,
    pub file_length: u32,
}

/// Extract metadata from Z-Machine file header
///
/// Z-Machine header layout (first 64 bytes):
/// - Byte 0: Version number (3, 4, 5, or 8)
/// - Bytes 2-3: Release number (big-endian)
/// - Bytes 18-23: Serial number (6 ASCII characters, usually YYMMDD)
/// - Bytes 28-29: File checksum (big-endian)
pub fn extract_metadata(file_bytes: &[u8]) -> Result<ZMachineHeader, ApiError> {
    if file_bytes.len() < 64 {
        return Err(ApiError::BadRequest(
            "File too small to be valid Z-Machine file (minimum 64 bytes required)".to_string(),
        ));
    }

    // Extract version (byte 0)
    let version = file_bytes[0];
    if ![3, 4, 5, 8].contains(&version) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported Z-Machine version: {} (expected 3, 4, 5, or 8)",
            version
        )));
    }

    // Extract release number (bytes 2-3, big-endian)
    let release = u16::from_be_bytes([file_bytes[2], file_bytes[3]]);

    // Extract serial number (bytes 18-23, ASCII)
    let serial_bytes = &file_bytes[18..24];
    let serial = String::from_utf8_lossy(serial_bytes).to_string();

    // Validate serial is ASCII digits (YYMMDD format)
    if !serial.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid serial number format: '{}' (expected 6 ASCII digits)",
            serial
        )));
    }

    // Extract checksum (bytes 28-29, big-endian)
    let checksum_value = u16::from_be_bytes([file_bytes[28], file_bytes[29]]);
    let checksum = format!("{:04x}", checksum_value);

    // File length
    let file_length = file_bytes.len() as u32;

    Ok(ZMachineHeader {
        version,
        release,
        serial,
        checksum,
        file_length,
    })
}

/// Validate that the file is a valid Z-Machine file
pub fn validate_zmachine_file(bytes: &[u8]) -> Result<(), ApiError> {
    if bytes.len() < 64 {
        return Err(ApiError::BadRequest(
            "File too small (minimum 64 bytes required)".to_string(),
        ));
    }

    // Check version byte (must be 3, 4, 5, or 8)
    let version = bytes[0];
    if ![3, 4, 5, 8].contains(&version) {
        return Err(ApiError::BadRequest(format!(
            "Invalid Z-Machine version: {} (expected 3, 4, 5, or 8)",
            version
        )));
    }

    // Check high memory mark is reasonable (bytes 4-5)
    let high_mem = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if high_mem > bytes.len() {
        return Err(ApiError::BadRequest(format!(
            "Invalid high memory mark: 0x{:04x} exceeds file size {}",
            high_mem,
            bytes.len()
        )));
    }

    // Check initial PC is reasonable (bytes 6-7)
    let initial_pc = u16::from_be_bytes([bytes[6], bytes[7]]) as usize;
    if initial_pc >= bytes.len() {
        return Err(ApiError::BadRequest(format!(
            "Invalid initial PC: 0x{:04x} exceeds file size {}",
            initial_pc,
            bytes.len()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_metadata_valid_v3() {
        // Create a minimal valid Z-Machine v3 header
        let mut header = vec![0u8; 64];
        header[0] = 3; // Version 3
        header[2] = 0; // Release high byte
        header[3] = 88; // Release low byte (release 88)
        header[18..24].copy_from_slice(b"840726"); // Serial: July 26, 1984
        header[28] = 0x80; // Checksum high byte
        header[29] = 0xb8; // Checksum low byte

        let result = extract_metadata(&header).unwrap();

        assert_eq!(result.version, 3);
        assert_eq!(result.release, 88);
        assert_eq!(result.serial, "840726");
        assert_eq!(result.checksum, "80b8");
        assert_eq!(result.file_length, 64);
    }

    #[test]
    fn test_validate_zmachine_file_too_small() {
        let small_file = vec![0u8; 32];
        let result = validate_zmachine_file(&small_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zmachine_file_invalid_version() {
        let mut header = vec![0u8; 64];
        header[0] = 2; // Invalid version
        let result = validate_zmachine_file(&header);
        assert!(result.is_err());
    }
}
