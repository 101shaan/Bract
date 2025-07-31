#ifndef PRISM_RUNTIME_H
#define PRISM_RUNTIME_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <assert.h>

#ifdef __cplusplus
extern "C" {
#endif

// Core Prism types
typedef struct {
    char* data;
    size_t length;
    size_t capacity;
    uint32_t ref_count;
} prism_str_t;

typedef struct {
    void* data;
    size_t length;
    size_t capacity;
    size_t element_size;
    uint32_t ref_count;
} prism_array_t;

typedef struct {
    int64_t start;
    int64_t end;
    bool inclusive;
} prism_range_t;

typedef struct {
    void* data;
    bool has_value;
} prism_optional_t;

typedef struct {
    void* data;
    void* error;
    bool is_ok;
} prism_result_t;

// Memory Management
void* prism_alloc(size_t size);
void* prism_realloc(void* ptr, size_t new_size);
void prism_free(void* ptr);
void* prism_calloc(size_t count, size_t size);

void prism_ref_inc(void* ptr);
void prism_ref_dec(void* ptr);
uint32_t prism_ref_count(void* ptr);

#ifdef PRISM_DEBUG
void prism_memory_report(void);
size_t prism_memory_usage(void);
#endif

// String Handling
prism_str_t prism_str_new(const char* cstr);
prism_str_t prism_str_from_bytes(const char* bytes, size_t length);
prism_str_t prism_str_with_capacity(size_t capacity);
prism_str_t prism_str_clone(const prism_str_t* str);

void prism_str_push(prism_str_t* str, char ch);
void prism_str_push_str(prism_str_t* str, const prism_str_t* other);
void prism_str_push_cstr(prism_str_t* str, const char* cstr);
prism_str_t prism_str_concat(const prism_str_t* a, const prism_str_t* b);

bool prism_str_eq(const prism_str_t* a, const prism_str_t* b);
bool prism_str_eq_cstr(const prism_str_t* str, const char* cstr);
int prism_str_cmp(const prism_str_t* a, const prism_str_t* b);

size_t prism_str_len(const prism_str_t* str);
bool prism_str_is_empty(const prism_str_t* str);
const char* prism_str_as_cstr(const prism_str_t* str);
void prism_str_clear(prism_str_t* str);
void prism_str_free(prism_str_t* str);

// Array Handling
prism_array_t prism_array_new(size_t element_size);
prism_array_t prism_array_with_capacity(size_t element_size, size_t capacity);
prism_array_t prism_array_clone(const prism_array_t* arr);

void prism_array_push(prism_array_t* arr, const void* element);
bool prism_array_pop(prism_array_t* arr, void* element);
void* prism_array_get(const prism_array_t* arr, size_t index);
bool prism_array_set(prism_array_t* arr, size_t index, const void* element);

size_t prism_array_len(const prism_array_t* arr);
bool prism_array_is_empty(const prism_array_t* arr);
void prism_array_clear(prism_array_t* arr);
void prism_array_free(prism_array_t* arr);

// Error Handling
void prism_panic(const char* message);
void prism_panic_fmt(const char* format, ...);
void prism_assert(bool condition, const char* message);

typedef enum {
    PRISM_ERROR_NONE,
    PRISM_ERROR_OUT_OF_MEMORY,
    PRISM_ERROR_INDEX_OUT_OF_BOUNDS,
    PRISM_ERROR_NULL_POINTER,
    PRISM_ERROR_INVALID_ARGUMENT,
    PRISM_ERROR_IO,
    PRISM_ERROR_CUSTOM
} prism_error_code_t;

void prism_set_error(prism_error_code_t code, const char* message);
prism_error_code_t prism_get_error(void);
const char* prism_get_error_message(void);
void prism_clear_error(void);

// I/O Operations
void prism_print_str(const prism_str_t* str);
void prism_print_cstr(const char* cstr);
void prism_print_int(int64_t value);
void prism_print_uint(uint64_t value);
void prism_print_float(double value);
void prism_print_bool(bool value);
void prism_println(void);

prism_str_t prism_read_line(void);
bool prism_read_int(int64_t* value);
bool prism_read_float(double* value);

// Utility Functions
prism_range_t prism_range(int64_t start, int64_t end, bool inclusive);
bool prism_range_contains(const prism_range_t* range, int64_t value);
int64_t prism_range_len(const prism_range_t* range);

prism_optional_t prism_some(void* value);
prism_optional_t prism_none(void);
bool prism_optional_is_some(const prism_optional_t* opt);
bool prism_optional_is_none(const prism_optional_t* opt);
void* prism_optional_unwrap(const prism_optional_t* opt);

prism_result_t prism_ok(void* value);
prism_result_t prism_err(void* error);
bool prism_result_is_ok(const prism_result_t* result);
bool prism_result_is_err(const prism_result_t* result);
void* prism_result_unwrap(const prism_result_t* result);

int64_t prism_abs_int(int64_t value);
double prism_abs_float(double value);
int64_t prism_min_int(int64_t a, int64_t b);
int64_t prism_max_int(int64_t a, int64_t b);
double prism_min_float(double a, double b);
double prism_max_float(double a, double b);
double prism_pow(double base, double exponent);
double prism_sqrt(double value);
double prism_sin(double radians);
double prism_cos(double radians);
double prism_tan(double radians);

#ifdef __cplusplus
}
#endif

#endif // PRISM_RUNTIME_H
