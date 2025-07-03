//! Runtime System Generation for Prism
//!
//! This module generates the C runtime system that supports Prism programs.
//! It includes memory management, string handling, error handling, and core data structures.

use super::{CCodeBuilder, CodegenResult, CodegenError};
use std::fs;
use std::path::Path;

/// Runtime system generator
pub struct RuntimeGenerator {
    /// Code builder for runtime
    builder: CCodeBuilder,
}

impl RuntimeGenerator {
    /// Create a new runtime generator
    pub fn new() -> Self {
        Self {
            builder: CCodeBuilder::with_capacity(16384), // 16KB for runtime
        }
    }
    
    /// Generate the complete runtime system
    pub fn generate_runtime(&mut self) -> CodegenResult<(String, String)> {
        // Generate header
        self.generate_runtime_header()?;
        
        // Generate implementation
        self.generate_runtime_implementation()?;
        
        Ok(self.builder.build())
    }
    
    /// Generate runtime header
    fn generate_runtime_header(&mut self) -> CodegenResult<()> {
        self.builder.header_context();
        
        // Header guard
        self.builder.line("#ifndef PRISM_RUNTIME_H");
        self.builder.line("#define PRISM_RUNTIME_H");
        self.builder.newline();
        
        // Standard includes
        self.builder.line("#include <stdint.h>");
        self.builder.line("#include <stdbool.h>");
        self.builder.line("#include <stddef.h>");
        self.builder.line("#include <stdlib.h>");
        self.builder.line("#include <string.h>");
        self.builder.line("#include <stdio.h>");
        self.builder.line("#include <assert.h>");
        self.builder.newline();
        
        // Platform-specific includes
        self.builder.line("#ifdef __cplusplus");
        self.builder.line("extern \"C\" {");
        self.builder.line("#endif");
        self.builder.newline();
        
        // Core type definitions
        self.generate_core_types()?;
        
        // Memory management
        self.generate_memory_management_header()?;
        
        // String handling
        self.generate_string_handling_header()?;
        
        // Array handling
        self.generate_array_handling_header()?;
        
        // Error handling
        self.generate_error_handling_header()?;
        
        // I/O operations
        self.generate_io_operations_header()?;
        
        // Utility functions
        self.generate_utility_functions_header()?;
        
        // Footer
        self.builder.line("#ifdef __cplusplus");
        self.builder.line("}");
        self.builder.line("#endif");
        self.builder.newline();
        self.builder.line("#endif // PRISM_RUNTIME_H");
        
        Ok(())
    }
    
    /// Generate core type definitions
    fn generate_core_types(&mut self) -> CodegenResult<()> {
        self.builder.comment("Core Prism types");
        
        // String type
        self.builder.line("typedef struct {");
        self.builder.indent_inc();
        self.builder.line("char* data;");
        self.builder.line("size_t length;");
        self.builder.line("size_t capacity;");
        self.builder.line("uint32_t ref_count;");
        self.builder.indent_dec();
        self.builder.line("} prism_str_t;");
        self.builder.newline();
        
        // Array type
        self.builder.line("typedef struct {");
        self.builder.indent_inc();
        self.builder.line("void* data;");
        self.builder.line("size_t length;");
        self.builder.line("size_t capacity;");
        self.builder.line("size_t element_size;");
        self.builder.line("uint32_t ref_count;");
        self.builder.indent_dec();
        self.builder.line("} prism_array_t;");
        self.builder.newline();
        
        // Range type
        self.builder.line("typedef struct {");
        self.builder.indent_inc();
        self.builder.line("int64_t start;");
        self.builder.line("int64_t end;");
        self.builder.line("bool inclusive;");
        self.builder.indent_dec();
        self.builder.line("} prism_range_t;");
        self.builder.newline();
        
        // Optional type
        self.builder.line("typedef struct {");
        self.builder.indent_inc();
        self.builder.line("void* data;");
        self.builder.line("bool has_value;");
        self.builder.indent_dec();
        self.builder.line("} prism_optional_t;");
        self.builder.newline();
        
        // Result type
        self.builder.line("typedef struct {");
        self.builder.indent_inc();
        self.builder.line("void* data;");
        self.builder.line("void* error;");
        self.builder.line("bool is_ok;");
        self.builder.indent_dec();
        self.builder.line("} prism_result_t;");
        self.builder.newline();
        
        Ok(())
    }
    
    /// Generate memory management header
    fn generate_memory_management_header(&mut self) -> CodegenResult<()> {
        self.builder.comment("Memory Management");
        
        // Allocation functions
        self.builder.line("void* prism_alloc(size_t size);");
        self.builder.line("void* prism_realloc(void* ptr, size_t new_size);");
        self.builder.line("void prism_free(void* ptr);");
        self.builder.line("void* prism_calloc(size_t count, size_t size);");
        self.builder.newline();
        
        // Reference counting
        self.builder.line("void prism_ref_inc(void* ptr);");
        self.builder.line("void prism_ref_dec(void* ptr);");
        self.builder.line("uint32_t prism_ref_count(void* ptr);");
        self.builder.newline();
        
        // Memory debugging
        self.builder.line("#ifdef PRISM_DEBUG");
        self.builder.line("void prism_memory_report(void);");
        self.builder.line("size_t prism_memory_usage(void);");
        self.builder.line("#endif");
        self.builder.newline();
        
        Ok(())
    }
    
    /// Generate string handling header
    fn generate_string_handling_header(&mut self) -> CodegenResult<()> {
        self.builder.comment("String Handling");
        
        // String creation
        self.builder.line("prism_str_t prism_str_new(const char* cstr);");
        self.builder.line("prism_str_t prism_str_from_bytes(const char* bytes, size_t length);");
        self.builder.line("prism_str_t prism_str_with_capacity(size_t capacity);");
        self.builder.line("prism_str_t prism_str_clone(const prism_str_t* str);");
        self.builder.newline();
        
        // String operations
        self.builder.line("void prism_str_push(prism_str_t* str, char ch);");
        self.builder.line("void prism_str_push_str(prism_str_t* str, const prism_str_t* other);");
        self.builder.line("void prism_str_push_cstr(prism_str_t* str, const char* cstr);");
        self.builder.line("prism_str_t prism_str_concat(const prism_str_t* a, const prism_str_t* b);");
        self.builder.newline();
        
        // String comparison
        self.builder.line("bool prism_str_eq(const prism_str_t* a, const prism_str_t* b);");
        self.builder.line("bool prism_str_eq_cstr(const prism_str_t* str, const char* cstr);");
        self.builder.line("int prism_str_cmp(const prism_str_t* a, const prism_str_t* b);");
        self.builder.newline();
        
        // String utilities
        self.builder.line("size_t prism_str_len(const prism_str_t* str);");
        self.builder.line("bool prism_str_is_empty(const prism_str_t* str);");
        self.builder.line("const char* prism_str_as_cstr(const prism_str_t* str);");
        self.builder.line("void prism_str_clear(prism_str_t* str);");
        self.builder.line("void prism_str_free(prism_str_t* str);");
        self.builder.newline();
        
        Ok(())
    }
    
    /// Generate array handling header
    fn generate_array_handling_header(&mut self) -> CodegenResult<()> {
        self.builder.comment("Array Handling");
        
        // Array creation
        self.builder.line("prism_array_t prism_array_new(size_t element_size);");
        self.builder.line("prism_array_t prism_array_with_capacity(size_t element_size, size_t capacity);");
        self.builder.line("prism_array_t prism_array_clone(const prism_array_t* arr);");
        self.builder.newline();
        
        // Array operations
        self.builder.line("void prism_array_push(prism_array_t* arr, const void* element);");
        self.builder.line("bool prism_array_pop(prism_array_t* arr, void* element);");
        self.builder.line("void* prism_array_get(const prism_array_t* arr, size_t index);");
        self.builder.line("bool prism_array_set(prism_array_t* arr, size_t index, const void* element);");
        self.builder.newline();
        
        // Array utilities
        self.builder.line("size_t prism_array_len(const prism_array_t* arr);");
        self.builder.line("bool prism_array_is_empty(const prism_array_t* arr);");
        self.builder.line("void prism_array_clear(prism_array_t* arr);");
        self.builder.line("void prism_array_free(prism_array_t* arr);");
        self.builder.newline();
        
        Ok(())
    }
    
    /// Generate error handling header
    fn generate_error_handling_header(&mut self) -> CodegenResult<()> {
        self.builder.comment("Error Handling");
        
        // Panic mechanism
        self.builder.line("void prism_panic(const char* message);");
        self.builder.line("void prism_panic_fmt(const char* format, ...);");
        self.builder.line("void prism_assert(bool condition, const char* message);");
        self.builder.newline();
        
        // Error types
        self.builder.line("typedef enum {");
        self.builder.indent_inc();
        self.builder.line("PRISM_ERROR_NONE,");
        self.builder.line("PRISM_ERROR_OUT_OF_MEMORY,");
        self.builder.line("PRISM_ERROR_INDEX_OUT_OF_BOUNDS,");
        self.builder.line("PRISM_ERROR_NULL_POINTER,");
        self.builder.line("PRISM_ERROR_INVALID_ARGUMENT,");
        self.builder.line("PRISM_ERROR_IO,");
        self.builder.line("PRISM_ERROR_CUSTOM");
        self.builder.indent_dec();
        self.builder.line("} prism_error_code_t;");
        self.builder.newline();
        
        // Error handling functions
        self.builder.line("void prism_set_error(prism_error_code_t code, const char* message);");
        self.builder.line("prism_error_code_t prism_get_error(void);");
        self.builder.line("const char* prism_get_error_message(void);");
        self.builder.line("void prism_clear_error(void);");
        self.builder.newline();
        
        Ok(())
    }
    
    /// Generate I/O operations header
    fn generate_io_operations_header(&mut self) -> CodegenResult<()> {
        self.builder.comment("I/O Operations");
        
        // Print functions
        self.builder.line("void prism_print_str(const prism_str_t* str);");
        self.builder.line("void prism_print_cstr(const char* cstr);");
        self.builder.line("void prism_print_int(int64_t value);");
        self.builder.line("void prism_print_uint(uint64_t value);");
        self.builder.line("void prism_print_float(double value);");
        self.builder.line("void prism_print_bool(bool value);");
        self.builder.line("void prism_println(void);");
        self.builder.newline();
        
        // Input functions
        self.builder.line("prism_str_t prism_read_line(void);");
        self.builder.line("bool prism_read_int(int64_t* value);");
        self.builder.line("bool prism_read_float(double* value);");
        self.builder.newline();
        
        Ok(())
    }
    
    /// Generate utility functions header
    fn generate_utility_functions_header(&mut self) -> CodegenResult<()> {
        self.builder.comment("Utility Functions");
        
        // Range functions
        self.builder.line("prism_range_t prism_range(int64_t start, int64_t end, bool inclusive);");
        self.builder.line("bool prism_range_contains(const prism_range_t* range, int64_t value);");
        self.builder.line("int64_t prism_range_len(const prism_range_t* range);");
        self.builder.newline();
        
        // Optional functions
        self.builder.line("prism_optional_t prism_some(void* value);");
        self.builder.line("prism_optional_t prism_none(void);");
        self.builder.line("bool prism_optional_is_some(const prism_optional_t* opt);");
        self.builder.line("bool prism_optional_is_none(const prism_optional_t* opt);");
        self.builder.line("void* prism_optional_unwrap(const prism_optional_t* opt);");
        self.builder.newline();
        
        // Result functions
        self.builder.line("prism_result_t prism_ok(void* value);");
        self.builder.line("prism_result_t prism_err(void* error);");
        self.builder.line("bool prism_result_is_ok(const prism_result_t* result);");
        self.builder.line("bool prism_result_is_err(const prism_result_t* result);");
        self.builder.line("void* prism_result_unwrap(const prism_result_t* result);");
        self.builder.newline();
        
        Ok(())
    }
    
    /// Generate runtime implementation
    fn generate_runtime_implementation(&mut self) -> CodegenResult<()> {
        self.builder.code_context();
        
        // Implementation includes
        self.builder.line("#include \"prism_runtime.h\"");
        self.builder.line("#include <stdarg.h>");
        self.builder.line("#include <stdio.h>");
        self.builder.line("#include <stdlib.h>");
        self.builder.line("#include <string.h>");
        self.builder.line("#include <assert.h>");
        self.builder.newline();
        
        // Static variables
        self.builder.comment("Static variables");
        self.builder.line("static prism_error_code_t g_error_code = PRISM_ERROR_NONE;");
        self.builder.line("static char g_error_message[256] = {0};");
        self.builder.newline();
        
        // Memory management implementation
        self.generate_memory_management_impl()?;
        
        // String handling implementation
        self.generate_string_handling_impl()?;
        
        // Array handling implementation
        self.generate_array_handling_impl()?;
        
        // Error handling implementation
        self.generate_error_handling_impl()?;
        
        // I/O operations implementation
        self.generate_io_operations_impl()?;
        
        // Utility functions implementation
        self.generate_utility_functions_impl()?;
        
        Ok(())
    }
    
    /// Generate memory management implementation
    fn generate_memory_management_impl(&mut self) -> CodegenResult<()> {
        self.builder.comment("Memory Management Implementation");
        
        // Basic allocation
        self.builder.function("void* prism_alloc(size_t size)", |b| {
            b.line("void* ptr = malloc(size);");
            b.line("if (!ptr && size > 0) {");
            b.indent_inc();
            b.line("prism_panic(\"Out of memory\");");
            b.indent_dec();
            b.line("}");
            b.line("return ptr;");
        });
        
        self.builder.function("void* prism_realloc(void* ptr, size_t new_size)", |b| {
            b.line("void* new_ptr = realloc(ptr, new_size);");
            b.line("if (!new_ptr && new_size > 0) {");
            b.indent_inc();
            b.line("prism_panic(\"Out of memory\");");
            b.indent_dec();
            b.line("}");
            b.line("return new_ptr;");
        });
        
        self.builder.function("void prism_free(void* ptr)", |b| {
            b.line("if (ptr) {");
            b.indent_inc();
            b.line("free(ptr);");
            b.indent_dec();
            b.line("}");
        });
        
        self.builder.function("void* prism_calloc(size_t count, size_t size)", |b| {
            b.line("void* ptr = calloc(count, size);");
            b.line("if (!ptr && count > 0 && size > 0) {");
            b.indent_inc();
            b.line("prism_panic(\"Out of memory\");");
            b.indent_dec();
            b.line("}");
            b.line("return ptr;");
        });
        
        Ok(())
    }
    
    /// Generate string handling implementation
    fn generate_string_handling_impl(&mut self) -> CodegenResult<()> {
        self.builder.comment("String Handling Implementation");
        
        // String creation
        self.builder.function("prism_str_t prism_str_new(const char* cstr)", |b| {
            b.line("prism_str_t str = {0};");
            b.line("if (cstr) {");
            b.indent_inc();
            b.line("str.length = strlen(cstr);");
            b.line("str.capacity = str.length + 1;");
            b.line("str.data = prism_alloc(str.capacity);");
            b.line("memcpy(str.data, cstr, str.length);");
            b.line("str.data[str.length] = '\\0';");
            b.line("str.ref_count = 1;");
            b.indent_dec();
            b.line("}");
            b.line("return str;");
        });
        
        // String comparison
        self.builder.function("bool prism_str_eq(const prism_str_t* a, const prism_str_t* b)", |b| {
            b.line("if (!a || !b) return false;");
            b.line("if (a->length != b->length) return false;");
            b.line("return memcmp(a->data, b->data, a->length) == 0;");
        });
        
        // String cleanup
        self.builder.function("void prism_str_free(prism_str_t* str)", |b| {
            b.line("if (str && str->data) {");
            b.indent_inc();
            b.line("prism_free(str->data);");
            b.line("str->data = NULL;");
            b.line("str->length = 0;");
            b.line("str->capacity = 0;");
            b.line("str->ref_count = 0;");
            b.indent_dec();
            b.line("}");
        });
        
        Ok(())
    }
    
    /// Generate array handling implementation
    fn generate_array_handling_impl(&mut self) -> CodegenResult<()> {
        self.builder.comment("Array Handling Implementation");
        
        // Array creation
        self.builder.function("prism_array_t prism_array_new(size_t element_size)", |b| {
            b.line("prism_array_t arr = {0};");
            b.line("arr.element_size = element_size;");
            b.line("arr.capacity = 4; // Start with small capacity");
            b.line("arr.data = prism_alloc(arr.capacity * element_size);");
            b.line("arr.ref_count = 1;");
            b.line("return arr;");
        });
        
        // Array cleanup
        self.builder.function("void prism_array_free(prism_array_t* arr)", |b| {
            b.line("if (arr && arr->data) {");
            b.indent_inc();
            b.line("prism_free(arr->data);");
            b.line("arr->data = NULL;");
            b.line("arr->length = 0;");
            b.line("arr->capacity = 0;");
            b.line("arr->ref_count = 0;");
            b.indent_dec();
            b.line("}");
        });
        
        Ok(())
    }
    
    /// Generate error handling implementation
    fn generate_error_handling_impl(&mut self) -> CodegenResult<()> {
        self.builder.comment("Error Handling Implementation");
        
        // Panic function
        self.builder.function("void prism_panic(const char* message)", |b| {
            b.line("fprintf(stderr, \"Prism panic: %s\\n\", message ? message : \"unknown error\");");
            b.line("abort();");
        });
        
        // Error state management
        self.builder.function("void prism_set_error(prism_error_code_t code, const char* message)", |b| {
            b.line("g_error_code = code;");
            b.line("if (message) {");
            b.indent_inc();
            b.line("strncpy(g_error_message, message, sizeof(g_error_message) - 1);");
            b.line("g_error_message[sizeof(g_error_message) - 1] = '\\0';");
            b.indent_dec();
            b.line("} else {");
            b.indent_inc();
            b.line("g_error_message[0] = '\\0';");
            b.indent_dec();
            b.line("}");
        });
        
        self.builder.function("prism_error_code_t prism_get_error(void)", |b| {
            b.line("return g_error_code;");
        });
        
        self.builder.function("const char* prism_get_error_message(void)", |b| {
            b.line("return g_error_message;");
        });
        
        Ok(())
    }
    
    /// Generate I/O operations implementation
    fn generate_io_operations_impl(&mut self) -> CodegenResult<()> {
        self.builder.comment("I/O Operations Implementation");
        
        // Print functions
        self.builder.function("void prism_print_str(const prism_str_t* str)", |b| {
            b.line("if (str && str->data) {");
            b.indent_inc();
            b.line("printf(\"%.*s\", (int)str->length, str->data);");
            b.indent_dec();
            b.line("}");
        });
        
        self.builder.function("void prism_print_cstr(const char* cstr)", |b| {
            b.line("if (cstr) {");
            b.indent_inc();
            b.line("printf(\"%s\", cstr);");
            b.indent_dec();
            b.line("}");
        });
        
        self.builder.function("void prism_print_int(int64_t value)", |b| {
            b.line("printf(\"%lld\", (long long)value);");
        });
        
        self.builder.function("void prism_println(void)", |b| {
            b.line("printf(\"\\n\");");
        });
        
        Ok(())
    }
    
    /// Generate utility functions implementation
    fn generate_utility_functions_impl(&mut self) -> CodegenResult<()> {
        self.builder.comment("Utility Functions Implementation");
        
        // Range functions
        self.builder.function("prism_range_t prism_range(int64_t start, int64_t end, bool inclusive)", |b| {
            b.line("prism_range_t range = { start, end, inclusive };");
            b.line("return range;");
        });
        
        // Optional functions
        self.builder.function("prism_optional_t prism_some(void* value)", |b| {
            b.line("prism_optional_t opt = { value, true };");
            b.line("return opt;");
        });
        
        self.builder.function("prism_optional_t prism_none(void)", |b| {
            b.line("prism_optional_t opt = { NULL, false };");
            b.line("return opt;");
        });
        
        // Result functions
        self.builder.function("prism_result_t prism_ok(void* value)", |b| {
            b.line("prism_result_t result = { value, NULL, true };");
            b.line("return result;");
        });
        
        self.builder.function("prism_result_t prism_err(void* error)", |b| {
            b.line("prism_result_t result = { NULL, error, false };");
            b.line("return result;");
        });
        
        Ok(())
    }
    
    /// Write runtime files to disk
    pub fn write_runtime_files(&mut self, output_dir: &Path) -> CodegenResult<()> {
        let (header, implementation) = self.generate_runtime()?;
        
        // Write header file
        let header_path = output_dir.join("prism_runtime.h");
        fs::write(&header_path, header)
            .map_err(|e| CodegenError::IoError(format!("Failed to write runtime header: {}", e)))?;
        
        // Write implementation file
        let impl_path = output_dir.join("prism_runtime.c");
        fs::write(&impl_path, implementation)
            .map_err(|e| CodegenError::IoError(format!("Failed to write runtime implementation: {}", e)))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    #[test]
    fn test_runtime_generation() {
        let mut generator = RuntimeGenerator::new();
        let result = generator.generate_runtime();
        
        assert!(result.is_ok());
        let (header, implementation) = result.unwrap();
        
        // Check header contains expected content
        assert!(header.contains("#ifndef PRISM_RUNTIME_H"));
        assert!(header.contains("prism_str_t"));
        assert!(header.contains("prism_array_t"));
        assert!(header.contains("void prism_panic(const char* message);"));
        
        // Check implementation contains expected content
        assert!(implementation.contains("#include \"prism_runtime.h\""));
        assert!(implementation.contains("void* prism_alloc(size_t size)"));
        assert!(implementation.contains("void prism_panic(const char* message)"));
    }
    
    #[test]
    fn test_core_types() {
        let mut generator = RuntimeGenerator::new();
        generator.builder.header_context();
        
        assert!(generator.generate_core_types().is_ok());
        
        let header = generator.builder.header();
        assert!(header.contains("prism_str_t"));
        assert!(header.contains("prism_array_t"));
        assert!(header.contains("prism_range_t"));
        assert!(header.contains("prism_optional_t"));
        assert!(header.contains("prism_result_t"));
    }
    
    #[test]
    fn test_memory_management() {
        let mut generator = RuntimeGenerator::new();
        generator.builder.header_context();
        
        assert!(generator.generate_memory_management_header().is_ok());
        
        let header = generator.builder.header();
        assert!(header.contains("void* prism_alloc(size_t size);"));
        assert!(header.contains("void prism_free(void* ptr);"));
        assert!(header.contains("void* prism_realloc(void* ptr, size_t new_size);"));
    }
} 