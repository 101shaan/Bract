//! Build System Integration for Prism
//!
//! This module handles the integration with C compilers to build the generated C code
//! into executables. It supports GCC, Clang, and MSVC with optimization and debugging options.

use super::{CodegenResult, CodegenError};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs;
use std::env;

/// C compiler types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CCompiler {
    Gcc,
    Clang,
    Msvc,
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Target compiler
    pub compiler: CCompiler,
    /// Optimization level (0-3)
    pub optimization_level: u8,
    /// Enable debug information
    pub debug_info: bool,
    /// Enable address sanitizer
    pub address_sanitizer: bool,
    /// Enable undefined behavior sanitizer
    pub undefined_sanitizer: bool,
    /// Additional compiler flags
    pub extra_flags: Vec<String>,
    /// Additional linker flags
    pub linker_flags: Vec<String>,
    /// Include directories
    pub include_dirs: Vec<PathBuf>,
    /// Library directories
    pub library_dirs: Vec<PathBuf>,
    /// Libraries to link
    pub libraries: Vec<String>,
    /// Output directory
    pub output_dir: PathBuf,
    /// Target architecture
    pub target_arch: Option<String>,
    /// Cross compilation target
    pub target_triple: Option<String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            compiler: CCompiler::Gcc,
            optimization_level: 0,
            debug_info: true,
            address_sanitizer: false,
            undefined_sanitizer: false,
            extra_flags: Vec::new(),
            linker_flags: Vec::new(),
            include_dirs: Vec::new(),
            library_dirs: Vec::new(),
            libraries: Vec::new(),
            output_dir: PathBuf::from("target"),
            target_arch: None,
            target_triple: None,
        }
    }
}

/// Build system for C code
pub struct BuildSystem {
    /// Build configuration
    config: BuildConfig,
}

impl BuildSystem {
    /// Create a new build system with default configuration
    pub fn new() -> Self {
        Self {
            config: BuildConfig::default(),
        }
    }
    
    /// Create build system with custom configuration
    pub fn with_config(config: BuildConfig) -> Self {
        Self { config }
    }
    
    /// Detect available C compiler
    pub fn detect_compiler() -> CodegenResult<CCompiler> {
        // Try to find compilers in order of preference
        let compilers = [
            (CCompiler::Clang, "clang"),
            (CCompiler::Gcc, "gcc"),
            (CCompiler::Msvc, "cl"),
        ];
        
        for (compiler_type, command) in &compilers {
            if Self::command_exists(command) {
                return Ok(*compiler_type);
            }
        }
        
        Err(CodegenError::InternalError(
            "No suitable C compiler found. Please install GCC, Clang, or MSVC.".to_string()
        ))
    }
    
    /// Check if a command exists in PATH
    fn command_exists(command: &str) -> bool {
        Command::new(command)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }
    
    /// Build C source files into an executable
    pub fn build_executable(
        &self,
        source_files: &[PathBuf],
        output_name: &str
    ) -> CodegenResult<PathBuf> {
        // Ensure output directory exists
        fs::create_dir_all(&self.config.output_dir)
            .map_err(|e| CodegenError::IoError(format!("Failed to create output directory: {}", e)))?;
        
        // Determine output path
        let output_path = self.config.output_dir.join(output_name);
        
        match self.config.compiler {
            CCompiler::Gcc => self.build_with_gcc(source_files, &output_path),
            CCompiler::Clang => self.build_with_clang(source_files, &output_path),
            CCompiler::Msvc => self.build_with_msvc(source_files, &output_path),
        }
    }
    
    /// Build with GCC
    fn build_with_gcc(&self, source_files: &[PathBuf], output_path: &Path) -> CodegenResult<PathBuf> {
        let mut cmd = Command::new("gcc");
        
        // Add source files
        for source in source_files {
            cmd.arg(source);
        }
        
        // Output file
        cmd.arg("-o").arg(output_path);
        
        // Optimization level
        if self.config.optimization_level > 0 {
            cmd.arg(format!("-O{}", self.config.optimization_level));
        }
        
        // Debug information
        if self.config.debug_info {
            cmd.arg("-g");
        }
        
        // Sanitizers
        if self.config.address_sanitizer {
            cmd.arg("-fsanitize=address");
        }
        if self.config.undefined_sanitizer {
            cmd.arg("-fsanitize=undefined");
        }
        
        // Warning flags
        cmd.args(&["-Wall", "-Wextra", "-Werror"]);
        
        // C standard
        cmd.arg("-std=c11");
        
        // Include directories
        for include_dir in &self.config.include_dirs {
            cmd.arg("-I").arg(include_dir);
        }
        
        // Library directories
        for lib_dir in &self.config.library_dirs {
            cmd.arg("-L").arg(lib_dir);
        }
        
        // Libraries
        for lib in &self.config.libraries {
            cmd.arg("-l").arg(lib);
        }
        
        // Extra flags
        cmd.args(&self.config.extra_flags);
        
        // Linker flags
        for flag in &self.config.linker_flags {
            cmd.arg("-Wl,").arg(flag);
        }
        
        // Target architecture
        if let Some(arch) = &self.config.target_arch {
            cmd.arg("-march").arg(arch);
        }
        
        // Execute compilation
        self.execute_command(cmd, "GCC compilation failed")?;
        
        Ok(output_path.to_path_buf())
    }
    
    /// Build with Clang
    fn build_with_clang(&self, source_files: &[PathBuf], output_path: &Path) -> CodegenResult<PathBuf> {
        let mut cmd = Command::new("clang");
        
        // Add source files
        for source in source_files {
            cmd.arg(source);
        }
        
        // Output file
        cmd.arg("-o").arg(output_path);
        
        // Optimization level
        if self.config.optimization_level > 0 {
            cmd.arg(format!("-O{}", self.config.optimization_level));
        }
        
        // Debug information
        if self.config.debug_info {
            cmd.arg("-g");
        }
        
        // Sanitizers
        if self.config.address_sanitizer {
            cmd.arg("-fsanitize=address");
        }
        if self.config.undefined_sanitizer {
            cmd.arg("-fsanitize=undefined");
        }
        
        // Warning flags
        cmd.args(&["-Wall", "-Wextra", "-Werror"]);
        
        // C standard
        cmd.arg("-std=c11");
        
        // Include directories
        for include_dir in &self.config.include_dirs {
            cmd.arg("-I").arg(include_dir);
        }
        
        // Library directories
        for lib_dir in &self.config.library_dirs {
            cmd.arg("-L").arg(lib_dir);
        }
        
        // Libraries
        for lib in &self.config.libraries {
            cmd.arg("-l").arg(lib);
        }
        
        // Extra flags
        cmd.args(&self.config.extra_flags);
        
        // Linker flags
        for flag in &self.config.linker_flags {
            cmd.arg("-Wl,").arg(flag);
        }
        
        // Target architecture
        if let Some(arch) = &self.config.target_arch {
            cmd.arg("-march").arg(arch);
        }
        
        // Execute compilation
        self.execute_command(cmd, "Clang compilation failed")?;
        
        Ok(output_path.to_path_buf())
    }
    
    /// Build with MSVC
    fn build_with_msvc(&self, source_files: &[PathBuf], output_path: &Path) -> CodegenResult<PathBuf> {
        let mut cmd = Command::new("cl");
        
        // Add source files
        for source in source_files {
            cmd.arg(source);
        }
        
        // Output file
        cmd.arg("/Fe:").arg(output_path);
        
        // Optimization level
        match self.config.optimization_level {
            0 => cmd.arg("/Od"),  // Disable optimization
            1 => cmd.arg("/O1"),  // Minimize size
            2 => cmd.arg("/O2"),  // Maximize speed
            3 => cmd.arg("/Ox"),  // Maximum optimization
            _ => cmd.arg("/O2"),  // Default to O2
        };
        
        // Debug information
        if self.config.debug_info {
            cmd.arg("/Zi");
        }
        
        // Warning level
        cmd.arg("/W4");
        cmd.arg("/WX"); // Treat warnings as errors
        
        // Include directories
        for include_dir in &self.config.include_dirs {
            cmd.arg("/I").arg(include_dir);
        }
        
        // Library directories
        for lib_dir in &self.config.library_dirs {
            cmd.arg(format!("/LIBPATH:{}", lib_dir.display()));
        }
        
        // Libraries
        for lib in &self.config.libraries {
            cmd.arg(format!("{}.lib", lib));
        }
        
        // Extra flags
        cmd.args(&self.config.extra_flags);
        
        // Execute compilation
        self.execute_command(cmd, "MSVC compilation failed")?;
        
        Ok(output_path.to_path_buf())
    }
    
    /// Execute a command and handle errors
    fn execute_command(&self, mut cmd: Command, error_message: &str) -> CodegenResult<()> {
        // Enable verbose output in debug mode
        if self.config.debug_info {
            eprintln!("Executing: {:?}", cmd);
        }
        
        let output = cmd.output()
            .map_err(|e| CodegenError::InternalError(format!("Failed to execute compiler: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            let error_details = format!(
                "{}\nStdout: {}\nStderr: {}",
                error_message, stdout, stderr
            );
            
            return Err(CodegenError::InternalError(error_details));
        }
        
        Ok(())
    }
    
    /// Compile a single C file to object file
    pub fn compile_object(
        &self,
        source_file: &Path,
        object_file: &Path
    ) -> CodegenResult<()> {
        match self.config.compiler {
            CCompiler::Gcc => {
                let mut cmd = Command::new("gcc");
                cmd.arg("-c").arg(source_file).arg("-o").arg(object_file);
                self.add_common_gcc_flags(&mut cmd);
                self.execute_command(cmd, "GCC object compilation failed")
            },
            CCompiler::Clang => {
                let mut cmd = Command::new("clang");
                cmd.arg("-c").arg(source_file).arg("-o").arg(object_file);
                self.add_common_clang_flags(&mut cmd);
                self.execute_command(cmd, "Clang object compilation failed")
            },
            CCompiler::Msvc => {
                let mut cmd = Command::new("cl");
                cmd.arg("/c").arg(source_file).arg("/Fo:").arg(object_file);
                self.add_common_msvc_flags(&mut cmd);
                self.execute_command(cmd, "MSVC object compilation failed")
            },
        }
    }
    
    /// Add common GCC flags to command
    fn add_common_gcc_flags(&self, cmd: &mut Command) {
        if self.config.optimization_level > 0 {
            cmd.arg(format!("-O{}", self.config.optimization_level));
        }
        if self.config.debug_info {
            cmd.arg("-g");
        }
        cmd.args(&["-Wall", "-Wextra", "-std=c11"]);
        
        for include_dir in &self.config.include_dirs {
            cmd.arg("-I").arg(include_dir);
        }
        
        cmd.args(&self.config.extra_flags);
    }
    
    /// Add common Clang flags to command
    fn add_common_clang_flags(&self, cmd: &mut Command) {
        if self.config.optimization_level > 0 {
            cmd.arg(format!("-O{}", self.config.optimization_level));
        }
        if self.config.debug_info {
            cmd.arg("-g");
        }
        cmd.args(&["-Wall", "-Wextra", "-std=c11"]);
        
        for include_dir in &self.config.include_dirs {
            cmd.arg("-I").arg(include_dir);
        }
        
        cmd.args(&self.config.extra_flags);
    }
    
    /// Add common MSVC flags to command
    fn add_common_msvc_flags(&self, cmd: &mut Command) {
        match self.config.optimization_level {
            0 => cmd.arg("/Od"),
            1 => cmd.arg("/O1"),
            2 => cmd.arg("/O2"),
            3 => cmd.arg("/Ox"),
            _ => cmd.arg("/O2"),
        };
        
        if self.config.debug_info {
            cmd.arg("/Zi");
        }
        
        cmd.arg("/W4");
        
        for include_dir in &self.config.include_dirs {
            cmd.arg("/I").arg(include_dir);
        }
        
        cmd.args(&self.config.extra_flags);
    }
    
    /// Link object files into executable
    pub fn link_executable(
        &self,
        object_files: &[PathBuf],
        output_path: &Path
    ) -> CodegenResult<()> {
        match self.config.compiler {
            CCompiler::Gcc => {
                let mut cmd = Command::new("gcc");
                cmd.args(object_files).arg("-o").arg(output_path);
                
                for lib_dir in &self.config.library_dirs {
                    cmd.arg("-L").arg(lib_dir);
                }
                for lib in &self.config.libraries {
                    cmd.arg("-l").arg(lib);
                }
                for flag in &self.config.linker_flags {
                    cmd.arg("-Wl,").arg(flag);
                }
                
                self.execute_command(cmd, "GCC linking failed")
            },
            CCompiler::Clang => {
                let mut cmd = Command::new("clang");
                cmd.args(object_files).arg("-o").arg(output_path);
                
                for lib_dir in &self.config.library_dirs {
                    cmd.arg("-L").arg(lib_dir);
                }
                for lib in &self.config.libraries {
                    cmd.arg("-l").arg(lib);
                }
                for flag in &self.config.linker_flags {
                    cmd.arg("-Wl,").arg(flag);
                }
                
                self.execute_command(cmd, "Clang linking failed")
            },
            CCompiler::Msvc => {
                let mut cmd = Command::new("link");
                cmd.args(object_files).arg("/OUT:").arg(output_path);
                
                for lib_dir in &self.config.library_dirs {
                    cmd.arg(format!("/LIBPATH:{}", lib_dir.display()));
                }
                for lib in &self.config.libraries {
                    cmd.arg(format!("{}.lib", lib));
                }
                
                self.execute_command(cmd, "MSVC linking failed")
            },
        }
    }
    
    /// Create a static library from object files
    pub fn create_static_library(
        &self,
        object_files: &[PathBuf],
        library_path: &Path
    ) -> CodegenResult<()> {
        match self.config.compiler {
            CCompiler::Gcc | CCompiler::Clang => {
                let mut cmd = Command::new("ar");
                cmd.arg("rcs").arg(library_path).args(object_files);
                self.execute_command(cmd, "Static library creation failed")
            },
            CCompiler::Msvc => {
                let mut cmd = Command::new("lib");
                cmd.arg("/OUT:").arg(library_path).args(object_files);
                self.execute_command(cmd, "Static library creation failed")
            },
        }
    }
    
    /// Get configuration
    pub fn config(&self) -> &BuildConfig {
        &self.config
    }
    
    /// Set configuration
    pub fn set_config(&mut self, config: BuildConfig) {
        self.config = config;
    }
}

/// Builder for BuildConfig
pub struct BuildConfigBuilder {
    config: BuildConfig,
}

impl BuildConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: BuildConfig::default(),
        }
    }
    
    /// Set compiler
    pub fn compiler(mut self, compiler: CCompiler) -> Self {
        self.config.compiler = compiler;
        self
    }
    
    /// Set optimization level
    pub fn optimization_level(mut self, level: u8) -> Self {
        self.config.optimization_level = level.min(3);
        self
    }
    
    /// Enable debug information
    pub fn debug_info(mut self, enable: bool) -> Self {
        self.config.debug_info = enable;
        self
    }
    
    /// Enable address sanitizer
    pub fn address_sanitizer(mut self, enable: bool) -> Self {
        self.config.address_sanitizer = enable;
        self
    }
    
    /// Add extra compiler flag
    pub fn extra_flag(mut self, flag: String) -> Self {
        self.config.extra_flags.push(flag);
        self
    }
    
    /// Add include directory
    pub fn include_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.include_dirs.push(path.into());
        self
    }
    
    /// Add library
    pub fn library(mut self, lib: String) -> Self {
        self.config.libraries.push(lib);
        self
    }
    
    /// Set output directory
    pub fn output_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.output_dir = path.into();
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> BuildConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_build_config_builder() {
        let config = BuildConfigBuilder::new()
            .compiler(CCompiler::Clang)
            .optimization_level(2)
            .debug_info(false)
            .extra_flag("-ffast-math".to_string())
            .library("m".to_string())
            .build();
        
        assert_eq!(config.compiler, CCompiler::Clang);
        assert_eq!(config.optimization_level, 2);
        assert_eq!(config.debug_info, false);
        assert!(config.extra_flags.contains(&"-ffast-math".to_string()));
        assert!(config.libraries.contains(&"m".to_string()));
    }
    
    #[test]
    fn test_compiler_detection() {
        // This test might fail on systems without compilers
        // but it's useful for development
        match BuildSystem::detect_compiler() {
            Ok(compiler) => {
                println!("Detected compiler: {:?}", compiler);
            },
            Err(e) => {
                println!("No compiler detected: {}", e);
            }
        }
    }
    
    #[test]
    fn test_command_exists() {
        // Test with a command that should exist on most systems
        #[cfg(unix)]
        assert!(BuildSystem::command_exists("ls"));
        
        #[cfg(windows)]
        assert!(BuildSystem::command_exists("dir"));
        
        // Test with a command that shouldn't exist
        assert!(!BuildSystem::command_exists("nonexistent_command_12345"));
    }
    
    #[test]
    fn test_build_system_creation() {
        let build_system = BuildSystem::new();
        assert_eq!(build_system.config.compiler, CCompiler::Gcc);
        assert_eq!(build_system.config.optimization_level, 0);
        assert!(build_system.config.debug_info);
    }
} 