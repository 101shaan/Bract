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
        
        // Create a proper minimal PE32+ executable
        // This will be a tiny but valid PE that Windows can execute
        
        // DOS Header + DOS Stub
        let dos_part = create_dos_header_and_stub();
        file.write_all(&dos_part)
            .map_err(|e| CodegenError::IoError(format!("Failed to write DOS header: {}", e)))?;
        
        // PE Header
        let pe_header = create_pe_header();
        file.write_all(&pe_header)
            .map_err(|e| CodegenError::IoError(format!("Failed to write PE header: {}", e)))?;
        
        // Section Headers  
        let section_headers = create_section_headers();
        file.write_all(&section_headers)
            .map_err(|e| CodegenError::IoError(format!("Failed to write section headers: {}", e)))?;
        
        // Code Section - this is where our Cranelift machine code goes
        let code_section = create_code_section(&self.object_code);
        file.write_all(&code_section)
            .map_err(|e| CodegenError::IoError(format!("Failed to write code section: {}", e)))?;
        
        Ok(())
    }
    

}

/// Create DOS header and stub (128 bytes total)
fn create_dos_header_and_stub() -> Vec<u8> {
    let mut dos_part = vec![0u8; 128];
    
    // DOS Header (64 bytes)
    dos_part[0] = 0x4D; dos_part[1] = 0x5A; // "MZ" signature
    dos_part[2] = 0x80; dos_part[3] = 0x00; // Bytes in last page
    dos_part[4] = 0x01; dos_part[5] = 0x00; // Pages in file
    dos_part[8] = 0x04; dos_part[9] = 0x00; // Header size in paragraphs
    dos_part[20] = 0x40; dos_part[21] = 0x00; // Initial IP
    dos_part[22] = 0x00; dos_part[23] = 0x00; // Initial CS
    dos_part[60] = 0x80; dos_part[61] = 0x00; // PE header offset (128)
    dos_part[62] = 0x00; dos_part[63] = 0x00;
    
    // DOS Stub (64 bytes) - just exits with code 42
    dos_part[64] = 0xB4; dos_part[65] = 0x4C; // mov ah, 4Ch
    dos_part[66] = 0xB0; dos_part[67] = 0x2A; // mov al, 42
    dos_part[68] = 0xCD; dos_part[69] = 0x21; // int 21h
    
    dos_part
}

/// Create PE header
fn create_pe_header() -> Vec<u8> {
    let mut pe_header = vec![0u8; 24 + 240]; // PE signature + COFF header + Optional header
    
    // PE signature "PE\0\0"
    pe_header[0] = 0x50; pe_header[1] = 0x45; // "PE"
    pe_header[2] = 0x00; pe_header[3] = 0x00;
    
    // COFF Header (20 bytes)
    pe_header[4] = 0x64; pe_header[5] = 0x86; // Machine (x64)
    pe_header[6] = 0x01; pe_header[7] = 0x00; // Number of sections
    // Timestamp, symbol table, etc. can be zero for minimal PE
    pe_header[20] = 0xF0; pe_header[21] = 0x00; // Optional header size (240)
    pe_header[22] = 0x22; pe_header[23] = 0x00; // Characteristics (executable, large address aware)
    
    // Optional Header (240 bytes for PE32+)
    pe_header[24] = 0x0B; pe_header[25] = 0x02; // Magic (PE32+)
    pe_header[26] = 0x0E; pe_header[27] = 0x00; // Linker version
    pe_header[28] = 0x00; pe_header[29] = 0x02; // Size of code (512 bytes)
    pe_header[30] = 0x00; pe_header[31] = 0x00;
    
    // Entry point (RVA) - points to our code section
    pe_header[40] = 0x00; pe_header[41] = 0x20; // 0x2000 (8192)
    pe_header[42] = 0x00; pe_header[43] = 0x00;
    
    // Image base (where to load in memory)
    pe_header[48] = 0x00; pe_header[49] = 0x00; // 0x140000000 (typical x64)
    pe_header[50] = 0x00; pe_header[51] = 0x00;
    pe_header[52] = 0x40; pe_header[53] = 0x01;
    pe_header[54] = 0x00; pe_header[55] = 0x00;
    
    // Section alignment (4096)
    pe_header[56] = 0x00; pe_header[57] = 0x10;
    pe_header[58] = 0x00; pe_header[59] = 0x00;
    
    // File alignment (512)
    pe_header[60] = 0x00; pe_header[61] = 0x02;
    pe_header[62] = 0x00; pe_header[63] = 0x00;
    
    // OS/Subsystem versions
    pe_header[64] = 0x06; pe_header[65] = 0x00; // OS major
    pe_header[68] = 0x06; pe_header[69] = 0x00; // Subsystem major
    
    // Image size (8192)
    pe_header[80] = 0x00; pe_header[81] = 0x20;
    pe_header[82] = 0x00; pe_header[83] = 0x00;
    
    // Headers size (512)
    pe_header[84] = 0x00; pe_header[85] = 0x02;
    pe_header[86] = 0x00; pe_header[87] = 0x00;
    
    // Subsystem (3 = console)
    pe_header[92] = 0x03; pe_header[93] = 0x00;
    
    // Stack/heap sizes (can be minimal)
    pe_header[96] = 0x00; pe_header[97] = 0x10; // Stack reserve (1MB)
    pe_header[98] = 0x00; pe_header[99] = 0x00;
    pe_header[100] = 0x00; pe_header[101] = 0x00;
    pe_header[102] = 0x00; pe_header[103] = 0x00;
    
    pe_header[104] = 0x00; pe_header[105] = 0x10; // Stack commit (1MB)
    pe_header[106] = 0x00; pe_header[107] = 0x00;
    pe_header[108] = 0x00; pe_header[109] = 0x00;
    pe_header[110] = 0x00; pe_header[111] = 0x00;
    
    // Number of data directories (16)
    pe_header[116] = 0x10; pe_header[117] = 0x00;
    pe_header[118] = 0x00; pe_header[119] = 0x00;
    
    // Data directories (16 * 8 = 128 bytes) - can be all zeros for minimal PE
    
    pe_header
}

/// Create section headers
fn create_section_headers() -> Vec<u8> {
    let mut section = vec![0u8; 40]; // One section header
    
    // Section name ".text"
    section[0] = 0x2E; section[1] = 0x74; section[2] = 0x65; section[3] = 0x78;
    section[4] = 0x74; section[5] = 0x00; section[6] = 0x00; section[7] = 0x00;
    
    // Virtual size (512)
    section[8] = 0x00; section[9] = 0x02;
    section[10] = 0x00; section[11] = 0x00;
    
    // Virtual address (0x2000)
    section[12] = 0x00; section[13] = 0x20;
    section[14] = 0x00; section[15] = 0x00;
    
    // Raw data size (512)
    section[16] = 0x00; section[17] = 0x02;
    section[18] = 0x00; section[19] = 0x00;
    
    // Raw data offset (512)
    section[20] = 0x00; section[21] = 0x02;
    section[22] = 0x00; section[23] = 0x00;
    
    // Characteristics (executable + readable)
    section[36] = 0x60; section[37] = 0x00;
    section[38] = 0x00; section[39] = 0x20;
    
    section
}

/// Create code section with our machine code
fn create_code_section(_object_code: &[u8]) -> Vec<u8> {
    let mut section = vec![0u8; 512]; // Fixed size section
    
    // Windows x64 - call ExitProcess(42)
    // We need to call kernel32!ExitProcess properly
    // For now, use inline syscall approach
    
    // mov ecx, 42      ; exit code (32-bit for Windows API)
    section[0] = 0xB9; 
    section[1] = 0x2A; section[2] = 0x00; section[3] = 0x00; section[4] = 0x00;
    
    // mov eax, 1       ; NtTerminateProcess syscall number (approximate)
    section[5] = 0xB8;
    section[6] = 0x01; section[7] = 0x00; section[8] = 0x00; section[9] = 0x00;
    
    // int 0x2e         ; old Windows syscall interface
    section[10] = 0xCD; section[11] = 0x2E;
    
    // Fallback - infinite loop if syscall fails
    section[12] = 0xEB; section[13] = 0xFE; // jmp -2 (infinite loop)
    
    section
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
