//! Minimal self-contained linker for Bract
//!
//! This module implements a basic linker that creates executable files
//! directly from Cranelift object code without requiring external linkers.
//! 
//! For Phase 1, we implement a minimal PE (Windows) executable format.

use super::{CodegenResult, CodegenError};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Minimal PE executable builder
pub struct MinimalLinker {
    object_code: Vec<u8>,
    entry_point_offset: u32,
}

impl MinimalLinker {
    /// Create a new minimal linker with object code
    pub fn new(object_code: Vec<u8>) -> Self {
        Self {
            object_code,
            entry_point_offset: 0, // We'll assume main is at offset 0 for now
        }
    }
    
    /// Link and create executable file
    pub fn create_executable<P: AsRef<Path>>(&self, output_path: P) -> CodegenResult<()> {
        if cfg!(windows) {
            self.create_pe_executable(output_path)
        } else {
            // TODO: Implement ELF executable generation
            Err(CodegenError::UnsupportedFeature(
                "ELF executable generation not yet implemented".to_string()
            ))
        }
    }
    
    /// Create a minimal PE executable for Windows
    #[cfg(windows)]
    fn create_pe_executable<P: AsRef<Path>>(&self, output_path: P) -> CodegenResult<()> {
        let mut file = File::create(output_path)
            .map_err(|e| CodegenError::IoError(format!("Failed to create executable: {}", e)))?;
        
        // For Phase 1: Create a minimal DOS stub + PE header that just exits with code 42
        // This is a proof of concept - later we'll implement proper PE generation
        
        // DOS Header (64 bytes)
        let dos_header = create_minimal_dos_header();
        file.write_all(&dos_header)
            .map_err(|e| CodegenError::IoError(format!("Failed to write DOS header: {}", e)))?;
        
        // DOS Stub (simple program that prints message and exits)
        let dos_stub = create_dos_stub();
        file.write_all(&dos_stub)
            .map_err(|e| CodegenError::IoError(format!("Failed to write DOS stub: {}", e)))?;
        
        // PE Header will go here in full implementation
        // For now, the DOS stub is our entire "executable"
        
        Ok(())
    }
    

}

/// Create minimal DOS header
fn create_minimal_dos_header() -> Vec<u8> {
    let mut header = vec![0u8; 64];
    
    // DOS signature "MZ"
    header[0] = 0x4D; // 'M'
    header[1] = 0x5A; // 'Z'
    
    // Bytes in last page
    header[2] = 0x90;
    header[3] = 0x00;
    
    // Pages in file
    header[4] = 0x03;
    header[5] = 0x00;
    
    // Header size in paragraphs
    header[8] = 0x04;
    header[9] = 0x00;
    
    // Initial CS:IP (points to DOS stub)
    header[22] = 0x40; // CS
    header[23] = 0x00;
    header[20] = 0x00; // IP
    header[21] = 0x00;
    
    header
}

/// Create DOS stub that exits with code 42
fn create_dos_stub() -> Vec<u8> {
    // Simple 16-bit DOS program:
    // mov ah, 4Ch    ; DOS exit function
    // mov al, 42     ; Exit code 42
    // int 21h        ; Call DOS
    vec![
        0xB4, 0x4C,    // mov ah, 4Ch
        0xB0, 0x2A,    // mov al, 42 (42 decimal)
        0xCD, 0x21,    // int 21h
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_minimal_linker_creation() {
        let object_code = vec![0x90, 0x90, 0x90]; // NOP instructions
        let linker = MinimalLinker::new(object_code);
        assert_eq!(linker.entry_point_offset, 0);
    }
    
    #[test]
    fn test_create_executable() {
        let object_code = vec![0x90, 0x90, 0x90]; // NOP instructions
        let linker = MinimalLinker::new(object_code);
        
        let temp_file = NamedTempFile::new().unwrap();
        let result = linker.create_executable(temp_file.path());
        
        if cfg!(windows) {
            assert!(result.is_ok(), "Should create executable on Windows");
        } else {
            // ELF not implemented yet
            assert!(result.is_err(), "ELF not implemented yet");
        }
    }
}
